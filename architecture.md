# VaLight Tracker Architecture

VaLight Tracker is an ultra-lightweight, native desktop application for tracking Valorant live match telemetry, player statistics, rank progression, and party groupings in real-time.

It is built as a hybrid desktop app using **Tauri v2**, pairing a native **Rust backend** with a **Next.js 15 (React 19)** frontend running inside the system's native **WebView2** runtime.

---

## 1. High-Level Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          DESKTOP HOST PROCESS                               │
├─────────────────────────────────────────────────────────────────────────────┤
│  Frontend (WebView2 Chromium Shell)                                         │
│  - Next.js 15 App Router (Static HTML/CSS/JS export in `out/`)              │
│  - React 19 UI (MatchHeader, PlayerTable, StatusBar, UpdaterCard)           │
│  - State Machine & Polling Hook (`useMatchPolling`)                         │
│  - Styling: Tailwind CSS v4 + Custom Theme CSS Variables                    │
│                                      │                                      │
│                                      │ Tauri IPC Bridge (`invoke`)          │
│                                      ▼                                      │
│  Backend (Native Rust Core Engine)                                          │
│  - Commands Interface (`commands::get_match`)                               │
│  - State Engine & Orchestrator (`service::match_service`)                   │
│  - Riot Client Local Interface (`lockfile.rs`, `config.rs`)                 │
│  - Network Clients (`riot::client`, `riot::endpoints`)                      │
│  - Two-Tier Cache: In-Memory LRU + Persistent Disk Storage (`disk_cache.rs`)│
│  - Analytics Engine (`player.rs`, `stats.rs`, `party.rs`, `side.rs`)        │
└──────────────────────────────────────┬──────────────────────────────────────┘
                                       │
                      HTTP / HTTPS     │
            ┌──────────────────────────┴──────────────────────────┐
            ▼                                                     ▼
┌───────────────────────────────┐             ┌───────────────────────────────┐
│ Local Riot Client (127.0.0.1) │             │ Riot Cloud Edge API           │
│ - Port & Basic Auth via       │             │ - GLZ: Pregame & Coregame     │
│   Riot Client Lockfile        │             │ - PD: MMR, Comp History,      │
│ - Entitlements & Presences    │             │   Match Details, Names        │
└───────────────────────────────┘             └───────────────────────────────┘
```

---

## 2. Process & Technology Stack

| Layer | Technology | Purpose |
|---|---|---|
| **Desktop Shell** | [Tauri v2](https://v2.tauri.app/) (`tauri-apps/api`, `cli`) | Native window management, IPC bridge, updater, system trays |
| **Backend Runtime** | Native Rust (Edition 2021) | Zero-overhead concurrency, secure HTTP client, file system caching |
| **Async Runtime** | [Tokio v1](https://tokio.rs/) + [Futures v0.3](https://docs.rs/futures) | Multi-threaded async network I/O and task scheduling |
| **HTTP Client** | [Reqwest v0.12](https://docs.rs/reqwest) with `rustls-tls` | High-performance HTTP client without OpenSSL dependency |
| **Frontend Framework** | [Next.js 15](https://nextjs.org/) + [React 19](https://react.dev/) | Declarative UI, component composition, reactive client hooks |
| **Bundler & Compiler** | Turbopack (`next build` with `output: "export"`) | Zero-runtime static compilation directly into pure HTML/CSS/JS |
| **Styling** | Tailwind CSS v4 + CSS Variables | Glassmorphic, low-overhead responsive styling |

---

## 3. Backend Architecture (`src-tauri/src`)

The Rust backend is structured into clear modular boundaries:

```
src-tauri/src/
├── main.rs                  # Application entry point, crash hook, window init
├── lib.rs                   # Library crate entry point (exports modules)
├── commands.rs              # Tauri IPC command definitions
├── state.rs                 # AppState, client handles, cache mutexes
├── model.rs                 # Domain models, serializable JSON API envelopes
├── riot/                    # Riot Games API communication layer
│   ├── lockfile.rs          # Riot Client lockfile detection & basic auth parser
│   ├── config.rs            # ShooterGame.log parser & Entitlements resolver
│   ├── client.rs            # Low-level reqwest client, retry loops & rate limits
│   └── endpoints.rs         # Remote Riot endpoint wrappers (GLZ, PD)
└── service/                 # Domain business logic & telemetry orchestration
    ├── match_service.rs     # Primary match orchestrator & state machine
    ├── disk_cache.rs        # Persistent atomic file cache for match details
    ├── player.rs            # Rank, peak rank, current season resolution
    ├── stats.rs             # Combat performance metrics (KD, ACS, ADR, HS%)
    ├── party.rs             # Party detection, presence matching & match history
    └── side.rs              # Attacking / Defending starting side resolver
