export const COMPETITIVE_TIER_UUID = "03621f52-342b-cf4e-4f86-9350a49c6d04";

export function rankIconUrl(tier: number, size: "small" | "large" = "large"): string {
  const safeTier = typeof tier === "number" && tier >= 0 && tier <= 27 ? tier : 0;
  return `https://media.valorant-api.com/competitivetiers/${COMPETITIVE_TIER_UUID}/${safeTier}/${size}icon.png`;
}

export const RANK_COLORS: Record<string, string> = {
  Iron: "#7a7a7a",
  Bronze: "#b8915a",
  Silver: "#c8c8d0",
  Gold: "#ecc236",
  Platinum: "#3bb4a0",
  Diamond: "#b688e8",
  Ascendant: "#2dbd8e",
  Immortal: "#e05b6f",
  Radiant: "#ffffa0",
};

export function rankColor(name: string): string {
  const t = Object.keys(RANK_COLORS).find((k) => name.toLowerCase().startsWith(k.toLowerCase()));
  return t ? RANK_COLORS[t] : "var(--ink-faint)";
}

export const RANK_NAMES_SHORT: Record<number, string> = {
  0: "Unranked",
  3: "I1",
  4: "I2",
  5: "I3",
  6: "B1",
  7: "B2",
  8: "B3",
  9: "S1",
  10: "S2",
  11: "S3",
  12: "G1",
  13: "G2",
  14: "G3",
  15: "P1",
  16: "P2",
  17: "P3",
  18: "D1",
  19: "D2",
  20: "D3",
  21: "A1",
  22: "A2",
  23: "A3",
  24: "IM1",
  25: "IM2",
  26: "IM3",
  27: "RD",
};
