import { test } from "node:test";
import assert from "node:assert";
import { isPlayerEqual, sharePlayers, isMatchEqual, shareMatch } from "../lib/share-structure.ts";
import type { Player, MatchInfo } from "../lib/types.ts";

function makePlayer(puuid: string, kills = 10, deaths = 5, extra: Partial<Player> = {}): Player {
  return {
    puuid,
    name: `Player_${puuid}`,
    tag: "NA1",
    agentId: "agent-1",
    agentName: "Jett",
    teamId: "Blue",
    accountLevel: 100,
    rank: 15,
    rankName: "Platinum 1",
    peakRank: 18,
    peakRankName: "Diamond 1",
    previousRank: 14,
    rr: 50,
    earnedRr: 20,
    leaderboardPosition: 0,
    headshots: 12,
    bodyshots: 20,
    legshots: 4,
    headshotPercent: 33.3,
    winrate: 55.0,
    kd: 2.0,
    kills,
    deaths,
    assists: 3,
    acs: 240,
    adr: 150.5,
    currentSeasonWins: 10,
    currentSeasonGames: 18,
    isCurrentActRank: true,
    recentGamesCount: 5,
    lastMatchKills: 15,
    lastMatchDeaths: 12,
    lastMatchAssists: 4,
    lastMatchKD: 1.25,
    recentResults: ["W", "L", "W"],
    ...extra,
  };
}

test("BUG-02: Identical input returns exact same array reference and element references", () => {
  const p1 = makePlayer("p1", 10, 5);
  const p2 = makePlayer("p2", 15, 8);
  const prevList = [p1, p2];

  const incomingList = [makePlayer("p1", 10, 5), makePlayer("p2", 15, 8)];
  const result = sharePlayers(prevList, incomingList);

  assert.strictEqual(result, prevList, "Array reference must be reused when all players and order match");
  assert.strictEqual(result[0], p1, "Player 1 reference must be preserved");
  assert.strictEqual(result[1], p2, "Player 2 reference must be preserved");
});

test("BUG-02: One changed field in one player returns new array with shared unchanged players", () => {
  const p1 = makePlayer("p1", 10, 5);
  const p2 = makePlayer("p2", 15, 8);
  const prevList = [p1, p2];

  // p2 got 1 more kill; p1 is unchanged
  const incomingList = [makePlayer("p1", 10, 5), makePlayer("p2", 16, 8)];
  const result = sharePlayers(prevList, incomingList);

  assert.notStrictEqual(result, prevList, "Array reference must be new when data changed");
  assert.strictEqual(result[0], p1, "Unchanged player 1 must preserve object reference");
  assert.notStrictEqual(result[1], p2, "Changed player 2 must have new object reference");
  assert.strictEqual(result[1].kills, 16);
});

test("BUG-02: Reordered input returns new array preserving incoming order, not old array", () => {
  const p1 = makePlayer("p1", 10, 5);
  const p2 = makePlayer("p2", 15, 8);
  const prevList = [p1, p2];

  // Incoming has reverse order [p2, p1]
  const incomingList = [makePlayer("p2", 15, 8), makePlayer("p1", 10, 5)];
  const result = sharePlayers(prevList, incomingList);

  assert.notStrictEqual(result, prevList, "Array reference must be new when order changes");
  assert.strictEqual(result[0].puuid, "p2", "First element must be p2");
  assert.strictEqual(result[1].puuid, "p1", "Second element must be p1");
  assert.strictEqual(result[0], p2, "p2 element reference should be reused");
  assert.strictEqual(result[1], p1, "p1 element reference should be reused");
});

test("BUG-02: Added or removed player produces new array", () => {
  const p1 = makePlayer("p1");
  const prevList = [p1];

  const incomingList = [makePlayer("p1"), makePlayer("p2")];
  const result = sharePlayers(prevList, incomingList);

  assert.notStrictEqual(result, prevList);
  assert.strictEqual(result.length, 2);
  assert.strictEqual(result[0], p1);
});

test("BUG-02: Optional key appearing or disappearing counts as a change", () => {
  const p1WithoutParty = makePlayer("p1");
  const p1WithParty = makePlayer("p1", 10, 5, { partyNumber: 1, partySize: 2 });

  assert.strictEqual(isPlayerEqual(p1WithoutParty, p1WithParty), false, "Party appearing must not be equal");
  assert.strictEqual(isPlayerEqual(p1WithParty, p1WithoutParty), false, "Party disappearing must not be equal");
});

test("BUG-02: NaN fields present in both are treated as equal", () => {
  const p1 = makePlayer("p1", 0, 0, { kd: NaN });
  const p2 = makePlayer("p1", 0, 0, { kd: NaN });

  assert.strictEqual(isPlayerEqual(p1, p2), true, "NaN kd should be equal to NaN kd");
});

test("BUG-02: MatchInfo structural sharing preserves reference on equal match", () => {
  const m1: MatchInfo = {
    matchId: "m1",
    mapId: "/Game/Maps/Ascent/Ascent",
    mapName: "Ascent",
    gameMode: "Standard",
    gameModeId: "competitive",
    gameModeName: "Competitive",
    isDeathmatch: false,
    server: "Frankfurt 1",
    isRanked: true,
    gameState: "INGAME",
    seasonId: "season-1",
  };

  const m2: MatchInfo = { ...m1 };

  const shared = shareMatch(m1, m2);
  assert.strictEqual(shared, m1, "Equal match must return original reference");

  const m3: MatchInfo = { ...m1, gameState: "MENUS" };
  const sharedDiff = shareMatch(m1, m3);
  assert.strictEqual(sharedDiff, m3, "Different match must return incoming reference");
});
