/**
 * generate-expected.ts
 *
 * Runs the current Node.js route logic against the redacted fixtures to produce
 * the oracle ApiResponse JSON files stored in src-tauri/fixtures/expected/.
 *
 * This is the ground truth that the Rust port must match exactly.
 *
 * Usage:
 *   npx tsx scripts/generate-expected.ts --scenario pregame_comp
 *   npx tsx scripts/generate-expected.ts --all
 *
 * The output file is: src-tauri/fixtures/expected/{scenario}.json
 */

import { readdir, readFile, writeFile, mkdir } from "node:fs/promises";
import path from "node:path";

// ─── Inline the route logic (no Next.js, no HTTP) ─────────────────────────────
// This is a minimal runner that feeds fixture data into the same functions
// used in app/api/match/route.ts, bypassing all I/O.

import { RANK_MAP, AGENT_MAP, MAP_MAP, GAMEMODE_MAP, DEATHMATCH_MODES, resolveStartingSide } from "../lib/constants";
import type { ValorantPlayer, MatchInfo, TeamSide } from "../lib/types";

const RECENT_GAMES_COUNT = 20;

function getPeakRank(
  seasonalInfo: Record<string, any>,
  recentMatches: any[] = [],
  latestComp?: any
): number {
  let peak = 0;
  for (const seasonId in seasonalInfo) {
    const season = seasonalInfo[seasonId];
    if (!season) continue;
    const compTier = season.CompetitiveTier;
    if (typeof compTier === "number" && compTier > peak && compTier <= 27) peak = compTier;
    const actRank = season.Rank;
    if (typeof actRank === "number" && actRank > peak && actRank <= 27) peak = actRank;
    if (season.WinsByTier && typeof season.WinsByTier === "object") {
      for (const [tierStr, winCount] of Object.entries(season.WinsByTier)) {
        const tier = Number(tierStr);
        if (Number.isInteger(tier) && tier > peak && tier <= 27 && Number(winCount) > 0) peak = tier;
      }
    }
  }
  if (latestComp) {
    const after = latestComp.TierAfterUpdate;
    const before = latestComp.TierBeforeUpdate;
    if (typeof after === "number" && after > peak && after <= 27) peak = after;
    if (typeof before === "number" && before > peak && before <= 27) peak = before;
  }
  for (const m of recentMatches) {
    const after = m?.TierAfterUpdate;
    const before = m?.TierBeforeUpdate;
    if (typeof after === "number" && after > peak && after <= 27) peak = after;
    if (typeof before === "number" && before > peak && before <= 27) peak = before;
  }
  return peak;
}

function getCurrentSeasonId(seasonalInfo: Record<string, any>, matchSeasonId: string): string | null {
  if (matchSeasonId && seasonalInfo[matchSeasonId]) return matchSeasonId;
  let best: string | null = null;
  for (const seasonId in seasonalInfo) {
    const games = seasonalInfo[seasonId]?.NumberOfGames ?? 0;
    if (games > 0 && (!best || seasonId > best)) best = seasonId;
  }
  return best;
}

function extractPlayerStats(matchDetail: any, puuid: string) {
  const playerStats = matchDetail?.players?.find((p: any) => p.subject === puuid || p.Subject === puuid);
  const stats = playerStats?.stats;
  const kills = stats?.kills ?? 0;
  const deaths = stats?.deaths ?? 0;
  const assists = stats?.assists ?? 0;
  const kd = deaths > 0 ? Math.round((kills / deaths) * 100) / 100 : kills;
  const rounds = matchDetail?.roundResults ?? [];
  let headshots = 0, bodyshots = 0, legshots = 0, totalDamage = 0, roundsParticipated = 0;
  for (const round of rounds) {
    const playerStats2 = (round?.playerStats ?? []).find((ps: any) => ps.subject === puuid || ps.Subject === puuid);
    if (!playerStats2) continue;
    roundsParticipated++;
    for (const dmg of playerStats2.damage ?? []) {
      headshots += dmg.headshots ?? 0;
      bodyshots += dmg.bodyshots ?? 0;
      legshots += dmg.legshots ?? 0;
      totalDamage += dmg.damage ?? 0;
    }
  }
  const totalShots = headshots + bodyshots + legshots;
  const headshotPercent = totalShots > 0 ? Math.round((headshots / totalShots) * 1000) / 10 : 0;
  const totalRounds = matchDetail?.roundResults?.length ?? 1;
  const acs = totalRounds > 0 ? Math.round((stats?.score ?? 0) / totalRounds) : 0;
  const adr = roundsParticipated > 0 ? Math.round((totalDamage / roundsParticipated) * 10) / 10 : 0;
  let winrate = 0;
  let won = false;
  const teamId = playerStats?.teamId ?? playerStats?.TeamID;
  if (teamId) {
    const team = matchDetail?.teams?.find((t: any) => t.teamId === teamId);
    const wins = team?.roundsWon ?? 0;
    const total = team?.roundsPlayed ?? 0;
    winrate = total > 0 ? Math.round((wins / total) * 1000) / 10 : 0;
    if (typeof team?.won === "boolean") {
      won = team.won;
    } else {
      const opponent = matchDetail?.teams?.find((t: any) => t.teamId !== teamId);
      won = wins > (opponent?.roundsWon ?? 0);
    }
  }
  return { kills, deaths, assists, kd, headshots, bodyshots, legshots, headshotPercent, winrate, acs, adr, won };
}

