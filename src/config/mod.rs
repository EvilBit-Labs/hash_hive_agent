pub mod defaults;

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use serde::Deserialize;

use defaults::{
    DEFAULT_BACKOFF_BASE, DEFAULT_BACKOFF_MAX, DEFAULT_DOWNLOAD_TIMEOUT,
    DEFAULT_HEARTBEAT_INTERVAL, DEFAULT_MAX_RETRIES, DEFAULT_POLL_INTERVAL,
    DEFAULT_REQUEST_TIMEOUT, DEFAULT_SERVER_URL,
};
#[cfg(not(target_os = "windows"))]
use defaults::{DEFAULT_CACHE_DIR, DEFAULT_DATA_DIR};

/// Agent configuration resolved from CLI flags, environment variables, and config file.
#[derive(Debug, Clone, Deserialize)]
pub struct AgentConfig {
    /// `HashHive` server base URL (including `/api/v1/agent` path).
    pub server_url: String,

    /// Pre-shared token used to authenticate this agent.
    pub agent_token: String,

    /// Path to the hashcat binary.
    pub hashcat_path: Option<PathBuf>,

    /// Interval between heartbeat messages.
    #[serde(with = "humantime_serde", default = "default_heartbeat_interval")]
    pub heartbeat_interval: Duration,

    /// Interval between task polls when idle.
    #[serde(with = "humantime_serde", default = "default_poll_interval")]
    pub poll_interval: Duration,

    /// Maximum API request retries.
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// Backoff base delay.
    #[serde(with = "humantime_serde", default = "default_backoff_base")]
    pub backoff_base: Duration,

    /// Backoff maximum delay.
    #[serde(with = "humantime_serde", default = "default_backoff_max")]
    pub backoff_max: Duration,

    /// HTTP request timeout.
    #[serde(with = "humantime_serde", default = "default_request_timeout")]
    pub request_timeout: Duration,

    /// Download timeout for large files.
    #[serde(with = "humantime_serde", default = "default_download_timeout")]
    pub download_timeout: Duration,

    /// Directory for downloaded task resources.
    #[serde(default = "default_data_dir")]
    pub data_dir: PathBuf,

    /// Directory for benchmark caches.
    #[serde(default = "default_cache_dir")]
    pub cache_dir: PathBuf,
}

impl AgentConfig {
    /// Load configuration from the given file path, merging with environment variables.
    ///
    /// Environment variables are prefixed with `HASH_HIVE_` and use uppercase `snake_case`
    /// (e.g. `HASH_HIVE_SERVER_URL`, `HASH_HIVE_AGENT_TOKEN`).
    pub fn load(path: Option<&str>) -> Result<Self> {
        let mut builder = config::Config::builder()
            .set_default("server_url", DEFAULT_SERVER_URL)?
            .set_default("heartbeat_interval", "30s")?
            .set_default("poll_interval", "10s")?
            .set_default("max_retries", i64::from(DEFAULT_MAX_RETRIES))?
            .set_default("backoff_base", "1s")?
            .set_default("backoff_max", "60s")?
            .set_default("request_timeout", "30s")?
            .set_default("download_timeout", "600s")?
            .set_default("data_dir", default_data_dir().to_string_lossy().as_ref())?
            .set_default("cache_dir", default_cache_dir().to_string_lossy().as_ref())?;

        if let Some(p) = path {
            builder = builder.add_source(config::File::with_name(p).required(false));
        }

        builder = builder.add_source(
            config::Environment::with_prefix("HASH_HIVE")
                .separator("_")
                .try_parsing(true),
        );

        let cfg: Self = builder
            .build()
            .context("failed to build config")?
            .try_deserialize()
            .context("failed to deserialize config")?;

        Ok(cfg)
    }
}

// Serde default functions (must be free functions, not const)
const fn default_heartbeat_interval() -> Duration {
    DEFAULT_HEARTBEAT_INTERVAL
}
const fn default_poll_interval() -> Duration {
    DEFAULT_POLL_INTERVAL
}
const fn default_max_retries() -> u32 {
    DEFAULT_MAX_RETRIES
}
const fn default_backoff_base() -> Duration {
    DEFAULT_BACKOFF_BASE
}
const fn default_backoff_max() -> Duration {
    DEFAULT_BACKOFF_MAX
}
const fn default_request_timeout() -> Duration {
    DEFAULT_REQUEST_TIMEOUT
}
const fn default_download_timeout() -> Duration {
    DEFAULT_DOWNLOAD_TIMEOUT
}
fn default_data_dir() -> PathBuf {
    #[cfg(not(target_os = "windows"))]
    {
        PathBuf::from(DEFAULT_DATA_DIR)
    }
    #[cfg(target_os = "windows")]
    {
        PathBuf::from(defaults::windows_program_data_subdir("data"))
    }
}
fn default_cache_dir() -> PathBuf {
    #[cfg(not(target_os = "windows"))]
    {
        PathBuf::from(DEFAULT_CACHE_DIR)
    }
    #[cfg(target_os = "windows")]
    {
        PathBuf::from(defaults::windows_program_data_subdir("cache"))
    }
}

/// Serde helper module for human-readable durations via the `humantime` crate format.
mod humantime_serde {
    use std::time::Duration;

    use serde::{Deserialize, Deserializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        humantime_parse(&s).map_err(serde::de::Error::custom)
    }

    #[allow(clippy::option_if_let_else, clippy::arithmetic_side_effects)]
    fn humantime_parse(s: &str) -> Result<Duration, String> {
        // Support simple formats: "30s", "10m", "1h", or raw seconds
        let trimmed = s.trim();
        if let Ok(secs) = trimmed.parse::<u64>() {
            return Ok(Duration::from_secs(secs));
        }
        if let Some(rest) = trimmed.strip_suffix('s') {
            rest.trim()
                .parse::<u64>()
                .map(Duration::from_secs)
                .map_err(|e| e.to_string())
        } else if let Some(rest) = trimmed.strip_suffix('m') {
            rest.trim()
                .parse::<u64>()
                .map(|m| Duration::from_secs(m * 60))
                .map_err(|e| e.to_string())
        } else if let Some(rest) = trimmed.strip_suffix('h') {
            rest.trim()
                .parse::<u64>()
                .map(|h| Duration::from_secs(h * 3600))
                .map_err(|e| e.to_string())
        } else {
            Err(format!("unsupported duration format: {trimmed}"))
        }
    }
}
