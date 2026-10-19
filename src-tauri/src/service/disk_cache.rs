// src-tauri/src/service/disk_cache.rs
//
// Persistent disk cache for match detail records.
// Completed match records are immutable, so storing them on disk
// eliminates redundant remote API queries across app restarts.

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::fs;

pub const DEFAULT_MAX_DISK_ENTRIES: usize = 2000;
pub const PRUNE_WRITE_THRESHOLD: usize = 50;

static WRITES_SINCE_PRUNE: AtomicUsize = AtomicUsize::new(0);

/// Resolves the default persistent match cache directory:
/// `%LOCALAPPDATA%/VaLight-Tracker/match_cache`
pub fn default_cache_dir() -> Option<PathBuf> {
    crate::riot::lockfile::local_app_data()
        .ok()
        .map(|p| p.join("VaLight-Tracker").join("match_cache"))
}

/// Sanitizes match_id so it can safely be used as a filename.
fn match_id_to_filename(match_id: &str) -> String {
    let clean: String = match_id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    format!("{}.json", clean)
}

/// Reads a match detail from the persistent disk cache.
pub async fn read_match_detail(cache_dir: &Path, match_id: &str) -> Option<Value> {
    if match_id.is_empty() {
        return None;
    }
    let file_path = cache_dir.join(match_id_to_filename(match_id));
    let bytes = fs::read(&file_path).await.ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Writes a match detail to the disk cache atomically using a temporary file.
pub async fn write_match_detail(cache_dir: &Path, match_id: &str, data: &Value) {
    if match_id.is_empty() {
        return;
    }
    if let Err(e) = fs::create_dir_all(cache_dir).await {
        tracing::warn!("Failed to create match cache dir {:?}: {}", cache_dir, e);
        return;
    }

    static TEMP_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let nonce = TEMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let target_path = cache_dir.join(match_id_to_filename(match_id));
    let temp_path = cache_dir.join(format!(
        "{}.tmp.{}.{}",
        match_id_to_filename(match_id),
        std::process::id(),
        nonce
    ));

    let bytes = match serde_json::to_vec(data) {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!("Failed to serialize match detail {}: {}", match_id, e);
            return;
        }
    };

    if let Err(e) = fs::write(&temp_path, &bytes).await {
        tracing::warn!(
            "Failed to write temp match cache file {:?}: {}",
            temp_path,
            e
        );
        let _ = fs::remove_file(&temp_path).await;
        return;
    }

    if let Err(e) = fs::rename(&temp_path, &target_path).await {
        // If rename fails (e.g. cross-filesystem on exotic configs), fall back to standard write
        let _ = fs::remove_file(&temp_path).await;
        if let Err(e2) = fs::write(&target_path, &bytes).await {
            tracing::warn!("Failed fallback write for match cache {}: {}", match_id, e2);
            return;
        }
        tracing::debug!("Direct write fallback for match cache {}: {}", match_id, e);
    }

    WRITES_SINCE_PRUNE.fetch_add(1, Ordering::Relaxed);
}

/// Checks if enough writes have occurred to justify scanning the disk cache directory for pruning.
pub async fn maybe_prune_cache(cache_dir: &Path, max_entries: usize) {
    if WRITES_SINCE_PRUNE.load(Ordering::Relaxed) >= PRUNE_WRITE_THRESHOLD {
        WRITES_SINCE_PRUNE.store(0, Ordering::Relaxed);
        prune_cache(cache_dir, max_entries).await;
    }
}

/// Prunes old cache files if entry count exceeds max_entries.
pub async fn prune_cache(cache_dir: &Path, max_entries: usize) {
    let mut entries = match fs::read_dir(cache_dir).await {
        Ok(rd) => rd,
        Err(_) => return,
    };

    let mut files = Vec::new();
    while let Ok(Some(entry)) = entries.next_entry().await {
        if let Ok(meta) = entry.metadata().await {
            if meta.is_file() {
                if let Some(ext) = entry.path().extension() {
                    if ext == "json" {
                        let modified = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                        files.push((entry.path(), modified));
                    }
                }
            }
        }
    }

    if files.len() > max_entries {
        // Sort oldest first
        files.sort_by_key(|(_, modified)| *modified);
        let to_remove = files.len() - max_entries;
        for (path, _) in files.into_iter().take(to_remove) {
            let _ = fs::remove_file(path).await;
        }
    }
}

