/**
 * capture-fixtures.ts
 *
 * Run this script while Valorant is in a specific game state to capture
 * a complete set of raw Riot API responses for use as test fixtures.
 *
 * Usage:
 *   npx tsx scripts/capture-fixtures.ts --scenario pregame_comp
 *
 * The script will create files under src-tauri/fixtures/raw/{scenario}/
 *
 * IMPORTANT: raw/ contains live tokens. Never commit raw/ to git.
 * After capturing, run redact-fixtures.ts to produce the safe redacted/ copies.
 */

import { readFile, mkdir, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import path from "node:path";

const SCENARIO = process.argv.find((a) => a.startsWith("--scenario="))?.split("=")[1]
  ?? process.argv[process.argv.indexOf("--scenario") + 1];

if (!SCENARIO) {
  console.error("Usage: npx tsx scripts/capture-fixtures.ts --scenario <name>");
  process.exit(1);
}

const OUT_DIR = path.join(process.cwd(), "src-tauri", "fixtures", "raw", SCENARIO);

async function save(name: string, data: unknown) {
  const file = path.join(OUT_DIR, name);
  await writeFile(file, JSON.stringify(data, null, 2), "utf-8");
  console.log(`  ✓  ${name}`);
}

async function localGet(port: string, password: string, endpoint: string) {
  const auth = Buffer.from(`riot:${password}`).toString("base64");
  const origTls = process.env.NODE_TLS_REJECT_UNAUTHORIZED;
  process.env.NODE_TLS_REJECT_UNAUTHORIZED = "0";
  try {
    const res = await fetch(`https://127.0.0.1:${port}${endpoint}`, {
      headers: { Authorization: `Basic ${auth}` },
    });
    return res.ok ? res.json() : null;
  } finally {
    if (origTls === undefined) delete process.env.NODE_TLS_REJECT_UNAUTHORIZED;
    else process.env.NODE_TLS_REJECT_UNAUTHORIZED = origTls;
  }
}

async function remoteGet(url: string, headers: Record<string, string>) {
  const res = await fetch(url, { headers });
  if (!res.ok) return null;
  const text = await res.text();
  try { return JSON.parse(text); } catch { return null; }
}

async function remotePut(url: string, headers: Record<string, string>, body: unknown) {
  const res = await fetch(url, {
    method: "PUT",
    headers: { ...headers, "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!res.ok) return null;
  try { return res.json(); } catch { return null; }
}

const REGION_CONFIG: Record<string, { pd: string; glz: string }> = {
  na: { pd: "https://pd.na.a.pvp.net", glz: "https://glz-na-1.na.a.pvp.net" },
  eu: { pd: "https://pd.eu.a.pvp.net", glz: "https://glz-eu-1.eu.a.pvp.net" },
  ap: { pd: "https://pd.ap.a.pvp.net", glz: "https://glz-ap-1.ap.a.pvp.net" },
  kr: { pd: "https://pd.kr.a.pvp.net", glz: "https://glz-kr-1.kr.a.pvp.net" },
  br: { pd: "https://pd.br.a.pvp.net", glz: "https://glz-br-1.br.a.pvp.net" },
  latam: { pd: "https://pd.latam.a.pvp.net", glz: "https://glz-latam-1.latam.a.pvp.net" },
};

async function main() {
  await mkdir(OUT_DIR, { recursive: true });
  console.log(`\nCapturing scenario: ${SCENARIO}`);
  console.log(`Output dir: ${OUT_DIR}\n`);

  // 1. Lockfile
  const lockfilePath = `${process.env.LOCALAPPDATA}/Riot Games/Riot Client/Config/lockfile`;
  const lockfileContent = await readFile(lockfilePath, "utf-8").catch(() => null);
  if (!lockfileContent) {
    console.log("  ! Lockfile not found — saving offline sentinel");
    await save("lockfile.txt", { _missing: true });
    return;
  }
  await writeFile(path.join(OUT_DIR, "lockfile.txt"), lockfileContent, "utf-8");
  console.log("  ✓  lockfile.txt");

  const [, , port, password] = lockfileContent.split(":");

  // 2. Entitlements
  const entitlements = await localGet(port, password, "/entitlements/v1/token");
  await save("entitlements.json", entitlements);

  const puuid: string = entitlements?.subject ?? "";
  const accessToken: string = entitlements?.accessToken ?? "";
  const entitlementsToken: string = entitlements?.token ?? "";

  if (!puuid || !accessToken) {
    console.error("  ! Could not get entitlements. Is Valorant running?");
    process.exit(1);
  }

  // 3. ShooterGame.log (first 64 KB)
  const logPath = `${process.env.LOCALAPPDATA}/VALORANT/Saved/Logs/ShooterGame.log`;
  const { open } = await import("node:fs/promises");
  let logSample = "";
  try {
    const f = await open(logPath, "r");
    const buf = Buffer.alloc(64 * 1024);
    const { bytesRead } = await f.read(buf, 0, buf.length, 0);
    await f.close();
    logSample = buf.toString("utf-8", 0, bytesRead);
  } catch { /* log may not exist */ }
  await writeFile(path.join(OUT_DIR, "shootergame_log.txt"), logSample, "utf-8");
  console.log("  ✓  shootergame_log.txt");

  // Parse region/version from log
  const region = logSample.match(/https:\/\/glz-(.+?)-\d+\.\1\.a\.pvp\.net/)?.[1] ?? "eu";
  const version = logSample.match(/release-(\d+\.\d+-shipping-\d+-\d+)/)?.[1] ?? "unknown";
  const rc = REGION_CONFIG[region] ?? REGION_CONFIG.na;

  const headers: Record<string, string> = {
    Authorization: `Bearer ${accessToken}`,
    "X-Riot-Entitlements-JWT": entitlementsToken,
    "X-Riot-ClientVersion": `release-${version}`,
    "X-Riot-ClientPlatform": Buffer.from(JSON.stringify({
      platformType: "PC", platformOS: "Windows",
      platformOSVersion: "10.0.19042.1.256.64bit", platformChipset: "Unknown",
    })).toString("base64"),
  };

  // 4. Presences
  const presences = await localGet(port, password, "/chat/v4/presences");
  await save("presences.json", presences);

  // 5. Core-game player check
  const coreGamePlayer = await remoteGet(`${rc.glz}/core-game/v1/players/${puuid}`, headers);
  await save("coregame_player.json", coreGamePlayer);
  const coreGameMatchId: string | null = coreGamePlayer?.MatchID ?? null;

  // 6. Pregame player check (only if not in core-game)
  const preGamePlayer = coreGameMatchId ? null : await remoteGet(`${rc.glz}/pregame/v1/players/${puuid}`, headers);
  await save("pregame_player.json", preGamePlayer);
  const preGameMatchId: string | null = preGamePlayer?.MatchID ?? null;

  // 7. Match data
  let players: Array<{ puuid: string }> = [];
  if (coreGameMatchId) {
    const match = await remoteGet(`${rc.glz}/core-game/v1/matches/${coreGameMatchId}`, headers);
    await save("coregame_match.json", match);
    await save("pregame_match.json", null);
    players = (match?.Players ?? []).map((p: any) => ({ puuid: p?.Subject ?? "" })).filter((p: any) => p.puuid);
  } else if (preGameMatchId) {
    const match = await remoteGet(`${rc.glz}/pregame/v1/matches/${preGameMatchId}`, headers);
    await save("pregame_match.json", match);
    await save("coregame_match.json", null);
    const ally = (match?.AllyTeam?.Players ?? []).map((p: any) => ({ puuid: p?.Subject ?? "" }));
    const enemy = (match?.EnemyTeam?.Players ?? []).map((p: any) => ({ puuid: p?.Subject ?? "" }));
    players = [...ally, ...enemy].filter((p) => p.puuid);
  } else {
    await save("pregame_match.json", null);
    await save("coregame_match.json", null);
    console.log("  → MENUS state — no match data");
  }

  if (players.length === 0) {
    console.log("\nDone (MENUS/OFFLINE — no players to fetch).");
    return;
  }

  // 8. Names
  const puuids = players.map((p) => p.puuid);
  const names = await remotePut(`${rc.pd}/name-service/v2/players`, headers, puuids);
  await save("names.json", names);

  // 9. MMR + competitive updates per player
  const recentMatchIds = new Set<string>();
  for (const { puuid: playerPuuid } of players) {
    const mmr = await remoteGet(`${rc.pd}/mmr/v1/players/${playerPuuid}`, headers);
    await save(`mmr_${playerPuuid}.json`, mmr);

    const comp = await remoteGet(
      `${rc.pd}/mmr/v1/players/${playerPuuid}/competitiveupdates?startIndex=0&endIndex=20&queue=competitive`,
      headers
    );
    await save(`comp_${playerPuuid}.json`, comp);

    // Collect match IDs for detail fetches
    const matches: any[] = Array.isArray(comp?.Matches) ? comp.Matches : [];
    for (const m of matches.slice(0, 20)) {
      if (m?.MatchID) recentMatchIds.add(m.MatchID);
    }
  }

  // 10. Match details
  let detailCount = 0;
  for (const matchId of recentMatchIds) {
    const detail = await remoteGet(`${rc.pd}/match-details/v1/matches/${matchId}`, headers);
    if (detail) {
      await save(`match_${matchId}.json`, detail);
      detailCount++;
    }
  }
  console.log(`  → Fetched ${detailCount} match details`);

  console.log(`\n✅ Capture complete for scenario: ${SCENARIO}`);
  console.log("   Next: run  npx tsx scripts/redact-fixtures.ts --scenario", SCENARIO);
  console.log("   IMPORTANT: Never commit src-tauri/fixtures/raw/ to git!");
}

main().catch((err) => {
  console.error("Capture failed:", err);
  process.exit(1);
});
