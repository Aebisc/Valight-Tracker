export interface ValorantPlayer {
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
  previousRank: number;
  rr: number;
  earnedRr: number;
  leaderboardPosition: number;
  headshots: number;
  bodyshots: number;
  legshots: number;
  headshotPercent: number;
  winrate: number;
  kd: number;
  kills: number;
  deaths: number;
  assists: number;
  acs: number;
  adr: number;
  currentSeasonWins: number;
  currentSeasonGames: number;
  isCurrentActRank: boolean;
  recentGamesCount: number;
  lastMatchKills: number;
  lastMatchDeaths: number;
  lastMatchAssists: number;
  lastMatchKD: number;
  /** Up to 5 most recent competitive match outcomes, newest first. */
  recentResults: ("W" | "L" | "D")[];
  partyId?: string;
  /** 1-indexed group number for 2+ player parties on the same team. */
  partyNumber?: number;
  partySize?: number;
}

export interface MatchInfo {
  matchId: string;
  mapId: string;
  mapName: string;
  gameMode: string;
  gameModeId: string;
  gameModeName: string;
  isDeathmatch: boolean;
  server: string;
  isRanked: boolean;
  gameState: string;
  seasonId: string;
}

export type Player = ValorantPlayer;

export interface ApiResponse {
  gameState: string;
  error?: string;
  match?: MatchInfo;
  players?: Player[];
  selfPuuid?: string;
}

export interface LockfileData {
  name: string;
  pid: string;
  port: string;
  password: string;
  protocol: string;
}

export interface ApiConfig {
  pdUrl: string;
  glzUrl: string;
  region: string;
  shard: string;
  puuid: string;
  headers: Record<string, string>;
  version: string;
}