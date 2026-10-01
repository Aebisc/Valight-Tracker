# Test Fixtures

This directory contains fixtures used as the oracle for the Node→Rust backend migration.

## Structure

```
fixtures/
  raw/          Raw Riot API responses, as received (with live tokens — DO NOT COMMIT)
  redacted/     Same responses with PUUIDs, names, tokens replaced with fake values
  expected/     Expected ApiResponse JSON produced by running the Node route over redacted/
  README.md     This file
```

## Scenarios to capture

| Filename prefix     | Description                                      |
|---------------------|--------------------------------------------------|
| `offline`           | lockfile absent / Valorant not running           |
| `menus`             | Lockfile present, no pregame or coregame         |
| `pregame_comp`      | Pregame — competitive queue                      |
| `pregame_unrated`   | Pregame — unrated queue                          |
| `pregame_custom`    | Pregame — custom game (empty queueId)            |
| `pregame_spikerush` | Pregame — spike rush                             |
| `ingame_comp`       | Core-game — competitive                          |
| `ingame_dm`         | Core-game — deathmatch (no rank data expected)   |

## Files per scenario

Each scenario has a set of raw Riot responses:

- `{scenario}.lockfile.txt`          — raw lockfile content (redacted password)
- `{scenario}.entitlements.json`     — `/entitlements/v1/token` response
- `{scenario}.pregame_player.json`   — `/pregame/v1/players/{puuid}` (null if INGAME/MENUS)
- `{scenario}.pregame_match.json`    — `/pregame/v1/matches/{id}` (null if INGAME/MENUS)
- `{scenario}.coregame_player.json`  — `/core-game/v1/players/{puuid}` (null if PREGAME/MENUS)
- `{scenario}.coregame_match.json`   — `/core-game/v1/matches/{id}` (null if PREGAME/MENUS)
- `{scenario}.names.json`            — `/name-service/v2/players` PUT response
- `{scenario}.mmr_{puuid}.json`      — `/mmr/v1/players/{puuid}` (one per player)
- `{scenario}.comp_{puuid}.json`     — `/mmr/v1/players/{puuid}/competitiveupdates`
- `{scenario}.match_{matchId}.json`  — `/match-details/v1/matches/{id}` (one per recent match)
- `{scenario}.presences.json`        — `/chat/v4/presences` response
- `{scenario}.shootergame_log.txt`   — First 64KB of ShooterGame.log
- `{scenario}.expected_response.json`— Expected ApiResponse (oracle output)

## How to capture

Run `scripts/capture-fixtures.ts` with the game in each state.
The script dumps all Riot responses into `raw/`, then you run `scripts/redact-fixtures.ts`
to scrub PUUIDs and produce the `redacted/` copies.

## Redaction rules

| Field              | Replacement                              |
|--------------------|------------------------------------------|
| PUUIDs             | `aaaaaaaa-{seq:04}-0000-0000-000000000000` |
| GameName / TagLine | `Player{seq}` / `TAG{seq}`               |
| accessToken        | `fake-access-token`                      |
| entitlementsToken  | `fake-entitlements-token`                |
| lockfile password  | `fakepassword`                           |

All structural data (team IDs, agent IDs, map IDs, match IDs, round data, tier numbers) is preserved — the Rust tests need real shapes.
