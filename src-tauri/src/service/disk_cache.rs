// src-tauri/src/service/disk_cache.rs
//
// Persistent disk cache for match detail records.
// Completed match records are immutable, so storing them on disk
// eliminates redundant remote API queries across app restarts.

use serde_json::Value;
use std::path::{Path, PathBuf};
use tokio::fs;

pub const DEFAULT_MAX_DISK_ENTRIES: usize = 2000;

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
        let _ = fs::write(&target_path, &bytes).await;
        tracing::debug!("Direct write fallback for match cache {}: {}", match_id, e);
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
}
