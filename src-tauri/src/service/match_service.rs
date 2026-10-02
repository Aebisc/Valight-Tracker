// src-tauri/src/service/match_service.rs
//
// Port of the GET handler in app/api/match/route.ts.
// This is the orchestrator — Phase 3 of the migration plan.
// It calls all the riot/* and service/* functions in the right order.
//
// Status: PHASE 3 STUB — structure is correct, implementation TBD.
// The full implementation follows the plan's Phase 3 ordering:
//   1. Offline (no lockfile)
//   2. Menus (no match)
//   3. INGAME fast-path (cache hit)
//   4. PREGAME build
//   5. INGAME build
//   6. Per-player MMR + comp updates (concurrent)
//   7. Match detail fetches (buffer_unordered)
//   8. Presence + party
//   9. Cache write

use std::sync::Arc;
use std::collections::{HashMap, HashSet};

use futures::stream::{self, StreamExt};
use serde_json::Value;


use crate::model::{ApiError, ApiResponse, MatchInfo, ValorantPlayer};
use crate::state::{AppState, MatchCache};
use crate::riot::{lockfile, config, endpoints, client::RiotResult};
use crate::service::{player, stats, party, side};

const RECENT_GAMES_COUNT: u32 = 20;
const DETAIL_CONCURRENCY: usize = 10;
const MMR_CONCURRENCY: usize = 4;

// ─── Data maps (loaded once at startup) ───────────────────────────────────────

use once_cell::sync::Lazy;

static RANK_MAP: Lazy<HashMap<u64, String>> = Lazy::new(|| {
    let raw = include_str!("../../data/rank_map.json");
    serde_json::from_str(raw).expect("rank_map.json is invalid")
});

static AGENT_MAP: Lazy<HashMap<String, String>> = Lazy::new(|| {
    let raw = include_str!("../../data/agent_map.json");
    serde_json::from_str(raw).expect("agent_map.json is invalid")
});

static MAP_MAP: Lazy<HashMap<String, String>> = Lazy::new(|| {
    let raw = include_str!("../../data/map_map.json");
    serde_json::from_str(raw).expect("map_map.json is invalid")
});

static GAMEMODE_MAP: Lazy<HashMap<String, String>> = Lazy::new(|| {
    let raw = include_str!("../../data/gamemode_map.json");
    serde_json::from_str(raw).expect("gamemode_map.json is invalid")
});

static DEATHMATCH_MODES: Lazy<HashSet<String>> = Lazy::new(|| {
    let raw = include_str!("../../data/deathmatch_modes.json");
    let arr: Vec<String> = serde_json::from_str(raw).expect("deathmatch_modes.json is invalid");
    arr.into_iter().collect()
});

// ─── Map / mode resolution helpers ────────────────────────────────────────────

fn strip_extension(s: &str) -> &str {
    if let Some(pos) = s.rfind('.') {
        if !s[pos..].contains('/') {
            return &s[..pos];
        }
    }
    s
}

pub fn strip_and_extract(path: &str) -> (String, String) {
    let stripped = strip_extension(path);
    let segs: Vec<&str> = stripped.split('/').filter(|s| !s.is_empty()).collect();
    let keyword = if segs.len() >= 3 { segs[2].to_string() } else { String::new() };
    (stripped.to_string(), keyword)
}

pub fn resolve_map_name(map_id: &str) -> String {
    let map_lower = map_id.to_lowercase();
    let map_lower_stripped = strip_extension(&map_lower);
    if let Some(name) = MAP_MAP.get(map_id)
        .or_else(|| MAP_MAP.get(map_lower_stripped))
        .or_else(|| MAP_MAP.get(&map_lower)) {
        return name.clone();
    }
    // Fallback: extract last path segment, strip extension, replace '_' with ' '
    let last = map_id.split('/').last().unwrap_or("");
    let no_ext = strip_extension(last).replace('_', " ");

    static WORD_RE: Lazy<regex::Regex> = Lazy::new(|| regex::Regex::new(r"\b\w").unwrap());
    let title_cased = WORD_RE.replace_all(&no_ext, |cap: &regex::Captures| {
        cap[0].to_uppercase()
    });

    if title_cased.is_empty() {
        "Unknown".to_string()
    } else {
        title_cased.into_owned()
    }
}

