import type { TeamSide } from "./types";
import rankMapJson from "@/src-tauri/data/rank_map.json";
import agentMapJson from "@/src-tauri/data/agent_map.json";
import deathmatchModesJson from "@/src-tauri/data/deathmatch_modes.json";

export const RANK_MAP: Record<number, string> = Object.fromEntries(
  Object.entries(rankMapJson).map(([k, v]) => [Number(k), v])
);

export const COMPETITIVE_TIER_UUID = "03621f52-342b-cf4e-4f86-9350a49c6d04";

export function rankIconUrl(tier: number, size: "small" | "large" = "large"): string {
  const safeTier = typeof tier === "number" && tier >= 0 && tier <= 27 ? tier : 0;
  return `https://media.valorant-api.com/competitivetiers/${COMPETITIVE_TIER_UUID}/${safeTier}/${size}icon.png`;
}

export const RANK_COLORS: Record<string, string> = {
  Iron: "#7a7a7a", Bronze: "#b8915a", Silver: "#c8c8d0", Gold: "#ecc236",
  Platinum: "#3bb4a0", Diamond: "#b688e8", Ascendant: "#2dbd8e",
  Immortal: "#e05b6f", Radiant: "#ffffa0",
};

export function rankColor(name: string): string {
  const t = Object.keys(RANK_COLORS).find((k) => name.toLowerCase().startsWith(k.toLowerCase()));
  return t ? RANK_COLORS[t] : "var(--ink-faint)";
}

export const RANK_NAMES_SHORT: Record<number, string> = {
  0: "Unranked", 3: "I1", 4: "I2", 5: "I3",
  6: "B1", 7: "B2", 8: "B3", 9: "S1", 10: "S2", 11: "S3",
  12: "G1", 13: "G2", 14: "G3", 15: "P1", 16: "P2", 17: "P3",
  18: "D1", 19: "D2", 20: "D3", 21: "A1", 22: "A2", 23: "A3",
  24: "IM1", 25: "IM2", 26: "IM3", 27: "RD",
};

export const AGENT_MAP: Record<string, string> = agentMapJson;

export const AGENT_ROLE_MAP: Record<string, string> = {
  Jett: "Duelist", Reyna: "Duelist", Phoenix: "Duelist", Raze: "Duelist", Yoru: "Duelist", Neon: "Duelist", Iso: "Duelist",
  Sova: "Initiator", Breach: "Initiator", Skye: "Initiator", "KAY/O": "Initiator", Fade: "Initiator", Gekko: "Initiator",
  Brimstone: "Controller", Omen: "Controller", Astra: "Controller", Viper: "Controller", Harbor: "Controller", Clove: "Controller",
  Sage: "Sentinel", Cypher: "Sentinel", Killjoy: "Sentinel", Chamber: "Sentinel", Deadlock: "Sentinel", Vyse: "Sentinel",
  Miks: "Controller", Tejo: "Initiator", Veto: "Sentinel", Waylay: "Duelist",
};

export const DEATHMATCH_MODES = new Set<string>(deathmatchModesJson);

export const SIDE_MODE_KEYWORDS = new Set(["bomb", "quickbomb", "spikerush", "swiftplay"]);
export const SIDE_QUEUES = new Set(["competitive", "unrated", "swiftplay", "spikerush"]);

export function resolveStartingSide(
  teamId: string | undefined | null,
  queueId: string,
  modeKeyword: string
): TeamSide {
  if (!teamId) return null;
  const q = queueId.toLowerCase();

  // Custom games have empty queues, so fall back to mode keyword check
  const eligible = q
    ? SIDE_QUEUES.has(q) || SIDE_MODE_KEYWORDS.has(modeKeyword)
    : SIDE_MODE_KEYWORDS.has(modeKeyword);

  if (!eligible) return null;

  const t = teamId.toLowerCase();
  return t === "blue" ? "defence" : t === "red" ? "attack" : null;
}

