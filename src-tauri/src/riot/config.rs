// src-tauri/src/riot/config.rs
//
// Port of getApiConfig() from lib/valorant-api.ts.
// Reads entitlements from the Riot local client and region/version from
// ShooterGame.log. Caches the result until force=true or a 401/403 clears it.

use std::path::PathBuf;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use tokio::sync::Mutex;
use thiserror::Error;
use once_cell::sync::Lazy;

use super::lockfile::Lockfile;
use super::client::build_local_client;

// ─── Region config ────────────────────────────────────────────────────────────

struct RegionEndpoints {
    pd: &'static str,
    glz: &'static str,
    shard: &'static str,
}

fn region_config(region: &str) -> &'static RegionEndpoints {
    static NA: RegionEndpoints = RegionEndpoints { pd: "https://pd.na.a.pvp.net", glz: "https://glz-na-1.na.a.pvp.net", shard: "na" };
    static EU: RegionEndpoints = RegionEndpoints { pd: "https://pd.eu.a.pvp.net", glz: "https://glz-eu-1.eu.a.pvp.net", shard: "eu" };
    static AP: RegionEndpoints = RegionEndpoints { pd: "https://pd.ap.a.pvp.net", glz: "https://glz-ap-1.ap.a.pvp.net", shard: "ap" };
    static KR: RegionEndpoints = RegionEndpoints { pd: "https://pd.kr.a.pvp.net", glz: "https://glz-kr-1.kr.a.pvp.net", shard: "kr" };
    static BR: RegionEndpoints = RegionEndpoints { pd: "https://pd.br.a.pvp.net", glz: "https://glz-br-1.br.a.pvp.net", shard: "br" };
    static LATAM: RegionEndpoints = RegionEndpoints { pd: "https://pd.latam.a.pvp.net", glz: "https://glz-latam-1.latam.a.pvp.net", shard: "latam" };
    match region {
        "eu" => &EU, "ap" => &AP, "kr" => &KR, "br" => &BR, "latam" => &LATAM, _ => &NA,
    }
}

// ─── ApiConfig ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ApiConfig {
    pub pd_url: String,
    pub glz_url: String,
    pub region: String,
    pub shard: String,
    pub puuid: String,
    pub version: String,
    /// Headers to attach to every Riot remote API request.
    pub headers: HeaderMap,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Valorant is not running or entitlements unavailable")]
    EntitlementsUnavailable,
    #[error("HTTP error fetching entitlements: {0}")]
    Http(#[from] reqwest::Error),
}

// ─── Cache ────────────────────────────────────────────────────────────────────

struct CacheEntry {
    cache_key: String, // "{port}:{password}"
    config: ApiConfig,
}

static CONFIG_CACHE: Lazy<Mutex<Option<CacheEntry>>> = Lazy::new(|| Mutex::new(None));

pub async fn clear_config_cache() {
    *CONFIG_CACHE.lock().await = None;
}

// ─── Log parsing ──────────────────────────────────────────────────────────────

/// Reads at most 512 KB of ShooterGame.log in 64 KB chunks and extracts
/// the region (from a glz URL) and client version (from a release-X.Y string).
/// Exits early once both are found. Matches extractLogMetadata() in valorant-api.ts.
async fn extract_log_metadata() -> (String, String) {
    let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_default();
    let log_path = PathBuf::from(&local_app_data)
        .join("VALORANT")
        .join("Saved")
        .join("Logs")
        .join("ShooterGame.log");

    let mut region = "na".to_string();
    let mut version = "unknown".to_string();

    let file = match tokio::fs::File::open(&log_path).await {
        Ok(f) => f,
        Err(e) => {
            tracing::warn!("Could not open ShooterGame.log: {}", e);
            return (region, version);
        }
    };

    use tokio::io::AsyncReadExt;
    let mut reader = tokio::io::BufReader::new(file);
    let chunk_size = 64 * 1024usize;
    let max_bytes = 512 * 1024usize;
    let mut accumulated = String::new();
    let mut total_read = 0usize;
    let mut buf = vec![0u8; chunk_size];

    let glz_re = regex::Regex::new(r"https://glz-([a-z]+)-\d+\.\1\.a\.pvp\.net").unwrap();
    let ver_re = regex::Regex::new(r"release-(\d+\.\d+-shipping-\d+-\d+)").unwrap();

    loop {
        if total_read >= max_bytes { break; }
        let n = match reader.read(&mut buf).await {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) => { tracing::warn!("Log read error: {}", e); break; }
        };
        total_read += n;
        accumulated.push_str(&String::from_utf8_lossy(&buf[..n]));

        if region == "na" {
            if let Some(cap) = glz_re.captures(&accumulated) {
                region = cap[1].to_string();
            }
        }
        if version == "unknown" {
            if let Some(cap) = ver_re.captures(&accumulated) {
                version = cap[1].to_string();
            }
        }
        if region != "na" && version != "unknown" { break; }
    }

    (region, version)
}

