use anyhow::{Context, Result};
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::api::ApiClient;
use crate::api::types::{CrackResult, TaskDescriptor, TaskProgress, TaskReport, TaskStatus};
use crate::config::AgentConfig;
use crate::hashcat::Session;
use crate::hashcat::exit_code::{ExitCategory, classify_exit_code};
use crate::hashcat::session::SessionEvent;

use super::downloader::download_file;

/// Execute a single task: download resources, run hashcat, report results.
pub async fn execute(client: &ApiClient, config: &AgentConfig, task: TaskDescriptor) -> Result<()> {
    let task_id = task.id;

    // Report that we're running
    report_status(client, task_id, TaskStatus::Running, None, None).await?;

    // Download resources
    let http = reqwest::Client::new();
    let task_dir = config.data_dir.join(format!("task-{task_id}"));

    if let Some(ref resources) = task.resources {
        if let Some(ref url) = resources.hash_list_url {
            download_file(&http, url, &task_dir)
                .await
                .context("failed to download hash list")?;
        }
        if let Some(ref url) = resources.wordlist_url {
            download_file(&http, url, &task_dir)
                .await
                .context("failed to download wordlist")?;
        }
        if let Some(ref url) = resources.rulelist_url {
            download_file(&http, url, &task_dir)
                .await
                .context("failed to download rulelist")?;
        }
        if let Some(ref url) = resources.masklist_url {
            download_file(&http, url, &task_dir)
                .await
                .context("failed to download masklist")?;
        }
    }

    // Build hashcat arguments
    let hashcat_path = config
        .hashcat_path
        .as_ref()
        .map_or_else(|| "hashcat".into(), std::borrow::ToOwned::to_owned);

    let args = build_hashcat_args(&task, &task_dir);
    let session = Session::new(hashcat_path, task_id, args);

    // Run hashcat
    let cancel = CancellationToken::new();
    let (mut events, mut child) = session.start(cancel.clone()).await?;

    let cracked: Vec<CrackResult> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    while let Some(event) = events.recv().await {
        match event {
            SessionEvent::Status(status) => {
                let progress = Some(parse_status_progress(&status));
                if let Err(e) =
                    report_status(client, task_id, TaskStatus::Running, progress, None).await
                {
                    warn!(error = %e, task_id, "failed to report task progress");
                }
            }
            SessionEvent::Message(msg) => {
                info!(
                    category = ?msg.category,
                    severity = ?msg.severity,
                    "hashcat message"
                );
                if matches!(msg.severity, crate::hashcat::error_parser::Severity::Error) {
                    errors.push(format!("{:?}: {:?}", msg.category, msg.context));
                }
            }
            SessionEvent::Exit { code } => {
                let exit_info = classify_exit_code(code);
                info!(
                    code = exit_info.raw_code,
                    category = ?exit_info.category,
                    description = exit_info.description,
                    "hashcat exited"
                );

                let final_status = match exit_info.category {
                    ExitCategory::Success => TaskStatus::Completed,
                    ExitCategory::Exhausted => TaskStatus::Exhausted,
                    ExitCategory::Aborted
                    | ExitCategory::RuntimeError
                    | ExitCategory::GpuError
                    | ExitCategory::InternalError
                    | ExitCategory::Unknown => TaskStatus::Failed,
                };

                let _results = if cracked.is_empty() {
                    None
                } else {
                    Some(cracked.clone())
                };
                let err_list = if errors.is_empty() {
                    None
                } else {
                    Some(errors.clone())
                };

                report_status(client, task_id, final_status, None, err_list).await?;
                return Ok(());
            }
        }
    }

    // If we get here without an Exit event, the process was likely killed
    let status = child.wait().await.context("failed to wait on hashcat")?;
    let code = status.code().unwrap_or(-1);
    let exit_info = classify_exit_code(code);

    warn!(
        code = exit_info.raw_code,
        description = exit_info.description,
        "hashcat process ended without exit event"
    );

    report_status(client, task_id, TaskStatus::Failed, None, None).await?;
    Ok(())
}

#[allow(clippy::arithmetic_side_effects)]
fn build_hashcat_args(task: &TaskDescriptor, task_dir: &std::path::Path) -> Vec<String> {
    let mut args = vec![
        "-m".to_owned(),
        task.hash_type_id.to_string(),
        "-a".to_owned(),
        task.mode.to_string(),
        "--potfile-disable".to_owned(),
        "-o".to_owned(),
        task_dir.join("cracked.txt").to_string_lossy().to_string(),
    ];

    if let Some(ref range) = task.work_range {
        args.push("--skip".to_owned());
        args.push(range.start.to_string());
        args.push("--limit".to_owned());
        args.push((range.end - range.start).to_string());
    }

    // Add hash list path (first positional arg for hashcat)
    args.push(task_dir.join("hashes").to_string_lossy().to_string());

    args
}

fn parse_status_progress(status: &serde_json::Value) -> TaskProgress {
    TaskProgress {
        keyspace_progress: status
            .get("progress")
            .and_then(|p| p.as_array())
            .and_then(|arr| {
                let done = arr.first()?.as_f64()?;
                let total = arr.get(1)?.as_f64()?;
                if total > 0.0 {
                    Some(done / total * 100.0)
                } else {
                    Some(0.0)
                }
            })
            .unwrap_or(0.0),
        speed: status
            .get("devices_status")
            .and_then(|d| d.as_array())
            .map_or(0.0, |devices| {
                devices
                    .iter()
                    .filter_map(|d| d.get("speed").and_then(serde_json::Value::as_f64))
                    .sum()
            }),
        temperature: None,
    }
}

async fn report_status(
    client: &ApiClient,
    task_id: i64,
    status: TaskStatus,
    progress: Option<TaskProgress>,
    errors: Option<Vec<String>>,
) -> Result<()> {
    let report = TaskReport {
        status,
        progress,
        results: None,
        errors,
    };

    client
        .report_task_progress(task_id, &report)
        .await
        .context("failed to report task status")?;

    Ok(())
}
