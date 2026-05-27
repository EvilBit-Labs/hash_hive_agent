use std::time::Duration;

/// Default server base URL.
pub const DEFAULT_SERVER_URL: &str = "http://localhost:3001/api/v1/agent";

/// Interval between heartbeat sends.
pub const DEFAULT_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);

/// Interval between task polling attempts when idle.
pub const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(10);

/// Maximum number of retry attempts for API requests.
pub const DEFAULT_MAX_RETRIES: u32 = 5;

/// Base delay for exponential backoff.
pub const DEFAULT_BACKOFF_BASE: Duration = Duration::from_secs(1);

/// Maximum backoff delay.
pub const DEFAULT_BACKOFF_MAX: Duration = Duration::from_secs(60);

/// Timeout for individual HTTP requests.
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Timeout for file downloads (large files can take a while).
pub const DEFAULT_DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(600);

/// Directory for storing downloaded resources.
///
/// Uses system-level paths appropriate for a service:
/// - Linux: `/var/lib/hash_hive_agent`
/// - macOS: `/var/lib/hash_hive_agent`
/// - Windows: `C:\ProgramData\HashHive\Agent\data`
#[cfg(not(target_os = "windows"))]
pub const DEFAULT_DATA_DIR: &str = "/var/lib/hash_hive_agent";

/// Directory for caching benchmark results.
///
/// - Linux/macOS: `/var/cache/hash_hive_agent`
/// - Windows: `%ProgramData%\HashHive\Agent\cache` (resolved at runtime)
#[cfg(not(target_os = "windows"))]
pub const DEFAULT_CACHE_DIR: &str = "/var/cache/hash_hive_agent";

/// Resolve the default data directory at runtime.
///
/// On Windows, reads `%ProgramData%` from the environment rather than
/// hardcoding `C:\ProgramData`.
#[cfg(target_os = "windows")]
pub fn windows_program_data_subdir(subdir: &str) -> String {
    let base = std::env::var("ProgramData").unwrap_or_else(|_| r"C:\ProgramData".to_owned());
    format!(r"{base}\HashHive\Agent\{subdir}")
}
