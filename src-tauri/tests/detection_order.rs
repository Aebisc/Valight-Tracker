use valorant_tracker::riot::client::{build_local_client, build_remote_client};
use valorant_tracker::riot::config::{ApiConfig, RiotEndpoints};
use valorant_tracker::riot::lockfile::Lockfile;
use valorant_tracker::service::match_service::{detect_state, DetectedState};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn mock_lockfile() -> Lockfile {
    Lockfile {
        port: 12345,
        password: "secret".into(),
        basic_auth: "Basic cmlvdDpzZWNyZXQ=".into(),
    }
}

fn mock_api_config(server_url: &str) -> ApiConfig {
    let endpoints = RiotEndpoints {
        local_base: server_url.into(),
        pd: server_url.into(),
        glz: server_url.into(),
    };
    ApiConfig {
        endpoints,
        pd_url: server_url.into(),
        glz_url: server_url.into(),
        region: "na".into(),
        shard: "na".into(),
        puuid: "test-puuid-123".into(),
        headers: reqwest::header::HeaderMap::new(),
        version: "release-10.0".into(),
        expires_at: u64::MAX,
    }
}

#[tokio::test]
async fn test_detect_state_ingame_steady_state_single_request() {
    let server = MockServer::start().await;
    let local = build_local_client();
    let remote = build_remote_client();
    let lockfile = mock_lockfile();
    let mut cfg = mock_api_config(&server.uri());

    // Expect exactly 1 core-game request
    Mock::given(method("GET"))
        .and(path("/core-game/v1/players/test-puuid-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "MatchID": "match-core-111"
        })))
        .expect(1)
        .mount(&server)
        .await;

    // Expect exactly 0 pregame requests
    Mock::given(method("GET"))
        .and(path("/pregame/v1/players/test-puuid-123"))
        .respond_with(ResponseTemplate::new(404))
        .expect(0)
        .mount(&server)
        .await;

    let endpoints = RiotEndpoints {
        local_base: server.uri(),
        pd: server.uri(),
        glz: server.uri(),
    };

    let result = detect_state(
        &remote,
        &local,
        &lockfile,
        Some(endpoints),
        &mut cfg,
        Some("INGAME"),
    )
    .await;
    assert_eq!(
        result.unwrap(),
        DetectedState::Ingame("match-core-111".into())
    );
}

#[tokio::test]
async fn test_detect_state_pregame_steady_state_single_request() {
    let server = MockServer::start().await;
    let local = build_local_client();
    let remote = build_remote_client();
    let lockfile = mock_lockfile();
    let mut cfg = mock_api_config(&server.uri());

    // Expect exactly 1 pregame request
    Mock::given(method("GET"))
        .and(path("/pregame/v1/players/test-puuid-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "MatchID": "match-pre-222"
        })))
        .expect(1)
        .mount(&server)
        .await;

    // Expect exactly 0 core-game requests
    Mock::given(method("GET"))
        .and(path("/core-game/v1/players/test-puuid-123"))
        .respond_with(ResponseTemplate::new(404))
        .expect(0)
        .mount(&server)
        .await;

    let endpoints = RiotEndpoints {
        local_base: server.uri(),
        pd: server.uri(),
        glz: server.uri(),
    };

    let result = detect_state(
        &remote,
        &local,
        &lockfile,
        Some(endpoints),
        &mut cfg,
        Some("PREGAME"),
    )
    .await;
    assert_eq!(
        result.unwrap(),
        DetectedState::Pregame("match-pre-222".into())
    );
}

