export const RANK_MAP: Record<number, string> = {
  0: "Unranked", 1: "Unused 1", 2: "Unused 2",
  3: "Iron 1", 4: "Iron 2", 5: "Iron 3",
  6: "Bronze 1", 7: "Bronze 2", 8: "Bronze 3",
  9: "Silver 1", 10: "Silver 2", 11: "Silver 3",
  12: "Gold 1", 13: "Gold 2", 14: "Gold 3",
  15: "Platinum 1", 16: "Platinum 2", 17: "Platinum 3",
  18: "Diamond 1", 19: "Diamond 2", 20: "Diamond 3",
  21: "Ascendant 1", 22: "Ascendant 2", 23: "Ascendant 3",
  24: "Immortal 1", 25: "Immortal 2", 26: "Immortal 3",
  27: "Radiant",
};

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

export const AGENT_MAP: Record<string, string> = {
  "41fb69c1-4189-7b37-f117-bcaf1e96f1bf": "Astra",
  "5f8d3a7f-467b-97f3-062c-13acf203c006": "Breach",
  "9f0d8ba9-4140-b941-57d3-a7ad57c6b417": "Brimstone",
  "22697a3d-45bf-8dd7-4fec-84a9e28c69d7": "Chamber",
  "1dbf2edd-4729-0984-3115-daa5eed44993": "Clove",
  "117ed9e3-49f3-6512-3ccf-0cada7e3823b": "Cypher",
  "cc8b64c8-4b25-4ff9-6e7f-37b4da43d235": "Deadlock",
  "dade69b4-4f5a-8528-247b-219e5a1facd6": "Fade",
  "e370fa57-4757-3604-3648-499e1f642d3f": "Gekko",
  "95b78ed7-4637-86d9-7e41-71ba8c293152": "Harbor",
  "0e38b510-41a8-5780-5e8f-568b2a4f2d6c": "Iso",
  "add6443a-41bd-e414-f6ad-e58d267f4e95": "Jett",
  "601dbbe7-43ce-be57-2a40-4abd24953621": "KAY/O",
  "1e58de9c-4950-5125-93e9-a0aee9f98746": "Killjoy",
  "bb2a4828-46eb-8cd1-e765-15848195d751": "Neon",
  "8e253930-4c05-31dd-1b6c-968525494517": "Omen",
  "eb93336a-449b-9c1b-0a54-a891f7921d69": "Phoenix",
  "f94c3b30-42be-e959-889c-5aa313dba261": "Raze",
  "a3bfb853-43b2-7238-a4f1-ad90e9e46bcc": "Reyna",
  "569fdd95-4d10-43ab-ca70-79becc718b46": "Sage",
  "6f2a04ca-43e0-be17-7f36-b3908627744d": "Skye",
  "320b2a48-4d9b-a075-30f1-1f93a9b638fa": "Sova",
  "b444168c-4e35-8076-db47-ef9bf368f384": "Tejo",
  "92eeef5d-43b5-1d4a-8d03-b3927a09034b": "Veto",
  "707eab51-4836-f488-046a-cda6bf494859": "Viper",
  "efba5359-4016-a1e5-7626-b1ae76895940": "Vyse",
  "7c8a4701-4de6-9355-b254-e09bc2a34b72": "Miks",
  "df1cb487-4902-002e-5c17-d28e83e78588": "Waylay",
  "7f94d92c-4234-0a36-9646-3a87eb8b5c89": "Yoru",
};

export const GAMEMODE_MAP: Record<string, string> = {
  competitive: "Competitive", unrated: "Unrated", spikerush: "Spike Rush",
  deathmatch: "Deathmatch", swiftplay: "Swiftplay", escalation: "Escalation",
  teamdeathmatch: "Team Deathmatch", premier: "Premier", newmap: "New Map",
  snowball: "Snowball Fight", ggteam: "Escalation", onefa: "Replication",
  hurm: "Team Deathmatch", aros: "All Random One Site", dodgeball: "Knockout",
  skirmish: "Skirmish", gungame: "Escalation", quickbomb: "Spike Rush",
  bomb: "Standard",
  "/game/gamemodes/bomb/bombgamemode": "Standard",
  "/game/gamemodes/deathmatch/deathmatchgamemode": "Deathmatch",
  "/game/gamemodes/quickbomb/quickbombgamemode": "Spike Rush",
  "/game/gamemodes/spikerush/spikerushgamemode": "Spike Rush",
  "/game/gamemodes/ggteam/ggteamgamemode": "Escalation",
  "/game/gamemodes/gungame/gungameteamsgamemode": "Escalation",
  "/game/gamemodes/onefa/onefagamemode": "Replication",
  "/game/gamemodes/oneforall/oneforall_gamemode": "Replication",
  "/game/gamemodes/swiftplay/swiftplaygamemode": "Swiftplay",
  "/game/gamemodes/hurm/hurmgamemode": "Team Deathmatch",
  "/game/gamemodes/aros/aros_gamemode": "All Random One Site",
  "/game/gamemodes/dodgeball/dodgeball_gamemode": "Knockout",
  "/game/gamemodes/skirmish/skirmishgamemode": "Skirmish",
  "/game/gamemodes/snowballfight/snowballfightgamemode": "Snowball Fight",
};

export const AGENT_ROLE_MAP: Record<string, string> = {
  Jett: "Duelist", Reyna: "Duelist", Phoenix: "Duelist", Raze: "Duelist", Yoru: "Duelist", Neon: "Duelist", Iso: "Duelist",
  Sova: "Initiator", Breach: "Initiator", Skye: "Initiator", "KAY/O": "Initiator", Fade: "Initiator", Gekko: "Initiator",
  Brimstone: "Controller", Omen: "Controller", Astra: "Controller", Viper: "Controller", Harbor: "Controller", Clove: "Controller",
  Sage: "Sentinel", Cypher: "Sentinel", Killjoy: "Sentinel", Chamber: "Sentinel", Deadlock: "Sentinel", Vyse: "Sentinel",
  Miks: "Controller", Tejo: "Initiator", Veto: "Sentinel", Waylay: "Duelist",
};

export const DEATHMATCH_MODES = new Set(["deathmatch"]);

export const MAP_MAP: Record<string, string> = {
  "/game/maps/ascent/ascent": "Ascent",
  "/game/maps/bonsai/bonsai": "Split",
  "/game/maps/canyon/canyon": "Fracture",
  "/game/maps/duality/duality": "Bind",
  "/game/maps/foxtrot/foxtrot": "Breeze",
  "/game/maps/infinity/infinity": "Abyss",
  "/game/maps/jam/jam": "Lotus",
  "/game/maps/juliett/juliett": "Sunset",
  "/game/maps/pitt/pitt": "Pearl",
  "/game/maps/port/port": "Icebox",
  "/game/maps/triad/triad": "Haven",
  "/game/maps/rook/rook": "Corrode",
  "/game/maps/hurm/hurm_alley/hurm_alley": "District",
  "/game/maps/hurm/hurm_bowl/hurm_bowl": "Kasbah",
  "/game/maps/hurm/hurm_helix/hurm_helix": "Drift",
  "/game/maps/hurm/hurm_hightide/hurm_hightide": "Glitch",
  "/game/maps/hurm/hurm_yard/hurm_yard": "Piazza",
};