// ─── Client platform header ───────────────────────────────────────────────────

fn client_platform_header() -> String {
    use base64::Engine;
    let platform = serde_json::json!({
        "platformType": "PC",
        "platformOS": "Windows",
        "platformOSVersion": "10.0.19042.1.256.64bit",
        "platformChipset": "Unknown"
    });
    base64::engine::general_purpose::STANDARD.encode(platform.to_string().as_bytes())
}

// ─── Main entry point ─────────────────────────────────────────────────────────

/// Port of getApiConfig(). Cached by (port, password) pair.
pub async fn get_api_config(lockfile: &Lockfile, force: bool) -> Result<ApiConfig, ConfigError> {
    let cache_key = format!("{}:{}", lockfile.port, lockfile.password);

    if !force {
        let guard = CONFIG_CACHE.lock().await;
        if let Some(entry) = guard.as_ref() {
            if entry.cache_key == cache_key {
                tracing::debug!("ApiConfig cache hit");
                return Ok(entry.config.clone());
            }
        }
    }

    tracing::debug!("Fetching entitlements from port {}", lockfile.port);

    // Use a temporary local client just for the entitlements call
    // (The shared one is in AppState, but config.rs needs to work standalone too)
    let local_client = build_local_client();
    let url = format!("https://127.0.0.1:{}/entitlements/v1/token", lockfile.port);

    let res = local_client
        .get(&url)
        .header("Authorization", &lockfile.basic_auth)
        .send()
        .await?;

    let entitlements: serde_json::Value = res.json().await?;

    let puuid = entitlements["subject"].as_str().unwrap_or("").to_string();
    let access_token = entitlements["accessToken"].as_str().unwrap_or("").to_string();
    let entitlements_token = entitlements["token"].as_str().unwrap_or("").to_string();

    if puuid.is_empty() || access_token.is_empty() {
        return Err(ConfigError::EntitlementsUnavailable);
    }

    let (region, version) = extract_log_metadata().await;
    let rc = region_config(&region);

    let mut headers = HeaderMap::new();
    headers.insert(
        reqwest::header::AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", access_token)).unwrap(),
    );
    headers.insert(
        HeaderName::from_static("x-riot-entitlements-jwt"),
        HeaderValue::from_str(&entitlements_token).unwrap(),
    );
    headers.insert(
        HeaderName::from_static("x-riot-clientversion"),
        HeaderValue::from_str(&format!("release-{}", version)).unwrap(),
    );
    headers.insert(
        HeaderName::from_static("x-riot-clientplatform"),
        HeaderValue::from_str(&client_platform_header()).unwrap(),
    );

    let config = ApiConfig {
        pd_url: rc.pd.to_string(),
        glz_url: rc.glz.to_string(),
        region: region.clone(),
        shard: rc.shard.to_string(),
        puuid: puuid.clone(),
        version: version.clone(),
        headers,
    };

    *CONFIG_CACHE.lock().await = Some(CacheEntry {
        cache_key,
        config: config.clone(),
    });

    tracing::info!("ApiConfig loaded: region={} version={} puuid={}...", region, version, &puuid[..8.min(puuid.len())]);
    Ok(config)
}