function getMatchResult(matchDetail: any, puuid: string): "W" | "L" | "D" {
  const playerStats = matchDetail?.players?.find((p: any) => p.subject === puuid || p.Subject === puuid);
  const teamId = playerStats?.teamId ?? playerStats?.TeamID;
  if (!teamId) return "L";
  const team = matchDetail?.teams?.find((t: any) => t.teamId === teamId);
  const opponent = matchDetail?.teams?.find((t: any) => t.teamId !== teamId);
  if (typeof team?.won === "boolean" && typeof opponent?.won === "boolean") {
    if (team.won && !opponent.won) return "W";
    if (!team.won && opponent.won) return "L";
    return "D";
  }
  const myRounds = team?.roundsWon ?? 0;
  const oppRounds = opponent?.roundsWon ?? 0;
  if (myRounds > oppRounds) return "W";
  if (myRounds < oppRounds) return "L";
  return "D";
}

function aggregatePlayerStats(matchDetails: any[], puuid: string) {
  let sumKills = 0, sumDeaths = 0, sumAssists = 0;
  let sumHeadshots = 0, sumBodyshots = 0, sumLegshots = 0;
  let sumAcs = 0, sumAdr = 0, wins = 0, gamesCounted = 0;
  for (const matchDetail of matchDetails) {
    if (!matchDetail) continue;
    const stats = extractPlayerStats(matchDetail, puuid);
    sumKills += stats.kills; sumDeaths += stats.deaths; sumAssists += stats.assists;
    sumHeadshots += stats.headshots; sumBodyshots += stats.bodyshots; sumLegshots += stats.legshots;
    sumAcs += stats.acs; sumAdr += stats.adr;
    if (stats.won) wins++;
    gamesCounted++;
  }
  if (gamesCounted === 0) return { kills: 0, deaths: 0, assists: 0, kd: 0, headshots: 0, bodyshots: 0, legshots: 0, headshotPercent: 0, winrate: 0, acs: 0, adr: 0, recentGamesCount: 0 };
  const kd = sumDeaths > 0 ? Math.round((sumKills / sumDeaths) * 100) / 100 : sumKills;
  const totalShots = sumHeadshots + sumBodyshots + sumLegshots;
  const headshotPercent = totalShots > 0 ? Math.round((sumHeadshots / totalShots) * 1000) / 10 : 0;
  const acs = Math.round(sumAcs / gamesCounted);
  const adr = Math.round((sumAdr / gamesCounted) * 10) / 10;
  const winrate = Math.round((wins / gamesCounted) * 1000) / 10;
  const kills = Math.round((sumKills / gamesCounted) * 10) / 10;
  const deaths = Math.round((sumDeaths / gamesCounted) * 10) / 10;
  const assists = Math.round((sumAssists / gamesCounted) * 10) / 10;
  return { kills, deaths, assists, kd, headshots: sumHeadshots, bodyshots: sumBodyshots, legshots: sumLegshots, headshotPercent, winrate, acs, adr, recentGamesCount: gamesCounted };
}

// ─── Fixture loader ────────────────────────────────────────────────────────────

async function loadJson(dir: string, name: string): Promise<any> {
  try {
    const content = await readFile(path.join(dir, name), "utf-8");
    return JSON.parse(content);
  } catch { return null; }
}

