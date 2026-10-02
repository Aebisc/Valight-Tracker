use std::path::{Path, PathBuf};
use serde_json::Value;
use wiremock::{MockServer, ResponseTemplate, Mock};
use wiremock::matchers::{method, path as url_path, path_regex};

use valorant_tracker::riot::config::RiotEndpoints;
use valorant_tracker::state::AppState;
use valorant_tracker::service::match_service::get_match;

fn allowlist_null_or_missing(key: &str) -> bool {
    matches!(key, "match" | "party" | "startingSide")
}

fn assert_json_match(actual: &Value, expected: &Value, path: &str) {
    match (actual, expected) {
        (Value::Object(act_map), Value::Object(exp_map)) => {
            let mut all_keys: std::collections::BTreeSet<&String> = act_map.keys().collect();
            all_keys.extend(exp_map.keys());

            for key in all_keys {
                let current_path = format!("{}.{}", path, key);
                let act_val = act_map.get(key);
                let exp_val = exp_map.get(key);

                match (act_val, exp_val) {
                    (Some(a), Some(e)) => {
                        assert_json_match(a, e, &current_path);
                    }
                    (Some(a), None) => {
                        if allowlist_null_or_missing(key) && a.is_null() {
                            continue;
                        }
                        panic!("Extra key '{}' at {} with value {:?}", key, current_path, a);
                    }
                    (None, Some(e)) => {
                        if allowlist_null_or_missing(key) && e.is_null() {
                            continue;
                        }
                        panic!("Missing expected key '{}' at {} with value {:?}", key, current_path, e);
                    }
                    (None, None) => unreachable!(),
                }
            }
        }
        (Value::Array(act_arr), Value::Array(exp_arr)) => {
            assert_eq!(
                act_arr.len(),
                exp_arr.len(),
                "Array length mismatch at {}: actual={}, expected={}",
                path,
                act_arr.len(),
                exp_arr.len()
            );
            for (i, (a, e)) in act_arr.iter().zip(exp_arr.iter()).enumerate() {
                assert_json_match(a, e, &format!("{}[{}]", path, i));
            }
        }
        (Value::Number(a), Value::Number(e)) => {
            if let (Some(fa), Some(fe)) = (a.as_f64(), e.as_f64()) {
                let diff = (fa - fe).abs();
                assert!(
                    diff < 1e-9,
                    "Float mismatch at {}: actual={}, expected={}, diff={}",
                    path,
                    fa,
                    fe,
                    diff
                );
            } else {
                assert_eq!(a, e, "Number mismatch at {}", path);
            }
        }
        (a, e) => {
            assert_eq!(a, e, "Value mismatch at {}", path);
        }
    }
}

