use std::time::Duration;

use anyhow::Result;
use tokio::select;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, warn};

use crate::api::ApiClient;
use crate::api::types::{AgentStatus, HeartbeatRequest};
use crate::platform;

/// Run the heartbeat loop, sending periodic status updates to the server.
///
/// Runs until `cancel` is triggered. Returns `Ok(())` on clean shutdown.
pub async fn run_heartbeat_loop(
    client: &ApiClient,
    interval: Duration,
    cancel: CancellationToken,
) -> Result<()> {
    debug!(?interval, "starting heartbeat loop");

    loop {
        let heartbeat = build_heartbeat().await;

        match client.send_heartbeat(&heartbeat).await {
            Ok(resp) => {
                debug!(acknowledged = resp.acknowledged, "heartbeat sent");
            }
            Err(e) => {
                warn!(error = %e, "heartbeat failed");
            }
        }

        select! {
            () = cancel.cancelled() => {
                debug!("heartbeat loop cancelled");
                return Ok(());
            }
            () = tokio::time::sleep(interval) => {
                // Continue to next heartbeat
            }
        }
    }
}

/// Send a single heartbeat indicating the agent is shutting down.
pub async fn send_shutdown_heartbeat(client: &ApiClient) {
    let heartbeat = HeartbeatRequest {
        status: AgentStatus::Error,
        capabilities: None,
        device_info: None,
    };

    match client.send_heartbeat(&heartbeat).await {
        Ok(_) => debug!("shutdown heartbeat sent"),
        Err(e) => error!(error = %e, "failed to send shutdown heartbeat"),
    }
}

async fn build_heartbeat() -> HeartbeatRequest {
    let device_info = platform::collect_device_info();

    HeartbeatRequest {
        status: AgentStatus::Online,
        capabilities: None,
        device_info: Some(device_info),
    }
}
