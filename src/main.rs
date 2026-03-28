use std::io::IsTerminal;

use clap::Parser;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

use hash_hive_agent::agent;
use hash_hive_agent::cli::Cli;
use hash_hive_agent::config::AgentConfig;
use hash_hive_agent::config::defaults::DEFAULT_SERVER_URL;

/// Exit code for configuration errors (bad config file, missing token, invalid URL).
const EXIT_CONFIG: i32 = 2;

/// Exit code for authentication failures (invalid or expired token).
const EXIT_AUTH: i32 = 3;

/// Exit code for unrecoverable runtime errors (network, hashcat, I/O).
const EXIT_RUNTIME: i32 = 1;

#[tokio::main]
#[allow(clippy::exit)]
async fn main() {
    let cli = Cli::parse();

    // Service management subcommands run without logging/config.
    if let Some(ref cmd) = cli.command {
        if let Err(e) = hash_hive_agent::service::handle(cmd) {
            eprintln!("error: {e:#}");
            std::process::exit(EXIT_RUNTIME);
        }
        return;
    }

    let log_level = resolve_log_level(&cli);
    if let Err(e) = init_logging(&log_level, cli.json_logs) {
        eprintln!("failed to initialize logging: {e}");
        std::process::exit(EXIT_CONFIG);
    }

    info!(
        version = env!("CARGO_PKG_VERSION"),
        platform = hash_hive_agent::platform::platform_name(),
        "starting hash_hive_agent"
    );

    if let Err(e) = run(cli).await {
        let code = classify_exit_code(&e);
        tracing::error!(error = %e, exit_code = code, "agent exited with error");
        std::process::exit(code);
    }
}

async fn run(cli: Cli) -> anyhow::Result<()> {
    use anyhow::{Context, bail};

    // Load config from file + env, then apply CLI overrides
    let config_path = cli.config.as_deref().and_then(|p| p.to_str());
    let mut config = AgentConfig::load(config_path).context(
        "failed to load configuration\n\n\
         Check:\n  \
         - Config file syntax (YAML/TOML)\n  \
         - Environment variables (HASH_HIVE_*)\n  \
         - Duration formats: '30s', '10m', '1h'",
    )?;

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

    if config.server_url == DEFAULT_SERVER_URL {
        warn!(
            url = DEFAULT_SERVER_URL,
            "using default server URL (set HASH_HIVE_SERVER_URL to override)"
        );
    }

    agent::run(config).await
}

/// Map verbose count to log level, with --verbose overriding --log-level.
fn resolve_log_level(cli: &Cli) -> String {
    match cli.verbose {
        0 => cli.log_level.clone(),
        1 => "debug".to_owned(),
        _ => "trace".to_owned(),
    }
}

/// Classify an anyhow error chain into an exit code.
fn classify_exit_code(err: &anyhow::Error) -> i32 {
    let msg = err.to_string();
    if msg.contains("agent token is required") || msg.contains("failed to load configuration") {
        EXIT_CONFIG
    } else if msg.contains("failed to authenticate") || msg.contains("authentication failed") {
        EXIT_AUTH
    } else {
        EXIT_RUNTIME
    }
}

fn init_logging(level: &str, json: bool) -> anyhow::Result<()> {
    use anyhow::Context;

    let filter = EnvFilter::try_new(level)
        .or_else(|_| EnvFilter::try_new("info"))
        .context("failed to create log filter")?;

    let use_ansi =
        !json && std::io::stderr().is_terminal() && std::env::var_os("NO_COLOR").is_none();

    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_ansi(use_ansi);

    if json {
        subscriber.json().init();
    } else {
        subscriber.init();
    }

    Ok(())
}
