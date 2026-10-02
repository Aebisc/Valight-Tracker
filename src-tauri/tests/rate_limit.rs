use std::time::Duration;
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path};
use valorant_tracker::riot::client::{build_local_client, safe_get_json, RiotResult};

#[tokio::test]
async fn test_rate_limit_retry_succeeds() {
    let server = MockServer::start().await;
    let client = build_local_client();
    let headers = reqwest::header::HeaderMap::new();

    // First request -> 429 with Retry-After: 1
    // Second request -> 200 with {"status": "ok"}
    Mock::given(method("GET"))
        .and(path("/test-retry"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("Retry-After", "1")
                .set_body_json(serde_json::json!({"message": "rate limited"}))
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/test-retry"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"status": "ok"})))
        .mount(&server)
        .await;

    let url = format!("{}/test-retry", server.uri());
    let start = std::time::Instant::now();
    let res = safe_get_json(&client, &url, &headers).await;
    let elapsed = start.elapsed();

    assert!(elapsed >= Duration::from_millis(900), "Should have waited for Retry-After duration");
    match res {
        RiotResult::Ok(val) => {
            assert_eq!(val["status"], "ok");
        }
        other => panic!("Expected RiotResult::Ok, got {:?}", other),
    }
}

#[tokio::test]
async fn test_rate_limit_exceeded_returns_rate_limited() {
    let server = MockServer::start().await;
    let client = build_local_client();
    let headers = reqwest::header::HeaderMap::new();

    Mock::given(method("GET"))
        .and(path("/test-exhausted"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("Retry-After", "1")
                .set_body_json(serde_json::json!({"message": "still limited"}))
        )
        .mount(&server)
        .await;

    let url = format!("{}/test-exhausted", server.uri());
    let res = safe_get_json(&client, &url, &headers).await;

    match res {
        RiotResult::RateLimited { retry_after } => {
            assert_eq!(retry_after, Some(Duration::from_secs(1)));
        }
        other => panic!("Expected RiotResult::RateLimited, got {:?}", other),
    }
}
