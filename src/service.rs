use std::ffi::OsString;

use anyhow::{Context, Result};
use service_manager::{
    ServiceInstallCtx, ServiceLabel, ServiceManager, ServiceStartCtx, ServiceStatusCtx,
    ServiceStopCtx, ServiceUninstallCtx,
};

use crate::cli::Command;

const SERVICE_LABEL: &str = "io.evilbitlabs.hash-hive-agent";

fn label() -> Result<ServiceLabel> {
    SERVICE_LABEL
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid service label: {e}"))
}

fn manager() -> Result<Box<dyn ServiceManager>> {
    <dyn ServiceManager>::native().context("no supported service manager found on this platform")
}

/// Handle a service management subcommand.
pub fn handle(command: &Command) -> Result<()> {
    match *command {
        Command::ServiceInstall => install(),
        Command::ServiceUninstall => uninstall(),
        Command::ServiceStart => start(),
        Command::ServiceStop => stop(),
        Command::ServiceStatus => status(),
    }
}

fn install() -> Result<()> {
    let mgr = manager()?;

    let program = std::env::current_exe().context("failed to determine current executable path")?;

    // Pass through environment-based config so the service inherits it.
    let mut env_vars: Vec<(String, String)> = Vec::new();
    for key in [
        "HASH_HIVE_SERVER_URL",
        "HASH_HIVE_AGENT_TOKEN",
        "HASH_HIVE_CONFIG",
        "HASH_HIVE_HASHCAT_PATH",
        "HASH_HIVE_LOG_LEVEL",
        "HASH_HIVE_JSON_LOGS",
    ] {
        if let Ok(val) = std::env::var(key) {
            env_vars.push((key.to_owned(), val));
        }
    }

    let ctx = ServiceInstallCtx {
        label: label()?,
        program,
        args: vec![OsString::from("--json-logs")],
        contents: None,
        username: None,
        working_directory: None,
        environment: if env_vars.is_empty() {
            None
        } else {
            Some(env_vars)
        },
        autostart: true,
        restart_policy: service_manager::RestartPolicy::OnFailure {
            delay_secs: Some(5),
            max_retries: None,
            reset_after_secs: None,
        },
    };

    mgr.install(ctx).context("failed to install service")?;

    println!("Service installed: {SERVICE_LABEL}");
    println!("Start with: hash-hive-agent service-start");
    Ok(())
}

fn uninstall() -> Result<()> {
    let mgr = manager()?;

    // Stop first if running — ignore errors (may already be stopped).
    drop(mgr.stop(ServiceStopCtx { label: label()? }));

    mgr.uninstall(ServiceUninstallCtx { label: label()? })
        .context("failed to uninstall service")?;

    println!("Service uninstalled: {SERVICE_LABEL}");
    Ok(())
}

fn start() -> Result<()> {
    let mgr = manager()?;

    mgr.start(ServiceStartCtx { label: label()? })
        .context("failed to start service")?;

    println!("Service started: {SERVICE_LABEL}");
    Ok(())
}

fn stop() -> Result<()> {
    let mgr = manager()?;

    mgr.stop(ServiceStopCtx { label: label()? })
        .context("failed to stop service")?;

    println!("Service stopped: {SERVICE_LABEL}");
    Ok(())
}

fn status() -> Result<()> {
    let mgr = manager()?;

    let status = mgr
        .status(ServiceStatusCtx { label: label()? })
        .context("failed to query service status")?;

    match status {
        service_manager::ServiceStatus::NotInstalled => {
            println!("Service not installed");
        }
        service_manager::ServiceStatus::Running => {
            println!("Service running");
        }
        service_manager::ServiceStatus::Stopped(reason) => {
            if let Some(r) = reason {
                println!("Service stopped: {r}");
            } else {
                println!("Service stopped");
            }
        }
    }

    Ok(())
}
