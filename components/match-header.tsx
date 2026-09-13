"use client";

import { useState, useEffect } from "react";
import type { MatchInfo } from "@/lib/types";

function useElapsedTimer(startTime?: number): string | null {
  const [elapsed, setElapsed] = useState<number>(0);

  useEffect(() => {
    if (!startTime) {
      setElapsed(0);
      return;
    }
    setElapsed(Math.floor((Date.now() - startTime) / 1000));

    const id = setInterval(() => {
      setElapsed(Math.floor((Date.now() - startTime) / 1000));
    }, 1000);

    return () => clearInterval(id);
  }, [startTime]);

  if (!startTime) return null;
  const mins = Math.floor(elapsed / 60);
  const secs = elapsed % 60;
  return `${mins}:${secs.toString().padStart(2, "0")}`;
}

interface MatchHeaderProps {
  matchInfo: MatchInfo;
  gameState: string;
  onRefresh: () => void;
  refreshing: boolean;
  stateStartTime?: number;
}

const tagTransition = "all 0.2s ease";

export default function MatchHeader({ gameState, onRefresh, refreshing, stateStartTime }: MatchHeaderProps) {
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