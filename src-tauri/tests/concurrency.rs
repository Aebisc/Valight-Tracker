use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use wiremock::matchers::{method, path as url_path, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

use valorant_tracker::model::{MatchInfo, MatchResult, ValorantPlayer};
use valorant_tracker::riot::config::RiotEndpoints;
use valorant_tracker::service::match_service::get_match;
use valorant_tracker::state::{AppState, MatchCache};

static TEST_MUTEX: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn setup_mock_server_with_delay(delay_ms: u64) -> MockServer {
    let server = MockServer::start().await;
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let scenario_dir = manifest_dir
        .join("fixtures")
        .join("redacted")
        .join("pregame_comp");

    // 1. Entitlements
    let entitlements_path = scenario_dir.join("entitlements.json");
    if let Ok(content) = std::fs::read_to_string(&entitlements_path) {
        if let Ok(json) = serde_json::from_str::<Value>(&content) {
            Mock::given(method("GET"))
                .and(url_path("/entitlements/v1/token"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(json)
                        .set_delay(Duration::from_millis(delay_ms)),
                )
                .mount(&server)
                .await;
        }
    }

    // 2. Presences
    let presences_path = scenario_dir.join("presences.json");
    if let Ok(content) = std::fs::read_to_string(&presences_path) {
        if let Ok(json) = serde_json::from_str::<Value>(&content) {
            Mock::given(method("GET"))
                .and(url_path("/chat/v4/presences"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(json)
                        .set_delay(Duration::from_millis(delay_ms)),
                )
                .mount(&server)
                .await;
        }
    }

    // 3. Core-game player (404 for pregame)
    Mock::given(method("GET"))
        .and(path_regex(r"^/core-game/v1/players/.*"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    // 4. Pregame player
    let pg_player_path = scenario_dir.join("pregame_player.json");
    if let Ok(content) = std::fs::read_to_string(&pg_player_path) {
        if let Ok(json) = serde_json::from_str::<Value>(&content) {
            Mock::given(method("GET"))
                .and(path_regex(r"^/pregame/v1/players/.*"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(json)
                        .set_delay(Duration::from_millis(delay_ms)),
                )
                .mount(&server)
                .await;
        }
    }

    // 5. Pregame match
    let pg_match_path = scenario_dir.join("pregame_match.json");
    if let Ok(content) = std::fs::read_to_string(&pg_match_path) {
        if let Ok(json) = serde_json::from_str::<Value>(&content) {
            Mock::given(method("GET"))
                .and(path_regex(r"^/pregame/v1/matches/.*"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(json)
                        .set_delay(Duration::from_millis(delay_ms)),
                )
                .mount(&server)
                .await;
        }
    }

    // 6. Names
    let names_path = scenario_dir.join("names.json");
    if let Ok(content) = std::fs::read_to_string(&names_path) {
        if let Ok(json) = serde_json::from_str::<Value>(&content) {
            Mock::given(method("PUT"))
                .and(url_path("/name-service/v2/players"))
                .respond_with(ResponseTemplate::new(200).set_body_json(json))
                .mount(&server)
                .await;
        }
    }

    // 7. Generic matchers for MMR / comp / match details
    if let Ok(entries) = std::fs::read_dir(&scenario_dir) {
        for entry in entries.flatten() {
            let filename = entry.file_name().to_string_lossy().to_string();
            let filepath = entry.path();
            if filename.starts_with("comp_") && filename.ends_with(".json") {
                let puuid = filename
                    .trim_start_matches("comp_")
                    .trim_end_matches(".json");
                if let Ok(content) = std::fs::read_to_string(&filepath) {
                    if let Ok(json) = serde_json::from_str::<Value>(&content) {
                        Mock::given(method("GET"))
                            .and(url_path(format!(
                                "/mmr/v1/players/{}/competitiveupdates",
                                puuid
                            )))
                            .respond_with(ResponseTemplate::new(200).set_body_json(json))
                            .mount(&server)
                            .await;
                    }
                }
            } else if filename.starts_with("mmr_") && filename.ends_with(".json") {
                let puuid = filename
                    .trim_start_matches("mmr_")
                    .trim_end_matches(".json");
                if let Ok(content) = std::fs::read_to_string(&filepath) {
                    if let Ok(json) = serde_json::from_str::<Value>(&content) {
                        Mock::given(method("GET"))
                            .and(url_path(format!("/mmr/v1/players/{}", puuid)))
                            .respond_with(ResponseTemplate::new(200).set_body_json(json))
                            .mount(&server)
                            .await;
                    }
                }
            } else if filename.starts_with("match_") && filename.ends_with(".json") {
                let match_id = filename
                    .trim_start_matches("match_")
                    .trim_end_matches(".json");
                if let Ok(content) = std::fs::read_to_string(&filepath) {
                    if let Ok(json) = serde_json::from_str::<Value>(&content) {
                        Mock::given(method("GET"))
                            .and(url_path(format!("/match-details/v1/matches/{}", match_id)))
                            .respond_with(ResponseTemplate::new(200).set_body_json(json))
                            .mount(&server)
                            .await;
                    }
                }
            }
        }
    }

    server
}

fn setup_temp_env() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let scenario_dir = manifest_dir
        .join("fixtures")
        .join("redacted")
        .join("pregame_comp");
    let temp_dir = std::env::temp_dir().join("valight-concurrency-test");
    let riot_dir = temp_dir
        .join("Riot Games")
        .join("Riot Client")
        .join("Config");
    let val_dir = temp_dir.join("VALORANT").join("Saved").join("Logs");
    let _ = std::fs::create_dir_all(&riot_dir);
    let _ = std::fs::create_dir_all(&val_dir);

    let lockfile_src = scenario_dir.join("lockfile.txt");
    if lockfile_src.exists() {
        let _ = std::fs::copy(&lockfile_src, riot_dir.join("lockfile"));
    }
    let log_src = scenario_dir.join("shootergame_log.txt");
    if log_src.exists() {
        let _ = std::fs::copy(&log_src, val_dir.join("ShooterGame.log"));
    }
    std::env::set_var("VALIGHT_LOCALAPPDATA", &temp_dir);
    temp_dir
}

#[tokio::test]
async fn test_concurrent_get_match_auto_poll_single_build() {
    let _guard = TEST_MUTEX.lock().await;
    let temp_dir = setup_temp_env();
    let server = setup_mock_server_with_delay(40).await;

    let endpoints = RiotEndpoints {
        local_base: server.uri(),
        pd: server.uri(),
        glz: server.uri(),
    };

    let local_client = valorant_tracker::riot::client::build_local_client();
    let remote_client = valorant_tracker::riot::client::build_remote_client();
    let state = Arc::new(AppState::new_with_override(
        local_client,
        remote_client,
        Some(endpoints),
    ));

    // Spawn 5 concurrent get_match(false) requests
    let mut handles = Vec::new();
    for _ in 0..5 {
        let s = state.clone();
        handles.push(tokio::spawn(async move { get_match(false, &s).await }));
    }

    // Must finish within 5 seconds without deadlock
    let results = tokio::time::timeout(Duration::from_secs(5), async {
        let mut res = Vec::new();
        for h in handles {
            res.push(h.await.unwrap());
        }
        res
    })
    .await
    .expect("Concurrency test timed out (deadlock detected)");

    for r in results {
        let resp = r.expect("get_match should succeed");
        assert_eq!(resp.game_state, "PREGAME");
    }

    // Exactly 1 build started and finished; the other 4 reused the cached build
    assert_eq!(
        state
            .build_started
            .load(std::sync::atomic::Ordering::SeqCst),
        1
    );
    assert_eq!(
        state
            .build_finished
            .load(std::sync::atomic::Ordering::SeqCst),
        1
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_forced_refresh_during_build_triggers_rebuild() {
    let _guard = TEST_MUTEX.lock().await;
    let temp_dir = setup_temp_env();
    let server = setup_mock_server_with_delay(50).await;

    let endpoints = RiotEndpoints {
        local_base: server.uri(),
        pd: server.uri(),
        glz: server.uri(),
    };

    let local_client = valorant_tracker::riot::client::build_local_client();
    let remote_client = valorant_tracker::riot::client::build_remote_client();
    let state = Arc::new(AppState::new_with_override(
        local_client,
        remote_client,
        Some(endpoints),
    ));

    // Start auto poll (build 1)
    let s1 = state.clone();
    let h1 = tokio::spawn(async move { get_match(false, &s1).await });

    // Small delay so Task 1 enters first and starts build 1
    tokio::time::sleep(Duration::from_millis(15)).await;

    // Start forced refresh (Task 2)
    let s2 = state.clone();
    let h2 = tokio::spawn(async move { get_match(true, &s2).await });

    let (r1, r2) = tokio::time::timeout(Duration::from_secs(5), async { tokio::join!(h1, h2) })
        .await
        .expect("Timed out waiting for concurrent calls");

    r1.unwrap().expect("Task 1 succeeded");
    r2.unwrap().expect("Task 2 succeeded");

    // Task 2 arrived during Task 1 and forced a refresh, so 2 builds occurred
    assert_eq!(
        state
            .build_started
            .load(std::sync::atomic::Ordering::SeqCst),
        2
    );
    assert_eq!(
        state
            .build_finished
            .load(std::sync::atomic::Ordering::SeqCst),
        2
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_error_path_releases_lock_and_preserves_counter_integrity() {
    let _guard = TEST_MUTEX.lock().await;
    // Set invalid directory so read_lockfile returns an error
    let temp_dir = std::env::temp_dir().join("valight-error-test-empty");
    let _ = std::fs::create_dir_all(&temp_dir);
    // Write an invalid lockfile to cause BadFormat error
    let riot_dir = temp_dir
        .join("Riot Games")
        .join("Riot Client")
        .join("Config");
    let _ = std::fs::create_dir_all(&riot_dir);
    std::fs::write(riot_dir.join("lockfile"), "invalid_format").unwrap();
    std::env::set_var("VALIGHT_LOCALAPPDATA", &temp_dir);

    let local_client = valorant_tracker::riot::client::build_local_client();
    let remote_client = valorant_tracker::riot::client::build_remote_client();
    let state = AppState::new(local_client, remote_client);

    let res = get_match(false, &state).await;
    assert!(res.is_err(), "Expected error due to invalid lockfile");

    // Counters must be synchronized even on error path!
    let started = state
        .build_started
        .load(std::sync::atomic::Ordering::SeqCst);
    let finished = state
        .build_finished
        .load(std::sync::atomic::Ordering::SeqCst);
    assert_eq!(started, 1);
    assert_eq!(finished, 1);

    // Lock must be released so next call can proceed immediately
    let lock_try = state.build_lock.try_lock();
    assert!(
        lock_try.is_ok(),
        "build_lock should be unlocked after error exit"
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_pregame_to_ingame_preserves_teammates_and_queries_only_enemies() {
    let _guard = TEST_MUTEX.lock().await;
    let temp_dir = setup_temp_env();
    let server = MockServer::start().await;

    let scenario_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("redacted")
        .join("pregame_comp");

    // 1. Entitlements
    let entitlements_json: Value = serde_json::from_str(
        &std::fs::read_to_string(scenario_dir.join("entitlements.json")).unwrap(),
    )
    .unwrap();
    Mock::given(method("GET"))
        .and(url_path("/entitlements/v1/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(entitlements_json))
        .mount(&server)
        .await;

    // 2. Presences
    Mock::given(method("GET"))
        .and(url_path("/chat/v4/presences"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "presences": []
        })))
        .mount(&server)
        .await;

    // 3. Pregame player (404, we are no longer in pregame)
    Mock::given(method("GET"))
        .and(path_regex(r"^/pregame/v1/players/.*"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    // 4. Core-game player (200 with match-core-100)
    Mock::given(method("GET"))
        .and(path_regex(r"^/core-game/v1/players/.*"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "MatchID": "match-core-100"
        })))
        .mount(&server)
        .await;

    // 5. Coregame match with 10 players (5 Blue, 5 Red)
    let teammate_puuids = vec![
        "aaaaaaaa-0014-0000-0000-000000000000",
        "tm-0002",
        "tm-0003",
        "tm-0004",
        "tm-0005",
    ];
    let enemy_puuids = vec![
        "en-0001",
        "en-0002",
        "en-0003",
        "en-0004",
        "en-0005",
    ];

    let mut cg_players = Vec::new();
    for puuid in &teammate_puuids {
        cg_players.push(serde_json::json!({
            "Subject": puuid,
            "TeamID": "Blue",
            "CharacterID": "add6443a-41bd-e414-f6ad-e58d267f4e95", // Jett
            "PlayerIdentity": {
                "Subject": puuid,
                "AccountLevel": 42
            }
        }));
    }
    for puuid in &enemy_puuids {
        cg_players.push(serde_json::json!({
            "Subject": puuid,
            "TeamID": "Red",
            "CharacterID": "f94c3b30-42be-e959-889c-5aa313dba261", // Raze
            "PlayerIdentity": {
                "Subject": puuid,
                "AccountLevel": 99
            }
        }));
    }

    Mock::given(method("GET"))
        .and(url_path("/core-game/v1/matches/match-core-100"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "MatchID": "match-core-100",
            "MapID": "/Game/Maps/Ascent/Ascent",
            "Mode": "/Game/GameModes/Bomb/BombGameMode.BombGameMode_C",
            "ModeID": "competitive",
            "IsRanked": true,
            "GamePodID": "aresriot.aws-rso-frankfurt-1",
            "SeasonID": "season-123",
            "Players": cg_players
        })))
        .mount(&server)
        .await;

    // 6. Name service: mock for enemy lookups
    Mock::given(method("PUT"))
        .and(url_path("/name-service/v2/players"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {"Subject": "en-0001", "GameName": "Enemy1", "TagLine": "EUW"},
            {"Subject": "en-0002", "GameName": "Enemy2", "TagLine": "EUW"},
            {"Subject": "en-0003", "GameName": "Enemy3", "TagLine": "EUW"},
            {"Subject": "en-0004", "GameName": "Enemy4", "TagLine": "EUW"},
            {"Subject": "en-0005", "GameName": "Enemy5", "TagLine": "EUW"}
        ])))
        .mount(&server)
        .await;

    // 7. MMR and comp updates: mock for all players
    Mock::given(method("GET"))
        .and(path_regex(r"^/mmr/v1/players/.*/competitiveupdates.*"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "Matches": []
        })))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path_regex(r"^/mmr/v1/players/.*"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "LatestCompetitiveUpdate": {
                "TierAfterUpdate": 15,
                "TierBeforeUpdate": 14,
                "RankedRatingAfterUpdate": 60,
                "RankedRatingEarned": 15
            },
            "QueueSkills": {
                "competitive": {
                    "SeasonalInfoBySeasonID": {}
                }
            }
        })))
        .mount(&server)
        .await;

    let endpoints = RiotEndpoints {
        local_base: server.uri(),
        pd: server.uri(),
        glz: server.uri(),
    };

    let local_client = valorant_tracker::riot::client::build_local_client();
    let remote_client = valorant_tracker::riot::client::build_remote_client();
    let state = Arc::new(AppState::new_with_override(
        local_client,
        remote_client,
        Some(endpoints),
    ));

    // Seed the cache with PREGAME state containing the 5 teammates
    let mut pregame_players = Vec::new();
    for puuid in &teammate_puuids {
        pregame_players.push(ValorantPlayer {
            puuid: puuid.to_string(),
            name: format!("Name-{}", puuid),
            tag: "TAG".to_string(),
            agent_id: "".to_string(),
            agent_name: "Unknown".to_string(),
            team_id: "Blue".to_string(),
            account_level: 20,
            rank: 18,
            rank_name: "Diamond 3".to_string(),
            peak_rank: 21,
            peak_rank_name: "Immortal 1".to_string(),
            previous_rank: 17,
            rr: 75,
            earned_rr: 20,
            leaderboard_position: 0,
            headshots: 100,
            bodyshots: 200,
            legshots: 10,
            headshot_percent: 32.2,
            winrate: 65.0,
            kd: 1.85,
            kills: 22.0,
            deaths: 12.0,
            assists: 5.0,
            acs: 280,
            adr: 165.0,
            current_season_wins: 15,
            current_season_games: 23,
            is_current_act_rank: true,
            recent_games_count: 5,
            last_match_kills: 24,
            last_match_deaths: 10,
            last_match_assists: 4,
            last_match_kd: 2.4,
            recent_results: vec![MatchResult::W, MatchResult::W],
            party_id: None,
            party_number: None,
            party_size: None,
        });
    }

    *state.match_cache.lock().await = Some(MatchCache {
        match_id: "match-pre-999".to_string(),
        game_state: "PREGAME".to_string(),
        players: pregame_players,
        match_info: MatchInfo {
            match_id: "match-pre-999".to_string(),
            map_id: "/Game/Maps/Ascent/Ascent".to_string(),
            map_name: "Ascent".to_string(),
            game_mode: "Bomb".to_string(),
            game_mode_id: "competitive".to_string(),
            game_mode_name: "Competitive".to_string(),
            is_deathmatch: false,
            server: "Frankfurt".to_string(),
            is_ranked: true,
            game_state: "PREGAME".to_string(),
            season_id: "season-123".to_string(),
            starting_side: None,
        },
        self_puuid: "aaaaaaaa-0014-0000-0000-000000000000".to_string(),
    });

    let res = get_match(false, &state).await.expect("get_match should succeed");
    assert_eq!(res.game_state, "INGAME");

    let players = res.players.expect("players should be present");
    assert_eq!(players.len(), 10);

    for p in players.iter().filter(|p| teammate_puuids.contains(&p.puuid.as_str())) {
        assert_eq!(p.kd, 1.85, "Teammate KD should be preserved from pregame");
        assert_eq!(p.acs, 280, "Teammate ACS should be preserved from pregame");
        assert_eq!(p.headshot_percent, 32.2, "Teammate HS% should be preserved");
        assert_eq!(p.agent_name, "Jett", "Teammate agent should be updated to Jett");
        assert_eq!(p.account_level, 42, "Teammate account level updated from coregame");
    }

    for p in players.iter().filter(|p| enemy_puuids.contains(&p.puuid.as_str())) {
        assert_eq!(p.rank, 15, "Enemy rank should be from fetched MMR");
        assert_eq!(p.agent_name, "Raze", "Enemy agent should be Raze");
    }

    let received = server.received_requests().await.unwrap();
    let mut mmr_requested_puuids = Vec::new();
    for req in &received {
        let path = req.url.path();
        if path.starts_with("/mmr/v1/players/") && !path.contains("competitiveupdates") {
            let puuid = path.strip_prefix("/mmr/v1/players/").unwrap();
            mmr_requested_puuids.push(puuid.to_string());
        }
    }

    for tm in &teammate_puuids {
        assert!(
            !mmr_requested_puuids.contains(&tm.to_string()),
            "Teammate {} should NOT have been queried for MMR!",
            tm
        );
    }
    for en in &enemy_puuids {
        assert!(
            mmr_requested_puuids.contains(&en.to_string()),
            "Enemy {} SHOULD have been queried for MMR!",
            en
        );
    }

    let _ = std::fs::remove_dir_all(&temp_dir);
}