```

### 3.1 Startup & Lifecycle (`main.rs`, `state.rs`)
1. **Crash Telemetry:** A custom `std::panic::set_hook` captures unhandled panics and logs them to `%LOCALAPPDATA%/VaLight-Tracker/crash.log`.
2. **Dual Client Model:**
   - `local_client`: Configured with `danger_accept_invalid_certs(true)` to connect securely over HTTPS to the Riot Client's self-signed local loopback address (`127.0.0.1:{port}`).
   - `remote_client`: Configured with strict standard TLS verification to communicate with Riot’s official `glz` and `pd` cloud servers.
3. **State Management (`AppState`):**
   - Held in Tauri's managed memory state.
   - `match_cache`: Mutex-guarded cache of the current live match state.
   - `detail_cache`: In-memory `LruCache<String, Arc<Value>>` of up to 500 recently inspected match records.
   - `cache_dir`: Path to persistent disk storage (`%LOCALAPPDATA%/VaLight-Tracker/match_cache`).

### 3.2 Authentication & Credential Discovery (`riot/lockfile.rs`, `riot/config.rs`)
To communicate with Riot services without requiring user credentials:
1. **Lockfile Parsing:** Reads `%LOCALAPPDATA%/Riot Games/Riot Client/Config/lockfile`. Format: `name:pid:port:password:protocol`. Converts `riot:{password}` to Basic Auth base64.
2. **Entitlements Extraction:** Hits `https://127.0.0.1:{port}/entitlements/v1/token` to extract:
   - `puuid`: Authenticated player unique ID.
   - `accessToken`: Bearer token for remote Riot API endpoints.
   - `token`: Entitlements JWT (`x-riot-entitlements-jwt`).
3. **Region & Version Discovery:** Asynchronously streams the first lines of `%LOCALAPPDATA%/VALORANT/Saved/Logs/ShooterGame.log` using buffered reads to detect:
   - Game Region / Shard (`na`, `eu`, `ap`, `kr`, `br`, `latam`).
   - Client version string (`x-riot-clientversion`).
4. **Token Expiry Caching:** Caches the `ApiConfig` in memory until 120 seconds before JWT token expiration.

### 3.3 State Detection & Orchestration (`service/match_service.rs`)
When `get_match(force)` is invoked:
1. **Single-Flight Lock & Generation Tracking (`AppState::build_lock`):**
   - Concurrent requests acquire an async build lock. If a build is already in progress, overlapping requests wait on the in-flight build and return its result, completely eliminating duplicate network and compute bursts.
2. **State Detection (`detect_state`):**
   - Utilizes previous state hints (`last_state`) to query with optimal priority:
     - Steady-state `INGAME`: Queries `/core-game/v1/players/{puuid}` first (1 GLZ request on hit).
     - Steady-state `PREGAME`: Queries `/pregame/v1/players/{puuid}` first (1 GLZ request on hit).
     - Menus or transitions: Queries both endpoints and resolves with priority `INGAME` > `PREGAME` > `MENUS`.
3. **Pregame Cache Hit (Agent Select Updates):**
   - While in `PREGAME`, polling fetches the lightweight pregame match payload to update newly locked or hovered agents (`CharacterID`), completely bypassing player stats, MMR, and match history re-fetches.
