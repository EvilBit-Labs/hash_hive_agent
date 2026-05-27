use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// `HashHive` Agent — distributed hashcat agent for the `HashHive` platform.
#[derive(Debug, Parser)]
#[command(
    name = "hash_hive_agent",
    version,
    author,
    about,
    long_about = "\
HashHive Agent — distributed hashcat agent for the HashHive platform.\n\
\n\
The agent authenticates with a HashHive server, sends periodic heartbeats,\n\
polls for hash-cracking tasks, downloads resources, runs hashcat, and reports\n\
results. It is designed to run as a long-lived system service.\n\
\n\
Configuration is resolved in order: built-in defaults < config file < \n\
environment variables (HASH_HIVE_*) < CLI flags.\n\
\n\
On shutdown (SIGTERM/SIGINT), the agent finishes the current operation,\n\
sends a final heartbeat, and exits cleanly."
)]
pub struct Cli {
    /// Path to the configuration file.
    #[arg(short, long, value_name = "FILE", env = "HASH_HIVE_CONFIG")]
    pub config: Option<PathBuf>,

    /// `HashHive` server URL (overrides config file).
    #[arg(long, value_name = "URL", env = "HASH_HIVE_SERVER_URL")]
    pub server_url: Option<String>,

    /// Agent authentication token (overrides config file).
    #[arg(long, value_name = "TOKEN", env = "HASH_HIVE_AGENT_TOKEN")]
    pub agent_token: Option<String>,

    /// Path to the hashcat binary (auto-detected if not specified).
    #[arg(long, value_name = "PATH", env = "HASH_HIVE_HASHCAT_PATH")]
    pub hashcat_path: Option<PathBuf>,

    /// Enable JSON-formatted log output.
    #[arg(long, default_value_t = false, env = "HASH_HIVE_JSON_LOGS")]
    pub json_logs: bool,

    /// Set the log level (trace, debug, info, warn, error).
    #[arg(long, default_value = "info", env = "HASH_HIVE_LOG_LEVEL")]
    pub log_level: String,

    /// Increase log verbosity (-v = debug, -vv = trace). Overrides --log-level.
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Option<Command>,
}

/// Service management subcommands.
#[derive(Debug, Subcommand)]
#[non_exhaustive]
pub enum Command {
    /// Install the agent as a system service.
    #[command(name = "service-install")]
    ServiceInstall,

    /// Uninstall the agent system service.
    #[command(name = "service-uninstall")]
    ServiceUninstall,

    /// Start the installed service.
    #[command(name = "service-start")]
    ServiceStart,

    /// Stop the installed service.
    #[command(name = "service-stop")]
    ServiceStop,

    /// Show the service status.
    #[command(name = "service-status")]
    ServiceStatus,
}
