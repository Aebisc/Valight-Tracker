import type { Player, MatchInfo } from "./types";

/**
 * Deeply compares two Player objects for equality.
 * Uses key union iteration so newly added optional or required fields
 * are automatically compared without requiring manual list maintenance.
 */
export function isPlayerEqual(a: Player, b: Player): boolean {
  if (a === b) return true;
  if (!a || !b) return false;

  const keys = new Set([...Object.keys(a), ...Object.keys(b)]);
  const recA = a as unknown as Record<string, unknown>;
  const recB = b as unknown as Record<string, unknown>;
  for (const k of keys) {
    const av = recA[k];
    const bv = recB[k];

    if (Array.isArray(av) && Array.isArray(bv)) {
      if (av.length !== bv.length) return false;
      for (let i = 0; i < av.length; i++) {
        if (!Object.is(av[i], bv[i])) return false;
      }
      continue;
    }

    if (!Object.is(av, bv)) return false;
  }
  return true;
}

/**
 * Applies structural sharing to an array of players:
 * - If a player's data is unchanged, reuses the previous Player object reference.
 * - If all players are unchanged and order is preserved, reuses the previous Array reference.
 * - If any player changed or order changed, returns a new Array reference with incoming order.
 */
export function sharePlayers(prevPlayers: Player[], nextPlayers: Player[]): Player[] {
  if (prevPlayers.length === 0 && nextPlayers.length === 0) return prevPlayers;

  const prevMap = new Map(prevPlayers.map((p) => [p.puuid, p]));
  let anyChanged = false;

  const shared = nextPlayers.map((newP) => {
    const oldP = prevMap.get(newP.puuid);
    if (oldP && isPlayerEqual(oldP, newP)) {
      return oldP;
    }
    anyChanged = true;
    return newP;
  });

  if (!anyChanged && prevPlayers.length === nextPlayers.length) {
    let sameOrder = true;
    for (let i = 0; i < prevPlayers.length; i++) {
      if (prevPlayers[i] !== shared[i]) {
        sameOrder = false;
        break;
      }
    }
    if (sameOrder) {
      return prevPlayers;
    }
  }

  return shared;
}

/**
 * Compares two MatchInfo objects for field-level equality.
 */
export function isMatchEqual(a: MatchInfo | null, b: MatchInfo | null): boolean {
  if (a === b) return true;
  if (!a || !b) return false;

  const keys = new Set([...Object.keys(a), ...Object.keys(b)]);
  const recA = a as unknown as Record<string, unknown>;
  const recB = b as unknown as Record<string, unknown>;
  for (const k of keys) {
    const av = recA[k];
    const bv = recB[k];
    if (!Object.is(av, bv)) return false;
  }
  return true;
}

/**
 * Preserves the previous MatchInfo reference if all fields are identical.
 */
export function shareMatch(prev: MatchInfo | null, next: MatchInfo | null): MatchInfo | null {
  if (prev && next && isMatchEqual(prev, next)) {
    return prev;
  }
  return next;
}
