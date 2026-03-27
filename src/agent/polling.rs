use std::time::Duration;

use anyhow::Result;
use tokio::select;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use crate::api::ApiClient;
use crate::api::types::TaskDescriptor;

/// Outcome of a single poll cycle.
pub enum PollResult {
    /// A task was assigned to this agent.
    Task(TaskDescriptor),
    /// No tasks are available right now.
    Idle,
    /// The poll loop was cancelled.
    Cancelled,
}

/// Poll the server for available tasks.
///
/// Returns immediately with a [`PollResult::Task`] when work is available,
/// waits `interval` between idle polls, and stops on cancellation.
pub async fn poll_for_task(
    client: &ApiClient,
    interval: Duration,
    cancel: CancellationToken,
) -> Result<PollResult> {
    loop {
        if cancel.is_cancelled() {
            return Ok(PollResult::Cancelled);
        }

        match client.get_next_task().await {
            Ok(resp) => {
                if let Some(task) = resp.task {
                    info!(task_id = task.id, mode = task.mode, "received task");
                    return Ok(PollResult::Task(task));
                }
                debug!("no tasks available");
            }
            Err(e) => {
                warn!(error = %e, "task poll failed");
            }
        }

        select! {
            () = cancel.cancelled() => {
                return Ok(PollResult::Cancelled);
            }
            () = tokio::time::sleep(interval) => {
                // Continue polling
            }
        }
    }
}
