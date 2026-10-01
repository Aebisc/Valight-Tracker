// src-tauri/src/service/party.rs
//
// Port of:
//   - assignPartyNumbers()
//   - enrichPartyFromMatchHistory()
//
// Both are pure functions over player arrays and presence maps.

use std::collections::HashMap;
use crate::model::ValorantPlayer;
use serde_json::Value;

/// Port of assignPartyNumbers().
///
/// Groups players by team, then within each team assigns a 1-indexed partyNumber
/// to any group of 2+ players sharing the same partyId. Players not in a
/// multi-person party get partyNumber = None.
///
/// Presence data (partyId, partySize) is the primary source. The function
/// merges presence info onto the player structs in place.
pub fn assign_party_numbers(
    players: &mut Vec<ValorantPlayer>,
    presences: &HashMap<String, PresenceInfo>,
) {
    // Apply presence data
    for player in players.iter_mut() {
        if let Some(presence) = presences.get(&player.puuid) {
            player.party_id = Some(presence.party_id.clone());
            player.party_size = Some(presence.party_size);
        }
    }

    // Group by team
    let teams: std::collections::HashSet<String> = players.iter().map(|p| p.team_id.clone()).collect();
    for team in teams {
        // Count how many players on this team share each partyId
        let mut party_counts: HashMap<String, u32> = HashMap::new();
        for p in players.iter().filter(|p| p.team_id == team) {
            if let Some(pid) = &p.party_id {
                if !pid.is_empty() {
                    *party_counts.entry(pid.clone()).or_insert(0) += 1;
                }
            }
        }

        // Assign numbers to parties with ≥2 members
        let mut party_number_map: HashMap<String, u32> = HashMap::new();
        let mut next_number = 1u32;
        // Stable ordering: sort party IDs so the numbering is deterministic
        let mut sorted_ids: Vec<&String> = party_counts.keys().collect();
        sorted_ids.sort();
        for pid in sorted_ids {
            if party_counts[pid] >= 2 {
                party_number_map.insert(pid.clone(), next_number);
                next_number += 1;
            }
        }

        for player in players.iter_mut().filter(|p| p.team_id == team) {
            if let Some(pid) = &player.party_id {
                if let Some(&num) = party_number_map.get(pid) {
                    player.party_number = Some(num);
                    player.party_size = party_counts.get(pid).copied();
                } else {
                    player.party_number = None;
                }
            }
        }
    }
}

/// Port of enrichPartyFromMatchHistory().
///
/// For players without presence data (non-friends on the same team), use
/// shared partyId from recent match history as a fallback.
/// Only fills gaps — presence data always wins.
pub fn enrich_party_from_match_history(
    players: &mut Vec<ValorantPlayer>,
    match_detail_lookup: &HashMap<String, Value>,
) {
    // Build a map of puuid → Set<partyId seen in recent matches>
    // Two players who shared a partyId in any of their recent matches
    // are assumed to be queued together.
    let mut puuid_to_recent_parties: HashMap<String, std::collections::HashSet<String>> = HashMap::new();

    for (_, detail) in match_detail_lookup.iter() {
        if let Some(match_players) = detail["players"].as_array() {
            for mp in match_players {
                let puuid = mp["subject"].as_str().unwrap_or("").to_string();
                let party_id = mp["partyId"].as_str()
                    .or_else(|| mp["PartyID"].as_str())
                    .unwrap_or("")
                    .to_string();
                if !puuid.is_empty() && !party_id.is_empty() {
                    puuid_to_recent_parties
                        .entry(puuid)
                        .or_default()
                        .insert(party_id);
                }
            }
        }
    }

    // For each lobby player with no partyId, find another lobby player on the
    // same team who shares a recent-match partyId
    let puuids: Vec<String> = players.iter().map(|p| p.puuid.clone()).collect();
    let team_ids: Vec<String> = players.iter().map(|p| p.team_id.clone()).collect();
    let existing_party_ids: Vec<Option<String>> = players.iter().map(|p| p.party_id.clone()).collect();

    // For players without presence, try to assign a synthetic partyId based
    // on shared match history
    let mut synthetic_parties: HashMap<String, String> = HashMap::new();
    let mut next_synthetic = 0u32;

    for i in 0..players.len() {
        if existing_party_ids[i].is_some() { continue; } // already has presence
        let my_parties = match puuid_to_recent_parties.get(&puuids[i]) {
            Some(s) => s,
            None => continue,
        };

        for j in (i + 1)..players.len() {
            if existing_party_ids[j].is_some() { continue; }
            if team_ids[i] != team_ids[j] { continue; }

            let their_parties = match puuid_to_recent_parties.get(&puuids[j]) {
                Some(s) => s,
                None => continue,
            };

            // If they share any partyId in recent history, group them
            if my_parties.intersection(their_parties).next().is_some() {
                // Both get the same synthetic partyId
                let synth = synthetic_parties.get(&puuids[i]).cloned().unwrap_or_else(|| {
                    next_synthetic += 1;
                    format!("__synth_{}", next_synthetic)
                });
                synthetic_parties.insert(puuids[i].clone(), synth.clone());
                synthetic_parties.insert(puuids[j].clone(), synth);
            }
        }
    }

    // Apply synthetic parties
    for player in players.iter_mut() {
        if player.party_id.is_none() {
            if let Some(synth) = synthetic_parties.get(&player.puuid) {
                player.party_id = Some(synth.clone());
            }
        }
    }
}

