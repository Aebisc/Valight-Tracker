// src-tauri/src/riot/client.rs
//
// Two reqwest clients:
//   - local_client:  danger_accept_invalid_certs(true) — for 127.0.0.1 (Riot self-signed cert)
//   - remote_client: normal TLS verification — for pd and glz Riot servers
//
// Both share the same 8s timeout and are built once at startup, then stored as
// managed Tauri state so they're reused across every command invocation.

use std::time::Duration;
use reqwest::Client;

const TIMEOUT: Duration = Duration::from_secs(8);

/// Builds the local client that accepts Riot's self-signed localhost cert.
/// ONLY used for 127.0.0.1 endpoints.
pub fn build_local_client() -> Client {
    Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(TIMEOUT)
        .build()
        .expect("failed to build local reqwest client")
}

/// Builds the normal TLS-verified remote client for Riot's servers.
pub fn build_remote_client() -> Client {
    Client::builder()
        .timeout(TIMEOUT)
        .build()
        .expect("failed to build remote reqwest client")
}

/// Helper: GET a URL and parse as JSON, returning None on any non-2xx or parse error.
pub async fn safe_get_json(client: &Client, url: &str, headers: &reqwest::header::HeaderMap) -> Option<serde_json::Value> {
    let res = client
        .get(url)
        .headers(headers.clone())
        .send()
        .await
        .ok()?;

    if !res.status().is_success() {
        tracing::warn!("safe_get_json: {} returned {}", url, res.status());
        return None;
    }

    let val: serde_json::Value = res.json().await.ok()?;

    // Riot error envelope: { httpStatus: 4xx, message: "..." }
    if let Some(status) = val.get("httpStatus").and_then(|v| v.as_u64()) {
        if status >= 400 {
            tracing::warn!("safe_get_json: Riot error envelope from {} — httpStatus {}", url, status);
            return None;
        }
    }

    Some(val)
}

/// Helper: PUT a URL with a JSON body, returning None on errors.
pub async fn safe_put_json(
    client: &Client,
    url: &str,
    headers: &reqwest::header::HeaderMap,
    body: &serde_json::Value,
) -> Option<serde_json::Value> {
    let res = client
        .put(url)
        .headers(headers.clone())
        .json(body)
        .send()
        .await
        .ok()?;

    if !res.status().is_success() {
        tracing::warn!("safe_put_json: {} returned {}", url, res.status());
        return None;
    }

    res.json().await.ok()
}
