use reqwest::{Client, StatusCode};
use tracing::{debug, warn};
use url::Url;

use super::error::ApiError;
use super::types::{
    AcknowledgedResponse, AgentErrorReport, BenchmarkSubmission, CreateSessionRequest,
    CreateSessionResponse, ErrorResponse, HeartbeatRequest, NextTaskResponse, TaskReport,
    ZapResponse,
};

/// HTTP client for the HashHive Agent API.
///
/// All methods return typed responses or [`ApiError`].
/// The client is cheaply cloneable (wraps `Arc` internally via `reqwest::Client`).
#[derive(Debug, Clone)]
pub struct ApiClient {
    http: Client,
    base_url: Url,
    session_token: Option<String>,
}

impl ApiClient {
    /// Create a new API client pointing at the given server base URL.
    ///
    /// `base_url` should include the `/api/v1/agent` prefix,
    /// e.g. `http://localhost:3001/api/v1/agent`.
    pub fn new(base_url: Url) -> Self {
        let http = Client::builder()
            .user_agent(format!("hash_hive_agent/{}", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("failed to build HTTP client");

        Self {
            http,
            base_url,
            session_token: None,
        }
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
        let url = self.url("sessions");
        let body = CreateSessionRequest {
            token: agent_token.to_owned(),
        };

        debug!(%url, "creating agent session");

        let resp = self.http.post(url).json(&body).send().await?;
        self.handle_response(resp).await
    }

    /// Send a heartbeat with current status and capabilities.
    pub async fn send_heartbeat(
        &self,
        heartbeat: &HeartbeatRequest,
    ) -> Result<AcknowledgedResponse, ApiError> {
        let url = self.url("heartbeat");
        let resp = self.authed_post(&url).json(heartbeat).send().await?;
        self.handle_response(resp).await
    }

    /// Poll for the next available task.
    pub async fn get_next_task(&self) -> Result<NextTaskResponse, ApiError> {
        let url = self.url("tasks/next");
        let resp = self.authed_post(&url).send().await?;
        self.handle_response(resp).await
    }

    /// Report task progress, results, or completion.
    pub async fn report_task_progress(
        &self,
        task_id: i64,
        report: &TaskReport,
    ) -> Result<AcknowledgedResponse, ApiError> {
        let url = self.url(&format!("tasks/{task_id}/report"));
        let resp = self.authed_post(&url).json(report).send().await?;
        self.handle_response(resp).await
    }

    /// Fetch cracked hash values (zaps) for a task, optionally filtered by time.
    pub async fn get_task_zaps(
        &self,
        task_id: i64,
        since: Option<&str>,
        limit: Option<i32>,
    ) -> Result<ZapResponse, ApiError> {
        let mut url = self.url(&format!("tasks/{task_id}/zaps"));
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
    }

    /// Submit benchmark results.
    pub async fn submit_benchmarks(
        &self,
        submission: &BenchmarkSubmission,
    ) -> Result<AcknowledgedResponse, ApiError> {
        let url = self.url("benchmark");
        let resp = self.authed_post(&url).json(submission).send().await?;
        self.handle_response(resp).await
    }

    /// Report an agent error to the server.
    pub async fn report_error(
        &self,
        error: &AgentErrorReport,
    ) -> Result<AcknowledgedResponse, ApiError> {
        let url = self.url("errors");
        let resp = self.authed_post(&url).json(error).send().await?;
        self.handle_response(resp).await
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn url(&self, path: &str) -> Url {
        let mut url = self.base_url.clone();
        // Ensure trailing slash before joining
        if !url.path().ends_with('/') {
            url.set_path(&format!("{}/", url.path()));
        }
        url.join(path).expect("invalid URL path segment")
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
        match serde_json::from_str::<ErrorResponse>(&body) {
            Ok(err_resp) => err_resp.error.and_then(|e| e.message).unwrap_or(body),
            Err(_) => {
                warn!("could not parse error response body");
                body
            }
        }
    }
}
