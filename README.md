# Valorant Tracker

Open source match tracker for Valorant. Pulls live match data from the local Riot Client API — everything runs on your machine, nothing is sent anywhere.

![Next.js](https://img.shields.io/badge/Next.js-16-black?logo=next.js)
![TypeScript](https://img.shields.io/badge/TypeScript-5-blue?logo=typescript)
![License](https://img.shields.io/badge/License-MIT-green)

## Features

- Live match data during Agent Select and In-Game
- All 10 players with rank, peak rank, K/D, headshot %, ACS, and more
- Party detection
- Copy match lobby to clipboard
- Dark/light theme toggle
- Auto-reconnects if Valorant restarts

## Requirements

- **Windows** — reads Riot Client files from AppData
- **Node.js 18+** — [nodejs.org](https://nodejs.org)
- **Valorant installed and running**

## Setup

```bash
git clone https://github.com/Aebisc/Light-Valorant-Tracker.git
cd valorant-tracker
npm install
npm run dev
```

Open [localhost:3000](http://localhost:3000) and launch Valorant. The tracker will detect it automatically.

## How it works

1. Reads the Riot Client lockfile (`%LOCALAPPDATA%/Riot Games/Riot Client/Config/lockfile`) for local auth
2. Fetches an access token from the Entitlements API on `127.0.0.1`
3. Determines your region from the Valorant game log
4. Pulls match data, MMR, and party info from Riot's API endpoints

Polls at adaptive intervals — faster during Agent Select and In-Game, slower in menus.

## License

MIT — see [LICENSE](LICENSE)
