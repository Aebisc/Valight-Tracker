// src-tauri/src/state.rs
//
// AppState: holds the two reqwest clients and the match cache.
// Managed as Tauri state so commands can access it without reconstructing each time.

use lru::LruCache;
use reqwest::Client;
use serde_json::Value;
use std::num::NonZeroUsize;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::model::{MatchInfo, ValorantPlayer};

const MATCH_DETAIL_CACHE_SIZE: usize = 500;

use std::sync::atomic::AtomicU64;

/// Cached result of the last successful `get_match` call.
#[derive(Debug, Clone)]
pub struct MatchCache {
    pub match_id: String,
    pub game_state: String,
    pub players: Vec<ValorantPlayer>,
    pub match_info: MatchInfo,
    pub self_puuid: String,
}

use crate::riot::config::RiotEndpoints;
use crate::service::disk_cache;
use std::path::PathBuf;

pub struct AppState {
    /// Self-signed cert OK — only used for 127.0.0.1 Riot local client.
    pub local_client: Client,
    /// Normal TLS — used for all pd/glz Riot server calls.
    pub remote_client: Client,
    /// Cached last match result. Cleared on match transition.
    pub match_cache: Mutex<Option<MatchCache>>,
    /// In-memory LRU of match detail responses.
    pub detail_cache: Mutex<LruCache<String, Arc<Value>>>,
    /// Single-flight lock: prevents concurrent get_match calls from rebuilding simultaneously.
    pub build_lock: Mutex<()>,
    /// Generation counters for single-flight waiter tracking.
    pub build_started: AtomicU64,
    pub build_finished: AtomicU64,
    /// Optional endpoint override for integration tests / mock server
    pub endpoints_override: Option<RiotEndpoints>,
    /// Directory for persistent match details on disk
    pub cache_dir: Option<PathBuf>,
}

impl AppState {
    pub fn new(local_client: Client, remote_client: Client) -> Self {
        Self::new_with_override(local_client, remote_client, None)
    }

    pub fn new_with_override(
        local_client: Client,
        remote_client: Client,
        endpoints_override: Option<RiotEndpoints>,
    ) -> Self {
        Self::new_with_all_overrides(
            local_client,
            remote_client,
            endpoints_override,
            disk_cache::default_cache_dir(),
        )
    }

    pub fn new_with_all_overrides(
        local_client: Client,
        remote_client: Client,
        endpoints_override: Option<RiotEndpoints>,
        cache_dir: Option<PathBuf>,
    ) -> Self {
        Self {
            local_client,
            remote_client,
            match_cache: Mutex::new(None),
            detail_cache: Mutex::new(LruCache::new(
                NonZeroUsize::new(MATCH_DETAIL_CACHE_SIZE).unwrap(),
            )),
            build_lock: Mutex::new(()),
            build_started: AtomicU64::new(0),
            build_finished: AtomicU64::new(0),
            endpoints_override,
            cache_dir,
        }
    }
}