pub fn resolve_game_mode_name(queue_id: &str, mode: &str, is_ranked: bool) -> (String, bool, String) {
    let queue_lower = queue_id.to_lowercase();
    let mode_lower = mode.to_lowercase();
    let (q_stripped, q_keyword) = strip_and_extract(&queue_lower);
    let (m_stripped, m_keyword) = strip_and_extract(&mode_lower);
    let mode_keyword = if !q_keyword.is_empty() { q_keyword.clone() } else { m_keyword.clone() };

    let clean_fallback = if !mode_keyword.is_empty() {
        let mut c = mode_keyword.chars();
        match c.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().to_string() + c.as_str().to_lowercase().as_str(),
        }
    } else if !queue_id.is_empty() && !queue_id.contains('/') {
        let mut c = queue_id.chars();
        match c.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().to_string() + c.as_str().to_lowercase().as_str(),
        }
    } else { "Unknown".to_string() };

    let mut name = GAMEMODE_MAP.get(&queue_lower)
        .or_else(|| GAMEMODE_MAP.get(&mode_lower))
        .or_else(|| GAMEMODE_MAP.get(&q_stripped))
        .or_else(|| GAMEMODE_MAP.get(&m_stripped))
        .or_else(|| GAMEMODE_MAP.get(&q_keyword))
        .or_else(|| GAMEMODE_MAP.get(&m_keyword))
        .cloned()
        .unwrap_or(clean_fallback);

    if name == "Standard" {
        name = if is_ranked { "Competitive".to_string() } else { "Unrated".to_string() };
    }

    let is_dm = DEATHMATCH_MODES.contains(&queue_lower) || DEATHMATCH_MODES.contains(&mode_keyword);
    (name, is_dm, mode_keyword)
}

// ─── State detection ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectedState {
    Ingame(String),
    Pregame(String),
    Menus,
}

pub async fn detect_state(
    remote: &reqwest::Client,
    local: &reqwest::Client,
    lockfile: &lockfile::Lockfile,
    endpoints: Option<config::RiotEndpoints>,
    cfg: &mut config::ApiConfig,
) -> Result<DetectedState, ApiError> {
    let mut core_res = endpoints::get_coregame_player_id(remote, cfg).await;
    if core_res == RiotResult::Unauthorized {
        config::clear_config_cache().await;
        match config::get_api_config_with_client_and_endpoints(local, lockfile, true, endpoints.clone()).await {
            Ok(c) => {
                *cfg = c;
                core_res = endpoints::get_coregame_player_id(remote, cfg).await;
            }
            Err(_) => return Err(ApiError::auth("Unauthorized: failed to refresh credentials")),
        }
    }

    match core_res {
        RiotResult::Ok(Some(mid)) if !mid.is_empty() => Ok(DetectedState::Ingame(mid)),
        RiotResult::NotFound | RiotResult::Ok(_) => {
            let mut pre_res = endpoints::get_pregame_player_id(remote, cfg).await;
            if pre_res == RiotResult::Unauthorized {
                config::clear_config_cache().await;
                match config::get_api_config_with_client_and_endpoints(local, lockfile, true, endpoints).await {
                    Ok(c) => {
                        *cfg = c;
                        pre_res = endpoints::get_pregame_player_id(remote, cfg).await;
                    }
                    Err(_) => return Err(ApiError::auth("Unauthorized: failed to refresh credentials")),
                }
            }
            match pre_res {
                RiotResult::Ok(Some(mid)) if !mid.is_empty() => Ok(DetectedState::Pregame(mid)),
                RiotResult::NotFound | RiotResult::Ok(_) => Ok(DetectedState::Menus),
                RiotResult::Unauthorized => Err(ApiError::auth("Unauthorized by Riot API")),
                RiotResult::RateLimited { .. } => Err(ApiError::transient("Rate limited by Riot API")),
                RiotResult::Transient(msg) => Err(ApiError::transient(msg)),
            }
        }
        RiotResult::Unauthorized => Err(ApiError::auth("Unauthorized by Riot API")),
        RiotResult::RateLimited { .. } => Err(ApiError::transient("Rate limited by Riot API")),
        RiotResult::Transient(msg) => Err(ApiError::transient(msg)),
    }
}