#[tokio::test]
async fn test_detect_state_ingame_to_pregame_transition() {
    let server = MockServer::start().await;
    let local = build_local_client();
    let remote = build_remote_client();
    let lockfile = mock_lockfile();
    let mut cfg = mock_api_config(&server.uri());

    // Core-game 404
    Mock::given(method("GET"))
        .and(path("/core-game/v1/players/test-puuid-123"))
        .respond_with(ResponseTemplate::new(404))
        .expect(1)
        .mount(&server)
        .await;

    // Pregame 200
    Mock::given(method("GET"))
        .and(path("/pregame/v1/players/test-puuid-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "MatchID": "match-pre-333"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let endpoints = RiotEndpoints {
        local_base: server.uri(),
        pd: server.uri(),
        glz: server.uri(),
    };

    let result = detect_state(
        &remote,
        &local,
        &lockfile,
        Some(endpoints),
        &mut cfg,
        Some("INGAME"),
    )
    .await;
    assert_eq!(
        result.unwrap(),
        DetectedState::Pregame("match-pre-333".into())
    );
}

#[tokio::test]
async fn test_detect_state_pregame_to_ingame_transition() {
    let server = MockServer::start().await;
    let local = build_local_client();
    let remote = build_remote_client();
    let lockfile = mock_lockfile();
    let mut cfg = mock_api_config(&server.uri());

    // Pregame 404
    Mock::given(method("GET"))
        .and(path("/pregame/v1/players/test-puuid-123"))
        .respond_with(ResponseTemplate::new(404))
        .expect(1)
        .mount(&server)
        .await;

    // Core-game 200
    Mock::given(method("GET"))
        .and(path("/core-game/v1/players/test-puuid-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "MatchID": "match-core-444"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let endpoints = RiotEndpoints {
        local_base: server.uri(),
        pd: server.uri(),
        glz: server.uri(),
    };

    let result = detect_state(
        &remote,
        &local,
        &lockfile,
        Some(endpoints),
        &mut cfg,
        Some("PREGAME"),
    )
    .await;
    assert_eq!(
        result.unwrap(),
        DetectedState::Ingame("match-core-444".into())
    );
}

#[tokio::test]
async fn test_detect_state_menus_both_queried() {
    let server = MockServer::start().await;
    let local = build_local_client();
    let remote = build_remote_client();
    let lockfile = mock_lockfile();
    let mut cfg = mock_api_config(&server.uri());

    Mock::given(method("GET"))
        .and(path("/core-game/v1/players/test-puuid-123"))
        .respond_with(ResponseTemplate::new(404))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/pregame/v1/players/test-puuid-123"))
        .respond_with(ResponseTemplate::new(404))
        .expect(1)
        .mount(&server)
        .await;

    let endpoints = RiotEndpoints {
        local_base: server.uri(),
        pd: server.uri(),
        glz: server.uri(),
    };

    let result = detect_state(
        &remote,
        &local,
        &lockfile,
        Some(endpoints),
        &mut cfg,
        Some("MENUS"),
    )
    .await;
    assert_eq!(result.unwrap(), DetectedState::Menus);
}

#[tokio::test]
async fn test_detect_state_error_never_becomes_menus() {
    let server = MockServer::start().await;
    let local = build_local_client();
    let remote = build_remote_client();
    let lockfile = mock_lockfile();
    let mut cfg = mock_api_config(&server.uri());

    Mock::given(method("GET"))
        .and(path("/core-game/v1/players/test-puuid-123"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/pregame/v1/players/test-puuid-123"))
        .respond_with(
            ResponseTemplate::new(429)
                .set_body_json(serde_json::json!({"message": "rate limited"})),
        )
        .mount(&server)
        .await;

    let endpoints = RiotEndpoints {
        local_base: server.uri(),
        pd: server.uri(),
        glz: server.uri(),
    };

    let result = detect_state(&remote, &local, &lockfile, Some(endpoints), &mut cfg, None).await;
    assert!(
        result.is_err(),
        "Should return error on 429 rather than falling back to Menus"
    );
}

#[tokio::test]
async fn test_detect_state_unauthorized_refreshes_token_and_succeeds() {
    let server = MockServer::start().await;
    let local = build_local_client();
    let remote = build_remote_client();
    let lockfile = mock_lockfile();
    let mut cfg = mock_api_config(&server.uri());

    // Local entitlements refresh mock
    Mock::given(method("GET"))
        .and(path("/entitlements/v1/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "accessToken": "new-token",
            "token": "new-entitlement",
            "subject": "test-puuid-123"
        })))
        .expect(1)
        .mount(&server)
        .await;

    // First request -> 401, second request -> 200
    Mock::given(method("GET"))
        .and(path("/core-game/v1/players/test-puuid-123"))
        .respond_with(ResponseTemplate::new(401))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/core-game/v1/players/test-puuid-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "MatchID": "match-core-after-refresh"
        })))
        .mount(&server)
        .await;

    let endpoints = RiotEndpoints {
        local_base: server.uri(),
        pd: server.uri(),
        glz: server.uri(),
    };

    let result = detect_state(
        &remote,
        &local,
        &lockfile,
        Some(endpoints),
        &mut cfg,
        Some("INGAME"),
    )
    .await;
    assert_eq!(
        result.unwrap(),
        DetectedState::Ingame("match-core-after-refresh".into())
    );
}
