// src-tauri/src/state.rs
//
// AppState: holds the two reqwest clients and the match cache.
// Managed as Tauri state so commands can access it without reconstructing each time.

use std::sync::Arc;
use tokio::sync::Mutex;
use lru::LruCache;
use std::num::NonZeroUsize;
use reqwest::Client;
use serde_json::Value;

use crate::model::{ValorantPlayer, MatchInfo};

const MATCH_DETAIL_CACHE_SIZE: usize = 500;

/// Cached result of the last successful `get_match` call.
#[derive(Debug, Clone)]
pub struct MatchCache {
    pub match_id: String,
    pub game_state: String,
    pub players: Vec<ValorantPlayer>,
    pub match_info: MatchInfo,
}

use crate::riot::config::RiotEndpoints;

pub struct AppState {
    /// Self-signed cert OK — only used for 127.0.0.1 Riot local client.
    pub local_client: Client,
    /// Normal TLS — used for all pd/glz Riot server calls.
    pub remote_client: Client,
    /// Cached last match result. Cleared on match transition.
    pub match_cache: Mutex<Option<MatchCache>>,
    /// Persistent LRU of up to 500 match detail responses.
    /// Survives match transitions so recent history is always warm.
    pub detail_cache: Mutex<LruCache<String, Arc<Value>>>,
    /// Single-flight lock: prevents two concurrent get_match calls from
    /// both rebuilding simultaneously (replaces Node's implicit serialisation).
    pub build_lock: Mutex<()>,
    /// Optional endpoint override for integration tests / mock server
    pub endpoints_override: Option<RiotEndpoints>,
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
        Self {
            local_client,
            remote_client,
            match_cache: Mutex::new(None),
            detail_cache: Mutex::new(LruCache::new(
                NonZeroUsize::new(MATCH_DETAIL_CACHE_SIZE).unwrap(),
            )),
            build_lock: Mutex::new(()),
            endpoints_override,
        }
    }
}
