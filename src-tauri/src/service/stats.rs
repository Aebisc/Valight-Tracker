// src-tauri/src/service/stats.rs
//
// Pure logic ports of:
//   - extractPlayerStats()
//   - getMatchResult()
//   - aggregatePlayerStats()

use serde_json::Value;
use crate::model::MatchResult;
use super::player::{round1, round2, round_pct};

// ─── extractPlayerStats ───────────────────────────────────────────────────────

pub struct PlayerStats {
    pub kills: u32,
    pub deaths: u32,
    pub assists: u32,
    pub kd: f64,
    pub headshots: u32,
    pub bodyshots: u32,
    pub legshots: u32,
    pub headshot_percent: f64,
    pub winrate: f64,
    pub acs: u32,
    pub adr: f64,
    pub won: bool,
}

/// Port of extractPlayerStats(). Uses safe accessors throughout.
pub fn extract_player_stats(match_detail: &Value, puuid: &str) -> PlayerStats {
    let _empty = PlayerStats {
        kills: 0, deaths: 0, assists: 0, kd: 0.0,
        headshots: 0, bodyshots: 0, legshots: 0, headshot_percent: 0.0,
        winrate: 0.0, acs: 0, adr: 0.0, won: false,
    };

    // Find the player entry — Riot uses lowercase 'subject' in match details
    let player_stats = match match_detail["players"].as_array() {
        Some(arr) => arr.iter().find(|p| {
            p["subject"].as_str() == Some(puuid) || p["Subject"].as_str() == Some(puuid)
        }),
        None => None,
    };

    let stats_node = player_stats.and_then(|p| Some(&p["stats"]));
    let kills = stats_node.and_then(|s| s["kills"].as_u64()).unwrap_or(0) as u32;
    let deaths = stats_node.and_then(|s| s["deaths"].as_u64()).unwrap_or(0) as u32;
    let assists = stats_node.and_then(|s| s["assists"].as_u64()).unwrap_or(0) as u32;
    let score = stats_node.and_then(|s| s["score"].as_u64()).unwrap_or(0);
    let kd = if deaths > 0 { round2(kills as f64 / deaths as f64) } else { kills as f64 };

    // Round-by-round headshots / damage
    let mut headshots = 0u32;
    let mut bodyshots = 0u32;
    let mut legshots = 0u32;
    let mut total_damage = 0u64;
    let mut rounds_participated = 0u32;

    if let Some(rounds) = match_detail["roundResults"].as_array() {
        for round in rounds {
            let ps = round["playerStats"].as_array()
                .and_then(|arr| arr.iter().find(|ps| {
                    ps["subject"].as_str() == Some(puuid) || ps["Subject"].as_str() == Some(puuid)
                }));
            if let Some(ps) = ps {
                rounds_participated += 1;
                if let Some(dmgs) = ps["damage"].as_array() {
                    for dmg in dmgs {
                        headshots += dmg["headshots"].as_u64().unwrap_or(0) as u32;
                        bodyshots += dmg["bodyshots"].as_u64().unwrap_or(0) as u32;
                        legshots  += dmg["legshots"].as_u64().unwrap_or(0) as u32;
                        total_damage += dmg["damage"].as_u64().unwrap_or(0);
                    }
                }
            }
        }
    }

    let total_shots = headshots + bodyshots + legshots;
    let headshot_percent = if total_shots > 0 {
        round_pct(headshots as f64 / total_shots as f64)
    } else { 0.0 };

    let total_rounds = match_detail["roundResults"].as_array().map(|a| a.len()).unwrap_or(1);
    let acs = if total_rounds > 0 { (score as f64 / total_rounds as f64).round() as u32 } else { 0 };
    let adr = if rounds_participated > 0 {
        round1(total_damage as f64 / rounds_participated as f64)
    } else { 0.0 };

    // Win/loss from team data
    let team_id = player_stats
        .and_then(|p| p["teamId"].as_str().or_else(|| p["TeamID"].as_str()))
        .unwrap_or("");

    let (winrate, won) = if !team_id.is_empty() {
        if let Some(teams) = match_detail["teams"].as_array() {
            let team = teams.iter().find(|t| t["teamId"].as_str() == Some(team_id));
            let opp = teams.iter().find(|t| t["teamId"].as_str() != Some(team_id));
            if let Some(team) = team {
                let wins = team["roundsWon"].as_u64().unwrap_or(0) as f64;
                let total = team["roundsPlayed"].as_u64().unwrap_or(0) as f64;
                let wr = if total > 0.0 { round_pct(wins / total) } else { 0.0 };
                let won = if let (Some(t_won), Some(_o_won)) = (
                    team["won"].as_bool(),
                    opp.and_then(|o| o["won"].as_bool()),
                ) {
                    t_won
                } else {
                    let opp_wins = opp.and_then(|o| o["roundsWon"].as_u64()).unwrap_or(0) as f64;
                    wins > opp_wins
                };
                (wr, won)
            } else { (0.0, false) }
        } else { (0.0, false) }
    } else { (0.0, false) };

    PlayerStats { kills, deaths, assists, kd, headshots, bodyshots, legshots, headshot_percent, winrate, acs, adr, won }
}

// ─── getMatchResult ───────────────────────────────────────────────────────────

