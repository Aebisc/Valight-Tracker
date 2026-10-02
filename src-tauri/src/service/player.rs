// src-tauri/src/service/player.rs
//
// Pure logic ports of:
//   - getPeakRank()
//   - getCurrentSeasonId()
//
// No I/O. All inputs are serde_json::Value (safe accessors, never unwrap).
// Rounding helpers match JS Math.round behaviour exactly.

use serde_json::Value;

// ─── Rounding helpers ─────────────────────────────────────────────────────────
// JS Math.round rounds to the nearest integer, with 0.5 rounding half toward
// +∞ (matching Rust's f64::round for non-negative numbers). Since all stats
// rounded here are non-negative, the behaviour is identical. We replicate
// Math.round(x * 10) / 10 and Math.round(x * 100) / 100 exactly.

/// Round to one decimal place, matching JS Math.round(x * 10) / 10.
#[inline]
pub fn round1(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
}

/// Round to two decimal places, matching JS Math.round(x * 100) / 100.
#[inline]
pub fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

/// Round to one decimal place for percentages: Math.round(x * 1000) / 10.
#[inline]
pub fn round_pct(x: f64) -> f64 {
    (x * 1000.0).round() / 10.0
}

// ─── getPeakRank ──────────────────────────────────────────────────────────────

/// Port of getPeakRank(). Scans all seasonal data, recent competitive updates,
/// and the latest comp entry to find the highest tier ≤ 27.
pub fn get_peak_rank(
    seasonal_info: &Value,
    recent_matches: &[Value],
    latest_comp: Option<&Value>,
) -> u32 {
    let mut peak: u32 = 0;

    if let Some(obj) = seasonal_info.as_object() {
        for (_season_id, season) in obj {
            // 1. CompetitiveTier
            if let Some(t) = season["CompetitiveTier"].as_u64() {
                let t = t as u32;
                if t > peak && t <= 27 { peak = t; }
            }
            // 2. Rank (peak act rank)
            if let Some(t) = season["Rank"].as_u64() {
                let t = t as u32;
                if t > peak && t <= 27 { peak = t; }
            }
            // 3. WinsByTier
            if let Some(wbt) = season["WinsByTier"].as_object() {
                for (tier_str, wins) in wbt {
                    if let (Ok(tier), Some(win_count)) = (tier_str.parse::<u32>(), wins.as_u64()) {
                        if tier > peak && tier <= 27 && win_count > 0 {
                            peak = tier;
                        }
                    }
                }
            }
        }
    }

    // 4. Latest competitive update
    if let Some(lc) = latest_comp {
        if let Some(t) = lc["TierAfterUpdate"].as_u64() {
            let t = t as u32; if t > peak && t <= 27 { peak = t; }
        }
        if let Some(t) = lc["TierBeforeUpdate"].as_u64() {
            let t = t as u32; if t > peak && t <= 27 { peak = t; }
        }
    }

    // 5. Recent matches
    for m in recent_matches {
        if let Some(t) = m["TierAfterUpdate"].as_u64() {
            let t = t as u32; if t > peak && t <= 27 { peak = t; }
        }
        if let Some(t) = m["TierBeforeUpdate"].as_u64() {
            let t = t as u32; if t > peak && t <= 27 { peak = t; }
        }
    }

    peak
}

// ─── getCurrentSeasonId ───────────────────────────────────────────────────────

/// Port of getCurrentSeasonId(). Returns the most recent season that has games,
/// falling back to the one with the lexicographically largest season ID.
pub fn get_current_season_id(seasonal_info: &Value, match_season_id: &str) -> Option<String> {
    if !match_season_id.is_empty() && seasonal_info[match_season_id].is_object() {
        return Some(match_season_id.to_string());
    }

    let mut best: Option<String> = None;
    if let Some(obj) = seasonal_info.as_object() {
        for (season_id, season) in obj {
            let games = season["NumberOfGames"].as_u64().unwrap_or(0);
            if games > 0 {
                match &best {
                    None => best = Some(season_id.clone()),
                    Some(b) if season_id > b => best = Some(season_id.clone()),
                    _ => {}
                }
            }
        }
    }
    best
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn peak_rank_from_seasonal_info() {
        let info = json!({
            "season1": { "CompetitiveTier": 15, "Rank": 12, "WinsByTier": { "16": 3 } },
            "season2": { "CompetitiveTier": 18, "Rank": 0 },
        });
        assert_eq!(get_peak_rank(&info, &[], None), 18);
    }

    #[test]
    fn peak_rank_capped_at_27() {
        let info = json!({ "s": { "CompetitiveTier": 99 } });
        assert_eq!(get_peak_rank(&info, &[], None), 0);
    }

    #[test]
    fn peak_rank_from_recent_matches() {
        let matches = vec![
            json!({ "TierAfterUpdate": 22, "TierBeforeUpdate": 21 }),
        ];
        assert_eq!(get_peak_rank(&json!({}), &matches, None), 22);
    }

    #[test]
    fn rounding_matches_js() {
        // JS: Math.round(0.15 * 1000) / 10 = 15.0
        assert_eq!(round_pct(0.15), 15.0);
        // JS: Math.round(1.005 * 100) / 100 = 1 (due to IEEE-754 1.005 being 1.0049999999999998934...)
        // Both JS and Rust produce 1.0 here due to IEEE 754 float representation.
        assert_eq!(round2(1.005_f64), 1.0);
    }

    #[test]
    fn current_season_falls_back_to_latest() {
        let info = json!({
            "s2023-1": { "NumberOfGames": 10 },
            "s2024-1": { "NumberOfGames": 5 },
        });
        assert_eq!(get_current_season_id(&info, ""), Some("s2024-1".to_string()));
    }

    #[test]
    fn current_season_uses_match_season_if_present() {
        let info = json!({
            "s2024-1": { "NumberOfGames": 5 },
        });
        assert_eq!(
            get_current_season_id(&info, "s2024-1"),
            Some("s2024-1".to_string())
        );
    }
}
