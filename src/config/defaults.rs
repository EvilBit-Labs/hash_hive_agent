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
pub const DEFAULT_DATA_DIR: &str = "data";

/// Directory for caching benchmark results.
pub const DEFAULT_CACHE_DIR: &str = "cache";
