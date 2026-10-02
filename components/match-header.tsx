"use client";

import { useState, useEffect } from "react";
import type { MatchInfo, GameState } from "@/lib/types";

function useElapsedTimer(startTime?: number): string | null {
  const [elapsed, setElapsed] = useState<number>(() =>
    startTime ? Math.max(0, Math.floor((Date.now() - startTime) / 1000)) : 0
  );

  useEffect(() => {
    if (!startTime) {
      setElapsed(0);
      return;
    }
    setElapsed(Math.max(0, Math.floor((Date.now() - startTime) / 1000)));

    let id: ReturnType<typeof setInterval> | null = null;
    const start = () => {
      if (id) clearInterval(id);
      id = setInterval(() => {
        if (!document.hidden) {
          setElapsed(Math.max(0, Math.floor((Date.now() - startTime) / 1000)));
        }
      }, 1000);
    };

    const handleVisibility = () => {
      if (!document.hidden) {
        setElapsed(Math.max(0, Math.floor((Date.now() - startTime) / 1000)));
      }
    };

    start();
    document.addEventListener("visibilitychange", handleVisibility);

    return () => {
      if (id) clearInterval(id);
      document.removeEventListener("visibilitychange", handleVisibility);
    };
  }, [startTime]);

  if (!startTime) return null;
  const mins = Math.floor(elapsed / 60);
  const secs = elapsed % 60;
  return `${mins}:${secs.toString().padStart(2, "0")}`;
}

interface MatchHeaderProps {
  matchInfo: MatchInfo;
  gameState: GameState;
  onRefresh: () => void;
  refreshing: boolean;
  stateStartTime?: number;
}

const tagTransition = "all 0.2s ease";

