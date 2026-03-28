#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::wildcard_enum_match_arm
)]

use std::time::Duration;

use hash_hive_agent::api::{ApiClient, ApiError, RetryConfig};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Build a client with fast retry config pointed at the mock server.
fn test_client(server: &MockServer, max_retries: u32) -> ApiClient {
    let base_url = format!("{}/api/v1/agent", server.uri())
        .parse()
        .expect("test URL");
    let retry = RetryConfig {
        backoff_base: Duration::from_millis(10),
        backoff_max: Duration::from_millis(50),
        max_retries,
    };
    ApiClient::new(base_url, retry).expect("test client")
}

#[tokio::test]
async fn retries_on_5xx_then_succeeds() {
    let server = MockServer::start().await;

    // First two requests return 503, third returns 200.
    Mock::given(method("POST"))
        .and(path("/api/v1/agent/sessions"))
        .respond_with(ResponseTemplate::new(503).set_body_json(serde_json::json!({
            "error": { "message": "service unavailable" }
        })))
        .up_to_n_times(2)
        .expect(2)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/v1/agent/sessions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "sessionToken": "tok_abc",
            "config": { "agentId": 1, "projectId": 2 }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server, 3);
    let resp = client.create_session("test-token").await;

    assert!(resp.is_ok(), "expected success after retries: {resp:?}");
    assert_eq!(resp.expect("checked above").session_token, "tok_abc");
}

#[tokio::test]
async fn does_not_retry_on_401() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/agent/sessions"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "error": { "message": "invalid token" }
        })))
        .expect(1) // Must be called exactly once — no retry.
        .mount(&server)
        .await;

    let client = test_client(&server, 3);
    let resp = client.create_session("bad-token").await;

    assert!(resp.is_err());
    assert!(
        matches!(resp.as_ref().unwrap_err(), ApiError::Auth { .. }),
        "expected Auth error, got: {resp:?}"
    );
}

#[tokio::test]
async fn does_not_retry_on_404() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/agent/tasks/next"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "error": { "message": "not found" }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server, 3);
    let resp = client.get_next_task().await;

    assert!(resp.is_err());
    assert!(matches!(resp.unwrap_err(), ApiError::NotFound { .. }));
}

#[tokio::test]
async fn exhausted_retries_propagate_server_error() {
    let server = MockServer::start().await;

    // Always returns 500 — should exhaust all retries.
    Mock::given(method("POST"))
        .and(path("/api/v1/agent/sessions"))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "error": { "message": "internal server error" }
        })))
        // 1 initial + 2 retries = 3 total
        .expect(3)
        .mount(&server)
        .await;

    let client = test_client(&server, 2);
    let resp = client.create_session("test-token").await;

    assert!(resp.is_err());
    match resp.unwrap_err() {
        ApiError::Server { status, .. } => {
            assert_eq!(status, reqwest::StatusCode::INTERNAL_SERVER_ERROR);
        }
        other => panic!("expected Server error, got: {other:?}"),
    }
}

#[tokio::test]
async fn zero_retries_sends_exactly_one_request() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/agent/sessions"))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "error": { "message": "internal server error" }
        })))
        .expect(1) // Exactly one attempt, no retries.
        .mount(&server)
        .await;

    let client = test_client(&server, 0);
    let resp = client.create_session("test-token").await;

    assert!(resp.is_err());
    assert!(matches!(resp.unwrap_err(), ApiError::Server { .. }));
}