4. **Pregame → Ingame Transition Optimization:**
   - When transitioning from `PREGAME` to `INGAME` on the same map:
     - Teammate stats (Rank, Peak Rank, RR, KD, ACS, ADR, HS%, recent results) already computed during Agent Select are preserved.
     - Only the 5 newly revealed enemy players are queried from Riot's remote endpoints (`/mmr/`, `/competitiveupdates`, `/name-service/`).
     - Agent IDs and account levels are updated from coregame.
     - Match detail fetching and stats computation are skipped for teammates (0 match detail fetches for teammates).
     - Party detection (`party::assign_party_numbers`) re-evaluates all 10 players to assign per-team party numbers.
5. **Fast-Path Ingame Caching:**
   - If the player is already `INGAME` and the match ID matches the in-memory cache, returns cached match details instantly (0 remote queries).
6. **Concurrent Data Aggregation (New Players):**
   - Fires concurrent asynchronous streams using `tokio::join!`:
     - Batch name lookup (`/name-service/v2/players`) guarded against empty PUUID sets.
     - MMR (`/mmr/v1/players/{puuid}`) & Competitive updates (`/competitiveupdates`) with bounded concurrency.
     - Local Riot Client presences for live party detection (`/chat/v4/presences`).

### 3.4 Two-Tier Cache Architecture (`service/disk_cache.rs`)
To prevent hitting Riot PD rate limits (~20 req/s):
- **Tier 1 (RAM):** Memory LRU cache of `Arc<Value>` for zero-copy sharing across threads.
- **Tier 2 (Disk):** Persistent file-based cache at `%LOCALAPPDATA%/VaLight-Tracker/match_cache/{match_id}.json`.
  - Uses atomic file writes (unique nonce temp files renamed atomically).
  - Queries disk cache using 16 concurrent bounded async readers.
  - Automatically prunes oldest match files when directory count exceeds 2,000 matches (~50MB max).

---

## 4. Frontend Architecture (`app/`, `components/`, `hooks/`, `lib/`)

```
app/
├── layout.tsx               # Root layout, Google fonts, ToastProvider, UpdaterCard
├── page.tsx                 # Master view: Splash, Offline, Menus, Live Match views
├── globals.css              # Core tokens, dark/light themes, animations
components/
├── match-header.tsx         # Map, mode, live server, elapsed match timer
├── player-table.tsx         # Team tables, player cards, expandable stat drawers
├── status-bar.tsx           # App version, external project links
├── updater-card.tsx         # Auto-updater download & progress dialog
└── toast.tsx                # Context-driven toast alert system
hooks/
├── use-match-polling.ts     # Core state machine, adaptive polling, reconnect loop
└── use-settings.ts          # Dark/light theme toggle and persistence
lib/
├── backend.ts               # IPC abstraction layer (`invoke` with fallback)
├── types.ts                 # TypeScript interfaces mirroring Rust models
└── constants.ts             # Static lookup maps, rank color maps, asset URLs
```

### 4.1 State Machine & Adaptive Polling (`hooks/use-match-polling.ts`)
The UI drives data collection through an adaptive polling interval engine:
- **`MENUS` / `OFFLINE`:** Polls every **15 seconds** to conserve resources.
- **`PREGAME` (Agent Select):** Polls every **5 seconds** to capture agent picks and locks in real-time.
- **`INGAME` (Live Game):** Polls every **8 seconds** (serviced instantly via backend RAM cache).
- **Background Throttling:** Detects browser / window visibility via `document.visibilitychange`. Halts polling when minimized or alt-tabbed to 0% CPU consumption; immediately polls once when restored.
- **Exponential Backoff:** If the game client closes or encounters connection failures, enters a reconnect state and triggers warning toasts.
- **Refresh vs. Refetch UI Controls:**
  - **`refreshMatch()` ("Refresh" button):** Manual fast poll (`force = false`) — instantly re-checks game state and updates agent picks without clearing caches or re-querying player stats.
  - **`hardRefreshMatch()` ("Refetch" button):** Full manual rebuild (`force = true`) — invalidates match and config caches and forces a complete fresh query of all 10 players from the Riot API.
