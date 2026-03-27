use tokio_util::sync::CancellationToken;
use tracing::info;

/// Listen for OS shutdown signals (SIGINT, SIGTERM) and trigger cancellation.
///
/// On Windows, only Ctrl+C is supported.
///
/// # Panics
///
/// Panics if the OS refuses to register signal handlers. This is called at
/// agent startup — if we cannot listen for shutdown signals, there is no safe
/// way to proceed.
#[allow(clippy::expect_used)]
pub async fn listen_for_shutdown(cancel: CancellationToken) {
    let signal = async {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{SignalKind, signal};
            let mut sigterm =
                signal(SignalKind::terminate()).expect("failed to register SIGTERM handler");
            let mut sigint =
                signal(SignalKind::interrupt()).expect("failed to register SIGINT handler");

            tokio::select! {
                _ = sigterm.recv() => info!("received SIGTERM"),
                _ = sigint.recv() => info!("received SIGINT"),
            }
        }

        #[cfg(not(unix))]
        {
            tokio::signal::ctrl_c()
                .await
                .expect("failed to register Ctrl+C handler");
            info!("received Ctrl+C");
        }
    };

    signal.await;
    info!("initiating graceful shutdown");
    cancel.cancel();
}
