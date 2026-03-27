use std::future::Future;
use std::time::Duration;

use backon::{ExponentialBuilder, Retryable};
use reqwest::{Client, StatusCode};
use tracing::{debug, warn};
use url::Url;

use super::error::ApiError;
use super::types::{
    AcknowledgedResponse, AgentErrorReport, BenchmarkSubmission, CreateSessionRequest,
    CreateSessionResponse, ErrorResponse, HeartbeatRequest, NextTaskResponse, TaskReport,
    ZapResponse,
};
use crate::config::defaults::{DEFAULT_BACKOFF_BASE, DEFAULT_BACKOFF_MAX, DEFAULT_MAX_RETRIES};

/// Configuration for exponential backoff retry behavior.
#[derive(Debug, Clone, Copy)]
pub struct RetryConfig {
    /// Base delay for exponential backoff.
    pub backoff_base: Duration,
    /// Maximum backoff delay cap.
    pub backoff_max: Duration,
    /// Maximum number of retry attempts.
    pub max_retries: u32,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            backoff_base: DEFAULT_BACKOFF_BASE,
            backoff_max: DEFAULT_BACKOFF_MAX,
            max_retries: DEFAULT_MAX_RETRIES,
        }
    }
}

impl From<&crate::config::AgentConfig> for RetryConfig {
    fn from(cfg: &crate::config::AgentConfig) -> Self {
        Self {
            backoff_base: cfg.backoff_base,
            backoff_max: cfg.backoff_max,
            max_retries: cfg.max_retries,
        }
    }
}

/// Returns `true` if the error is transient and the request should be retried.
///
/// Only server errors (5xx) and demonstrably transient network failures (timeouts
/// and connection errors) are retryable. Builder errors, decode failures, redirect
/// loops, body errors, and other permanent `reqwest::Error` variants fail fast.
fn is_retryable(err: &ApiError) -> bool {
    match *err {
        ApiError::Server { .. } => true,
        ApiError::Request(ref req_err) => req_err.is_timeout() || req_err.is_connect(),
        ApiError::Auth { .. }
        | ApiError::NotFound { .. }
        | ApiError::Parse(_)
        | ApiError::Unexpected { .. }
        | ApiError::UrlParse(_) => false,
    }
}

/// HTTP client for the `HashHive` Agent API.
///
/// All methods return typed responses or [`ApiError`].
/// The client is cheaply cloneable (wraps `Arc` internally via `reqwest::Client`).
#[derive(Debug, Clone)]
pub struct ApiClient {
    http: Client,
    base_url: Url,
    session_token: Option<String>,
    retry_config: RetryConfig,
}

impl ApiClient {
    /// Create a new API client pointing at the given server base URL.
    ///
    /// `base_url` should include the `/api/v1/agent` prefix,
    /// e.g. `http://localhost:3001/api/v1/agent`.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client fails to initialize.
    pub fn new(base_url: Url, retry_config: RetryConfig) -> Result<Self, ApiError> {
        let http = Client::builder()
            .user_agent(format!("hash_hive_agent/{}", env!("CARGO_PKG_VERSION")))
            .build()?;

        Ok(Self {
            http,
            base_url,
            session_token: None,
            retry_config,
        })
    }

    /// Store the session token obtained from [`create_session`](Self::create_session).
    pub fn set_session_token(&mut self, token: String) {
        self.session_token = Some(token);
    }

    /// Authenticate with the server using a pre-shared agent token.
    pub async fn create_session(
        &self,
        agent_token: &str,
    ) -> Result<CreateSessionResponse, ApiError> {
        self.with_retry(|| async {
            let url = self.url("sessions")?;
            let body = CreateSessionRequest {
                token: agent_token.to_owned(),
            };
            debug!(%url, "creating agent session");
            let resp = self.http.post(url).json(&body).send().await?;
            self.handle_response(resp).await
        })
        .await
    }

