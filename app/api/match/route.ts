import {
  readLockfile,
  getApiConfig,
  clearApiConfigCache,
  getPreGamePlayerId,
  getCoreGamePlayerId,
  getPreGameMatch,
  getCoreGameMatch,
  getPlayerMMR,
  getCompetitiveUpdates,
  getMatchDetails,
  getNameFromPuuid,
  getPlayerPresences,
  type PlayerPresenceInfo,
} from "@/lib/valorant-api";
import { RANK_MAP, AGENT_MAP, MAP_MAP, GAMEMODE_MAP, DEATHMATCH_MODES } from "@/lib/constants";
import type { ValorantPlayer, MatchInfo, ApiConfig } from "@/lib/types";

export const dynamic = "force-dynamic";

const RECENT_GAMES_COUNT = 20;

let matchCache: {
  matchId: string;
  players: ValorantPlayer[];
  matchInfo: MatchInfo;
  gameState: string;
} | null = null;

function getPeakRank(
  seasonalInfo: Record<string, any>,
  recentMatches: any[] = [],
  latestComp?: any
): number {
  let peak = 0;

  for (const seasonId in seasonalInfo) {
    const season = seasonalInfo[seasonId];
    if (!season) continue;

    // 1. CompetitiveTier: tier ended on or current tier in that season
    const compTier = season.CompetitiveTier;
    if (typeof compTier === "number" && compTier > peak && compTier <= 27) {
      peak = compTier;
    }

    // 2. Rank: peak act rank tier achieved in that season
    const actRank = season.Rank;
    if (typeof actRank === "number" && actRank > peak && actRank <= 27) {
      peak = actRank;
    }

    // 3. WinsByTier: keys are tier numbers where wins were registered during that season
    if (season.WinsByTier && typeof season.WinsByTier === "object") {
      for (const [tierStr, winCount] of Object.entries(season.WinsByTier)) {
        const tier = Number(tierStr);
        if (
          Number.isInteger(tier) &&
          tier > peak &&
          tier <= 27 &&
          Number(winCount) > 0
        ) {
          peak = tier;
        }
      }
    }
  }

  // 4. Latest competitive update (current tier and tier before update)
  if (latestComp) {
    const after = latestComp.TierAfterUpdate;
    const before = latestComp.TierBeforeUpdate;
    if (typeof after === "number" && after > peak && after <= 27) peak = after;
    if (typeof before === "number" && before > peak && before <= 27) peak = before;
  }

  // 5. Recent matches in competitive updates
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
    if (games > 0 && (!best || seasonId > best)) {
      best = seasonId;
    }
  }
  return best;
}

/**
 * Fills in `partyId` for lobby players who have no presence data, using match
 * history as a fallback.  The local presence endpoint only surfaces friends and
 * the user's own party — players who are a premade on the same team but not on
 * the friend list are invisible to it.  Match details expose a `partyId` per
 * player, so two lobby players who shared a partyId in any recently-loaded
 * match are highly likely to be queued together right now.
 *
 * Presence data always wins; this only fills gaps where presence is absent.
 */
function enrichPartyFromMatchHistory(
  players: ValorantPlayer[],
  detailLookup: Map<string, any>
): void {
  const lobbyPuuids = new Set(players.map((p) => p.puuid));

  for (const matchDetail of detailLookup.values()) {
    const matchPlayers: Array<{ subject?: string; partyId?: string }> =
      matchDetail?.players ?? [];

    // Group current-lobby players that appeared in this match by partyId.
    const partyGroups = new Map<string, string[]>();
    for (const mp of matchPlayers) {
      const puuid = mp.subject ?? "";
      const partyId = mp.partyId ?? "";
      if (!puuid || !partyId || !lobbyPuuids.has(puuid)) continue;
      const g = partyGroups.get(partyId) ?? [];
      g.push(puuid);
      partyGroups.set(partyId, g);
    }

    // For groups where 2+ current-lobby players were in the same party,
    // set partyId on those that don't already have one (presence wins later).
    for (const [partyId, members] of partyGroups) {
      if (members.length < 2) continue;
      for (const puuid of members) {
        const player = players.find((p) => p.puuid === puuid);
        if (player && !player.partyId) {
          player.partyId = partyId;
          // partySize = how many current lobby players shared this party
          // (the real match party could be larger; we only see the lobby overlap).
          player.partySize = members.length;
        }
      }
    }
  }
}