/// Extracts a compact subset of match details preserving only the fields needed by stats and party calculations.
/// Strips out coordinate arrays, xpModifications, behaviorFactors, roundDamage, etc.
pub fn slim_match_detail(raw: &Value) -> Value {
    let players = raw["players"].as_array().map(|arr| {
        let slim_players: Vec<Value> = arr
            .iter()
            .map(|p| {
                let mut map = serde_json::Map::new();
                if let Some(subject) = p.get("subject").or_else(|| p.get("Subject")) {
                    map.insert("subject".to_string(), subject.clone());
                }
                if let Some(team_id) = p.get("teamId").or_else(|| p.get("TeamID")) {
                    map.insert("teamId".to_string(), team_id.clone());
                }
                if let Some(party_id) = p.get("partyId").or_else(|| p.get("PartyID")) {
                    map.insert("partyId".to_string(), party_id.clone());
                }
                if let Some(s) = p.get("stats").and_then(|s| s.as_object()) {
                    let mut stats_map = serde_json::Map::new();
                    if let Some(v) = s.get("kills") {
                        stats_map.insert("kills".to_string(), v.clone());
                    }
                    if let Some(v) = s.get("deaths") {
                        stats_map.insert("deaths".to_string(), v.clone());
                    }
                    if let Some(v) = s.get("assists") {
                        stats_map.insert("assists".to_string(), v.clone());
                    }
                    if let Some(v) = s.get("score") {
                        stats_map.insert("score".to_string(), v.clone());
                    }
                    if let Some(v) = s.get("roundsPlayed") {
                        stats_map.insert("roundsPlayed".to_string(), v.clone());
                    }
                    map.insert("stats".to_string(), Value::Object(stats_map));
                }
                Value::Object(map)
            })
            .collect();
        Value::Array(slim_players)
    }).unwrap_or_else(|| Value::Array(Vec::new()));

    let teams = raw["teams"].as_array().map(|arr| {
        let slim_teams: Vec<Value> = arr
            .iter()
            .map(|t| {
                let mut map = serde_json::Map::new();
                if let Some(v) = t.get("teamId") {
                    map.insert("teamId".to_string(), v.clone());
                }
                if let Some(v) = t.get("won") {
                    map.insert("won".to_string(), v.clone());
                }
                if let Some(v) = t.get("roundsWon") {
                    map.insert("roundsWon".to_string(), v.clone());
                }
                if let Some(v) = t.get("roundsPlayed") {
                    map.insert("roundsPlayed".to_string(), v.clone());
                }
                Value::Object(map)
            })
            .collect();
        Value::Array(slim_teams)
    }).unwrap_or_else(|| Value::Array(Vec::new()));

    let round_results = raw["roundResults"].as_array().map(|arr| {
        let slim_rounds: Vec<Value> = arr
            .iter()
            .map(|r| {
                let mut map = serde_json::Map::new();
                if let Some(ps_arr) = r.get("playerStats").and_then(|ps| ps.as_array()) {
                    let slim_ps: Vec<Value> = ps_arr
                        .iter()
                        .map(|ps| {
                            let mut ps_map = serde_json::Map::new();
                            if let Some(subject) = ps.get("subject").or_else(|| ps.get("Subject")) {
                                ps_map.insert("subject".to_string(), subject.clone());
                            }
                            if let Some(dmgs) = ps.get("damage").and_then(|d| d.as_array()) {
                                let slim_dmgs: Vec<Value> = dmgs
                                    .iter()
                                    .map(|d| {
                                        let mut d_map = serde_json::Map::new();
                                        if let Some(v) = d.get("damage") {
                                            d_map.insert("damage".to_string(), v.clone());
                                        }
                                        if let Some(v) = d.get("headshots") {
                                            d_map.insert("headshots".to_string(), v.clone());
                                        }
                                        if let Some(v) = d.get("bodyshots") {
                                            d_map.insert("bodyshots".to_string(), v.clone());
                                        }
                                        if let Some(v) = d.get("legshots") {
                                            d_map.insert("legshots".to_string(), v.clone());
                                        }
                                        Value::Object(d_map)
                                    })
                                    .collect();
                                ps_map.insert("damage".to_string(), Value::Array(slim_dmgs));
                            }
                            Value::Object(ps_map)
                        })
                        .collect();
                    map.insert("playerStats".to_string(), Value::Array(slim_ps));
                }
                Value::Object(map)
            })
            .collect();
        Value::Array(slim_rounds)
    }).unwrap_or_else(|| Value::Array(Vec::new()));

    serde_json::json!({
        "players": players,
        "teams": teams,
        "roundResults": round_results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_disk_cache_roundtrip() {
        let temp_dir = std::env::temp_dir().join("valight_test_cache_roundtrip");
        let _ = fs::remove_dir_all(&temp_dir).await;

        let test_match_id = "test-match-12345";
        let test_data = serde_json::json!({
            "matchInfo": {
                "matchId": test_match_id,
                "mapId": "/Game/Maps/Ascent/Ascent"
            }
        });

        // Read non-existent
        assert!(read_match_detail(&temp_dir, test_match_id).await.is_none());

        // Write
        write_match_detail(&temp_dir, test_match_id, &test_data).await;

        // Read back
        let read = read_match_detail(&temp_dir, test_match_id).await;
        assert!(read.is_some());
        assert_eq!(read.unwrap()["matchInfo"]["matchId"], test_match_id);

        let _ = fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn test_disk_cache_prune() {
        let temp_dir = std::env::temp_dir().join("valight_test_cache_prune");
        let _ = fs::remove_dir_all(&temp_dir).await;

        for i in 0..5 {
            let mid = format!("match-{}", i);
            let data = serde_json::json!({ "id": i });
            write_match_detail(&temp_dir, &mid, &data).await;
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }

        prune_cache(&temp_dir, 3).await;

        let mut count = 0;
        let mut rd = fs::read_dir(&temp_dir).await.unwrap();
        while let Ok(Some(_)) = rd.next_entry().await {
            count += 1;
        }
        assert_eq!(count, 3);

        let _ = fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn test_maybe_prune_cache_threshold() {
        let temp_dir = std::env::temp_dir().join("valight_test_maybe_prune");
        let _ = fs::remove_dir_all(&temp_dir).await;

        WRITES_SINCE_PRUNE.store(0, Ordering::Relaxed);

        // Under threshold: does not prune
        for i in 0..10 {
            let mid = format!("under-{}", i);
            let data = serde_json::json!({ "id": i });
            write_match_detail(&temp_dir, &mid, &data).await;
        }
        maybe_prune_cache(&temp_dir, 5).await;
        let mut count = 0;
        let mut rd = fs::read_dir(&temp_dir).await.unwrap();
        while let Ok(Some(_)) = rd.next_entry().await {
            count += 1;
        }
        assert_eq!(count, 10);

        // At or above threshold: triggers prune
        WRITES_SINCE_PRUNE.store(PRUNE_WRITE_THRESHOLD, Ordering::Relaxed);
        maybe_prune_cache(&temp_dir, 5).await;
        assert_eq!(WRITES_SINCE_PRUNE.load(Ordering::Relaxed), 0);

        let mut count_after = 0;
        let mut rd_after = fs::read_dir(&temp_dir).await.unwrap();
        while let Ok(Some(_)) = rd_after.next_entry().await {
            count_after += 1;
        }
        assert_eq!(count_after, 5);

        let _ = fs::remove_dir_all(&temp_dir).await;
    }

    #[test]
    fn test_slim_match_detail_preserves_stats_and_parties() {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let fixture_path = manifest_dir
            .join("fixtures")
            .join("redacted")
            .join("pregame_comp")
            .join("match_02fde978-8f52-4145-a3c2-b93917e8d8a9.json");
        let raw_str = std::fs::read_to_string(&fixture_path).expect("fixture should exist");
        let raw: Value = serde_json::from_str(&raw_str).expect("fixture is valid JSON");
        let slim = slim_match_detail(&raw);

        // Verify payload size reduction (> 80%)
        let raw_len = serde_json::to_vec(&raw).unwrap().len();
        let slim_len = serde_json::to_vec(&slim).unwrap().len();
        assert!(slim_len < raw_len / 5, "Slimmed payload should be significantly smaller than raw");

        let puuid = "aaaaaaaa-0048-0000-0000-000000000000";
        let stats_raw = crate::service::stats::extract_player_stats(&raw, puuid);
        let stats_slim = crate::service::stats::extract_player_stats(&slim, puuid);
        assert_eq!(stats_raw.kills, stats_slim.kills);
        assert_eq!(stats_raw.deaths, stats_slim.deaths);
        assert_eq!(stats_raw.assists, stats_slim.assists);
        assert_eq!(stats_raw.kd, stats_slim.kd);
        assert_eq!(stats_raw.headshots, stats_slim.headshots);
        assert_eq!(stats_raw.bodyshots, stats_slim.bodyshots);
        assert_eq!(stats_raw.legshots, stats_slim.legshots);
        assert_eq!(stats_raw.headshot_percent, stats_slim.headshot_percent);
        assert_eq!(stats_raw.acs, stats_slim.acs);
        assert_eq!(stats_raw.adr, stats_slim.adr);
        assert_eq!(stats_raw.won, stats_slim.won);

        let res_raw = crate::service::stats::get_match_result(&raw, puuid);
        let res_slim = crate::service::stats::get_match_result(&slim, puuid);
        assert_eq!(res_raw, res_slim);

        let p1 = crate::model::ValorantPlayer {
            puuid: puuid.to_string(),
            ..Default::default()
        };
        let mut players_raw = vec![p1.clone()];
        let mut players_slim = vec![p1];
        crate::service::party::enrich_party_from_match_history(&mut players_raw, &[&raw]);
        crate::service::party::enrich_party_from_match_history(&mut players_slim, &[&slim]);
        assert_eq!(players_raw[0].party_id, players_slim[0].party_id);
        assert_eq!(players_raw[0].party_size, players_slim[0].party_size);
    }
}