// ─── Main command entry point ──────────────────────────────────────────────────

/// The `get_match` Tauri command. Equivalent to the GET /api/match handler.
pub async fn get_match(force: bool, state: &AppState) -> Result<ApiResponse, ApiError> {
    // Force-clear caches
    if force {
        *state.match_cache.lock().await = None;
        config::clear_config_cache().await;
    }

    // 1. Read lockfile — OFFLINE if absent
    let lockfile = match lockfile::read_lockfile().await {
        Ok(lf) => lf,
        Err(lockfile::LockfileError::NotFound { .. }) => return Ok(ApiResponse::offline()),
        Err(e) => return Err(ApiError::internal(e.to_string())),
    };

    // 2. Get API config — may fail if Valorant not running
    let mut cfg = match config::get_api_config_with_client_and_endpoints(
        &state.local_client,
        &lockfile,
        force,
        state.endpoints_override.clone(),
    ).await {
        Ok(c) => c,
        Err(config::ConfigError::EntitlementsUnavailable) => return Ok(ApiResponse::offline()),
        Err(config::ConfigError::Http(e)) => {
            if e.is_connect() {
                return Ok(ApiResponse::offline());
            }
            return Err(ApiError::transient(format!("Entitlements error: {}", e)));
        }
    };

    // 3. State detection with typed errors and auth re-try
    let detected = detect_state(
        &state.remote_client,
        &state.local_client,
        &lockfile,
        state.endpoints_override.clone(),
        &mut cfg,
    ).await?;

    // INGAME fast-path: cache hit for the same match
    if let DetectedState::Ingame(ref mid) = detected {
        let cache = state.match_cache.lock().await;
        if let Some(ref cached) = *cache {
            if &cached.match_id == mid && cached.game_state == "INGAME" {
                return Ok(ApiResponse {
                    game_state: cached.game_state.clone(),
                    r#match: Some(cached.match_info.clone()),
                    players: Some(cached.players.clone()),
                    self_puuid: Some(cfg.puuid.clone()),
                    error: None,
                });
            }
        }
    }

    // MENUS
    if detected == DetectedState::Menus {
        *state.match_cache.lock().await = None;
        return Ok(ApiResponse::menus(cfg.puuid.clone()));
    }

    // We're in a match — acquire the single-flight build lock
    let _build_guard = state.build_lock.lock().await;

    // Re-check cache under the lock (another concurrent call may have just built it)
    if let DetectedState::Ingame(ref mid) = detected {
        let cache = state.match_cache.lock().await;
        if let Some(ref cached) = *cache {
            if &cached.match_id == mid && cached.game_state == "INGAME" {
                return Ok(ApiResponse {
                    game_state: cached.game_state.clone(),
                    r#match: Some(cached.match_info.clone()),
                    players: Some(cached.players.clone()),
                    self_puuid: Some(cfg.puuid.clone()),
                    error: None,
                });
            }
        }
    }

    // 6. Fetch presences for party detection
    let presences_raw = endpoints::get_presences(
        &state.local_client,
        &cfg.endpoints.local_base,
        &lockfile.basic_auth,
    ).await;
    let presences = presences_raw.as_ref()
        .map(|v| party::parse_presences(v))
        .unwrap_or_default();

    // 7. Fetch match data
    let (resolved_match_id, resolved_game_state, raw_players, map_id, game_mode, game_mode_id,
         is_ranked, server, season_id, ally_team_id) = match detected {
        DetectedState::Ingame(ref mid) => {
            let mut cg = match endpoints::get_coregame_match(&state.remote_client, &cfg, mid).await {
                RiotResult::Ok(v) => v,
                RiotResult::NotFound => {
                    *state.match_cache.lock().await = None;
                    return Ok(ApiResponse::menus(cfg.puuid.clone()));
                }
                RiotResult::Unauthorized => return Err(ApiError::auth("Unauthorized fetching coregame match")),
                RiotResult::RateLimited { .. } => return Err(ApiError::transient("Rate limited fetching coregame match")),
                RiotResult::Transient(msg) => return Err(ApiError::transient(msg)),
            };
            let players: Vec<Value> = cg["Players"].as_array_mut().map(std::mem::take).unwrap_or_default();
            let game_mode_id = cg["QueueID"].as_str().or_else(|| cg["ModeID"].as_str()).unwrap_or("").to_string();
            let is_ranked = cg["IsRanked"].as_bool().unwrap_or(game_mode_id == "competitive");
            (
                mid.clone(),
                "INGAME".to_string(),
                players,
                cg["MapID"].as_str().unwrap_or("").to_string(),
                cg["Mode"].as_str().unwrap_or("").to_string(),
                game_mode_id,
                is_ranked,
                cg["GamePodID"].as_str().unwrap_or("").to_string(),
                cg["SeasonID"].as_str().unwrap_or("").to_string(),
                None,
            )
        }
        DetectedState::Pregame(ref mid) => {
            let mut pg = match endpoints::get_pregame_match(&state.remote_client, &cfg, mid).await {
                RiotResult::Ok(v) => v,
                RiotResult::NotFound => {
                    *state.match_cache.lock().await = None;
                    return Ok(ApiResponse::menus(cfg.puuid.clone()));
                }
                RiotResult::Unauthorized => return Err(ApiError::auth("Unauthorized fetching pregame match")),
                RiotResult::RateLimited { .. } => return Err(ApiError::transient("Rate limited fetching pregame match")),
                RiotResult::Transient(msg) => return Err(ApiError::transient(msg)),
            };
            let ally_team_id: Option<String> = pg["AllyTeam"]["TeamID"].as_str().map(|s| s.to_string());
            let enemy_team_id = match ally_team_id.as_deref() {
                Some("Blue") => Some("Red".to_string()),
                Some("Red")  => Some("Blue".to_string()),
                _            => pg["EnemyTeam"]["TeamID"].as_str().map(|s| s.to_string()),
            };
            let ally_tid = ally_team_id.as_deref().unwrap_or("Blue").to_string();
            let enemy_tid = enemy_team_id.as_deref().unwrap_or("Red").to_string();
            let mut players = pg["AllyTeam"]["Players"].as_array_mut().map(std::mem::take).unwrap_or_default();
            for p in &mut players {
                if p.get("TeamID").is_none() || p["TeamID"].is_null() {
                    p["TeamID"] = Value::String(ally_tid.clone());
                }
            }
            let mut enemy_players = pg["EnemyTeam"]["Players"].as_array_mut().map(std::mem::take).unwrap_or_default();
            for p in &mut enemy_players {
                if p.get("TeamID").is_none() || p["TeamID"].is_null() {
                    p["TeamID"] = Value::String(enemy_tid.clone());
                }
            }
            players.extend(enemy_players);
            (
                mid.clone(),
                "PREGAME".to_string(),
                players,
                pg["MapID"].as_str().unwrap_or("").to_string(),
                pg["Mode"].as_str().unwrap_or("").to_string(),
                pg["QueueID"].as_str().or_else(|| pg["ModeID"].as_str()).unwrap_or("").to_string(),
                pg["IsRanked"].as_bool().unwrap_or(false),
                String::new(),
                pg["SeasonID"].as_str().unwrap_or("").to_string(),
                ally_team_id,
            )
        }
        DetectedState::Menus => unreachable!(),
    };


    // Invalidate match cache if match changed
    {
        let mut cache = state.match_cache.lock().await;
        if let Some(ref c) = *cache {
            if c.match_id != resolved_match_id || c.game_state != resolved_game_state {
                *cache = None;
            }
        }
    }

    // PREGAME cache hit: just update agent selections
    {
        let mut cache = state.match_cache.lock().await;
        if let Some(ref mut cached) = *cache {
            if cached.match_id == resolved_match_id && cached.game_state == resolved_game_state {
                if resolved_game_state == "PREGAME" {
                    for p in cached.players.iter_mut() {
                        if let Some(fresh) = raw_players.iter().find(|fp| {
                            fp["Subject"].as_str() == Some(&p.puuid) || fp["PlayerIdentity"]["Subject"].as_str() == Some(&p.puuid)
                        }) {
                            let fresh_agent_id = fresh["CharacterID"].as_str()
                                .or_else(|| fresh["CharacterSelectionID"].as_str())
                                .unwrap_or("");
                            if !fresh_agent_id.is_empty() && fresh_agent_id != p.agent_id {
                                p.agent_id = fresh_agent_id.to_string();
                                p.agent_name = AGENT_MAP.get(fresh_agent_id)
                                    .cloned()
                                    .unwrap_or_else(|| "Unknown".to_string());
                            }
                        }
                    }
                }
                if !presences.is_empty() {
                    party::assign_party_numbers(&mut cached.players, &presences);
                }
                return Ok(ApiResponse {
                    game_state: cached.game_state.clone(),
                    r#match: Some(cached.match_info.clone()),
                    players: Some(cached.players.clone()),
                    self_puuid: Some(cfg.puuid.clone()),
                    error: None,
                });
            }

        }
    }

    // 8. Full build: fetch names, MMR, comp updates, match details
    let puuids: Vec<String> = raw_players.iter()
        .filter_map(|p| p["Subject"].as_str().or_else(|| p["PlayerIdentity"]["Subject"].as_str()))
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let remote_client = state.remote_client.clone();
    let cfg_clone = cfg.clone();
    let mmr_stream = stream::iter(puuids.clone())
        .map(move |puuid| {
            let remote = remote_client.clone();
            let cfg_ref = cfg_clone.clone();
            async move {
                let (mmr, comp) = tokio::join!(
                    endpoints::get_player_mmr(&remote, &cfg_ref, &puuid),
                    endpoints::get_competitive_updates(&remote, &cfg_ref, &puuid, RECENT_GAMES_COUNT),
                );
                (puuid, (mmr, comp))
            }
        })
        .buffer_unordered(MMR_CONCURRENCY);

    let (names_raw, mmr_comp_results): (Vec<Value>, HashMap<String, (Option<Value>, Option<Value>)>) = tokio::join!(
        endpoints::get_names_from_puuids(&state.remote_client, &cfg, &puuids),
        mmr_stream.collect()
    );

    let name_map: HashMap<String, (String, String)> = names_raw.into_iter()
        .filter_map(|n| {
            let s = n["Subject"].as_str()?.to_string();
            let name = n["GameName"].as_str().or_else(|| n["DisplayName"].as_str()).unwrap_or("").to_string();
            let tag = n["TagLine"].as_str().unwrap_or("").to_string();
            Some((s, (name, tag)))
        })
        .collect();

    // Build intermediate player data and collect recent match IDs
    struct RawPlayer {
        built: ValorantPlayer,
        recent_match_ids: Vec<String>,
    }

    let mut raw_built: Vec<RawPlayer> = Vec::new();
    for p in &raw_players {
        let puuid = match p["Subject"].as_str().or_else(|| p["PlayerIdentity"]["Subject"].as_str()) {
            Some(s) if !s.is_empty() => s,
            _ => continue,
        };
        let (mmr_raw, comp_raw) = match mmr_comp_results.get(puuid) {
            Some(v) => v,
            None => continue,
        };

        let mmr_data = mmr_raw.as_ref()
            .filter(|v| v["httpStatus"].is_null() || v["httpStatus"].as_u64().unwrap_or(0) < 400);
        let latest_comp = mmr_data.map(|m| &m["LatestCompetitiveUpdate"]);
        let seasonal_info = mmr_data
            .map(|m| &m["QueueSkills"]["competitive"]["SeasonalInfoBySeasonID"])
            .unwrap_or(&Value::Null);

        let recent_matches: Vec<Value> = comp_raw.as_ref()
            .and_then(|c| c["Matches"].as_array())
            .cloned()
            .unwrap_or_default();

        let rank = latest_comp.and_then(|lc| lc["TierAfterUpdate"].as_u64()).unwrap_or(0) as u32;
        let previous_rank = latest_comp.and_then(|lc| lc["TierBeforeUpdate"].as_u64()).unwrap_or(0) as u32;

        let mut peak_rank = player::get_peak_rank(seasonal_info, &recent_matches, latest_comp);
        if rank > peak_rank && rank <= 27 { peak_rank = rank; }
        if previous_rank > peak_rank && previous_rank <= 27 { peak_rank = previous_rank; }

        let latest_match_season_id = recent_matches.first()
            .and_then(|m| m["SeasonID"].as_str())
            .unwrap_or("")
            .to_string();
        let current_season = if !latest_match_season_id.is_empty() {
            Some(latest_match_season_id.clone())
        } else {
            player::get_current_season_id(seasonal_info, &season_id)
        };

        let current_season_data = current_season.as_deref()
            .and_then(|cs| seasonal_info[cs].as_object().map(|_| &seasonal_info[cs]));
        let current_season_wins = current_season_data
            .and_then(|d| d["NumberOfWinsWithPlacements"].as_u64().or_else(|| d["NumberOfWins"].as_u64()))
            .unwrap_or(0) as u32;
        let current_season_games = current_season_data
            .and_then(|d| d["NumberOfGames"].as_u64())
            .unwrap_or(0) as u32;
        let is_current_act_rank = current_season.is_some() && current_season_games > 0;
        let act_winrate = if current_season_games > 0 {
            player::round_pct(current_season_wins as f64 / current_season_games as f64)
        } else { 0.0 };

        // Collect current-act match IDs (newest first, stop at season boundary)
        let current_act_matches: Vec<&Value> = recent_matches.iter()
            .take_while(|m| {
                current_season.as_deref()
                    .map(|cs| m["SeasonID"].as_str() == Some(cs))
                    .unwrap_or(true)
            })
            .collect();

        let latest_update = current_act_matches.first().copied()
            .or_else(|| recent_matches.first());
        let rr = latest_comp.and_then(|lc| lc["RankedRatingAfterUpdate"].as_i64())
            .or_else(|| latest_update.and_then(|u| u["RankedRatingAfterUpdate"].as_i64()))
            .unwrap_or(0) as i32;
        let earned_rr = latest_comp.and_then(|lc| lc["RankedRatingEarned"].as_i64())
            .or_else(|| latest_update.and_then(|u| u["RankedRatingEarned"].as_i64()))
            .unwrap_or(0) as i32;
        let leaderboard_position = latest_update
            .and_then(|u| u["LeaderboardPosition"].as_u64())
            .unwrap_or(0) as u32;

        let recent_match_ids: Vec<String> = current_act_matches.iter()
            .filter_map(|m| m["MatchID"].as_str())
            .map(|s| s.to_string())
            .collect();

        let agent_id = p["CharacterID"].as_str()
            .or_else(|| p["CharacterSelectionID"].as_str())
            .unwrap_or("")
            .to_string();
        let identity = &p["PlayerIdentity"];
        let (display_name, tag_line) = match name_map.get(puuid) {
            Some((name, tag)) => (name.clone(), tag.clone()),
            None => (String::new(), String::new()),
        };
        let team_id = p["TeamID"].as_str().unwrap_or("").to_string();
        let account_level = identity["AccountLevel"].as_u64().unwrap_or(0) as u32;

        raw_built.push(RawPlayer {
            built: ValorantPlayer {
                puuid: puuid.to_string(),
                name: display_name,
                tag: tag_line,
                agent_id: agent_id.clone(),
                agent_name: AGENT_MAP.get(&agent_id).cloned().unwrap_or_else(|| "Unknown".to_string()),
                team_id,
                account_level,
                rank,
                rank_name: RANK_MAP.get(&(rank as u64)).cloned().unwrap_or_else(|| "Unranked".to_string()),
                peak_rank,
                peak_rank_name: RANK_MAP.get(&(peak_rank as u64)).cloned().unwrap_or_else(|| "Unranked".to_string()),
                previous_rank,
                rr, earned_rr, leaderboard_position,
                headshots: 0, bodyshots: 0, legshots: 0, headshot_percent: 0.0,
                winrate: act_winrate,
                kd: 0.0, kills: 0.0, deaths: 0.0, assists: 0.0,
                acs: 0, adr: 0.0,
                current_season_wins, current_season_games,
                is_current_act_rank,
                recent_games_count: 0,
                last_match_kills: 0, last_match_deaths: 0, last_match_assists: 0, last_match_kd: 0.0,
                recent_results: vec![],
                party_id: None, party_number: None, party_size: None,
            },
            recent_match_ids,
        });
    }

    // 9. Fetch match details (only uncached ones)
    let mut needed_ids: HashSet<String> = HashSet::new();
    {
        let mut detail_cache = state.detail_cache.lock().await;
        for rp in &raw_built {
            for mid in &rp.recent_match_ids {
                if detail_cache.get(mid).is_none() {
                    needed_ids.insert(mid.clone());
                }
            }
        }
    }

    let remote_client2 = state.remote_client.clone();
    let cfg_clone2 = cfg.clone();
    let fetched_details: Vec<(String, Option<Value>)> = stream::iter(needed_ids.into_iter())
        .map(move |mid| {
            let remote = remote_client2.clone();
            let cfg_ref = cfg_clone2.clone();
            async move {
                let detail = endpoints::get_match_details(&remote, &cfg_ref, &mid).await;
                (mid, detail)
            }
        })
        .buffer_unordered(DETAIL_CONCURRENCY)
        .collect()
        .await;

    // Write new details into LRU cache
    {
        let mut detail_cache = state.detail_cache.lock().await;
        for (mid, detail) in fetched_details {
            if let Some(d) = detail {
                detail_cache.put(mid, Arc::new(d));
            }
        }
    }

    // 10. Collect Arc<Value> references under lock, then drop lock immediately (C1)
    let (mut player_details_map, ordered_details): (HashMap<String, Vec<Arc<Value>>>, Vec<Arc<Value>>) = {
        let detail_cache = state.detail_cache.lock().await;
        let mut player_map = HashMap::new();
        let mut seen = HashSet::new();
        let mut ordered = Vec::new();

        for rp in &raw_built {
            let mut list = Vec::new();
            for mid in &rp.recent_match_ids {
                if let Some(d) = detail_cache.peek(mid) {
                    list.push(d.clone());
                    if seen.insert(mid.clone()) {
                        ordered.push(d.clone());
                    }
                }
            }
            player_map.insert(rp.built.puuid.clone(), list);
        }
        (player_map, ordered)
    };

    // Now detail_cache lock is released! Do CPU stats computation:
    let mut built_players: Vec<ValorantPlayer> = Vec::with_capacity(raw_built.len());
    for rp in raw_built {
        let details = player_details_map.remove(&rp.built.puuid).unwrap_or_default();
        let mut player = rp.built;

        if !details.is_empty() {
            // Compute extract_player_stats once per match and reuse
            let mut player_stats_list = Vec::with_capacity(details.len());
            for d in &details {
                player_stats_list.push(stats::extract_player_stats(d, &player.puuid));
            }

            // Last match stats
            if let Some(first) = player_stats_list.first() {
                player.last_match_kills = first.kills;
                player.last_match_deaths = first.deaths;
                player.last_match_assists = first.assists;
                player.last_match_kd = first.kd;
            }

            // Recent results
            player.recent_results = details.iter().take(5)
                .map(|d| stats::get_match_result(d, &player.puuid))
                .collect();

            // Aggregated stats from precomputed details (no re-parsing match details)
            let agg = stats::aggregate_player_stats_from_extracted(&player_stats_list);
            player.kills = agg.kills;
            player.deaths = agg.deaths;
            player.assists = agg.assists;
            player.kd = agg.kd;
            player.headshots = agg.headshots;
            player.bodyshots = agg.bodyshots;
            player.legshots = agg.legshots;
            player.headshot_percent = agg.headshot_percent;
            player.acs = agg.acs;
            player.adr = agg.adr;
            player.recent_games_count = agg.recent_games_count;
        }

        built_players.push(player);
    }

    // 11. Party enrichment + assignment (D5, D6)
    let mut players_with_party = built_players;
    party::enrich_party_from_match_history(&mut players_with_party, &ordered_details);
    party::assign_party_numbers(&mut players_with_party, &presences);

     // 12. Resolve map / mode / side
     let map_name = resolve_map_name(&map_id);
     let (game_mode_name, is_deathmatch, mode_keyword) =
         resolve_game_mode_name(&game_mode_id, &game_mode, is_ranked);

     let starting_side = if resolved_game_state == "PREGAME" {
         side::resolve_starting_side(ally_team_id.as_deref(), &game_mode_id, &mode_keyword)
     } else { None };

     let match_info = MatchInfo {
         match_id: resolved_match_id.clone(),
         map_id, map_name, game_mode, game_mode_id, game_mode_name, is_deathmatch,
         server, is_ranked, game_state: resolved_game_state.clone(), season_id,
         starting_side,
     };

     // 13. Store in match cache
     *state.match_cache.lock().await = Some(MatchCache {
         match_id: resolved_match_id,
         game_state: resolved_game_state.clone(),
         players: players_with_party.clone(),
         match_info: match_info.clone(),
     });

     Ok(ApiResponse {
         game_state: resolved_game_state,
         r#match: Some(match_info),
         players: Some(players_with_party),
         self_puuid: Some(cfg.puuid),
         error: None,
     })
 }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_map_name() {
        // Known maps from MAP_MAP
        assert_eq!(resolve_map_name("/Game/Maps/Ascent/Ascent"), "Ascent");
        assert_eq!(resolve_map_name("/Game/Maps/Duality/Duality"), "Bind");
        assert_eq!(resolve_map_name("/Game/Maps/Bonsai/Bonsai"), "Split");

        // Map with extension stripped
        assert_eq!(resolve_map_name("/game/maps/ascent/ascent.umap"), "Ascent");

        // Fallback title-cased with word boundary
        assert_eq!(
            resolve_map_name("/Game/Maps/Special/secret_underground_temple.umap"),
            "Secret Underground Temple"
        );
        assert_eq!(resolve_map_name("unknown_map"), "Unknown Map");
        assert_eq!(resolve_map_name(""), "Unknown");
    }

    #[test]
    fn test_strip_and_extract() {
        let (stripped, keyword) = strip_and_extract("/Game/GameModes/Bomb/BombGameMode.BombGameMode_C");
        assert_eq!(stripped, "/Game/GameModes/Bomb/BombGameMode");
        assert_eq!(keyword, "Bomb");

        let (_stripped_lower, keyword_lower) = strip_and_extract("/game/gamemodes/bomb/bombgamemode.bombgamemode_c");
        assert_eq!(keyword_lower, "bomb");

        let (stripped2, keyword2) = strip_and_extract("competitive");
        assert_eq!(stripped2, "competitive");
        assert_eq!(keyword2, "");
    }

    #[test]
    fn test_resolve_game_mode_name() {
        // Standard competitive
        let (name, is_dm, kw) = resolve_game_mode_name("competitive", "", true);
        assert_eq!(name, "Competitive");
        assert!(!is_dm);
        assert_eq!(kw, "");

        // Standard unrated
        let (name, is_dm, _) = resolve_game_mode_name("unrated", "", false);
        assert_eq!(name, "Unrated");
        assert!(!is_dm);

        // Deathmatch
        let (name, is_dm, _) = resolve_game_mode_name("deathmatch", "", false);
        assert_eq!(name, "Deathmatch");
        assert!(is_dm);

        // Standard mode resolved by is_ranked
        let (name_ranked, _, _) = resolve_game_mode_name("", "/Game/GameModes/Bomb/BombGameMode.BombGameMode_C", true);
        assert_eq!(name_ranked, "Competitive");

        let (name_unranked, _, _) = resolve_game_mode_name("", "/Game/GameModes/Bomb/BombGameMode.BombGameMode_C", false);
        assert_eq!(name_unranked, "Unrated");
    }
}