async fn setup_wiremock(scenario_dir: &Path) -> MockServer {
    let server = MockServer::start().await;

    // 1. Entitlements
    let entitlements_path = scenario_dir.join("entitlements.json");
    if let Ok(content) = std::fs::read_to_string(&entitlements_path) {
        if let Ok(json) = serde_json::from_str::<Value>(&content) {
            Mock::given(method("GET"))
                .and(url_path("/entitlements/v1/token"))
                .respond_with(ResponseTemplate::new(200).set_body_json(json))
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
                .respond_with(ResponseTemplate::new(200).set_body_json(json))
                .mount(&server)
                .await;
        }
    }

    // 3. Core-game player
    let cg_player_path = scenario_dir.join("coregame_player.json");
    if let Ok(content) = std::fs::read_to_string(&cg_player_path) {
        if let Ok(json) = serde_json::from_str::<Value>(&content) {
            if json.is_null() || json.get("MatchID").is_none() {
                Mock::given(method("GET"))
                    .and(path_regex(r"^/core-game/v1/players/.*"))
                    .respond_with(ResponseTemplate::new(404))
                    .mount(&server)
                    .await;
            } else {
                Mock::given(method("GET"))
                    .and(path_regex(r"^/core-game/v1/players/.*"))
                    .respond_with(ResponseTemplate::new(200).set_body_json(json))
                    .mount(&server)
                    .await;
            }
        }
    }

    // 4. Pregame player
    let pg_player_path = scenario_dir.join("pregame_player.json");
    if let Ok(content) = std::fs::read_to_string(&pg_player_path) {
        if let Ok(json) = serde_json::from_str::<Value>(&content) {
            if json.is_null() || json.get("MatchID").is_none() {
                Mock::given(method("GET"))
                    .and(path_regex(r"^/pregame/v1/players/.*"))
                    .respond_with(ResponseTemplate::new(404))
                    .mount(&server)
                    .await;
            } else {
                Mock::given(method("GET"))
                    .and(path_regex(r"^/pregame/v1/players/.*"))
                    .respond_with(ResponseTemplate::new(200).set_body_json(json))
                    .mount(&server)
                    .await;
            }
        }
    }

    // 5. Core-game match
    let cg_match_path = scenario_dir.join("coregame_match.json");
    if let Ok(content) = std::fs::read_to_string(&cg_match_path) {
        if let Ok(json) = serde_json::from_str::<Value>(&content) {
            if !json.is_null() {
                Mock::given(method("GET"))
                    .and(path_regex(r"^/core-game/v1/matches/.*"))
                    .respond_with(ResponseTemplate::new(200).set_body_json(json))
                    .mount(&server)
                    .await;
            }
        }
    }

    // 6. Pregame match
    let pg_match_path = scenario_dir.join("pregame_match.json");
    if let Ok(content) = std::fs::read_to_string(&pg_match_path) {
        if let Ok(json) = serde_json::from_str::<Value>(&content) {
            if !json.is_null() {
                Mock::given(method("GET"))
                    .and(path_regex(r"^/pregame/v1/matches/.*"))
                    .respond_with(ResponseTemplate::new(200).set_body_json(json))
                    .mount(&server)
                    .await;
            }
        }
    }

    // 7. Names
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

    // Read all scenario files for comp, mmr, and match-details
    if let Ok(entries) = std::fs::read_dir(scenario_dir) {
        for entry in entries.flatten() {
            let filename = entry.file_name().to_string_lossy().to_string();
            let filepath = entry.path();

            if filename.starts_with("comp_") && filename.ends_with(".json") {
                let puuid = filename.trim_start_matches("comp_").trim_end_matches(".json");
                if let Ok(content) = std::fs::read_to_string(&filepath) {
                    if let Ok(json) = serde_json::from_str::<Value>(&content) {
                        let path_str = format!("/mmr/v1/players/{}/competitiveupdates", puuid);
                        Mock::given(method("GET"))
                            .and(url_path(path_str))
                            .respond_with(ResponseTemplate::new(200).set_body_json(json))
                            .mount(&server)
                            .await;
                    }
                }
            } else if filename.starts_with("mmr_") && filename.ends_with(".json") {
                let puuid = filename.trim_start_matches("mmr_").trim_end_matches(".json");
                if let Ok(content) = std::fs::read_to_string(&filepath) {
                    if let Ok(json) = serde_json::from_str::<Value>(&content) {
                        let path_str = format!("/mmr/v1/players/{}", puuid);
                        Mock::given(method("GET"))
                            .and(url_path(path_str))
                            .respond_with(ResponseTemplate::new(200).set_body_json(json))
                            .mount(&server)
                            .await;
                    }
                }
            } else if filename.starts_with("match_") && filename.ends_with(".json") {
                let match_id = filename.trim_start_matches("match_").trim_end_matches(".json");
                if let Ok(content) = std::fs::read_to_string(&filepath) {
                    if let Ok(json) = serde_json::from_str::<Value>(&content) {
                        let path_str = format!("/match-details/v1/matches/{}", match_id);
                        Mock::given(method("GET"))
                            .and(url_path(path_str))
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

static TEST_MUTEX: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn run_scenario_test(scenario_name: &str) {
    let _guard = TEST_MUTEX.lock().await;
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let scenario_dir = manifest_dir.join("fixtures").join("redacted").join(scenario_name);
    let expected_file = manifest_dir.join("fixtures").join("expected").join(format!("{}.json", scenario_name));

    assert!(scenario_dir.exists(), "Scenario dir does not exist: {:?}", scenario_dir);
    assert!(expected_file.exists(), "Expected file does not exist: {:?}", expected_file);

    // Setup temp LOCALAPPDATA directory
    let temp_dir = std::env::temp_dir().join(format!("valight-parity-{}", scenario_name));
    let riot_dir = temp_dir.join("Riot Games").join("Riot Client").join("Config");
    let val_dir = temp_dir.join("VALORANT").join("Saved").join("Logs");
    std::fs::create_dir_all(&riot_dir).unwrap();
    std::fs::create_dir_all(&val_dir).unwrap();

    let lockfile_src = scenario_dir.join("lockfile.txt");
    if lockfile_src.exists() {
        std::fs::copy(&lockfile_src, riot_dir.join("lockfile")).unwrap();
    }
    let log_src = scenario_dir.join("shootergame_log.txt");
    if log_src.exists() {
        std::fs::copy(&log_src, val_dir.join("ShooterGame.log")).unwrap();
    }

    std::env::set_var("VALIGHT_LOCALAPPDATA", &temp_dir);

    // Start wiremock
    let server = setup_wiremock(&scenario_dir).await;

    let endpoints = RiotEndpoints {
        local_base: server.uri(),
        pd: server.uri(),
        glz: server.uri(),
    };

    let local_client = valorant_tracker::riot::client::build_local_client();
    let remote_client = valorant_tracker::riot::client::build_remote_client();
    let state = AppState::new_with_override(local_client, remote_client, Some(endpoints));

    let actual_response = get_match(true, &state).await.expect("get_match failed");
    let actual_json = serde_json::to_value(&actual_response).unwrap();

    let expected_content = std::fs::read_to_string(&expected_file).unwrap();
    let expected_json: Value = serde_json::from_str(&expected_content).unwrap();

    // Clean up temp dir
    let _ = std::fs::remove_dir_all(&temp_dir);

    assert_json_match(&actual_json, &expected_json, "$");
}

#[tokio::test]
async fn test_parity_menus() {
    run_scenario_test("menus").await;
}

#[tokio::test]
async fn test_parity_pregame_comp() {
    run_scenario_test("pregame_comp").await;
}
