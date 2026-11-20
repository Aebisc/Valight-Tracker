"use client";

import { useState } from "react";
import type { SavedMatch } from "@/lib/types";
import { rankColor, rankIconUrl, getMapName } from "@/lib/constants";
import PlayerTable from "./player-table";

function formatMatchTime(savedAt: number): string {
  if (!savedAt) return "";
  const diffMs = Date.now() - savedAt;
  const diffMins = Math.floor(diffMs / 60000);
  if (diffMins < 60) {
    return diffMins <= 1 ? "Just now" : `${diffMins}m ago`;
  }
  const diffHours = Math.floor(diffMins / 60);
  if (diffHours < 24) {
    return `${diffHours}h ago`;
  }
  const diffDays = Math.floor(diffHours / 24);
  if (diffDays === 1) return "Yesterday";
  if (diffDays < 7) return `${diffDays}d ago`;
  return new Date(savedAt).toLocaleDateString(undefined, { month: "short", day: "numeric" });
}

function agentImg(id: string) {
  return `https://media.valorant-api.com/agents/${id}/displayicon.png`;
}

interface Props {
  matches: SavedMatch[];
}

export default function RecentGamesHistory({ matches }: Props) {
  const [expandedMatchId, setExpandedMatchId] = useState<string | null>(matches[0]?.matchId ?? null);

  if (!matches || matches.length === 0) return null;

  return (
    <div className="a-enter" style={{ width: "100%", display: "flex", flexDirection: "column", gap: 12 }}>
      {/* List Header */}
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", padding: "0 4px" }}>
        <div style={{ display: "flex", alignItems: "baseline", gap: 10 }}>
          <span className="t-title" style={{ fontSize: 18 }}>Recent Matches</span>
          <span className="t-label" style={{ color: "var(--ink-dim)", letterSpacing: 1 }}>
            Last {matches.length} game{matches.length === 1 ? "" : "s"} • Click to view lobby stats
          </span>
        </div>
      </div>

      {/* Match History List */}
      <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        {matches.map((m, idx) => {
          const isExpanded = m.matchId === expandedMatchId;
          const selfPlayer = m.players.find((p) => p.puuid === m.selfPuuid);
          const isWin = m.result === "W";
          const isLoss = m.result === "L";
          const resultColor = isWin ? "var(--up)" : isLoss ? "var(--down)" : "var(--ink-muted)";
          const mapName = getMapName(m.matchInfo.mapId);
          const timeText = formatMatchTime(m.savedAt);
          const hasScore = (m.roundsWon ?? 0) > 0 || (m.roundsLost ?? 0) > 0;

          return (
            <div
              key={m.matchId || idx}
              style={{
                display: "flex",
                flexDirection: "column",
                borderRadius: 12,
                background: isExpanded ? "var(--surface-1)" : "var(--surface-0)",
                border: isExpanded ? "1px solid var(--accent)" : "1px solid var(--border)",
                boxShadow: isExpanded ? "0 0 16px rgba(var(--accent-raw), 0.12)" : "var(--shadow-sm)",
                transition: "all 0.2s cubic-bezier(0.22, 1, 0.36, 1)",
                overflow: "hidden",
              }}
            >
              {/* List Row Item */}
              <button
                type="button"
                onClick={() => setExpandedMatchId((prev) => (prev === m.matchId ? null : m.matchId))}
                aria-expanded={isExpanded}
                style={{
                  width: "100%",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "space-between",
                  padding: "14px 20px",
                  background: "transparent",
                  border: "none",
                  cursor: "pointer",
                  textAlign: "left",
                  gap: 16,
                  flexWrap: "wrap",
                  outline: "none",
                  transition: "background-color 0.15s ease",
                }}
                onMouseEnter={(e) => {
                  if (!isExpanded) e.currentTarget.style.backgroundColor = "var(--surface-1)";
                }}
                onMouseLeave={(e) => {
                  if (!isExpanded) e.currentTarget.style.backgroundColor = "transparent";
                }}
              >
                {/* Left: Result pill, Map Name, Time */}
                <div style={{ display: "flex", alignItems: "center", gap: 14, minWidth: 220 }}>
                  {m.result ? (
                    <span
                      style={{
                        display: "inline-flex",
                        alignItems: "center",
                        gap: 6,
                        fontSize: 12,
                        fontWeight: 700,
                        color: resultColor,
                        padding: "3px 8px",
                        borderRadius: 6,
                        background: isWin
                          ? "rgba(74, 222, 128, 0.12)"
                          : isLoss
                          ? "rgba(248, 113, 113, 0.12)"
                          : "rgba(255, 255, 255, 0.06)",
                        border: `1px solid ${isWin ? "rgba(74, 222, 128, 0.25)" : isLoss ? "rgba(248, 113, 113, 0.25)" : "var(--border)"}`,
                        lineHeight: 1,
                      }}
                    >
                      <span>{m.result}</span>
                      {hasScore && (
                        <span style={{ fontSize: 11, fontWeight: 600, opacity: 0.85, fontVariantNumeric: "tabular-nums" }}>
                          {m.roundsWon}:{m.roundsLost}
                        </span>
                      )}
                    </span>
                  ) : hasScore ? (
                    <span
                      style={{
                        fontSize: 11,
                        fontWeight: 600,
                        color: "var(--ink-dim)",
                        padding: "3px 8px",
                        borderRadius: 6,
                        background: "var(--surface-2)",
                        border: "1px solid var(--border)",
                        fontVariantNumeric: "tabular-nums",
                      }}
                    >
                      {m.roundsWon}:{m.roundsLost}
                    </span>
                  ) : null}

                  <div>
                    <span className="t-body" style={{ fontWeight: 700, fontSize: 15, color: "var(--ink)" }}>
                      {mapName}
                    </span>
                    {timeText && (
                      <span className="t-label" style={{ fontSize: 11, color: "var(--ink-dim)", marginLeft: 8 }}>
                        {timeText}
                      </span>
                    )}
                  </div>
                </div>

                {/* Center: Agent icon & Rank */}
                <div style={{ display: "flex", alignItems: "center", gap: 16 }}>
                  {selfPlayer?.agentId ? (
                    <div style={{ display: "flex", alignItems: "center", gap: 7 }}>
                      {/* eslint-disable-next-line @next/next/no-img-element */}
                      <img
                        src={agentImg(selfPlayer.agentId)}
                        alt={selfPlayer.agentName || "Agent"}
                        width={24}
                        height={24}
                        style={{ borderRadius: 6, border: "1px solid var(--border)" }}
                      />
                      <span className="t-body" style={{ fontSize: 13, fontWeight: 500 }}>
                        {selfPlayer.agentName || "Agent"}
                      </span>
                    </div>
                  ) : null}

                  {selfPlayer && selfPlayer.rank > 0 ? (
                    <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                      {/* eslint-disable-next-line @next/next/no-img-element */}
                      <img
                        src={rankIconUrl(selfPlayer.rank, "small")}
                        alt={selfPlayer.rankName}
                        width={20}
                        height={20}
                      />
                      <span
                        style={{
                          fontSize: 12,
                          fontWeight: 600,
                          color: rankColor(selfPlayer.rankName),
                          whiteSpace: "nowrap",
                        }}
                      >
                        {selfPlayer.rankName}
                      </span>
                    </div>
                  ) : null}
                </div>

                {/* Right: Player count & Chevron */}
                <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
                  <span className="tag" style={{ fontSize: 10, padding: "2px 8px" }}>
                    {m.players.length} Players
                  </span>
                  <span
                    style={{
                      display: "inline-flex",
                      alignItems: "center",
                      gap: 4,
                      fontSize: 12,
                      color: isExpanded ? "var(--accent)" : "var(--ink-muted)",
                      fontWeight: 500,
                    }}
                  >
                    {isExpanded ? "Hide Lobby" : "View Lobby"}
                    <svg
                      width="14"
                      height="14"
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      strokeWidth="2"
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      style={{
                        transform: isExpanded ? "rotate(180deg)" : "rotate(0deg)",
                        transition: "transform 0.24s cubic-bezier(0.22, 1, 0.36, 1)",
                      }}
                      aria-hidden="true"
                    >
                      <polyline points="6 9 12 15 18 9" />
                    </svg>
                  </span>
                </div>
              </button>

              {/* Expanded Match Lobby Table */}
              {isExpanded && (
                <div
                  style={{
                    padding: "8px 16px 20px",
                    borderTop: "1px solid var(--border)",
                    background: "var(--surface-0)",
                  }}
                >
                  <PlayerTable
                    players={m.players}
                    isDeathmatch={m.matchInfo.isDeathmatch}
                    selfPuuid={m.selfPuuid}
                  />
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