/// Port of getMatchResult(). Returns W/L/D, not just won/lost.
pub fn get_match_result(match_detail: &Value, puuid: &str) -> MatchResult {
    let player = match_detail["players"].as_array()
        .and_then(|arr| arr.iter().find(|p| {
            p["subject"].as_str() == Some(puuid) || p["Subject"].as_str() == Some(puuid)
        }));

    let team_id = player
        .and_then(|p| p["teamId"].as_str().or_else(|| p["TeamID"].as_str()))
        .unwrap_or("");

    if team_id.is_empty() { return MatchResult::L; }

    if let Some(teams) = match_detail["teams"].as_array() {
        let team = teams.iter().find(|t| t["teamId"].as_str() == Some(team_id));
        let opp  = teams.iter().find(|t| t["teamId"].as_str() != Some(team_id));

        if let (Some(t), Some(o)) = (team, opp) {
            if let (Some(t_won), Some(o_won)) = (t["won"].as_bool(), o["won"].as_bool()) {
                return match (t_won, o_won) {
                    (true, false)  => MatchResult::W,
                    (false, true)  => MatchResult::L,
                    _              => MatchResult::D,
                };
            }
            // Fallback: compare rounds won
            let my_rounds = t["roundsWon"].as_u64().unwrap_or(0);
            let op_rounds = o["roundsWon"].as_u64().unwrap_or(0);
            return match my_rounds.cmp(&op_rounds) {
                std::cmp::Ordering::Greater => MatchResult::W,
                std::cmp::Ordering::Less    => MatchResult::L,
                std::cmp::Ordering::Equal   => MatchResult::D,
            };
        }
    }

    MatchResult::L
}

// ─── aggregatePlayerStats ─────────────────────────────────────────────────────

pub struct AggregatedStats {
    pub kills: f64,
    pub deaths: f64,
    pub assists: f64,
    pub kd: f64,
    pub headshots: u32,
    pub bodyshots: u32,
    pub legshots: u32,
    pub headshot_percent: f64,
    pub winrate: f64,
    pub acs: u32,
    pub adr: f64,
    pub recent_games_count: u32,
}

/// Port of aggregatePlayerStats(). Averages stats across N matches.
pub fn aggregate_player_stats(match_details: &[Value], puuid: &str) -> AggregatedStats {
    let zero = AggregatedStats {
        kills: 0.0, deaths: 0.0, assists: 0.0, kd: 0.0,
        headshots: 0, bodyshots: 0, legshots: 0, headshot_percent: 0.0,
        winrate: 0.0, acs: 0, adr: 0.0, recent_games_count: 0,
    };

    let mut sum_kills = 0u32;
    let mut sum_deaths = 0u32;
    let mut sum_assists = 0u32;
    let mut sum_headshots = 0u32;
    let mut sum_bodyshots = 0u32;
    let mut sum_legshots = 0u32;
    let mut sum_acs = 0u64;
    let mut sum_adr = 0.0f64;
    let mut wins = 0u32;
    let mut counted = 0u32;

    for detail in match_details {
        let s = extract_player_stats(detail, puuid);
        sum_kills    += s.kills;
        sum_deaths   += s.deaths;
        sum_assists  += s.assists;
        sum_headshots += s.headshots;
        sum_bodyshots += s.bodyshots;
        sum_legshots  += s.legshots;
        sum_acs      += s.acs as u64;
        sum_adr      += s.adr;
        if s.won { wins += 1; }
        counted += 1;
    }

    if counted == 0 { return zero; }

    let n = counted as f64;
    let kd = if sum_deaths > 0 { round2(sum_kills as f64 / sum_deaths as f64) } else { sum_kills as f64 };
    let total_shots = sum_headshots + sum_bodyshots + sum_legshots;
    let headshot_percent = if total_shots > 0 {
        round_pct(sum_headshots as f64 / total_shots as f64)
    } else { 0.0 };

    AggregatedStats {
        kills:    round1(sum_kills as f64 / n),
        deaths:   round1(sum_deaths as f64 / n),
        assists:  round1(sum_assists as f64 / n),
        kd,
        headshots: sum_headshots,
        bodyshots: sum_bodyshots,
        legshots:  sum_legshots,
        headshot_percent,
        winrate:   round_pct(wins as f64 / n),
        acs:       (sum_acs as f64 / n).round() as u32,
        adr:       round1(sum_adr / n),
        recent_games_count: counted,
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn match_result_uses_won_field() {
        let detail = json!({
            "players": [{ "subject": "p1", "teamId": "Blue" }],
            "teams": [
                { "teamId": "Blue", "won": true, "roundsWon": 13 },
                { "teamId": "Red",  "won": false, "roundsWon": 7 },
            ]
        });
        assert_eq!(get_match_result(&detail, "p1"), MatchResult::W);
    }

    #[test]
    fn match_result_fallback_rounds() {
        let detail = json!({
            "players": [{ "subject": "p1", "teamId": "Blue" }],
            "teams": [
                { "teamId": "Blue", "roundsWon": 12 },
                { "teamId": "Red",  "roundsWon": 12 },
            ]
        });
        assert_eq!(get_match_result(&detail, "p1"), MatchResult::D);
    }

    #[test]
    fn match_result_missing_player_is_loss() {
        let detail = json!({ "players": [], "teams": [] });
        assert_eq!(get_match_result(&detail, "ghost"), MatchResult::L);
    }
}
