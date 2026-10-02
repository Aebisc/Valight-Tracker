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

    // Group by team preserving lobby order
    let mut team_order: Vec<String> = Vec::new();
    let mut team_map: HashMap<String, Vec<usize>> = HashMap::new();
    for (idx, p) in players.iter().enumerate() {
        let team_key = if p.team_id.is_empty() { "default".to_string() } else { p.team_id.clone() };
        if !team_map.contains_key(&team_key) {
            team_order.push(team_key.clone());
        }
        team_map.entry(team_key).or_default().push(idx);
    }

    for team_key in team_order {
        let indices = match team_map.get(&team_key) {
            Some(idxs) => idxs,
            None => continue,
        };

        // Count how many players on this team share each partyId
        let mut party_counts: HashMap<String, u32> = HashMap::new();
        for &idx in indices {
            if let Some(pid) = &players[idx].party_id {
                if !pid.is_empty() {
                    *party_counts.entry(pid.clone()).or_insert(0) += 1;
                }
            }
        }

        let valid_parties: std::collections::HashSet<String> = party_counts
            .into_iter()
            .filter(|(_, count)| *count >= 2)
            .map(|(pid, _)| pid)
            .collect();

        // Assign numbers in lobby order (D2)
        let mut party_number_map: HashMap<String, u32> = HashMap::new();
        let mut next_number = 1u32;

        for &idx in indices {
            let pid_opt = players[idx].party_id.clone();
            if let Some(pid) = pid_opt {
                if valid_parties.contains(&pid) {
                    let num = *party_number_map.entry(pid).or_insert_with(|| {
                        let n = next_number;
                        next_number += 1;
                        n
                    });
                    players[idx].party_number = Some(num);
                    // D3: party_size is NOT overwritten by party_counts
                } else {
                    // D4: Stale party numbers -> None
                    players[idx].party_number = None;
                }
            } else {
                // D4: Stale party numbers -> None
                players[idx].party_number = None;
            }
        }
    }
}

/// Port of enrichPartyFromMatchHistory().
///
/// For players without presence data (non-friends on the same team), use
/// shared partyId from recent match history as a fallback.
/// Only fills gaps — presence data always wins.
pub fn enrich_party_from_match_history<T: std::borrow::Borrow<Value>>(
    players: &mut [ValorantPlayer],
    match_details: &[T],
) {
    let lobby_puuids: std::collections::HashSet<String> = players.iter().map(|p| p.puuid.clone()).collect();

    for detail in match_details {
        let detail_val: &Value = detail.borrow();
        let match_players = match detail_val.get("players").and_then(|p| p.as_array()) {
            Some(arr) => arr,
            None => continue,
        };

        let mut party_groups: HashMap<String, Vec<String>> = HashMap::new();
        for mp in match_players {
            let puuid = mp["subject"].as_str()
                .or_else(|| mp["Subject"].as_str())
                .unwrap_or("");
            let party_id = mp["partyId"].as_str()
                .or_else(|| mp["PartyID"].as_str())
                .unwrap_or("");
            if puuid.is_empty() || party_id.is_empty() || !lobby_puuids.contains(puuid) {
                continue;
            }
            party_groups.entry(party_id.to_string()).or_default().push(puuid.to_string());
        }

        for (party_id, members) in party_groups {
            if members.len() < 2 { continue; }
            for puuid in members.iter() {
                if let Some(player) = players.iter_mut().find(|p| &p.puuid == puuid) {
                    if player.party_id.is_none() {
                        player.party_id = Some(party_id.clone());
                        player.party_size = Some(members.len() as u32);
                    }
                }
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

    #[test]
    fn numbers_parties_in_lobby_order_and_preserves_presence_size() {
        let mut players = vec![
            make_player("p1", "Blue"),
            make_player("p2", "Blue"),
            make_player("p3", "Blue"),
            make_player("p4", "Blue"),
        ];
        let mut presences = HashMap::new();
        // p1 and p3 are in party-Z
        presences.insert("p1".to_string(), PresenceInfo { party_id: "party-Z".to_string(), party_size: 5 });
        presences.insert("p3".to_string(), PresenceInfo { party_id: "party-Z".to_string(), party_size: 5 });
        // p2 and p4 are in party-A
        presences.insert("p2".to_string(), PresenceInfo { party_id: "party-A".to_string(), party_size: 2 });
        presences.insert("p4".to_string(), PresenceInfo { party_id: "party-A".to_string(), party_size: 2 });

        assign_party_numbers(&mut players, &presences);

        // Lobby order: party-Z appears first (at index 0), so it gets party_number 1
        assert_eq!(players[0].party_number, Some(1));
        assert_eq!(players[2].party_number, Some(1));
        // party-A appears next (at index 1), so it gets party_number 2
        assert_eq!(players[1].party_number, Some(2));
        assert_eq!(players[3].party_number, Some(2));

        // D3: party_size is preserved as 5 from presence, not overwritten by team count (2)
        assert_eq!(players[0].party_size, Some(5));
    }

    #[test]
    fn enrich_party_from_match_history_fallback() {
        let mut players = vec![
            make_player("p1", "Blue"),
            make_player("p2", "Blue"),
            make_player("p3", "Blue"),
        ];

        let detail = serde_json::json!({
            "players": [
                { "subject": "p1", "partyId": "hist-party-1" },
                { "subject": "p2", "partyId": "hist-party-1" },
                { "subject": "p3", "partyId": "hist-party-2" },
            ]
        });

        enrich_party_from_match_history(&mut players, &[detail]);

        assert_eq!(players[0].party_id.as_deref(), Some("hist-party-1"));
        assert_eq!(players[0].party_size, Some(2));
        assert_eq!(players[1].party_id.as_deref(), Some("hist-party-1"));
        assert_eq!(players[1].party_size, Some(2));
        assert_eq!(players[2].party_id, None); // only 1 lobby member
    }
}