    /// Send a heartbeat with current status and capabilities.
    pub async fn send_heartbeat(
        &self,
        heartbeat: &HeartbeatRequest,
    ) -> Result<AcknowledgedResponse, ApiError> {
        self.with_retry(|| async {
            let url = self.url("heartbeat")?;
            let resp = self.authed_post(&url).json(heartbeat).send().await?;
            self.handle_response(resp).await
        })
        .await
    }

    /// Poll for the next available task.
    pub async fn get_next_task(&self) -> Result<NextTaskResponse, ApiError> {
        self.with_retry(|| async {
            let url = self.url("tasks/next")?;
            let resp = self.authed_post(&url).send().await?;
            self.handle_response(resp).await
        })
        .await
    }

    /// Report task progress, results, or completion.
    pub async fn report_task_progress(
        &self,
        task_id: i64,
        report: &TaskReport,
    ) -> Result<AcknowledgedResponse, ApiError> {
        self.with_retry(|| async {
            let url = self.url(&format!("tasks/{task_id}/report"))?;
            let resp = self.authed_post(&url).json(report).send().await?;
            self.handle_response(resp).await
        })
        .await
    }

    /// Fetch cracked hash values (zaps) for a task, optionally filtered by time.
    pub async fn get_task_zaps(
        &self,
        task_id: i64,
        since: Option<&str>,
        limit: Option<i32>,
    ) -> Result<ZapResponse, ApiError> {
        self.with_retry(|| async {
            let mut url = self.url(&format!("tasks/{task_id}/zaps"))?;
            {
                let mut query = url.query_pairs_mut();
                if let Some(s) = since {
                    query.append_pair("since", s);
                }
                if let Some(l) = limit {
                    query.append_pair("limit", &l.to_string());
                }
            }
            let resp = self.authed_get(&url).send().await?;
            self.handle_response(resp).await
        })
        .await
    }

    /// Submit benchmark results.
    pub async fn submit_benchmarks(
        &self,
        submission: &BenchmarkSubmission,
    ) -> Result<AcknowledgedResponse, ApiError> {
        self.with_retry(|| async {
            let url = self.url("benchmark")?;
            let resp = self.authed_post(&url).json(submission).send().await?;
            self.handle_response(resp).await
        })
        .await
    }

    /// Report an agent error to the server.
    pub async fn report_error(
        &self,
        error: &AgentErrorReport,
    ) -> Result<AcknowledgedResponse, ApiError> {
        self.with_retry(|| async {
            let url = self.url("errors")?;
            let resp = self.authed_post(&url).json(error).send().await?;
            self.handle_response(resp).await
        })
        .await
    }

    // -----------------------------------------------------------------------
    // Retry
    // -----------------------------------------------------------------------