- **Request Guarding & Structural Sharing (`lib/share-structure.ts`):**
  - **Sequence Guard:** Ignores out-of-order or late-arriving responses when a newer manual or automated request finishes first.
  - **Structural Sharing:** Reuses existing player object and array references when backend responses contain identical data, eliminating spurious React re-renders.

### 4.2 Interactive Player Table (`components/player-table.tsx`)
- **Row Memoization:** `<MemoizedRow>` and `<MemoizedTeam>` components prevent re-rendering unaffected players when expanding rows or updating timers.
- **Smooth Layout Transitions:** Uses CSS Grid (`grid-template-rows: 0fr -> 1fr`) for zero-reflow accordion expansions.
- **Metrics Breakdown:**
  - Rank badge, current RR, Peak Rank across all episodes.
  - Calculated K/D, ACS, ADR, and Headshot percentage.
  - Shot distribution bar (Head / Body / Leg shots with proportional visual bars).
  - Last match performance pill.
  - Recent 5 matches W/L/D streak pills.
  - Multi-person party badge grouping with dynamic color palettes.

---

## 5. IPC Bridge Protocol (`src-tauri/src/commands.rs` ⟷ `lib/backend.ts`)

Communication between the Next.js frontend and the Rust engine is handled through Tauri's binary IPC bridge using serialized JSON payloads:

### Commands

| Command | Arguments | Return Type | Description |
|---|---|---|---|
| `get_match` | `force: bool` | `Result<ApiResponse, ApiError>` | Primary polling interface for game status, match telemetry, and player stats |

### Core Data Models (`lib/types.ts` & `model.rs`)

```typescript
export interface ApiResponse {
  gameState: "PREGAME" | "INGAME" | "MENUS" | "OFFLINE" | "ERROR";
  error?: string;
  match?: MatchInfo;
  players?: Player[];
  selfPuuid?: string;
}

export interface Player {
  puuid: string;
  name: string;
  tag: string;
  agentId: string;
  agentName: string;
  teamId: string;
  accountLevel: number;
  rank: number;
  rankName: string;
  peakRank: number;
  peakRankName: string;
  rr: number;
  headshotPercent: number;
  winrate: number;
  kd: number;
  acs: number;
  adr: number;
  recentResults: ("W" | "L" | "D")[];
  partyNumber?: number;
  partySize?: number;
}
```

---

## 6. Build, Packaging & Distribution Pipeline

The release pipeline produces a native Windows installer and self-updating binary without requiring a bundled Node.js runtime:

```
                      ┌────────────────────────────┐
                      │    npm run tauri build     │
                      └─────────────┬──────────────┘
                                    │
               ┌────────────────────┴────────────────────┐
               ▼                                         ▼
    [ Frontend Compilation ]                  [ Rust Native Compilation ]
    - `next build` (Next.js 15)               - `cargo build --release`
    - Generates static HTML/JS/CSS            - Target: `x86_64-pc-windows-msvc`
    - Output written to `out/`                - LTO enabled, symbols stripped
               │                                         │
               └────────────────────┬────────────────────┘
                                    │
                                    ▼
                      [ Tauri Asset Embedding ]
                      - Embeds `out/` into the Rust `.exe`
                      - Links Windows WebView2 bootstrapper
                                    │
                                    ▼
                      [ Installer Packaging ]
                      - NSIS Installer (`.exe`)
                      - Generates update signature & `latest.json`
```

### Build Output Locations
- **Raw Executable:** `src-tauri/target/release/valorant-tracker.exe`
- **NSIS Installer:** `src-tauri/target/release/bundle/nsis/VaLight Tracker_x.x.x_x64-setup.exe`
- **Updater Artifacts:** `latest.json` with Minisign cryptographic verification signatures.
