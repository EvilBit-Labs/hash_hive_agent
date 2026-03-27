use reqwest::StatusCode;

/// Errors returned by the HashHive API client.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("authentication failed: {message}")]
    Auth { message: String },

    #[error("not found: {message}")]
    NotFound { message: String },

    #[error("server error ({status}): {message}")]
    Server { status: StatusCode, message: String },

    #[error("request failed: {0}")]
    Request(#[from] reqwest::Error),

    #[error("failed to parse response: {0}")]
    Parse(#[from] serde_json::Error),

    #[error("unexpected status {status}: {body}")]
    Unexpected { status: StatusCode, body: String },
}
