use std::path::PathBuf;

use clap::Parser;

/// `HashHive` Agent — distributed hashcat agent for the `HashHive` platform.
#[derive(Debug, Parser)]
#[command(name = "hash_hive_agent", version, about)]
pub struct Cli {
    /// Path to the configuration file.
    #[arg(short, long, env = "HASH_HIVE_CONFIG")]
    pub config: Option<PathBuf>,

    /// `HashHive` server URL (overrides config file).
    #[arg(long, env = "HASH_HIVE_SERVER_URL")]
    pub server_url: Option<String>,

    /// Agent authentication token (overrides config file).
    #[arg(long, env = "HASH_HIVE_AGENT_TOKEN")]
    pub agent_token: Option<String>,

    /// Path to the hashcat binary.
    #[arg(long, env = "HASH_HIVE_HASHCAT_PATH")]
    pub hashcat_path: Option<PathBuf>,

    /// Enable JSON-formatted log output.
    #[arg(long, default_value_t = false)]
    pub json_logs: bool,

    /// Set the log level (trace, debug, info, warn, error).
    #[arg(long, default_value = "info", env = "HASH_HIVE_LOG_LEVEL")]
    pub log_level: String,
}
