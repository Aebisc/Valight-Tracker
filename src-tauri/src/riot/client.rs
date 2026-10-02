// src-tauri/src/riot/client.rs
//
// Two reqwest clients:
//   - local_client:  danger_accept_invalid_certs(true) — for 127.0.0.1 (Riot self-signed cert)
//   - remote_client: normal TLS verification — for pd and glz Riot servers
//
// Both share the same 8s timeout and are built once at startup, then stored as
// managed Tauri state so they're reused across every command invocation.

use std::time::Duration;
use reqwest::{Client, StatusCode};
use serde_json::Value;

const TIMEOUT: Duration = Duration::from_secs(8);
pub const MAX_RATE_LIMIT_RETRIES: usize = 1;
pub const MAX_RATE_LIMIT_WAIT_SECS: u64 = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RiotResult<T> {
    Ok(T),
    NotFound,
    Unauthorized,
    RateLimited { retry_after: Option<Duration> },
    Transient(String),
}

impl<T> RiotResult<T> {
    pub fn ok(self) -> Option<T> {
        match self {
            RiotResult::Ok(v) => Some(v),
            _ => None,
        }
    }
}

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

/// Helper: GET a URL and parse as JSON, with typed RiotResult and 429 retry support.
pub async fn safe_get_json(
    client: &Client,
    url: &str,
    headers: &reqwest::header::HeaderMap,
) -> RiotResult<Value> {
    let mut retries = 0;
    let req_builder = client.get(url).headers(headers.clone());
    loop {
        let req = match req_builder.try_clone() {
            Some(b) => b,
            None => client.get(url).headers(headers.clone()),
        };
        let res = match req.send().await {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("safe_get_json send error for {}: {}", url, e);
                return RiotResult::Transient(e.to_string());
            }
        };

        let status = res.status();
        if status == StatusCode::NOT_FOUND {
            return RiotResult::NotFound;
        }
        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            return RiotResult::Unauthorized;
        }
        if status == StatusCode::TOO_MANY_REQUESTS {
            let retry_after_dur = res
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .map(Duration::from_secs);

            if retries < MAX_RATE_LIMIT_RETRIES {
                retries += 1;
                let wait = retry_after_dur
                    .unwrap_or(Duration::from_secs(1))
                    .min(Duration::from_secs(MAX_RATE_LIMIT_WAIT_SECS));
                tracing::warn!("429 Rate limited on {}, retrying after {:?}", url, wait);
                tokio::time::sleep(wait).await;
                continue;
            }
            return RiotResult::RateLimited { retry_after: retry_after_dur };
        }
        if !status.is_success() {
            tracing::warn!("safe_get_json: {} returned {}", url, status);
            return RiotResult::Transient(format!("HTTP {}", status));
        }

        let val: Value = match res.json().await {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("safe_get_json json parse error for {}: {}", url, e);
                return RiotResult::Transient(e.to_string());
            }
        };

        // Riot error envelope: { httpStatus: 4xx, message: "..." }
        if let Some(envelope_status) = val.get("httpStatus").and_then(|v| v.as_u64()) {
            if envelope_status == 404 {
                return RiotResult::NotFound;
            }
            if envelope_status == 401 || envelope_status == 403 {
                return RiotResult::Unauthorized;
            }
            if envelope_status == 429 {
                return RiotResult::RateLimited { retry_after: None };
            }
            if envelope_status >= 400 {
                tracing::warn!("safe_get_json: Riot error envelope from {} — httpStatus {}", url, envelope_status);
                return RiotResult::Transient(format!("Riot error envelope httpStatus {}", envelope_status));
            }
        }

        return RiotResult::Ok(val);
    }
}

/// Helper: PUT a URL with a JSON body, returning RiotResult with 429 retry support.
pub async fn safe_put_json<T: serde::Serialize + ?Sized>(
    client: &Client,
    url: &str,
    headers: &reqwest::header::HeaderMap,
    body: &T,
) -> RiotResult<Value> {
    let mut retries = 0;
    let req_builder = client.put(url).headers(headers.clone()).json(body);
    loop {
        let req = match req_builder.try_clone() {
            Some(b) => b,
            None => client.put(url).headers(headers.clone()).json(body),
        };
        let res = match req.send().await {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("safe_put_json send error for {}: {}", url, e);
                return RiotResult::Transient(e.to_string());
            }
        };

        let status = res.status();
        if status == StatusCode::NOT_FOUND {
            return RiotResult::NotFound;
        }
        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            return RiotResult::Unauthorized;
        }
        if status == StatusCode::TOO_MANY_REQUESTS {
            let retry_after_dur = res
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .map(Duration::from_secs);

            if retries < MAX_RATE_LIMIT_RETRIES {
                retries += 1;
                let wait = retry_after_dur
                    .unwrap_or(Duration::from_secs(1))
                    .min(Duration::from_secs(MAX_RATE_LIMIT_WAIT_SECS));
                tracing::warn!("429 Rate limited on PUT {}, retrying after {:?}", url, wait);
                tokio::time::sleep(wait).await;
                continue;
            }
            return RiotResult::RateLimited { retry_after: retry_after_dur };
        }
        if !status.is_success() {
            tracing::warn!("safe_put_json: {} returned {}", url, status);
            return RiotResult::Transient(format!("HTTP {}", status));
        }

        match res.json().await {
            Ok(val) => return RiotResult::Ok(val),
            Err(e) => return RiotResult::Transient(e.to_string()),
        }
    }
}