    /// Execute an async operation with exponential backoff and jitter.
    ///
    /// Only transient errors ([`ApiError::Server`] and [`ApiError::Request`])
    /// are retried; all other variants fail immediately.
    pub(crate) async fn with_retry<F, Fut, T>(&self, operation: F) -> Result<T, ApiError>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, ApiError>>,
    {
        let backoff = ExponentialBuilder::default()
            .with_min_delay(self.retry_config.backoff_base)
            .with_max_delay(self.retry_config.backoff_max)
            .with_max_times(
                self.retry_config
                    .max_retries
                    .try_into()
                    .unwrap_or(usize::MAX),
            )
            .with_jitter();

        // 1-based retry attempt counter for structured logging.
        let attempt = std::sync::atomic::AtomicU32::new(1);

        operation
            .retry(backoff)
            .sleep(tokio::time::sleep)
            .when(is_retryable)
            .notify(|err, dur| {
                let n = attempt.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                warn!(error = %err, attempt = n, delay = ?dur, "retrying API request");
            })
            .await
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn url(&self, path: &str) -> Result<Url, ApiError> {
        let mut url = self.base_url.clone();
        // Ensure trailing slash before joining
        if !url.path().ends_with('/') {
            url.set_path(&format!("{}/", url.path()));
        }
        Ok(url.join(path)?)
    }

    fn authed_post(&self, url: &Url) -> reqwest::RequestBuilder {
        let mut builder = self.http.post(url.clone());
        if let Some(ref token) = self.session_token {
            builder = builder.bearer_auth(token);
        }
        builder
    }

    fn authed_get(&self, url: &Url) -> reqwest::RequestBuilder {
        let mut builder = self.http.get(url.clone());
        if let Some(ref token) = self.session_token {
            builder = builder.bearer_auth(token);
        }
        builder
    }

    async fn handle_response<T: serde::de::DeserializeOwned>(
        &self,
        resp: reqwest::Response,
    ) -> Result<T, ApiError> {
        let status = resp.status();

        match status {
            s if s.is_success() => {
                let body = resp.text().await?;
                serde_json::from_str(&body).map_err(ApiError::Parse)
            }
            StatusCode::UNAUTHORIZED => {
                let msg = self.extract_error_message(resp).await;
                Err(ApiError::Auth { message: msg })
            }
            StatusCode::NOT_FOUND => {
                let msg = self.extract_error_message(resp).await;
                Err(ApiError::NotFound { message: msg })
            }
            s if s.is_server_error() => {
                let msg = self.extract_error_message(resp).await;
                Err(ApiError::Server {
                    status: s,
                    message: msg,
                })
            }
            _ => {
                let body = resp.text().await.unwrap_or_default();
                Err(ApiError::Unexpected { status, body })
            }
        }
    }

    async fn extract_error_message(&self, resp: reqwest::Response) -> String {
        let body = resp.text().await.unwrap_or_default();
        if let Ok(err_resp) = serde_json::from_str::<ErrorResponse>(&body) {
            err_resp.error.and_then(|e| e.message).unwrap_or(body)
        } else {
            warn!("could not parse error response body");
            body
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use reqwest::StatusCode;

    use super::is_retryable;
    use crate::api::error::ApiError;

    #[test]
    fn server_error_is_retryable() {
        let err = ApiError::Server {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: "internal".to_owned(),
        };
        assert!(is_retryable(&err));
    }

    #[test]
    fn bad_gateway_is_retryable() {
        let err = ApiError::Server {
            status: StatusCode::BAD_GATEWAY,
            message: "bad gateway".to_owned(),
        };
        assert!(is_retryable(&err));
    }

    #[test]
    fn service_unavailable_is_retryable() {
        let err = ApiError::Server {
            status: StatusCode::SERVICE_UNAVAILABLE,
            message: "unavailable".to_owned(),
        };
        assert!(is_retryable(&err));
    }

    #[test]
    fn timeout_request_error_is_retryable() {
        // Build a reqwest timeout error via an expired client timeout.
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_nanos(1))
            .build()
            .expect("client build");

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");

        let result = rt.block_on(async {
            // Request to a non-routable address to trigger timeout.
            client.get("http://192.0.2.1/").send().await
        });

        if let Err(req_err) = result {
            if req_err.is_timeout() || req_err.is_connect() {
                let err = ApiError::Request(req_err);
                assert!(is_retryable(&err));
            }
            // If neither timeout nor connect, the error type doesn't match our test intent;
            // skip rather than assert on an unrelated variant.
        }
    }

    #[test]
    fn auth_error_is_not_retryable() {
        let err = ApiError::Auth {
            message: "unauthorized".to_owned(),
        };
        assert!(!is_retryable(&err));
    }

    #[test]
    fn not_found_error_is_not_retryable() {
        let err = ApiError::NotFound {
            message: "not found".to_owned(),
        };
        assert!(!is_retryable(&err));
    }

    #[test]
    fn parse_error_is_not_retryable() {
        let json_err = serde_json::from_str::<String>("not json").unwrap_err();
        let err = ApiError::Parse(json_err);
        assert!(!is_retryable(&err));
    }

    #[test]
    fn unexpected_error_is_not_retryable() {
        let err = ApiError::Unexpected {
            status: StatusCode::IM_A_TEAPOT,
            body: "teapot".to_owned(),
        };
        assert!(!is_retryable(&err));
    }
}
