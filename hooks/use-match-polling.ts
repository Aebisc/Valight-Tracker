"use client";

import { useEffect, useState, useCallback, useRef } from "react";
import { useToast } from "../components/toast";
import type { Player, MatchInfo, ApiResponse } from "@/lib/types";

const POLL_INTERVALS: Record<string, number> = {
  MENUS: 15000,
  OFFLINE: 15000,
  PREGAME: 5000,
  INGAME: 8000,
};
const RETRY_DELAY = 3000;

function getPollingInterval(state: string): number {
  return POLL_INTERVALS[state] ?? 15000;
}

export interface MatchPollingResult {
  players: Player[];
  matchInfo: MatchInfo | null;
  gameState: string;
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
  const [gameState, setGameState] = useState<string>("OFFLINE");
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState("");
  const [lastUpdated, setLastUpdated] = useState<Date | null>(null);
  const [connected, setConnected] = useState(false);
  const [selfPuuid, setSelfPuuid] = useState("");
  const [reconnecting, setReconnecting] = useState(false);
  const [stateStartTime, setStateStartTime] = useState<number | null>(null);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const prevGameState = useRef<string>("OFFLINE");
  const failCountRef = useRef<number>(0);
  const retryPendingRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const { toast } = useToast();

  const fetchMatchRef = useRef<(manual?: boolean) => Promise<void>>(async () => {});

  const resetInterval = useCallback((state: string) => {
    if (intervalRef.current) clearInterval(intervalRef.current);
    const ms = getPollingInterval(state);
    intervalRef.current = setInterval(() => fetchMatchRef.current(), ms);
  }, []);

  const fetchMatch = useCallback(async (manual = false) => {
    if (manual) setRefreshing(true);
    try {
      const res = await fetch(manual ? "/api/match?force=1" : "/api/match");
      const data: ApiResponse = await res.json();
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
          if (prevGameState.current !== "PREGAME" && prevGameState.current !== "INGAME") {
            setStateStartTime(Date.now());
          }
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
      if (failCountRef.current === 1 && !retryPendingRef.current) {
        if (intervalRef.current) clearInterval(intervalRef.current);
        retryPendingRef.current = setTimeout(() => {
          retryPendingRef.current = null;
          fetchMatchRef.current().finally(() => {
            resetInterval(prevGameState.current);
          });
        }, RETRY_DELAY);
      }
    } finally {
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