function assignPartyNumbers(
  players: ValorantPlayer[],
  presences: Map<string, PlayerPresenceInfo>
): ValorantPlayer[] {
  const updated = players.map((p) => {
    const presence = presences.get(p.puuid);
    return {
      ...p,
      // Presence is authoritative; fall back to whatever was set by
      // enrichPartyFromMatchHistory (or undefined if neither source has data).
      partyId: presence?.partyId ?? p.partyId,
      partySize: presence?.partySize ?? p.partySize,
    };
  });

  const teamMap = new Map<string, ValorantPlayer[]>();
  for (const p of updated) {
    const teamKey = p.teamId || "default";
    const list = teamMap.get(teamKey) || [];
    list.push(p);
    teamMap.set(teamKey, list);
  }

  for (const [, teamPlayers] of teamMap) {
    const partyCounts = new Map<string, number>();
    for (const p of teamPlayers) {
      if (p.partyId) {
        partyCounts.set(p.partyId, (partyCounts.get(p.partyId) || 0) + 1);
      }
    }

    const validParties = new Set<string>();
    for (const [partyId, count] of partyCounts) {
      if (count >= 2) {
        validParties.add(partyId);
      }
    }

    const partyNumberMap = new Map<string, number>();
    let nextPartyNumber = 1;
    for (const p of teamPlayers) {
      if (p.partyId && validParties.has(p.partyId)) {
        if (!partyNumberMap.has(p.partyId)) {
          partyNumberMap.set(p.partyId, nextPartyNumber++);
        }
        p.partyNumber = partyNumberMap.get(p.partyId);
      } else {
        p.partyNumber = undefined;
      }
    }
  }

  return updated;
}

// Persistent LRU cache sized to comfortably hold ~20 recent games for up to 25 players
// across matches (~500 matches ≈ 4-5MB of RAM). Details are keyed by immutable match id.
const MAX_MATCH_DETAIL_CACHE = 500;
const matchDetailCache = new Map<string, any>();

function cacheMatchDetail(id: string, detail: any) {
  if (matchDetailCache.has(id)) {
    matchDetailCache.delete(id);
  } else if (matchDetailCache.size >= MAX_MATCH_DETAIL_CACHE) {
    const oldest = matchDetailCache.keys().next().value;
    if (oldest) matchDetailCache.delete(oldest);
  }
  matchDetailCache.set(id, detail);
}

