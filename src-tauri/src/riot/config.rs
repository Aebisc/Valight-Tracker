// src-tauri/src/riot/config.rs
//
// Port of getApiConfig() from lib/valorant-api.ts.
// Reads entitlements from the Riot local client and region/version from
// ShooterGame.log. Caches the result until force=true or a 401/403 clears it.

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
    static BR: RegionEndpoints = RegionEndpoints { pd: "https://pd.na.a.pvp.net", glz: "https://glz-br-1.na.a.pvp.net", shard: "na" };
    static LATAM: RegionEndpoints = RegionEndpoints { pd: "https://pd.na.a.pvp.net", glz: "https://glz-latam-1.na.a.pvp.net", shard: "na" };
    match region {
        "eu" => &EU, "ap" => &AP, "kr" => &KR, "br" => &BR, "latam" => &LATAM, "na" => &NA,
        other => {
            tracing::warn!("Unknown region '{}', defaulting to NA endpoints", other);
            &NA
        }
    }
}

// ─── ApiConfig ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RiotEndpoints {
    pub local_base: String,
    pub pd: String,
    pub glz: String,
}

#[derive(Debug, Clone)]
pub struct ApiConfig {
    pub endpoints: RiotEndpoints,
    pub pd_url: String,
    pub glz_url: String,
    pub region: String,
    pub shard: String,
    pub puuid: String,
    pub version: String,
    /// Headers to attach to every Riot remote API request.
    pub headers: HeaderMap,
    /// Epoch timestamp (seconds) when the access token expires.
    pub expires_at: u64,
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

// ─── JWT parsing ─────────────────────────────────────────────────────────────

pub fn parse_jwt_exp(token: &str) -> Option<u64> {
    use base64::Engine;
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() < 2 {
        return None;
    }
    let payload_b64 = parts[1];
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload_b64)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(payload_b64))
        .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(payload_b64))
        .or_else(|_| base64::engine::general_purpose::STANDARD.decode(payload_b64))
        .ok()?;
    let json: serde_json::Value = serde_json::from_slice(&decoded).ok()?;
    json.get("exp").and_then(|v| v.as_u64())
}

// ─── Log parsing ──────────────────────────────────────────────────────────────

static GLZ_RE: Lazy<regex::Regex> = Lazy::new(|| {
    regex::Regex::new(r"https://glz-([a-z]+)-\d+\.([a-z]+)\.a\.pvp\.net").unwrap()
});
static VER_RE: Lazy<regex::Regex> = Lazy::new(|| {
    regex::Regex::new(r"release-(\d+\.\d+-shipping-\d+-\d+)").unwrap()
});

pub fn parse_log_metadata_from_str(content: &str) -> (String, String) {
    let mut region = "na".to_string();
    let mut version = "unknown".to_string();

    if let Some(cap) = GLZ_RE.captures(content) {
        region = cap[1].to_string();
    }
    if let Some(cap) = VER_RE.captures(content) {
        version = cap[1].to_string();
    }

    (region, version)
}

/// Reads at most 512 KB of ShooterGame.log in 64 KB chunks and extracts
/// the region (from a glz URL) and client version (from a release-X.Y string).
/// Exits early once both are found. Matches extractLogMetadata() in valorant-api.ts.
pub async fn extract_log_metadata() -> (String, String) {
    let local_app_data = match super::lockfile::local_app_data() {
        Ok(p) => p,
        Err(_) => return ("na".to_string(), "unknown".to_string()),
    };
    let log_path = local_app_data
        .join("VALORANT")
        .join("Saved")
        .join("Logs")
        .join("ShooterGame.log");

    let mut region = "na".to_string();
    let mut version = "unknown".to_string();
    let mut region_found = false;
    let mut version_found = false;

    let file = match tokio::fs::File::open(&log_path).await {
        Ok(f) => f,
        Err(e) => {
            tracing::warn!("Could not open ShooterGame.log: {}", e);
            return (region, version);
        }
    };

    use tokio::io::AsyncBufReadExt;
    let mut reader = tokio::io::BufReader::new(file);
    let max_bytes = 512 * 1024usize;
    let mut total_read = 0usize;
    let mut line = String::new();

    while total_read < max_bytes {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break,
            Ok(n) => {
                total_read += n;
                if !region_found {
                    if let Some(cap) = GLZ_RE.captures(&line) {
                        region = cap[1].to_string();
                        region_found = true;
                    }
                }
                if !version_found {
                    if let Some(cap) = VER_RE.captures(&line) {
                        version = cap[1].to_string();
                        version_found = true;
                    }
                }
                if region_found && version_found {
                    break;
                }
            }
            Err(e) => {
                tracing::warn!("Log read error: {}", e);
                break;
            }
        }
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
    let local_client = build_local_client();
    get_api_config_with_client_and_endpoints(&local_client, lockfile, force, None).await
}

