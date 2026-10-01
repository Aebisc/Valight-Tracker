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


use crate::model::{ApiResponse, MatchInfo, ValorantPlayer};
use crate::state::{AppState, MatchCache};
use crate::riot::{lockfile, config, endpoints};
use crate::service::{player, stats, party, side};

const RECENT_GAMES_COUNT: u32 = 20;
const DETAIL_CONCURRENCY: usize = 10;

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

fn strip_and_extract(path: &str) -> (String, String) {
    let stripped = path.trim_end_matches(|c: char| c == '.' || c.is_ascii_alphabetic() && !path.contains('/'));
    // Remove file extension (everything after last dot, if no slash follows)
    let stripped = if let Some(pos) = stripped.rfind('.') {
        if !stripped[pos..].contains('/') { &stripped[..pos] } else { stripped }
    } else { stripped };
    let segs: Vec<&str> = stripped.split('/').filter(|s| !s.is_empty()).collect();
    let keyword = if segs.len() >= 3 { segs[2].to_string() } else { String::new() };
    (stripped.to_string(), keyword)
}

fn resolve_map_name(map_id: &str) -> String {
    let map_lower = map_id.to_lowercase();
    if let Some(name) = MAP_MAP.get(map_id).or_else(|| MAP_MAP.get(&map_lower)) {
        return name.clone();
    }
    // Fallback: extract last path segment
    map_id.split('/').last()
        .map(|s| {
            let no_ext = if let Some(p) = s.rfind('.') { &s[..p] } else { s };
            no_ext.replace('_', " ")
                .split_whitespace()
                .map(|w| {
                    let mut c = w.chars();
                    match c.next() {
                        None => String::new(),
                        Some(f) => f.to_uppercase().to_string() + c.as_str(),
                    }
                })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_else(|| "Unknown".to_string())
}

fn resolve_game_mode_name(queue_id: &str, mode: &str, is_ranked: bool) -> (String, bool, String) {
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

// ─── Main command entry point ──────────────────────────────────────────────────

/// The `get_match` Tauri command. Equivalent to the GET /api/match handler.
pub async fn get_match(force: bool, state: &AppState) -> ApiResponse {
    // Force-clear caches
    if force {
        *state.match_cache.lock().await = None;
        config::clear_config_cache().await;
    }

    // 1. Read lockfile — OFFLINE if absent
    let lockfile = match lockfile::read_lockfile().await {
        Ok(lf) => lf,
        Err(_) => return ApiResponse::offline(),
    };

    // 2. Get API config — may fail if Valorant not running
    let cfg = match config::get_api_config(&lockfile, force).await {
        Ok(c) => c,
        Err(_) => return ApiResponse::offline(),
    };

    // 3. Check core-game first
    let core_match_id = endpoints::get_coregame_player_id(&state.remote_client, &cfg).await;

    // INGAME fast-path: cache hit for the same match
    if let Some(ref mid) = core_match_id {
        let cache = state.match_cache.lock().await;
        if let Some(ref cached) = *cache {
            if &cached.match_id == mid && cached.game_state == "INGAME" {
                return ApiResponse {
                    game_state: cached.game_state.clone(),
                    r#match: Some(cached.match_info.clone()),
                    players: Some(cached.players.clone()),
                    self_puuid: Some(cfg.puuid.clone()),
                    error: None,
                };
            }
        }
    }

    // 4. Check pregame (only if not INGAME)
    let pre_match_id = if core_match_id.is_some() {
        None
    } else {
        endpoints::get_pregame_player_id(&state.remote_client, &cfg).await
    };

    // 5. MENUS
    if core_match_id.is_none() && pre_match_id.is_none() {
        *state.match_cache.lock().await = None;
        return ApiResponse::menus(cfg.puuid.clone());
    }

    // We're in a match — acquire the single-flight build lock
    let _build_guard = state.build_lock.lock().await;

    // Re-check cache under the lock (another concurrent call may have just built it)
    if let Some(ref mid) = core_match_id {
        let cache = state.match_cache.lock().await;
        if let Some(ref cached) = *cache {
            if &cached.match_id == mid && cached.game_state == "INGAME" {
                return ApiResponse {
                    game_state: cached.game_state.clone(),
                    r#match: Some(cached.match_info.clone()),
                    players: Some(cached.players.clone()),
                    self_puuid: Some(cfg.puuid.clone()),
                    error: None,
                };
            }
        }
    }

    // 6. Fetch presences for party detection
    let presences_raw = endpoints::get_presences(&state.local_client, lockfile.port, &lockfile.basic_auth).await;
    let presences = presences_raw.as_ref()
        .map(|v| party::parse_presences(v))
        .unwrap_or_default();

    // 7. Fetch match data
    let (resolved_match_id, resolved_game_state, raw_players, map_id, game_mode, game_mode_id,
         is_ranked, server, season_id, ally_team_id) = if let Some(ref mid) = core_match_id {
        let cg = match endpoints::get_coregame_match(&state.remote_client, &cfg, mid).await {
            Some(v) => v,
            None => return ApiResponse::error("Failed to fetch core-game match".to_string()),
        };
        let players: Vec<Value> = cg["Players"].as_array().cloned().unwrap_or_default();
        (
            mid.clone(),
            "INGAME".to_string(),
            players,
            cg["MapID"].as_str().unwrap_or("").to_string(),
            cg["Mode"].as_str().unwrap_or("").to_string(),
            cg["QueueID"].as_str().or_else(|| cg["ModeID"].as_str()).unwrap_or("").to_string(),
            cg["IsRanked"].as_bool().unwrap_or(false),
            cg["GamePodID"].as_str().unwrap_or("").to_string(),
            cg["SeasonID"].as_str().unwrap_or("").to_string(),
            None,
        )
    } else {
        let mid = pre_match_id.as_ref().unwrap();
        let pg = match endpoints::get_pregame_match(&state.remote_client, &cfg, mid).await {
            Some(v) => v,
            None => return ApiResponse::error("Failed to fetch pregame match".to_string()),
        };
        let ally_team_id: Option<String> = pg["AllyTeam"]["TeamID"].as_str().map(|s| s.to_string());
        let enemy_team_id = match ally_team_id.as_deref() {
            Some("Blue") => Some("Red".to_string()),
            Some("Red")  => Some("Blue".to_string()),
            _            => pg["EnemyTeam"]["TeamID"].as_str().map(|s| s.to_string()),
        };
        let mut players: Vec<Value> = Vec::new();
        for p in pg["AllyTeam"]["Players"].as_array().into_iter().flatten() {
            let mut p = p.clone();
            if p.get("TeamID").is_none() || p["TeamID"].is_null() {
                p["TeamID"] = serde_json::json!(ally_team_id.as_deref().unwrap_or("Blue"));
            }
            players.push(p);
        }
        for p in pg["EnemyTeam"]["Players"].as_array().into_iter().flatten() {
            let mut p = p.clone();
            if p.get("TeamID").is_none() || p["TeamID"].is_null() {
                p["TeamID"] = serde_json::json!(enemy_team_id.as_deref().unwrap_or("Red"));
            }
            players.push(p);
        }
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
                return ApiResponse {
                    game_state: cached.game_state.clone(),
                    r#match: Some(cached.match_info.clone()),
                    players: Some(cached.players.clone()),
                    self_puuid: Some(cfg.puuid.clone()),
                    error: None,
                };
            }
        }
    }

    // 8. Full build: fetch names, MMR, comp updates, match details
    let puuids: Vec<String> = raw_players.iter()
        .filter_map(|p| p["Subject"].as_str().or_else(|| p["PlayerIdentity"]["Subject"].as_str()))
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let names_raw = endpoints::get_names_from_puuids(&state.remote_client, &cfg, &puuids).await;
    let name_map: HashMap<String, Value> = names_raw.into_iter()
        .filter_map(|n| n["Subject"].as_str().map(|s| (s.to_string(), n.clone())))
        .collect();

    // Concurrent MMR + comp updates per player
    let mmr_comp_results: Vec<(Option<Value>, Option<Value>)> = stream::iter(puuids.iter())
        .map(|puuid| {
            let remote = &state.remote_client;
            let cfg_ref = &cfg;
            async move {
                let mmr = endpoints::get_player_mmr(remote, cfg_ref, puuid).await;
                let comp = endpoints::get_competitive_updates(remote, cfg_ref, puuid, RECENT_GAMES_COUNT).await;
                (mmr, comp)
            }
        })
        .buffer_unordered(DETAIL_CONCURRENCY)
        .collect()
        .await;

    // Build intermediate player data and collect recent match IDs
    struct RawPlayer {
        built: ValorantPlayer,
        recent_match_ids: Vec<String>,
    }

    let mut raw_built: Vec<RawPlayer> = Vec::new();
    for (i, puuid) in puuids.iter().enumerate() {
        let p = match raw_players.get(i) { Some(v) => v, None => continue };
        let (mmr_raw, comp_raw) = match mmr_comp_results.get(i) { Some(v) => v, None => continue };

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
        let is_current_act_rank = current_season.is_some() && (current_season_wins + current_season_games) > 0;
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
        let name_entry = name_map.get(puuid.as_str());
        let display_name = name_entry
            .and_then(|n| n["GameName"].as_str().or_else(|| n["DisplayName"].as_str()))
            .unwrap_or("")
            .to_string();
        let tag_line = name_entry.and_then(|n| n["TagLine"].as_str()).unwrap_or("").to_string();
        let team_id = p["TeamID"].as_str().unwrap_or("").to_string();
        let account_level = identity["AccountLevel"].as_u64().unwrap_or(0) as u32;

        raw_built.push(RawPlayer {
            built: ValorantPlayer {
                puuid: puuid.clone(),
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

    let fetched_details: Vec<(String, Option<Value>)> = stream::iter(needed_ids.iter().cloned())
        .map(|mid| {
            let remote = &state.remote_client;
            let cfg_ref = &cfg;
            async move {
                let detail = endpoints::get_match_details(remote, cfg_ref, &mid).await;
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

    // 10. Build final players with stats
    let built_players: Vec<ValorantPlayer> = {
        let detail_cache = state.detail_cache.lock().await;
        let mut result = Vec::new();
        for rp in raw_built {
            let details: Vec<Arc<Value>> = rp.recent_match_ids.iter()
                .filter_map(|mid| detail_cache.peek(mid).cloned())
                .collect();

            let mut player = rp.built;
            let last_stats = details.first()
                .map(|d| stats::extract_player_stats(d, &player.puuid));
            player.last_match_kills = last_stats.as_ref().map(|s| s.kills).unwrap_or(0);
            player.last_match_deaths = last_stats.as_ref().map(|s| s.deaths).unwrap_or(0);
            player.last_match_assists = last_stats.as_ref().map(|s| s.assists).unwrap_or(0);
            player.last_match_kd = last_stats.as_ref().map(|s| s.kd).unwrap_or(0.0);
            player.recent_results = details.iter().take(5)
                .map(|d| stats::get_match_result(d, &player.puuid))
                .collect();

            if !details.is_empty() {
                let detail_values: Vec<Value> = details.iter().map(|d| (**d).clone()).collect();
                let agg = stats::aggregate_player_stats(&detail_values, &player.puuid);
                player.kills = agg.kills;
                player.deaths = agg.deaths;
                player.assists = agg.assists;
                player.kd = agg.kd;
                player.headshots = agg.headshots;
                player.bodyshots = agg.bodyshots;
                player.legshots = agg.legshots;
                player.headshot_percent = agg.headshot_percent;
                // winrate stays as actWinrate (not match-detail winrate)
                player.acs = agg.acs;
                player.adr = agg.adr;
                player.recent_games_count = agg.recent_games_count;
            }

            result.push(player);
        }
        result
    };

    // 11. Party enrichment + assignment
    let detail_lookup: HashMap<String, Value> = {
        let detail_cache = state.detail_cache.lock().await;
        // Export only the entries referenced by this lobby's players
        let mut map = HashMap::new();
        for mid in raw_built_ids(&built_players, &state) {
            if let Some(d) = detail_cache.peek(&mid) {
                map.insert(mid, (**d).clone());
            }
        }
        map
    };

    let mut players_with_party = built_players;
    party::enrich_party_from_match_history(&mut players_with_party, &detail_lookup);
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

    ApiResponse {
        game_state: resolved_game_state,
        r#match: Some(match_info),
        players: Some(players_with_party),
        self_puuid: Some(cfg.puuid),
        error: None,
    }
}

// Helper: collect all recent match IDs referenced by the current lobby's players
// (needed to export relevant entries from the LRU for party enrichment)
fn raw_built_ids(_players: &[ValorantPlayer], _state: &AppState) -> Vec<String> {
    // Phase 3 TODO: store _recentMatchIds temporarily during build
    // For now returns empty — enrichPartyFromMatchHistory will use whatever's in the LRU
    vec![]
}
