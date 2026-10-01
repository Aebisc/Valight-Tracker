// src-tauri/src/service/side.rs
//
// Port of resolveStartingSide() from lib/constants.ts.

use crate::model::TeamSide;

/// Queue IDs where starting side is meaningful (Blue=defence, Red=attack).
pub fn is_side_eligible(queue_id: &str, mode_keyword: &str) -> bool {
    const SIDE_QUEUES: &[&str] = &["competitive", "unrated", "swiftplay", "spikerush"];
    const SIDE_MODE_KEYWORDS: &[&str] = &["bomb", "quickbomb", "spikerush", "swiftplay"];

    let q = queue_id.to_lowercase();
    if !q.is_empty() {
        SIDE_QUEUES.contains(&q.as_str()) || SIDE_MODE_KEYWORDS.contains(&mode_keyword)
    } else {
        // Custom games have empty queue — fall back to mode keyword
        SIDE_MODE_KEYWORDS.contains(&mode_keyword)
    }
}

/// Port of resolveStartingSide().
/// Blue = first half defending, Red = first half attacking.
pub fn resolve_starting_side(
    team_id: Option<&str>,
    queue_id: &str,
    mode_keyword: &str,
) -> Option<TeamSide> {
    let team_id = team_id?;
    if team_id.is_empty() { return None; }
    if !is_side_eligible(queue_id, mode_keyword) { return None; }
    match team_id.to_lowercase().as_str() {
        "blue" => Some(TeamSide::Defence),
        "red"  => Some(TeamSide::Attack),
        _      => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blue_team_is_defence() {
        assert_eq!(
            resolve_starting_side(Some("Blue"), "competitive", ""),
            Some(TeamSide::Defence)
        );
    }

    #[test]
    fn red_team_is_attack() {
        assert_eq!(
            resolve_starting_side(Some("Red"), "competitive", ""),
            Some(TeamSide::Attack)
        );
    }

    #[test]
    fn deathmatch_returns_none() {
        assert_eq!(
            resolve_starting_side(Some("Blue"), "deathmatch", ""),
            None
        );
    }

    #[test]
    fn custom_game_uses_mode_keyword() {
        // Empty queue, but bomb mode keyword → eligible
        assert_eq!(
            resolve_starting_side(Some("Red"), "", "bomb"),
            Some(TeamSide::Attack)
        );
    }

    #[test]
    fn no_team_id_returns_none() {
        assert_eq!(resolve_starting_side(None, "competitive", ""), None);
    }
}
