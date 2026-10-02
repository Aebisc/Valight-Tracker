"use client";

import { useEffect, useState, useCallback, useRef } from "react";
import { useToast } from "../components/toast";
import { fetchMatchData } from "@/lib/backend";
import type { Player, MatchInfo, ApiResponse, GameState } from "@/lib/types";

const POLL_INTERVALS: Record<GameState, number> = {
  MENUS: 15000,
  OFFLINE: 15000,
  PREGAME: 5000,
  INGAME: 8000,
  ERROR: 10000,
};
const RETRY_DELAY = 3000;

function getPollingInterval(state: GameState): number {
  return POLL_INTERVALS[state] ?? 15000;
}

export interface MatchPollingResult {
  players: Player[];
  matchInfo: MatchInfo | null;
  gameState: GameState;
  loading: boolean;
  refreshing: boolean;
  error: string;
  lastUpdated: Date | null;
  connected: boolean;
  selfPuuid: string;
  reconnecting: boolean;
  stateStartTime: number | null;
  refreshMatch: () => void;
}

export function useMatchPolling(): MatchPollingResult {
  const [players, setPlayers] = useState<Player[]>([]);
  const [matchInfo, setMatchInfo] = useState<MatchInfo | null>(null);
  const [gameState, setGameState] = useState<GameState>("OFFLINE");
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState("");
  const [lastUpdated, setLastUpdated] = useState<Date | null>(null);
  const [connected, setConnected] = useState(false);
  const [selfPuuid, setSelfPuuid] = useState("");
  const [reconnecting, setReconnecting] = useState(false);
  const [stateStartTime, setStateStartTime] = useState<number | null>(null);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const prevGameState = useRef<GameState>("OFFLINE");
  const failCountRef = useRef<number>(0);
  const retryPendingRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const isFetchingRef = useRef<boolean>(false);
  const { toast } = useToast();

  const fetchMatchRef = useRef<(manual?: boolean) => Promise<void>>(async () => {});

  const resetInterval = useCallback((state: GameState) => {
    if (intervalRef.current) clearInterval(intervalRef.current);
    const ms = getPollingInterval(state);
    intervalRef.current = setInterval(() => fetchMatchRef.current(), ms);
  }, []);

  const fetchMatch = useCallback(async (manual = false) => {
    if (isFetchingRef.current && !manual) return;
    isFetchingRef.current = true;
    if (manual) setRefreshing(true);
    try {
      const data: ApiResponse = await fetchMatchData(manual);
      setError(data.error ?? "");
      setGameState(data.gameState);
      setMatchInfo(data.match ?? null);
      setPlayers(data.players ?? []);
      setSelfPuuid(data.selfPuuid ?? "");
      setConnected(true);
      setLastUpdated(new Date());

      failCountRef.current = 0;
      setReconnecting(false);

      if (data.gameState !== prevGameState.current) {
        resetInterval(data.gameState);

        if (data.gameState === "PREGAME" || data.gameState === "INGAME") {
          setStateStartTime(Date.now());
        } else {
          setStateStartTime(null);
        }
      }

      const wasInMenus = prevGameState.current === "MENUS" || prevGameState.current === "OFFLINE";
      const nowInMatch = data.gameState === "PREGAME" || data.gameState === "INGAME";
      if (wasInMenus && nowInMatch && data.match) {
        toast("Match found — Agent Select", "success");
      }

      if (prevGameState.current === "PREGAME" && data.gameState === "INGAME" && data.match) {
        toast(`Match started — Live on ${data.match.mapName}`, "info");
      }

      if (prevGameState.current === "INGAME" && data.gameState === "MENUS") {
        toast("Match ended", "warning");
      }

      prevGameState.current = data.gameState;
    } catch (e) {
      console.error("[fetchMatch] error:", e);
      failCountRef.current += 1;
      if (failCountRef.current >= 3) {
        setReconnecting(true);
        setError("Reconnecting...");
      } else {
        setError("Failed to connect");
      }
      setConnected(false);
      if (failCountRef.current === 1) {
        toast("Connection lost — retrying...", "error");
      }

      // Always clear running polling interval when in failed state
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
        intervalRef.current = null;
      }

      // Schedule retry if not already pending
      if (!retryPendingRef.current) {
        const delay = failCountRef.current >= 3 ? RETRY_DELAY * 2 : RETRY_DELAY;
        retryPendingRef.current = setTimeout(() => {
          retryPendingRef.current = null;
          fetchMatchRef.current().finally(() => {
            if (failCountRef.current === 0) {
              resetInterval(prevGameState.current);
            }
          });
        }, delay);
      }
    } finally {
      isFetchingRef.current = false;
      setLoading(false);
      setRefreshing(false);
    }
  }, [resetInterval, toast]);

  useEffect(() => {
    fetchMatchRef.current = fetchMatch;
  }, [fetchMatch]);

  const refreshMatch = useCallback(() => {
    if (retryPendingRef.current) {
      clearTimeout(retryPendingRef.current);
      retryPendingRef.current = null;
    }
    if (intervalRef.current) clearInterval(intervalRef.current);
    fetchMatch(true).then(() => {
      resetInterval(prevGameState.current);
    });
  }, [fetchMatch, resetInterval]);

  useEffect(() => {
    fetchMatchRef.current = fetchMatch;
    fetchMatch();
    intervalRef.current = setInterval(() => fetchMatchRef.current(), getPollingInterval("OFFLINE"));
    return () => {
      if (intervalRef.current) clearInterval(intervalRef.current);
      if (retryPendingRef.current) clearTimeout(retryPendingRef.current);
    };
  }, []);

  useEffect(() => {
    const handleVisibilityChange = () => {
      if (document.hidden) {
        if (intervalRef.current) {
          clearInterval(intervalRef.current);
          intervalRef.current = null;
        }
      } else {
        fetchMatchRef.current();
        resetInterval(prevGameState.current);
      }
    };
    document.addEventListener("visibilitychange", handleVisibilityChange);
    return () => document.removeEventListener("visibilitychange", handleVisibilityChange);
  }, [resetInterval]);

  return {
    players,
    matchInfo,
    gameState,
    loading,
    refreshing,
    error,
    lastUpdated,
    connected,
    selfPuuid,
    reconnecting,
    stateStartTime,
    refreshMatch,
  };
}
