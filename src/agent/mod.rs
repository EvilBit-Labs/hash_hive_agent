pub mod heartbeat;
pub mod polling;
pub mod shutdown;

use anyhow::{Context, Result};
use tokio_util::sync::CancellationToken;
use tracing::{error, info};
use url::Url;

use crate::api::ApiClient;
use crate::config::AgentConfig;
use crate::task;

use heartbeat::{run_heartbeat_loop, send_shutdown_heartbeat};
use polling::{PollResult, poll_for_task};
use shutdown::listen_for_shutdown;

/// Run the agent main loop: authenticate, heartbeat, poll, execute tasks.
pub async fn run(config: AgentConfig) -> Result<()> {
    let base_url: Url = config.server_url.parse().context("invalid server URL")?;

    let retry_config = crate::api::RetryConfig::from(&config);
    let mut client =
        ApiClient::new(base_url, retry_config).context("failed to initialize API client")?;

    // Authenticate
    info!("authenticating with server");
    let session = client
        .create_session(&config.agent_token)
        .await
        .context("failed to authenticate")?;

    client.set_session_token(session.session_token);

    if let Some(ref cfg) = session.config {
        info!(
            agent_id = cfg.agent_id,
            project_id = cfg.project_id,
            "authenticated"
        );
    }

    let cancel = CancellationToken::new();

    // Spawn shutdown signal listener
    let shutdown_cancel = cancel.clone();
    tokio::spawn(async move {
        listen_for_shutdown(shutdown_cancel).await;
    });

    // Spawn heartbeat loop
    let hb_client = client.clone();
    let hb_cancel = cancel.clone();
    let hb_interval = config.heartbeat_interval;
    let hb_handle = tokio::spawn(async move {
        if let Err(e) = run_heartbeat_loop(&hb_client, hb_interval, hb_cancel).await {
            error!(error = %e, "heartbeat loop failed");
        }
    });

    // Main task loop
    info!("entering task polling loop");
    loop {
        match poll_for_task(&client, config.poll_interval, cancel.clone()).await? {
            PollResult::Task(descriptor) => {
                info!(task_id = descriptor.id, "executing task");
                if let Err(e) = task::execute(&client, &config, descriptor).await {
                    error!(error = %e, "task execution failed");
                }
            }
            PollResult::Idle => {}
            PollResult::Cancelled => {
                info!("task polling cancelled, shutting down");
                break;
            }
        }
    }

    // Graceful shutdown
    send_shutdown_heartbeat(&client).await;
    hb_handle.abort();

    info!("agent stopped");
    Ok(())
}
