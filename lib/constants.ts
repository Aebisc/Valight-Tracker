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

export const MAP_NAMES: Record<string, string> = {
  "/Game/Maps/Ascent/Ascent": "Ascent",
  "/Game/Maps/Bonsai/Bonsai": "Split",
  "/Game/Maps/Canyon/Canyon": "Fracture",
  "/Game/Maps/Duality/Duality": "Bind",
  "/Game/Maps/Foxtrot/Foxtrot": "Breeze",
  "/Game/Maps/HURM/HURM_Alley/HURM_Alley": "District",
  "/Game/Maps/HURM/HURM_Bowl/HURM_Bowl": "Kasbah",
  "/Game/Maps/HURM/HURM_Helix/HURM_Helix": "Drift",
  "/Game/Maps/HURM/HURM_HighTide/HURM_HighTide": "Glitch",
  "/Game/Maps/HURM/HURM_Yard/HURM_Yard": "Piazza",
  "/Game/Maps/Infinity/Infinity": "Abyss",
  "/Game/Maps/Jam/Jam": "Lotus",
  "/Game/Maps/Juliett/Juliett": "Sunset",
  "/Game/Maps/Pitt/Pitt": "Pearl",
  "/Game/Maps/Plummet/Plummet": "Summit",
  "/Game/Maps/Port/Port": "Icebox",
  "/Game/Maps/Rook/Rook": "Corrode",
  "/Game/Maps/Triad/Triad": "Haven",
  "/Game/Maps/Poveglia/Range": "The Range",
  "/Game/Maps/PovegliaV2/RangeV2": "The Range",
  "/Game/Maps/NPEV2/NPEV2": "Basic Training",
  "/Game/Maps/AbilityDraft/AbilityDraft": "Gauntlet",
};

const MAP_CODENAMES: Record<string, string> = {
  ascent: "Ascent",
  bonsai: "Split",
  split: "Split",
  canyon: "Fracture",
  fracture: "Fracture",
  duality: "Bind",
  bind: "Bind",
  foxtrot: "Breeze",
  breeze: "Breeze",
  infinity: "Abyss",
  abyss: "Abyss",
  jam: "Lotus",
  lotus: "Lotus",
  juliett: "Sunset",
  jules: "Sunset",
  sunset: "Sunset",
  pitt: "Pearl",
  pearl: "Pearl",
  port: "Icebox",
  icebox: "Icebox",
  triad: "Haven",
  haven: "Haven",
  corrode: "Corrode",
  rook: "Corrode",
  summit: "Summit",
  plummet: "Summit",
  hurm_alley: "District",
  district: "District",
  hurm_bowl: "Kasbah",
  kasbah: "Kasbah",
  hurm_helix: "Drift",
  drift: "Drift",
  hurm_hightide: "Glitch",
  glitch: "Glitch",
  hurm_yard: "Piazza",
  piazza: "Piazza",
  poveglia: "The Range",
  range: "The Range",
};

export function getMapName(mapId: string): string {
  if (!mapId) return "Unknown Map";
  if (MAP_NAMES[mapId]) return MAP_NAMES[mapId];

  const lower = mapId.toLowerCase().trim();
  for (const [path, name] of Object.entries(MAP_NAMES)) {
    if (path.toLowerCase() === lower) return name;
  }

  const segs = mapId.split("/").filter(Boolean);
  const lastSeg = segs[segs.length - 1]?.toLowerCase() || "";
  if (MAP_CODENAMES[lastSeg]) return MAP_CODENAMES[lastSeg];

  if (segs.length > 0) {
    const raw = segs[segs.length - 1];
    return raw.charAt(0).toUpperCase() + raw.slice(1);
  }

  return "Unknown Map";
}
