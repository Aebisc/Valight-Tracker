// src-tauri/src/model.rs
//
// Serde structs for the JSON contract frozen at v1.2.0.
// These must stay byte-compatible with lib/types.ts.
// All fields use Option + #[serde(default)] so Riot shape drift never panics.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ValorantPlayer {
    pub puuid: String,
    pub name: String,
    pub tag: String,
    pub agent_id: String,
    pub agent_name: String,
    pub team_id: String,
    pub account_level: u32,
    pub rank: u32,
    pub rank_name: String,
    pub peak_rank: u32,
    pub peak_rank_name: String,
    pub previous_rank: u32,
    pub rr: i32,
    pub earned_rr: i32,
    pub leaderboard_position: u32,
    pub headshots: u32,
    pub bodyshots: u32,
    pub legshots: u32,
    pub headshot_percent: f64,
    pub winrate: f64,
    pub kd: f64,
    pub kills: f64,
    pub deaths: f64,
    pub assists: f64,
    pub acs: u32,
    pub adr: f64,
    pub current_season_wins: u32,
    pub current_season_games: u32,
    pub is_current_act_rank: bool,
    pub recent_games_count: u32,
    pub last_match_kills: u32,
    pub last_match_deaths: u32,
    pub last_match_assists: u32,
    #[serde(rename = "lastMatchKD")]
    pub last_match_kd: f64,
    /// Up to 5 most recent competitive match outcomes, newest first.
    pub recent_results: Vec<MatchResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub party_id: Option<String>,
    /// 1-indexed group number for 2+ player parties on the same team.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub party_number: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub party_size: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchResult {
    W,
    L,
    D,
}

/// Blue = first half defending, Red = first half attacking.
/// Matches TeamSide in lib/types.ts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TeamSide {
    Attack,
    Defence,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MatchInfo {
    pub match_id: String,
    pub map_id: String,
    pub map_name: String,
    pub game_mode: String,
    pub game_mode_id: String,
    pub game_mode_name: String,
    pub is_deathmatch: bool,
    pub server: String,
    pub is_ranked: bool,
    pub game_state: String,
    pub season_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub starting_side: Option<TeamSide>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiResponse {
    pub game_state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#match: Option<MatchInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub players: Option<Vec<ValorantPlayer>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub self_puuid: Option<String>,
}

impl ApiResponse {
    pub fn offline() -> Self {
        Self {
            game_state: "OFFLINE".into(),
            error: Some("Valorant is not running".into()),
            r#match: None, players: None, self_puuid: None,
        }
    }

    pub fn menus(puuid: String) -> Self {
        Self {
            game_state: "MENUS".into(),
            error: None,
            r#match: None,
            players: Some(vec![]),
            self_puuid: Some(puuid),
        }
    }

    pub fn error(msg: String) -> Self {
        Self {
            game_state: "ERROR".into(),
            error: Some(msg),
            r#match: None, players: None, self_puuid: None,
        }
    }
}
