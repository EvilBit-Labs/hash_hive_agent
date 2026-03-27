use anyhow::{Context, Result, bail};
use clap::Parser;
use tracing::info;
use tracing_subscriber::EnvFilter;

use hash_hive_agent::agent;
use hash_hive_agent::cli::Cli;
use hash_hive_agent::config::AgentConfig;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    init_logging(&cli.log_level, cli.json_logs)?;

    info!(
        version = env!("CARGO_PKG_VERSION"),
        platform = hash_hive_agent::platform::platform_name(),
        "starting hash_hive_agent"
    );

    // Load config from file + env, then apply CLI overrides
    let config_path = cli.config.as_deref().and_then(|p| p.to_str());
    let mut config = AgentConfig::load(config_path).context("failed to load configuration")?;

    // CLI flags override config file / env
    if let Some(url) = cli.server_url {
        config.server_url = url;
    }
    if let Some(token) = cli.agent_token {
        config.agent_token = token;
    }
    if let Some(path) = cli.hashcat_path {
        config.hashcat_path = Some(path);
    }

    if config.agent_token.is_empty() {
        bail!("agent token is required (set HASH_HIVE_AGENT_TOKEN or --agent-token)");
    }

    agent::run(config).await
}

fn init_logging(level: &str, json: bool) -> Result<()> {
    let filter = EnvFilter::try_new(level)
        .or_else(|_| EnvFilter::try_new("info"))
        .context("failed to create log filter")?;

    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true);

    if json {
        subscriber.json().init();
    } else {
        subscriber.init();
    }

    Ok(())
}
