// src-tauri/src/riot/endpoints.rs
//
// All Riot API calls. Each function wraps one endpoint and returns
// serde_json::Value (never a strict struct) so Riot shape drift never breaks us.

use reqwest::Client;
use serde_json::Value;

use super::config::ApiConfig;
use super::client::{safe_get_json, safe_put_json, RiotResult};

// ─── Local (loopback) endpoints ───────────────────────────────────────────────

/// GET /chat/v4/presences — returns the raw presences response.
pub async fn get_presences(local_client: &Client, local_base: &str, basic_auth: &str) -> Option<Value> {
    let url = format!("{}/chat/v4/presences", local_base);
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::AUTHORIZATION,
        reqwest::header::HeaderValue::from_str(basic_auth).ok()?,
    );
    safe_get_json(local_client, &url, &headers).await.ok()
}

// ─── Remote (Riot server) endpoints ───────────────────────────────────────────

/// GET /pregame/v1/players/{puuid} — returns None if not in pregame.
pub async fn get_pregame_player_id(remote: &Client, cfg: &ApiConfig) -> RiotResult<Option<String>> {
    let url = format!("{}/pregame/v1/players/{}", cfg.glz_url, cfg.puuid);
    match safe_get_json(remote, &url, &cfg.headers).await {
        RiotResult::Ok(val) => RiotResult::Ok(val["MatchID"].as_str().map(|s| s.to_string())),
        RiotResult::NotFound => RiotResult::NotFound,
        RiotResult::Unauthorized => RiotResult::Unauthorized,
        RiotResult::RateLimited { retry_after } => RiotResult::RateLimited { retry_after },
        RiotResult::Transient(msg) => RiotResult::Transient(msg),
    }
}

/// GET /core-game/v1/players/{puuid} — returns None if not in core-game.
pub async fn get_coregame_player_id(remote: &Client, cfg: &ApiConfig) -> RiotResult<Option<String>> {
    let url = format!("{}/core-game/v1/players/{}", cfg.glz_url, cfg.puuid);
    match safe_get_json(remote, &url, &cfg.headers).await {
        RiotResult::Ok(val) => RiotResult::Ok(val["MatchID"].as_str().map(|s| s.to_string())),
        RiotResult::NotFound => RiotResult::NotFound,
        RiotResult::Unauthorized => RiotResult::Unauthorized,
        RiotResult::RateLimited { retry_after } => RiotResult::RateLimited { retry_after },
        RiotResult::Transient(msg) => RiotResult::Transient(msg),
    }
}

/// GET /pregame/v1/matches/{matchId}
pub async fn get_pregame_match(remote: &Client, cfg: &ApiConfig, match_id: &str) -> RiotResult<Value> {
    let url = format!("{}/pregame/v1/matches/{}", cfg.glz_url, match_id);
    safe_get_json(remote, &url, &cfg.headers).await
}

/// GET /core-game/v1/matches/{matchId}
pub async fn get_coregame_match(remote: &Client, cfg: &ApiConfig, match_id: &str) -> RiotResult<Value> {
    let url = format!("{}/core-game/v1/matches/{}", cfg.glz_url, match_id);
    safe_get_json(remote, &url, &cfg.headers).await
}

/// GET /mmr/v1/players/{puuid}
pub async fn get_player_mmr(remote: &Client, cfg: &ApiConfig, puuid: &str) -> Option<Value> {
    let url = format!("{}/mmr/v1/players/{}", cfg.pd_url, puuid);
    safe_get_json(remote, &url, &cfg.headers).await.ok()
}

/// GET /mmr/v1/players/{puuid}/competitiveupdates?startIndex=0&endIndex={count}&queue=competitive
pub async fn get_competitive_updates(
    remote: &Client,
    cfg: &ApiConfig,
    puuid: &str,
    count: u32,
) -> Option<Value> {
    let url = format!(
        "{}/mmr/v1/players/{}/competitiveupdates?startIndex=0&endIndex={}&queue=competitive",
        cfg.pd_url, puuid, count
    );
    safe_get_json(remote, &url, &cfg.headers).await.ok()
}

/// GET /match-details/v1/matches/{matchId}
pub async fn get_match_details(remote: &Client, cfg: &ApiConfig, match_id: &str) -> Option<Value> {
    let url = format!("{}/match-details/v1/matches/{}", cfg.pd_url, match_id);
    safe_get_json(remote, &url, &cfg.headers).await.ok()
}

/// PUT /name-service/v2/players — batch PUUID → display name lookup.
pub async fn get_names_from_puuids(
    remote: &Client,
    cfg: &ApiConfig,
    puuids: &[String],
) -> Vec<Value> {
    let url = format!("{}/name-service/v2/players", cfg.pd_url);
    let body = serde_json::to_value(puuids).unwrap_or(Value::Array(vec![]));
    let result = safe_put_json(remote, &url, &cfg.headers, &body).await;
    match result {
        RiotResult::Ok(Value::Array(arr)) => arr,
        _ => vec![],
    }
}