function extractPlayerStats(matchDetail: any, puuid: string) {
  const playerStats = matchDetail?.players?.find((p: any) => p.subject === puuid);
  let kills = 0, deaths = 0, assists = 0, kd = 0;
  let headshots = 0, bodyshots = 0, legshots = 0, headshotPercent = 0;
  let winrate = 0;

  if (playerStats?.stats) {
    kills = playerStats.stats.kills ?? 0;
    deaths = playerStats.stats.deaths ?? 0;
    assists = playerStats.stats.assists ?? 0;
    kd = deaths > 0 ? Math.round((kills / deaths) * 100) / 100 : kills;
  }

  const rounds = matchDetail?.roundResults ?? [];
  let totalHeadshots = 0, totalBodyshots = 0, totalLegshots = 0;
  let totalDamage = 0;
  for (const round of rounds) {
    const playerRound = round?.playerStats?.find((p: any) => p.subject === puuid);
    if (playerRound?.damage) {
      for (const d of playerRound.damage) {
        totalHeadshots += d.headshots ?? 0;
        totalBodyshots += d.bodyshots ?? 0;
        totalLegshots += d.legshots ?? 0;
        totalDamage += d.damage ?? 0;
      }
    }
  }
  headshots = totalHeadshots;
  bodyshots = totalBodyshots;
  legshots = totalLegshots;
  const totalShots = totalHeadshots + totalBodyshots + totalLegshots;
  headshotPercent = totalShots > 0
    ? Math.round((totalHeadshots / totalShots) * 1000) / 10
    : 0;

  const roundsPlayed = playerStats?.stats?.roundsPlayed ?? rounds.length;
  const acs = roundsPlayed > 0 ? Math.round((playerStats?.stats?.score ?? 0) / roundsPlayed) : 0;
  const adr = roundsPlayed > 0 ? Math.round((totalDamage / roundsPlayed) * 10) / 10 : 0;

  const teamId = playerStats?.teamId;
  let won = false;
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

/** Like extractPlayerStats' `won` field, but distinguishes an actual draw
 *  (rare, but possible if a match ends with equal rounds) from a loss. */
function getMatchResult(matchDetail: any, puuid: string): "W" | "L" | "D" {
  const playerStats = matchDetail?.players?.find((p: any) => p.subject === puuid);
  const teamId = playerStats?.teamId;
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

/** Averages a player's stats across their N most recent completed matches. */
function aggregatePlayerStats(matchDetails: any[], puuid: string) {
  let sumKills = 0, sumDeaths = 0, sumAssists = 0;
  let sumHeadshots = 0, sumBodyshots = 0, sumLegshots = 0;
  let sumAcs = 0, sumAdr = 0, wins = 0;
  let gamesCounted = 0;

  for (const matchDetail of matchDetails) {
    if (!matchDetail) continue;
    const stats = extractPlayerStats(matchDetail, puuid);
    sumKills += stats.kills;
    sumDeaths += stats.deaths;
    sumAssists += stats.assists;
    sumHeadshots += stats.headshots;
    sumBodyshots += stats.bodyshots;
    sumLegshots += stats.legshots;
    sumAcs += stats.acs;
    sumAdr += stats.adr;
    if (stats.won) wins++;
    gamesCounted++;
  }

  if (gamesCounted === 0) {
    return {
      kills: 0, deaths: 0, assists: 0, kd: 0,
      headshots: 0, bodyshots: 0, legshots: 0, headshotPercent: 0,
      winrate: 0, acs: 0, adr: 0, recentGamesCount: 0,
    };
  }

  const kd = sumDeaths > 0 ? Math.round((sumKills / sumDeaths) * 100) / 100 : sumKills;
  const totalShots = sumHeadshots + sumBodyshots + sumLegshots;
  const headshotPercent = totalShots > 0 ? Math.round((sumHeadshots / totalShots) * 1000) / 10 : 0;
  const acs = Math.round(sumAcs / gamesCounted);
  const adr = Math.round((sumAdr / gamesCounted) * 10) / 10;
  const winrate = Math.round((wins / gamesCounted) * 1000) / 10;
  const kills = Math.round((sumKills / gamesCounted) * 10) / 10;
  const deaths = Math.round((sumDeaths / gamesCounted) * 10) / 10;
  const assists = Math.round((sumAssists / gamesCounted) * 10) / 10;

  return {
    kills, deaths, assists, kd,
    headshots: sumHeadshots, bodyshots: sumBodyshots, legshots: sumLegshots, headshotPercent,
    winrate, acs, adr, recentGamesCount: gamesCounted,
  };
}

async function buildPlayer(
  puuid: string,
  config: ApiConfig,
  matchPlayerData: any,
  nameData: { GameName?: string; DisplayName?: string; TagLine?: string } | null,
  matchSeasonId: string
): Promise<ValorantPlayer & { _recentMatchIds?: string[] }> {
  const [mmrRaw, compUpdates] = await Promise.all([
    getPlayerMMR(config, puuid).catch((e) => { console.error("[mmr throw]", puuid, e?.message); return null; }),
    getCompetitiveUpdates(config, puuid, RECENT_GAMES_COUNT).catch((e) => { console.error("[comp throw]", puuid, e?.message); return null; }),
  ]);

  const mmrData = mmrRaw?.httpStatus ? null : mmrRaw;
  const latestComp = mmrData?.LatestCompetitiveUpdate;
  const seasonalInfo = mmrData?.QueueSkills?.competitive?.SeasonalInfoBySeasonID ?? {};

  const recentMatches: any[] = Array.isArray(compUpdates?.Matches) ? compUpdates.Matches : [];

  const rank = latestComp?.TierAfterUpdate ?? 0;
  const previousRank = latestComp?.TierBeforeUpdate ?? 0;

  let peakRank = getPeakRank(seasonalInfo, recentMatches, latestComp);
  if (rank > peakRank && rank <= 27) peakRank = rank;
  if (previousRank > peakRank && previousRank <= 27) peakRank = previousRank;

  // The live-match APIs (pregame/core-game) don't actually expose a SeasonID field
  // (contrary to what `matchSeasonId` implies), so it's always empty in practice.
  // The most reliable signal for "current act" we actually have is the season of the
  // player's own most recent competitive match.
  const latestMatchSeasonId: string = recentMatches[0]?.SeasonID || "";
  const currentSeason = latestMatchSeasonId || getCurrentSeasonId(seasonalInfo, matchSeasonId);

  const currentSeasonData = currentSeason ? seasonalInfo[currentSeason] : null;
  const currentSeasonWins = currentSeasonData?.NumberOfWinsWithPlacements ?? currentSeasonData?.NumberOfWins ?? 0;
  const currentSeasonGames = currentSeasonData?.NumberOfGames ?? 0;
  const isCurrentActRank = !!(currentSeason && (currentSeasonWins + currentSeasonGames) > 0);
  const actWinrate =
    currentSeasonGames > 0
      ? Math.round((currentSeasonWins / currentSeasonGames) * 1000) / 10
      : 0;

  // Matches come back newest-first, so take the leading run that shares the same
  // season as the most recent match — once the season changes we've crossed into
  // a previous act and should stop there.
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

  const agentId = matchPlayerData?.CharacterID ?? matchPlayerData?.CharacterSelectionID ?? "";
  const identity = matchPlayerData?.PlayerIdentity ?? {};

  const displayName = nameData?.GameName ?? (nameData as any)?.DisplayName ?? "";
  const tagLine = nameData?.TagLine ?? "";

  return {
    puuid,
    name: displayName,
    tag: tagLine,
    agentId,
    agentName: AGENT_MAP[agentId] ?? "Unknown",
    teamId: matchPlayerData?.TeamID ?? "",
    accountLevel: identity?.AccountLevel ?? 0,
    rank,
    rankName: RANK_MAP[rank] ?? "Unranked",
    peakRank,
    peakRankName: RANK_MAP[peakRank] ?? "Unranked",
    previousRank,
    rr,
    earnedRr,
    leaderboardPosition,
    headshots: 0,
    bodyshots: 0,
    legshots: 0,
    headshotPercent: 0,
    winrate: actWinrate,
    kd: 0,
    kills: 0,
    deaths: 0,
    assists: 0,
    acs: 0,
    adr: 0,
    currentSeasonWins,
    currentSeasonGames,
    isCurrentActRank,
    recentGamesCount: 0,
    lastMatchKills: 0,
    lastMatchDeaths: 0,
    lastMatchAssists: 0,
    lastMatchKD: 0,
    recentResults: [],
    _recentMatchIds: recentMatchIds,
  };
}

export async function GET(request: Request) {
  const { searchParams } = new URL(request.url);
  const forceRefresh = searchParams.get("force") === "1";

  if (forceRefresh) {
    matchCache = null;
    clearApiConfigCache();
  }

  try {
    const lockfile = await readLockfile();
    const config = await getApiConfig(lockfile, forceRefresh);

    // 1. Check if user is currently INGAME
    const coreGameMatchId = await getCoreGamePlayerId(config);

    // Fast-path: if already INGAME and we have cached data for this match,
    // return immediately! No additional API calls needed (no pregame check, no presence check).
    if (coreGameMatchId && matchCache && matchCache.matchId === coreGameMatchId) {
      return Response.json({
        gameState: matchCache.gameState,
        match: matchCache.matchInfo,
        players: matchCache.players,
        selfPuuid: config.puuid,
      });
    }

    // 2. If not INGAME, check if in PREGAME
    const preGameMatchId = coreGameMatchId ? null : await getPreGamePlayerId(config);

    // 3. If neither, we are in MENUS (no presences fetch needed)
    if (!coreGameMatchId && !preGameMatchId) {
      matchCache = null;
      return Response.json({
        gameState: "MENUS",
        players: [],
        match: null,
        party: null,
        selfPuuid: config.puuid,
      });
    }

    // Since we are in a match (PREGAME or initial INGAME load), fetch presences for party detection
    const presences = await getPlayerPresences(lockfile).catch(
      () => new Map<string, PlayerPresenceInfo>()
    );

    let players: any[] = [];
    let mapId = "";
    let gameMode = "";
    let gameModeId = "";
    let server = "";
    let isRanked = false;
    let resolvedMatchId = "";
    let seasonId = "";
    let resolvedGameState: "PREGAME" | "INGAME" = "PREGAME";

    if (coreGameMatchId) {
      resolvedGameState = "INGAME";
      resolvedMatchId = coreGameMatchId;
      const coreGame = await getCoreGameMatch(config, coreGameMatchId);
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
      const preGame = await getPreGameMatch(config, preGameMatchId);
      mapId = preGame?.MapID ?? "";
      gameMode = preGame?.Mode ?? "";
      gameModeId = preGame?.QueueID || preGame?.ModeID || "";
      isRanked = preGame?.IsRanked ?? gameModeId === "competitive";
      seasonId = preGame?.SeasonID ?? "";

      const allyPlayers = (preGame?.AllyTeam?.Players ?? []).map((p: any) => ({
        ...p,
        TeamID: p?.TeamID ?? "Blue",
      }));
      const enemyPlayers = (preGame?.EnemyTeam?.Players ?? []).map((p: any) => ({
        ...p,
        TeamID: p?.TeamID ?? "Red",
      }));
      players = [...allyPlayers, ...enemyPlayers];
    }

    const puuids = players
      .map((p: any) => p?.Subject ?? p?.PlayerIdentity?.Subject ?? "")
      .filter(Boolean);

    if (matchCache && matchCache.matchId !== resolvedMatchId) {
      matchCache = null;
    }

    if (matchCache && matchCache.matchId === resolvedMatchId) {
      if (resolvedGameState === "PREGAME") {
        for (const p of matchCache.players) {
          const fresh = players.find((fp: any) => (fp?.Subject ?? fp?.PlayerIdentity?.Subject) === p.puuid);
          const freshAgentId = fresh?.CharacterID ?? fresh?.CharacterSelectionID ?? "";
          if (freshAgentId && freshAgentId !== p.agentId) {
            p.agentId = freshAgentId;
            p.agentName = AGENT_MAP[freshAgentId] ?? "Unknown";
          }
        }
      }

      if (presences.size > 0) {
        matchCache.players = assignPartyNumbers(matchCache.players, presences);
      }

      return Response.json({
        gameState: matchCache.gameState,
        match: matchCache.matchInfo,
        players: matchCache.players,
        selfPuuid: config.puuid,
      });
    }

    const allNames = await getNameFromPuuid(config, puuids).catch(() => []);
    const nameMap = new Map<string, any>();
    if (Array.isArray(allNames)) {
      for (const n of allNames) {
        if (n?.Subject) nameMap.set(n.Subject, n);
      }
    }

    const rawPlayers = await Promise.all(
      players.map((p: any, i: number) => {
        const puuid = puuids[i];
        if (!puuid) return null;
        const nameData = nameMap.get(puuid) ?? null;
        return buildPlayer(puuid, config, p, nameData, seasonId)
          .catch(() => null);
      })
    );
    const validPlayers = rawPlayers.filter((p): p is (ValorantPlayer & { _recentMatchIds?: string[] }) => p !== null);

    const uniqueMatchIds = new Set<string>();
    for (const p of validPlayers) {
      for (const mid of p._recentMatchIds ?? []) {
        if (matchDetailCache.has(mid)) {
          const cached = matchDetailCache.get(mid);
          matchDetailCache.delete(mid);
          matchDetailCache.set(mid, cached);
        } else {
          uniqueMatchIds.add(mid);
        }
      }
    }

    const detailResults = await Promise.all(
      [...uniqueMatchIds].map((mid) =>
        getMatchDetails(config, mid)
          .then((detail) => ({ mid, detail }))
          .catch(() => ({ mid, detail: null }))
      )
    );

    // Combine newly fetched details with whatever's already cached before writing back —
    // this avoids losing entries mid-computation if the cache cap gets hit during this batch.
    const detailLookup = new Map<string, any>(matchDetailCache);
    for (const { mid, detail } of detailResults) {
      if (detail) detailLookup.set(mid, detail);
    }
    for (const { mid, detail } of detailResults) {
      if (detail) cacheMatchDetail(mid, detail);
    }

    const builtPlayers: ValorantPlayer[] = validPlayers.map((p) => {
      const { _recentMatchIds, ...player } = p;
      const details = (_recentMatchIds ?? [])
        .map((mid) => detailLookup.get(mid))
        .filter(Boolean);
      const lastMatchStats = details.length > 0 ? extractPlayerStats(details[0], p.puuid) : null;
      const lastMatch = {
        lastMatchKills: lastMatchStats?.kills ?? 0,
        lastMatchDeaths: lastMatchStats?.deaths ?? 0,
        lastMatchAssists: lastMatchStats?.assists ?? 0,
        lastMatchKD: lastMatchStats?.kd ?? 0,
      };
      // Up to 5 most recent results, newest first — details is already
      // ordered this way since _recentMatchIds comes from the newest-first
      // currentActMatches list.
      const recentResults = details.slice(0, 5).map((d) => getMatchResult(d, p.puuid));
      if (details.length > 0) {
        const stats = aggregatePlayerStats(details, p.puuid);
        return { ...player, ...stats, winrate: player.winrate, ...lastMatch, recentResults };
      }
      return { ...player, ...lastMatch, recentResults };
    });
    const mapLower = mapId.toLowerCase().replace(/\.[^/]+$/, "");
    const mapName =
      MAP_MAP[mapId] ??
      MAP_MAP[mapLower] ??
      MAP_MAP[mapId.toLowerCase()] ??
      (mapId?.split("/").pop()?.replace(/\.[^.]+$/, "").replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase())) ??
      "Unknown";

    const queueLower = gameModeId.toLowerCase();
    const modeLower = gameMode.toLowerCase();

    function stripAndExtract(path: string) {
      const stripped = path.replace(/\.[^/]+$/, "");
      const segs = stripped.split("/").filter(Boolean);
      const keyword = segs.length >= 3 ? segs[2] : "";
      return { stripped, keyword };
    }

    const q = stripAndExtract(queueLower);
    const m = stripAndExtract(modeLower);
    const modeKeyword = q.keyword || m.keyword;

    const cleanFallback = modeKeyword
      ? modeKeyword.charAt(0).toUpperCase() + modeKeyword.slice(1).toLowerCase()
      : gameModeId && !gameModeId.includes("/")
        ? gameModeId.charAt(0).toUpperCase() + gameModeId.slice(1).toLowerCase()
        : "Unknown";

    let gameModeName =
      GAMEMODE_MAP[queueLower] ??
      GAMEMODE_MAP[modeLower] ??
      GAMEMODE_MAP[q.stripped] ??
      GAMEMODE_MAP[m.stripped] ??
      GAMEMODE_MAP[q.keyword] ??
      GAMEMODE_MAP[m.keyword] ??
      cleanFallback;

    if (gameModeName === "Standard") {
      gameModeName = isRanked ? "Competitive" : "Unrated";
    }
    const isDeathmatch = DEATHMATCH_MODES.has(queueLower) || DEATHMATCH_MODES.has(modeKeyword);

    const matchInfo: MatchInfo = {
      matchId: resolvedMatchId,
      mapId,
      mapName,
      gameMode,
      gameModeId,
      gameModeName,
      isDeathmatch,
      server,
      isRanked,
      gameState: resolvedGameState,
      seasonId,
    };

    // Fill in partyId for players whose presence was unavailable (non-friends
    // on the same team), using shared partyId from recent match history.
    enrichPartyFromMatchHistory(builtPlayers, detailLookup);

    const playersWithParty = assignPartyNumbers(builtPlayers, presences);

    matchCache = {
      matchId: resolvedMatchId,
      players: playersWithParty,
      matchInfo,
      gameState: resolvedGameState,
    };

    return Response.json({
      gameState: resolvedGameState,
      match: matchInfo,
      players: playersWithParty,
      selfPuuid: config.puuid,
    });
  } catch (err: any) {
    const message = err?.message ?? String(err);
    if (
      message.includes("ENOENT") ||
      message.includes("lockfile") ||
      message.includes("ECONNREFUSED") ||
      message.includes("not running")
    ) {
      return Response.json({
        error: "Valorant is not running",
        gameState: "OFFLINE",
      });
    }
    return Response.json(
      { error: message, gameState: "ERROR" },
      { status: 500 }
    );
  }
}