async function processScenario(scenario: string) {
  const redactedDir = path.join(process.cwd(), "src-tauri", "fixtures", "redacted", scenario);
  const outDir = path.join(process.cwd(), "src-tauri", "fixtures", "expected");
  await mkdir(outDir, { recursive: true });

  console.log(`\nGenerating oracle for: ${scenario}`);

  const lockfileRaw = await readFile(path.join(redactedDir, "lockfile.txt"), "utf-8").catch(() => null);
  if (!lockfileRaw || lockfileRaw.includes("_missing")) {
    const result = { gameState: "OFFLINE", error: "Valorant is not running" };
    await writeFile(path.join(outDir, `${scenario}.json`), JSON.stringify(result, null, 2), "utf-8");
    console.log(`  → OFFLINE`);
    return;
  }

  const [, , port, password] = lockfileRaw.trim().split(":");
  const entitlements = await loadJson(redactedDir, "entitlements.json");
  const puuid: string = entitlements?.subject ?? "";

  const coreGamePlayer = await loadJson(redactedDir, "coregame_player.json");
  const preGamePlayer = await loadJson(redactedDir, "pregame_player.json");
  const coreGameMatchId: string | null = coreGamePlayer?.MatchID ?? null;
  const preGameMatchId: string | null = preGamePlayer?.MatchID ?? null;

  if (!coreGameMatchId && !preGameMatchId) {
    const result = { gameState: "MENUS", players: [], match: null, party: null, selfPuuid: puuid };
    await writeFile(path.join(outDir, `${scenario}.json`), JSON.stringify(result, null, 2), "utf-8");
    console.log(`  → MENUS`);
    return;
  }

  // Load match data
  let players: any[] = [];
  let mapId = "", gameMode = "", gameModeId = "", server = "", seasonId = "";
  let isRanked = false;
  let resolvedMatchId = "";
  let resolvedGameState: "PREGAME" | "INGAME" = "PREGAME";
  let allyTeamId: string | null = null;

  if (coreGameMatchId) {
    resolvedGameState = "INGAME";
    resolvedMatchId = coreGameMatchId;
    const coreGame = await loadJson(redactedDir, "coregame_match.json");
    mapId = coreGame?.MapID ?? "";
    gameMode = coreGame?.Mode ?? "";
    gameModeId = coreGame?.QueueID || coreGame?.ModeID || "";
    isRanked = coreGame?.IsRanked ?? gameModeId === "competitive";
    server = coreGame?.GamePodID ?? "";
    seasonId = coreGame?.SeasonID ?? "";
    players = coreGame?.Players ?? [];
  } else if (preGameMatchId) {
    resolvedGameState = "PREGAME";
    resolvedMatchId = preGameMatchId;
    const preGame = await loadJson(redactedDir, "pregame_match.json");
    mapId = preGame?.MapID ?? "";
    gameMode = preGame?.Mode ?? "";
    gameModeId = preGame?.QueueID || preGame?.ModeID || "";
    isRanked = preGame?.IsRanked ?? gameModeId === "competitive";
    seasonId = preGame?.SeasonID ?? "";
    allyTeamId = preGame?.AllyTeam?.TeamID ?? null;
    const enemyTeamId = preGame?.EnemyTeam?.TeamID ?? (allyTeamId === "Blue" ? "Red" : allyTeamId === "Red" ? "Blue" : null);
    const allyPlayers = (preGame?.AllyTeam?.Players ?? []).map((p: any) => ({ ...p, TeamID: p?.TeamID ?? allyTeamId ?? "Blue" }));
    const enemyPlayers = (preGame?.EnemyTeam?.Players ?? []).map((p: any) => ({ ...p, TeamID: p?.TeamID ?? enemyTeamId ?? "Red" }));
    players = [...allyPlayers, ...enemyPlayers];
  }

  const puuids = players.map((p: any) => p?.Subject ?? p?.PlayerIdentity?.Subject ?? "").filter(Boolean);

  // Load names
  const namesRaw = await loadJson(redactedDir, "names.json") ?? [];
  const nameMap = new Map<string, any>();
  if (Array.isArray(namesRaw)) {
    for (const n of namesRaw) { if (n?.Subject) nameMap.set(n.Subject, n); }
  }

  // Load all fixture files
  const allFiles = await readdir(redactedDir);
  const matchDetailMap = new Map<string, any>();
  const mmrMap = new Map<string, any>();
  const compMap = new Map<string, any>();

  for (const f of allFiles) {
    if (f.startsWith("match_") && f.endsWith(".json")) {
      const matchId = f.replace("match_", "").replace(".json", "");
      const detail = await loadJson(redactedDir, f);
      if (detail) matchDetailMap.set(matchId, detail);
    }
    if (f.startsWith("mmr_") && f.endsWith(".json")) {
      const playerPuuid = f.replace("mmr_", "").replace(".json", "");
      const mmr = await loadJson(redactedDir, f);
      mmrMap.set(playerPuuid, mmr);
    }
    if (f.startsWith("comp_") && f.endsWith(".json")) {
      const playerPuuid = f.replace("comp_", "").replace(".json", "");
      const comp = await loadJson(redactedDir, f);
      compMap.set(playerPuuid, comp);
    }
  }

  // Build players
  const builtPlayers: ValorantPlayer[] = [];
  for (const p of players) {
    const playerPuuid = p?.Subject ?? p?.PlayerIdentity?.Subject ?? "";
    if (!playerPuuid) continue;

    const mmrRaw = mmrMap.get(playerPuuid);
    const compUpdates = compMap.get(playerPuuid);
    const mmrData = mmrRaw?.httpStatus ? null : mmrRaw;
    const latestComp = mmrData?.LatestCompetitiveUpdate;
    const seasonalInfo = mmrData?.QueueSkills?.competitive?.SeasonalInfoBySeasonID ?? {};
    const recentMatches: any[] = Array.isArray(compUpdates?.Matches) ? compUpdates.Matches : [];

    const rank = latestComp?.TierAfterUpdate ?? 0;
    const previousRank = latestComp?.TierBeforeUpdate ?? 0;
    let peakRank = getPeakRank(seasonalInfo, recentMatches, latestComp);
    if (rank > peakRank && rank <= 27) peakRank = rank;
    if (previousRank > peakRank && previousRank <= 27) peakRank = previousRank;

    const latestMatchSeasonId: string = recentMatches[0]?.SeasonID || "";
    const currentSeason = latestMatchSeasonId || getCurrentSeasonId(seasonalInfo, seasonId);
    const currentSeasonData = currentSeason ? seasonalInfo[currentSeason] : null;
    const currentSeasonWins = currentSeasonData?.NumberOfWinsWithPlacements ?? currentSeasonData?.NumberOfWins ?? 0;
    const currentSeasonGames = currentSeasonData?.NumberOfGames ?? 0;
    const isCurrentActRank = !!(currentSeason && (currentSeasonWins + currentSeasonGames) > 0);
    const actWinrate = currentSeasonGames > 0 ? Math.round((currentSeasonWins / currentSeasonGames) * 1000) / 10 : 0;

    const currentActMatches: any[] = [];
    for (const m of recentMatches) {
      if (currentSeason && m?.SeasonID !== currentSeason) break;
      currentActMatches.push(m);
    }

    const latestUpdate = currentActMatches[0] ?? recentMatches[0];
    const rr = latestComp?.RankedRatingAfterUpdate ?? latestUpdate?.RankedRatingAfterUpdate ?? 0;
    const earnedRr = latestComp?.RankedRatingEarned ?? latestUpdate?.RankedRatingEarned ?? 0;
    const leaderboardPosition = latestUpdate?.LeaderboardPosition ?? 0;
    const recentMatchIds = currentActMatches.map((m) => m?.MatchID).filter(Boolean) as string[];

    const agentId = p?.CharacterID ?? p?.CharacterSelectionID ?? "";
    const identity = p?.PlayerIdentity ?? {};
    const nameData = nameMap.get(playerPuuid);
    const displayName = nameData?.GameName ?? nameData?.DisplayName ?? "";
    const tagLine = nameData?.TagLine ?? "";

    // Get match details for this player
    const playerMatchDetails = recentMatchIds.map((mid) => matchDetailMap.get(mid)).filter(Boolean);
    const lastMatchStats = playerMatchDetails.length > 0 ? extractPlayerStats(playerMatchDetails[0], playerPuuid) : null;
    const recentResults = playerMatchDetails.slice(0, 5).map((d) => getMatchResult(d, playerPuuid));

    let basePlayer: ValorantPlayer = {
      puuid: playerPuuid,
      name: displayName,
      tag: tagLine,
      agentId,
      agentName: AGENT_MAP[agentId] ?? "Unknown",
      teamId: p?.TeamID ?? "",
      accountLevel: identity?.AccountLevel ?? 0,
      rank,
      rankName: RANK_MAP[rank] ?? "Unranked",
      peakRank,
      peakRankName: RANK_MAP[peakRank] ?? "Unranked",
      previousRank,
      rr,
      earnedRr,
      leaderboardPosition,
      headshots: 0, bodyshots: 0, legshots: 0, headshotPercent: 0,
      winrate: actWinrate,
      kd: 0, kills: 0, deaths: 0, assists: 0, acs: 0, adr: 0,
      currentSeasonWins, currentSeasonGames, isCurrentActRank,
      recentGamesCount: 0,
      lastMatchKills: lastMatchStats?.kills ?? 0,
      lastMatchDeaths: lastMatchStats?.deaths ?? 0,
      lastMatchAssists: lastMatchStats?.assists ?? 0,
      lastMatchKD: lastMatchStats?.kd ?? 0,
      recentResults,
    };

    if (playerMatchDetails.length > 0) {
      const aggStats = aggregatePlayerStats(playerMatchDetails, playerPuuid);
      basePlayer = { ...basePlayer, ...aggStats, winrate: actWinrate };
    }

    builtPlayers.push(basePlayer);
  }

  // Map / mode resolution
  const mapLower = mapId.toLowerCase().replace(/\.[^/]+$/, "");
  const mapName = MAP_MAP[mapId] ?? MAP_MAP[mapLower] ?? MAP_MAP[mapId.toLowerCase()] ??
    (mapId?.split("/").pop()?.replace(/\.[^.]+$/, "").replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase())) ?? "Unknown";

  const queueLower = gameModeId.toLowerCase();
  const modeLower = gameMode.toLowerCase();
  function stripAndExtract(p2: string) {
    const stripped = p2.replace(/\.[^/]+$/, "");
    const segs = stripped.split("/").filter(Boolean);
    const keyword = segs.length >= 3 ? segs[2] : "";
    return { stripped, keyword };
  }
  const q = stripAndExtract(queueLower);
  const m = stripAndExtract(modeLower);
  const modeKeyword = q.keyword || m.keyword;
  const cleanFallback = modeKeyword
    ? modeKeyword.charAt(0).toUpperCase() + modeKeyword.slice(1).toLowerCase()
    : gameModeId && !gameModeId.includes("/") ? gameModeId.charAt(0).toUpperCase() + gameModeId.slice(1).toLowerCase() : "Unknown";

  let gameModeName = GAMEMODE_MAP[queueLower] ?? GAMEMODE_MAP[modeLower] ?? GAMEMODE_MAP[q.stripped] ?? GAMEMODE_MAP[m.stripped] ?? GAMEMODE_MAP[q.keyword] ?? GAMEMODE_MAP[m.keyword] ?? cleanFallback;
  if (gameModeName === "Standard") gameModeName = isRanked ? "Competitive" : "Unrated";

  const isDeathmatch = DEATHMATCH_MODES.has(queueLower) || DEATHMATCH_MODES.has(modeKeyword);
  const startingSide: TeamSide = resolvedGameState === "PREGAME"
    ? resolveStartingSide(allyTeamId, gameModeId, modeKeyword)
    : null;

  const matchInfo: MatchInfo = {
    matchId: resolvedMatchId, mapId, mapName, gameMode, gameModeId, gameModeName,
    isDeathmatch, server, isRanked, gameState: resolvedGameState, seasonId, startingSide,
  };

  const result = {
    gameState: resolvedGameState,
    match: matchInfo,
    players: builtPlayers,
    selfPuuid: puuid,
  };

  await writeFile(path.join(outDir, `${scenario}.json`), JSON.stringify(result, null, 2), "utf-8");
  console.log(`  → ${resolvedGameState} (${builtPlayers.length} players)`);
  console.log(`  ✅ Written to src-tauri/fixtures/expected/${scenario}.json`);
}

async function main() {
  const all = process.argv.includes("--all");
  const scenario = process.argv.find((a) => a.startsWith("--scenario="))?.split("=")[1]
    ?? process.argv[process.argv.indexOf("--scenario") + 1];

  if (all) {
    const redactedBase = path.join(process.cwd(), "src-tauri", "fixtures", "redacted");
    const { readdir: rd } = await import("node:fs/promises");
    const scenarios = await rd(redactedBase).catch(() => [] as string[]);
    for (const s of scenarios) await processScenario(s);
  } else if (scenario) {
    await processScenario(scenario);
  } else {
    console.error("Usage: npx tsx scripts/generate-expected.ts --scenario <name> | --all");
    process.exit(1);
  }
}

main().catch((err) => {
  console.error("Generation failed:", err);
  process.exit(1);
});