export default function MatchHeader({ matchInfo, gameState, onRefresh, refreshing, stateStartTime }: MatchHeaderProps) {
  const live = gameState === "INGAME";
  const showTimer = gameState === "PREGAME" || gameState === "INGAME";
  const timerDisplay = useElapsedTimer(showTimer ? stateStartTime : undefined);
  return (
    <div
      className="a-enter"
      style={{
        position: "relative",
        display: "flex",
        flexDirection: "column",
        gap: 16,
        overflow: "hidden",
        background: "linear-gradient(to bottom, var(--surface-1), var(--surface-0))",
        borderRadius: 12,
        border: "1px solid var(--border)",
        boxShadow: "0 0 0 1px var(--surface-1), var(--shadow-md)",
        padding: "20px 24px",
      }}
    >
      <div style={{
        position: "absolute", top: 0, left: 0, right: 0, height: 2, zIndex: 2,
        background: "linear-gradient(90deg, transparent, rgba(var(--accent-raw), 0.5), transparent)",
      }} />
      <div
        style={{
          position: "absolute",
          top: 0,
          left: 0,
          right: 0,
          height: 1,
          background: "linear-gradient(90deg, transparent, var(--border-hover) 30%, var(--border-hover) 70%, transparent)",
          zIndex: 2,
        }}
      />
      <div style={{
        position: "absolute", inset: 0, pointerEvents: "none", zIndex: 1,
        background: "repeating-linear-gradient(0deg, transparent, transparent 2px, rgba(0,0,0,0.03) 2px, rgba(0,0,0,0.03) 4px)",
        mixBlendMode: "multiply",
      }} />

      <div
        style={{
          position: "absolute",
          inset: 0,
          background: "radial-gradient(ellipse 80% 100% at 0% 0%, var(--accent-soft) 0%, transparent 60%)",
          opacity: 0.85,
          borderRadius: "var(--radius-md)",
          pointerEvents: "none",
          zIndex: 1,
        }}
      />

      <div
        style={{
          position: "relative",
          zIndex: 2,
          display: "flex",
          alignItems: "center",
          gap: 10,
          flexWrap: "wrap",
        }}
      >
        <span
          className="tag"
          style={{
            transition: tagTransition,
            borderColor: live ? "rgba(74, 222, 128, 0.3)" : "rgba(251, 191, 36, 0.25)",
            ...(live
              ? { boxShadow: "0 0 16px rgba(74, 222, 128, 0.2), 0 0 6px rgba(74, 222, 128, 0.1), inset 0 0 8px rgba(74, 222, 128, 0.06), inset 0 1px 0 rgba(74, 222, 128, 0.08)" }
              : { boxShadow: "inset 0 1px 0 var(--surface-1), inset 0 0 4px var(--surface-3)" }),
          }}
        >
          <span
            className="a-pulse"
            style={{
              width: 6,
              height: 6,
              borderRadius: "50%",
              background: live ? "var(--up)" : "var(--warn)",
              transition: tagTransition,
              ...(live ? { boxShadow: "0 0 6px var(--up), 0 0 12px rgba(74, 222, 128, 0.3)" } : {}),
              animationDuration: "2.4s",
              animationTimingFunction: "ease-in-out",
            }}
          />
          <span style={{ color: live ? "var(--up)" : "var(--warn)", transition: "color 0.2s ease" }}>
            {live ? "Live" : "Agent Select"}
          </span>
          {timerDisplay && (
            <span style={{
              fontFamily: "var(--font-mono, monospace)",
              fontSize: 11,
              color: "var(--ink-dim)",
              opacity: 0.7,
              marginLeft: 2,
              fontVariantNumeric: "tabular-nums",
              padding: "2px 10px",
              borderRadius: "var(--radius-sm)",
              background: "rgba(var(--accent-raw), 0.06)",
              border: "1px solid rgba(var(--accent-raw), 0.12)",
            }}>
              {timerDisplay}
            </span>
          )}
        </span>

        {gameState === "PREGAME" && matchInfo?.startingSide && (
          <span
            className="tag"
            style={{
              borderColor: matchInfo.startingSide === "attack"
                ? "rgba(248, 113, 113, 0.25)"
                : "rgba(96, 165, 250, 0.25)",
              color: matchInfo.startingSide === "attack" ? "var(--down)" : "var(--info)",
              background: matchInfo.startingSide === "attack"
                ? "rgba(248, 113, 113, 0.08)"
                : "rgba(96, 165, 250, 0.08)",
              gap: 5,
              transition: "color 0.2s ease, border-color 0.2s ease, background-color 0.2s ease",
            }}
          >
            {matchInfo.startingSide === "attack" ? (
              <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                <circle cx="12" cy="12" r="10" />
                <line x1="22" y1="12" x2="18" y2="12" />
                <line x1="6" y1="12" x2="2" y2="12" />
                <line x1="12" y1="6" x2="12" y2="2" />
                <line x1="12" y1="22" x2="12" y2="18" />
              </svg>
            ) : (
              <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
              </svg>
            )}
            <span>
              {matchInfo.startingSide === "attack" ? "Attacking First" : "Defending First"}
            </span>
          </span>
        )}

        <button
          onClick={onRefresh}
          disabled={refreshing}
          className="btn-ghost"
          aria-label={refreshing ? "Refreshing match data" : "Refresh match data"}
          style={{
            transition: "all 0.2s ease, transform 0.15s cubic-bezier(0.22, 1, 0.36, 1)",
          }}
          onMouseDown={(e) => {
            if (!refreshing) e.currentTarget.style.transform = "scale(0.93)";
          }}
          onMouseUp={(e) => {
            e.currentTarget.style.transform = "scale(1)";
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.transform = "scale(1)";
          }}
        >
          <svg
            width="10"
            height="10"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
            className={refreshing ? "a-spin" : ""}
          >
            <path d="M21.5 2v6h-6M2.5 22v-6h6M2 11.5a10 10 0 0 1 18.8-4.3M22 12.5a10 10 0 0 1-18.8 4.3" />
          </svg>
          Refresh
        </button>
      </div>
    </div>
  );
}