pub async fn get_api_config_with_client_and_endpoints(
    local_client: &reqwest::Client,
    lockfile: &Lockfile,
    force: bool,
    endpoints_override: Option<RiotEndpoints>,
) -> Result<ApiConfig, ConfigError> {
    let cache_key = format!("{}:{}", lockfile.port, lockfile.password);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    if !force {
        let guard = CONFIG_CACHE.lock().await;
        if let Some(entry) = guard.as_ref() {
            if entry.cache_key == cache_key && entry.config.expires_at > now + 120 {
                tracing::debug!("ApiConfig cache hit");
                return Ok(entry.config.clone());
            }
        }
    }

    tracing::debug!("Fetching entitlements from port {}", lockfile.port);

    let default_local_base = format!("https://127.0.0.1:{}", lockfile.port);
    let local_base = endpoints_override
        .as_ref()
        .map(|ep| ep.local_base.as_str())
        .unwrap_or(&default_local_base);
    let url = format!("{}/entitlements/v1/token", local_base);

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
    if let Ok(val) = HeaderValue::from_str(&format!("Bearer {}", access_token.trim())) {
        headers.insert(reqwest::header::AUTHORIZATION, val);
    }
    if let Ok(val) = HeaderValue::from_str(entitlements_token.trim()) {
        headers.insert(HeaderName::from_static("x-riot-entitlements-jwt"), val);
    }
    if let Ok(val) = HeaderValue::from_str(&format!("release-{}", version.trim())) {
        headers.insert(HeaderName::from_static("x-riot-clientversion"), val);
    }
    if let Ok(val) = HeaderValue::from_str(&client_platform_header()) {
        headers.insert(HeaderName::from_static("x-riot-clientplatform"), val);
    }

    let endpoints = endpoints_override.unwrap_or_else(|| RiotEndpoints {
        local_base: default_local_base,
        pd: rc.pd.to_string(),
        glz: rc.glz.to_string(),
    });

    let expires_at = parse_jwt_exp(&access_token).unwrap_or(now + 1800);

    let config = ApiConfig {
        pd_url: endpoints.pd.clone(),
        glz_url: endpoints.glz.clone(),
        endpoints,
        region: region.clone(),
        shard: rc.shard.to_string(),
        puuid: puuid.clone(),
        version: version.clone(),
        headers,
        expires_at,
    };

    *CONFIG_CACHE.lock().await = Some(CacheEntry {
        cache_key,
        config: config.clone(),
    });

    tracing::info!("ApiConfig loaded: region={} version={} puuid={}...", region, version, &puuid[..8.min(puuid.len())]);
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex_compilation() {
        let glz_re = regex::Regex::new(r"https://glz-([a-z]+)-\d+\.([a-z]+)\.a\.pvp\.net").unwrap();
        let ver_re = regex::Regex::new(r"release-(\d+\.\d+-shipping-\d+-\d+)").unwrap();

        let sample_line = "LogNet: Browse: https://glz-eu-1.eu.a.pvp.net/sessions";
        let cap = glz_re.captures(sample_line).unwrap();
        assert_eq!(&cap[1], "eu");
        assert_eq!(&cap[2], "eu");

        let sample_ver = "CI server version: release-09.08-shipping-17-2917751";
        let cap_ver = ver_re.captures(sample_ver).unwrap();
        assert_eq!(&cap_ver[1], "09.08-shipping-17-2917751");
    }

    #[test]
    fn test_parse_log_metadata_regions() {
        let cases = [
            ("na", "https://glz-na-1.na.a.pvp.net", "release-09.08-shipping-17-2917751"),
            ("eu", "https://glz-eu-1.eu.a.pvp.net", "release-13.06-shipping-18-5590001"),
            ("ap", "https://glz-ap-1.ap.a.pvp.net", "release-10.01-shipping-1-1000000"),
            ("kr", "https://glz-kr-1.kr.a.pvp.net", "release-11.00-shipping-2-2000000"),
            ("latam", "https://glz-latam-1.na.a.pvp.net", "release-12.00-shipping-3-3000000"),
            ("br", "https://glz-br-1.na.a.pvp.net", "release-12.05-shipping-4-4000000"),
        ];

        for (expected_region, glz_url, ver_str) in cases {
            let log = format!("Log: Browse {}\nLog: CI server version: {}\n", glz_url, ver_str);
            let (region, version) = parse_log_metadata_from_str(&log);
            assert_eq!(region, expected_region);
            assert!(ver_str.contains(&version));
        }

        assert_eq!(region_config("latam").shard, "na");
        assert_eq!(region_config("latam").pd, "https://pd.na.a.pvp.net");
        assert_eq!(region_config("br").shard, "na");
        assert_eq!(region_config("br").pd, "https://pd.na.a.pvp.net");
    }

    #[test]
    fn test_parse_jwt_exp() {
        // Sample JWT payload: {"sub":"123","exp":1893456000}
        // Base64Url of {"sub":"123","exp":1893456000} is eyJzdWIiOiIxMjMiLCJleHAiOjE4OTM0NTYwMDB9
        let sample_jwt = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjMiLCJleHAiOjE4OTM0NTYwMDB9.signature";
        assert_eq!(parse_jwt_exp(sample_jwt), Some(1893456000));
        assert_eq!(parse_jwt_exp("invalid.token"), None);
    }

    #[test]
    fn test_parse_log_metadata_defaults_when_empty() {
        let (region, version) = parse_log_metadata_from_str("empty log without matches");
        assert_eq!(region, "na");
        assert_eq!(version, "unknown");
    }

    #[test]
    fn test_parse_log_metadata_fixture_log() {
        let fixture_log = include_str!("../../fixtures/redacted/pregame_comp/shootergame_log.txt");
        let (region, version) = parse_log_metadata_from_str(fixture_log);
        assert_eq!(region, "eu");
        assert_eq!(version, "13.06-shipping-18-5590001");
    }
}