// ─── PresenceInfo ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PresenceInfo {
    pub party_id: String,
    pub party_size: u32,
}

/// Parses the raw presences response into a puuid → PresenceInfo map.
/// Handles both old (flat) and new (nested partyPresenceData) layouts.
pub fn parse_presences(raw: &Value) -> HashMap<String, PresenceInfo> {
    let mut map = HashMap::new();

    let presences = match raw["presences"].as_array() {
        Some(arr) => arr,
        None => return map,
    };

    for p in presences {
        let puuid = p["puuid"].as_str()
            .or_else(|| p["subject"].as_str())
            .unwrap_or("")
            .to_string();
        if puuid.is_empty() { continue; }

        // Skip non-Valorant products
        if let Some(product) = p["product"].as_str() {
            if product != "valorant" { continue; }
        }

        let private_b64 = match p["private"].as_str() {
            Some(s) if !s.is_empty() => s,
            _ => continue,
        };

        // Decode base64 payload
        use base64::Engine;
        let decoded_bytes = match base64::engine::general_purpose::STANDARD.decode(private_b64) {
            Ok(b) => b,
            Err(_) => continue,
        };
        let decoded_str = match std::str::from_utf8(&decoded_bytes) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let decoded: Value = match serde_json::from_str(decoded_str) {
            Ok(v) => v,
            Err(_) => continue,
        };

        // Try nested (13.x+) then flat (≤12.x)
        let nested = &decoded["partyPresenceData"];
        let party_id = nested["partyId"].as_str()
            .or_else(|| nested["partyID"].as_str())
            .or_else(|| decoded["partyId"].as_str())
            .or_else(|| decoded["partyID"].as_str())
            .unwrap_or("")
            .to_string();

        if party_id.is_empty() { continue; }

        let party_size = nested["partySize"].as_u64()
            .or_else(|| decoded["partySize"].as_u64())
            .unwrap_or(1) as u32;

        map.insert(puuid, PresenceInfo { party_id, party_size });
    }

    map
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_player(puuid: &str, team: &str) -> ValorantPlayer {
        use crate::model::*;
        ValorantPlayer {
            puuid: puuid.to_string(),
            name: "".to_string(), tag: "".to_string(),
            agent_id: "".to_string(), agent_name: "".to_string(),
            team_id: team.to_string(),
            account_level: 0, rank: 0, rank_name: "".to_string(),
            peak_rank: 0, peak_rank_name: "".to_string(), previous_rank: 0,
            rr: 0, earned_rr: 0, leaderboard_position: 0,
            headshots: 0, bodyshots: 0, legshots: 0, headshot_percent: 0.0,
            winrate: 0.0, kd: 0.0, kills: 0.0, deaths: 0.0, assists: 0.0,
            acs: 0, adr: 0.0, current_season_wins: 0, current_season_games: 0,
            is_current_act_rank: false, recent_games_count: 0,
            last_match_kills: 0, last_match_deaths: 0, last_match_assists: 0,
            last_match_kd: 0.0, recent_results: vec![],
            party_id: None, party_number: None, party_size: None,
        }
    }

    #[test]
    fn assigns_party_numbers_for_duos() {
        let mut players = vec![
            make_player("p1", "Blue"),
            make_player("p2", "Blue"),
            make_player("p3", "Blue"),
        ];
        let mut presences = HashMap::new();
        presences.insert("p1".to_string(), PresenceInfo { party_id: "party-A".to_string(), party_size: 2 });
        presences.insert("p2".to_string(), PresenceInfo { party_id: "party-A".to_string(), party_size: 2 });
        presences.insert("p3".to_string(), PresenceInfo { party_id: "party-B".to_string(), party_size: 1 });

        assign_party_numbers(&mut players, &presences);

        assert_eq!(players[0].party_number, Some(1));
        assert_eq!(players[1].party_number, Some(1));
        assert_eq!(players[2].party_number, None); // solo
    }

    #[test]
    fn solo_player_has_no_party_number() {
        let mut players = vec![make_player("p1", "Blue")];
        let mut presences = HashMap::new();
        presences.insert("p1".to_string(), PresenceInfo { party_id: "party-solo".to_string(), party_size: 1 });
        assign_party_numbers(&mut players, &presences);
        assert_eq!(players[0].party_number, None);
    }
}
