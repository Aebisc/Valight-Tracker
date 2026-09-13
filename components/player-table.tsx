"use client";

import { useState, useRef, useEffect, useCallback, useMemo } from "react";
import type { Player } from "@/lib/types";
import { rankColor, RANK_NAMES_SHORT, rankIconUrl } from "@/lib/constants";


interface Props {
  players: Player[];
  isDeathmatch?: boolean;
  selfPuuid?: string;
}

function agentImg(id: string) {
  return `https://media.valorant-api.com/agents/${id}/displayicon.png`;
}

const RANK_RANGES: Record<string, [number, number]> = {
  Iron: [3, 5], Bronze: [6, 8], Silver: [9, 11], Gold: [12, 14],
  Platinum: [15, 17], Diamond: [18, 20], Ascendant: [21, 23],
  Immortal: [24, 26], Radiant: [27, 27],
};

function avgTeamRank(players: Player[]): { avg: number; tier: number; name: string; color: string } | null {
  const ranked = players.filter((p) => p.rank >= 3);
  if (ranked.length === 0) return null;
  const avg = ranked.reduce((s, p) => s + p.rank + p.rr / 100, 0) / ranked.length;
  const tier = Math.max(3, Math.min(27, Math.round(avg)));
  const name = RANK_NAMES_SHORT[tier] ?? "??";
  const tintKey = Object.keys(RANK_RANGES).find((k) => {
    const r = RANK_RANGES[k];
    return tier >= r[0] && tier <= r[1];
  }) ?? "";
  return { avg, tier, name, color: rankColor(tintKey) };
}

function ExpandCollapse({ open, children }: { open: boolean; children: React.ReactNode }) {
  const contentRef = useRef<HTMLDivElement>(null);
  const [height, setHeight] = useState(0);

  const measure = useCallback(() => {
    if (contentRef.current) {
      setHeight(contentRef.current.scrollHeight);
    }
  }, []);

  useEffect(() => {
    measure();
    window.addEventListener("resize", measure);
    return () => window.removeEventListener("resize", measure);
  }, [open, measure]);

  return (
    <div
      style={{
        maxHeight: open ? height : 0,
        overflow: "hidden",
        transition: "max-height 0.32s cubic-bezier(0.22, 1, 0.36, 1), opacity 0.24s ease",
        opacity: open ? 1 : 0,
      }}
    >
      <div ref={contentRef}>{children}</div>
    </div>
  );
}

type BadgeType = "mvp" | "top" | "tf";

function Badge({ type }: { type: BadgeType }) {
  const config = {
    mvp: { label: "MVP", bg: "rgba(236, 194, 54, 0.15)", color: "#ecc236", border: "rgba(236, 194, 54, 0.3)" },
    top: { label: "TOP", bg: "rgba(59, 180, 160, 0.12)", color: "#3bb4a0", border: "rgba(59, 180, 160, 0.25)" },
    tf: { label: "TF", bg: "rgba(255, 255, 255, 0.06)", color: "var(--ink-faint)", border: "rgba(255, 255, 255, 0.1)" },
  }[type];

  return (
    <span style={{
      display: "inline-flex",
      alignItems: "center",
      gap: 3,
      padding: "1px 5px",
      borderRadius: 3,
      fontSize: 8,
      fontWeight: 700,
      letterSpacing: "0.6px",
      lineHeight: 1,
      background: config.bg,
      color: config.color,
      border: `1px solid ${config.border}`,
      boxShadow: type === "mvp" ? "0 0 8px rgba(251, 191, 36, 0.2)" : "none",
      transition: "all 0.2s ease",
      whiteSpace: "nowrap",
    }}>
      {type === "tf" && (
        <svg width="7" height="7" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round">
          <circle cx="12" cy="12" r="10" />
          <line x1="12" y1="2" x2="12" y2="6" />
          <line x1="12" y1="18" x2="12" y2="22" />
          <line x1="2" y1="12" x2="6" y2="12" />
          <line x1="18" y1="12" x2="22" y2="12" />
        </svg>
      )}
      {config.label}
    </span>
  );
}

const PARTY_PALETTES = [
  { bg: "rgba(167, 139, 250, 0.14)", color: "#c4b5fd", border: "rgba(167, 139, 250, 0.3)" }, // Violet
  { bg: "rgba(34, 211, 238, 0.14)", color: "#67e8f9", border: "rgba(34, 211, 238, 0.3)" }, // Cyan
  { bg: "rgba(251, 146, 60, 0.14)", color: "#fdba74", border: "rgba(251, 146, 60, 0.3)" }, // Orange
  { bg: "rgba(52, 211, 153, 0.14)", color: "#6ee7b7", border: "rgba(52, 211, 153, 0.3)" }, // Emerald
  { bg: "rgba(244, 114, 182, 0.14)", color: "#f472b6", border: "rgba(244, 114, 182, 0.3)" }, // Pink
];

function PartyBadge({ partyNumber, partySize }: { partyNumber: number; partySize?: number }) {
  const palette = PARTY_PALETTES[(partyNumber - 1) % PARTY_PALETTES.length];

  return (
    <span
      title={`Queued together in Party ${partyNumber}${partySize && partySize > 1 ? ` (${partySize}-stack)` : ""}`}
      style={{
        display: "inline-flex",
        alignItems: "center",
        gap: 3.5,
        padding: "1px 5px",
        borderRadius: 3,
        fontSize: 8,
        fontWeight: 700,
        letterSpacing: "0.5px",
        lineHeight: 1,
        background: palette.bg,
        color: palette.color,
        border: `1px solid ${palette.border}`,
        whiteSpace: "nowrap",
        userSelect: "none",
      }}
    >
      <svg
        width="8"
        height="8"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" />
        <circle cx="9" cy="7" r="4" />
        <path d="M23 21v-2a4 4 0 0 0-3-3.87" />
        <path d="M16 3.13a4 4 0 0 1 0 7.75" />
      </svg>
      Party {partyNumber}
    </span>
  );
}

function Row({ p, self, i, expanded, onToggle, badges }: {
  p: Player; self: boolean; i: number;
  expanded: boolean; onToggle: () => void;
  badges?: BadgeType[];
}) {
  const rc = rankColor(p.rankName);
  const pc = rankColor(p.peakRankName);
  const delay = i < 10 ? `a-d${i + 1}` : "a-enter";
  const isMvp = badges?.includes("mvp");

  return (
    <div>
      <div
        className={`player-row ${delay}`}
        data-self={self || undefined}
        role="button"
        tabIndex={0}
        aria-expanded={expanded}
        onClick={onToggle}
        onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); onToggle(); } }}
        style={{
          cursor: "pointer",
          userSelect: "none",
          ...(self && !isMvp ? {
            borderLeft: "2px solid rgba(var(--accent-raw, 99, 102, 241), 0.35)",
            background: "rgba(var(--accent-raw, 99, 102, 241), 0.03)",
            boxShadow: "inset 2px 0 12px rgba(var(--accent-raw, 99, 102, 241), 0.06), 0 0 0 0 transparent",
          } : {}),
          ...(isMvp ? {
            borderLeft: "2px solid rgba(236, 194, 54, 0.5)",
            background: "rgba(236, 194, 54, 0.04)",
          } : {}),
        }}
      >
        <div
          className="agent-icon agent-icon-hover"
          style={{
            "--team-glow": p.teamId === "Red" ? "rgba(239, 68, 68, 0.35)" : "rgba(59, 130, 246, 0.35)",
            transition: "border-color 0.2s ease, box-shadow 0.2s ease",
          } as React.CSSProperties}
        >
          {p.agentId ? (
            <img src={agentImg(p.agentId)} alt={p.agentName} style={{ width: "100%", height: "100%", objectFit: "cover", transition: "transform 0.2s ease" }} loading="lazy" />
          ) : (
            <div style={{ width: "100%", height: "100%", display: "flex", alignItems: "center", justifyContent: "center" }} className="t-label">?</div>
          )}
        </div>

        <div style={{ minWidth: 0, flex: 1 }}>
          <div style={{ display: "flex", alignItems: "center", gap: 6, flexWrap: "wrap" }}>
            {p.name && p.tag ? (
              <a
                href={`https://tracker.gg/valorant/profile/riot/${encodeURIComponent(p.name)}%23${encodeURIComponent(p.tag)}/overview`}
                target="_blank"
                rel="noopener noreferrer"
                onClick={(e) => e.stopPropagation()}
                className="tracker-link"
                style={{ display: "inline-flex", alignItems: "center", gap: 4, textDecoration: "none", color: "inherit", transition: "color 0.2s ease" }}
              >
                <span className="t-title" style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", maxWidth: 160 }}>
                  {p.name}
                </span>
                <svg className="tracker-link-icon" width="8" height="8" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" style={{ opacity: 0, flexShrink: 0, transition: "opacity 0.2s ease, transform 0.2s ease" }}>
                  <path d="M18 13v6a2 2 0 01-2 2H5a2 2 0 01-2-2V8a2 2 0 012-2h6M15 3h6v6M10 14L21 3" />
                </svg>
              </a>
            ) : (
              <span className="t-title" style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", maxWidth: 160 }}>
                {p.name || "Unknown"}
              </span>
            )}
            {p.tag && <span className="t-label" style={{ color: "var(--ink-dim)", transition: "color 0.2s ease" }}>#{p.tag}</span>}
            {self && <span className="you-badge" style={{ transition: "background 0.2s ease, border-color 0.2s ease" }}>you</span>}
            {badges && badges.map((b) => <Badge key={b} type={b} />)}
            {p.partyNumber && <PartyBadge partyNumber={p.partyNumber} partySize={p.partySize} />}
          </div>
          <div style={{ display: "flex", alignItems: "center", gap: 6, marginTop: 4 }}>
            <span className="t-label">{p.agentName}</span>
            <span className="dot" />
            <span className="t-label" style={{ fontVariantNumeric: "tabular-nums" }}>LVL {p.accountLevel}</span>
          </div>
        </div>

        <div style={{ flexShrink: 0, display: "flex", alignItems: "center", gap: 8 }}>
          <img
            src={rankIconUrl(p.rank)}
            alt={p.rankName}
            width={28}
            height={28}
            style={{
              flexShrink: 0,
              objectFit: "contain",
              opacity: p.rank >= 3 && !p.isCurrentActRank ? 0.6 : 1,
              filter: "drop-shadow(0 0 4px rgba(0, 0, 0, 0.25))",
            }}
            loading="lazy"
          />
          <div style={{ textAlign: "right", minWidth: 60 }}>
            <div style={{ display: "flex", alignItems: "center", justifyContent: "flex-end", gap: 4 }}>
              <span className="t-title" style={{ color: rc, fontSize: 13, opacity: p.rank >= 3 && !p.isCurrentActRank ? 0.55 : 1, transition: "color 0.2s ease, opacity 0.2s ease" }}>{p.rankName}</span>
              {p.rank >= 3 && !p.isCurrentActRank && (
                <span className="t-micro" style={{ color: "var(--warn)", fontSize: 8, letterSpacing: "0.5px" }}>PREV</span>
              )}
            </div>
            <div className="t-mono" style={{ marginTop: 2 }}>{p.rr} RR</div>
          </div>
        </div>

        <div className="hidden md:flex" style={{ flexShrink: 0, alignItems: "center", gap: 6 }}>
          {p.peakRank > 0 && (
            <img
              src={rankIconUrl(p.peakRank)}
              alt={p.peakRankName}
              width={20}
              height={20}
              style={{
                flexShrink: 0,
                objectFit: "contain",
                filter: "drop-shadow(0 0 3px rgba(0, 0, 0, 0.2))",
              }}
              loading="lazy"
            />
          )}
          <div style={{ textAlign: "right", minWidth: 48 }}>
            <div className="t-micro" style={{ color: "var(--ink-dim)" }}>Peak</div>
            <div className="t-body" style={{ color: pc, marginTop: 2, fontSize: 11, transition: "color 0.2s ease" }}>{p.peakRankName}</div>
          </div>
        </div>

        <div className="hidden lg:flex" style={{ alignItems: "center", gap: 14, flexShrink: 0 }}>
          <StatCell label="ACS" val={p.acs > 0 ? `${p.acs}` : "-"} warn={p.acs >= 250} />
          <StatCell label="HS%" val={p.headshotPercent > 0 ? `${p.headshotPercent.toFixed(1)}` : "-"} warn={p.headshotPercent >= 30} />
          <StatCell label="K/D" val={p.kd > 0 ? p.kd.toFixed(2) : "-"} warn={p.kd >= 1.5} />
          <StatCell label="WR" val={p.currentSeasonGames > 0 ? `${p.winrate.toFixed(0)}%` : "-"} warn={p.winrate >= 55} />
        </div>

        <svg
          width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="var(--ink-dim)"
          strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"
          style={{
            flexShrink: 0,
            transition: "transform 0.28s cubic-bezier(0.22, 1, 0.36, 1), stroke 0.2s ease",
            transform: expanded ? "rotate(180deg)" : "rotate(0deg)",
          }}
        >
          <polyline points="6 9 12 15 18 9" />
        </svg>
      </div>

      <ExpandCollapse open={expanded}>
        <ExpandedRow p={p} />
      </ExpandCollapse>
    </div>
  );
}

function ExpandedRow({ p }: {
  p: Player;
}) {
  const totalShots = p.headshots + p.bodyshots + p.legshots;
  const hsPct = totalShots > 0 ? (p.headshots / totalShots * 100) : 0;
  const bsPct = totalShots > 0 ? (p.bodyshots / totalShots * 100) : 0;
  const lsPct = totalShots > 0 ? (p.legshots / totalShots * 100) : 0;

  const detailRowStyle: React.CSSProperties = {
    display: "flex",
    justifyContent: "space-between",
    alignItems: "baseline",
    padding: "3px 0",
  };

  return (
    <div className="expanded-panel" style={{
      background: "rgba(0, 0, 0, 0.12)",
      boxShadow: "inset 0 1px 4px rgba(0, 0, 0, 0.3)",
    }}>
      <div className="expanded-grid">
        <div className="expanded-section">
          <div className="t-micro" style={{ color: "var(--ink-dim)", marginBottom: 8 }}>Last Match</div>
          <div style={{ display: "flex", gap: 14, alignItems: "baseline" }}>
            <div>
              <span className="t-title" style={{ fontSize: 20, fontVariantNumeric: "tabular-nums", letterSpacing: "-0.02em" }}>
                {p.lastMatchKills}<span className="t-label" style={{ opacity: 0.35, margin: "0 1px" }}>/</span>{p.lastMatchDeaths}<span className="t-label" style={{ opacity: 0.35, margin: "0 1px" }}>/</span>{p.lastMatchAssists}
              </span>
              <div className="t-micro" style={{ color: "var(--ink-dim)", marginTop: 3 }}>K / D / A</div>
            </div>
            <div style={{ width: 1, height: 28, background: "var(--border)", flexShrink: 0 }} />
            <div>
              <div className="t-title" style={{ fontSize: 16, color: p.lastMatchKD >= 1 ? "var(--up)" : "var(--down)", fontVariantNumeric: "tabular-nums", transition: "color 0.2s ease" }}>
                {p.lastMatchKD > 0 ? p.lastMatchKD.toFixed(2) : "-"}
              </div>
              <div className="t-micro" style={{ color: "var(--ink-dim)", marginTop: 3 }}>K/D</div>
            </div>
          </div>
          {p.recentResults && p.recentResults.length > 0 && (
            <div style={{ display: "flex", gap: 5, marginTop: 10 }}>
              {p.recentResults.map((r, i) => (
                <div
                  key={i}
                  title={r === "W" ? "Win" : r === "L" ? "Loss" : "Draw"}
                  style={{
                    width: 22,
                    height: 22,
                    borderRadius: 5,
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "center",
                    fontSize: 11,
                    fontWeight: 700,
                    fontVariantNumeric: "tabular-nums",
                    color: r === "W" ? "var(--up)" : r === "L" ? "var(--down)" : "var(--warn)",
                    background: r === "W" ? "color-mix(in srgb, var(--up) 16%, transparent)" : r === "L" ? "color-mix(in srgb, var(--down) 16%, transparent)" : "color-mix(in srgb, var(--warn) 16%, transparent)",
                  }}
                >
                  {r}
                </div>
              ))}
            </div>
          )}
        </div>

        {totalShots > 0 && (
          <div className="expanded-section">
            <div className="t-micro" style={{ color: "var(--ink-dim)", marginBottom: 8 }}>Shot Distribution</div>
            <div style={{ display: "flex", flexDirection: "column", gap: 5 }}>
              <ShotBar label="Head" pct={hsPct} count={p.headshots} color="var(--up)" />
              <ShotBar label="Body" pct={bsPct} count={p.bodyshots} color="var(--info)" />
              <ShotBar label="Legs" pct={lsPct} count={p.legshots} color="var(--warn)" />
            </div>
          </div>
        )}

        <div className="expanded-section">
          <div className="t-micro" style={{ color: "var(--ink-dim)", marginBottom: 8 }}>Details</div>
          <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
            <div style={detailRowStyle}>
              <span className="t-label">ACS</span>
              <span className="t-body" style={{ fontVariantNumeric: "tabular-nums", color: p.acs >= 250 ? "var(--up)" : undefined, transition: "color 0.2s ease" }}>{p.acs > 0 ? p.acs : "-"}</span>
            </div>
            <div style={detailRowStyle}>
              <span className="t-label">ADR</span>
              <span className="t-body" style={{ fontVariantNumeric: "tabular-nums" }}>{p.adr > 0 ? p.adr.toFixed(1) : "-"}</span>
            </div>
            <div style={detailRowStyle}>
              <span className="t-label">Winrate</span>
              <span className="t-body" style={{ fontVariantNumeric: "tabular-nums" }}>{p.currentSeasonGames > 0 ? `${p.winrate.toFixed(1)}%` : "-"}</span>
            </div>
            <div style={detailRowStyle}>
              <span className="t-label">Act Record</span>
              <span className="t-body" style={{ fontVariantNumeric: "tabular-nums" }}>
                {p.currentSeasonGames > 0
                  ? <>
                      <span style={{ color: "var(--up)", transition: "color 0.2s ease" }}>{p.currentSeasonWins}W</span>
                      {" / "}
                      <span style={{ color: "var(--down)", transition: "color 0.2s ease" }}>{Math.max(0, p.currentSeasonGames - p.currentSeasonWins)}L</span>
                    </>
                  : <span style={{ color: "var(--ink-dim)" }}>No games</span>
                }
              </span>
            </div>
            <div style={detailRowStyle}>
              <span className="t-label">Level</span>
              <span className="t-body" style={{ fontVariantNumeric: "tabular-nums" }}>{p.accountLevel}</span>
            </div>
            <div style={detailRowStyle}>
              <span className="t-label">Agent</span>
              <span className="t-body">{p.agentName}</span>
            </div>
            {p.partyNumber && (
              <div style={detailRowStyle}>
                <span className="t-label">Party</span>
                <span className="t-body" style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
                  <PartyBadge partyNumber={p.partyNumber} partySize={p.partySize} />
                  {p.partySize && p.partySize > 1 && (
                    <span className="t-micro" style={{ color: "var(--ink-dim)" }}>
                      ({p.partySize}-stack)
                    </span>
                  )}
                </span>
              </div>
            )}
          </div>
        </div>
      </div>

    </div>
  );
}

function ShotBar({ label, pct, count, color }: { label: string; pct: number; count: number; color: string }) {
  return (
    <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
      <span className="t-micro" style={{ width: 30, textAlign: "right", flexShrink: 0 }}>{label}</span>
      <div style={{ flex: 1, height: 4, borderRadius: "var(--radius-xs)", background: "var(--surface-2)", overflow: "hidden" }}>
        <div style={{
          width: `${pct}%`,
          height: "100%",
          borderRadius: "var(--radius-xs)",
          background: color,
          transition: "width 0.5s cubic-bezier(0.22, 1, 0.36, 1)",
        }} />
      </div>
      <span className="t-mono" style={{ width: 40, textAlign: "right", fontSize: 10, flexShrink: 0 }}>{pct.toFixed(0)}%</span>
      <span className="t-micro" style={{ width: 24, textAlign: "right", color: "var(--ink-dim)", flexShrink: 0 }}>{count}</span>
    </div>
  );
}

function StatCell({ label, val, warn }: { label: string; val: string; warn?: boolean }) {
  return (
    <div style={{ textAlign: "center", minWidth: 36 }}>
      <div className="t-micro">{label}</div>
      <div className="t-body" style={{
        fontVariantNumeric: "tabular-nums",
        marginTop: 3,
        color: warn ? "var(--up)" : undefined,
        transition: "color 0.2s ease",
      }}>{val}</div>
    </div>
  );
}

function Team({ label, color, players, selfPuuid, expandedPuuid, setExpanded, playerBadges }: {
  label: string; color: string; players: Player[]; selfPuuid: string;
  expandedPuuid: string | null; setExpanded: (id: string | null) => void;
  playerBadges?: Map<string, BadgeType[]>;
}) {
  const avg = avgTeamRank(players);

  return (
    <div>
      <div className="section-label">
        <div style={{ width: 3, height: 14, borderRadius: 1, background: color, transition: "background 0.2s ease" }} />
        <span className="t-label" style={{ color, transition: "color 0.2s ease" }}>{label}</span>
        {avg && (
          <span style={{ marginLeft: "auto", display: "flex", alignItems: "center", gap: 5 }}>
            <span className="t-label" style={{ color: avg.color, fontWeight: 600, letterSpacing: 0.5, transition: "color 0.2s ease" }}>{avg.name}</span>
          </span>
        )}
      </div>
      <div style={{
        height: 1, marginTop: 4,
        background: `linear-gradient(90deg, ${color}, transparent 60%)`,
        opacity: 0.2,
      }} />
      <div className="card" style={{ padding: 4 }}>
        {players.map((p, i) => (
          <Row
            key={p.puuid}
            p={p}
            self={p.puuid === selfPuuid}
            i={i}
            expanded={expandedPuuid === p.puuid}
            onToggle={() => setExpanded(expandedPuuid === p.puuid ? null : p.puuid)}
            badges={playerBadges?.get(p.puuid)}
          />
        ))}
      </div>
    </div>
  );
}

export default function PlayerTable({ players, isDeathmatch, selfPuuid = "" }: Props) {
  const [expandedPuuid, setExpanded] = useState<string | null>(null);

  if (isDeathmatch) {
    return (
      <Team
        label="Players" color="var(--ink-faint)" players={players}
        selfPuuid={selfPuuid} expandedPuuid={expandedPuuid} setExpanded={setExpanded}
      />
    );
  }

  const blue = players.filter((p) => p.teamId === "Blue");
  const red = players.filter((p) => p.teamId === "Red");
  const selfTeam = players.find((p) => p.puuid === selfPuuid)?.teamId ?? "Blue";
  const my = selfTeam === "Blue" ? blue : red;
  const enemy = selfTeam === "Blue" ? red : blue;
  const myColor = selfTeam === "Blue" ? "var(--blue)" : "var(--red)";
  const enemyColor = selfTeam === "Blue" ? "var(--red)" : "var(--blue)";

  const playerBadges = useMemo(() => {
    const badges = new Map<string, BadgeType[]>();
    if (players.length === 0) return badges;

    const sorted = [...players].filter(p => p.acs > 0).sort((a, b) => b.acs - a.acs);
    const mvp = sorted[0];
    if (mvp) {
      badges.set(mvp.puuid, ["mvp"]);
    }

    const topFragger = [...players].filter(p => p.kills > 0).sort((a, b) => b.kills - a.kills)[0];
    if (topFragger && topFragger.puuid !== mvp?.puuid) {
      const existing = badges.get(topFragger.puuid) ?? [];
      badges.set(topFragger.puuid, [...existing, "tf"]);
    }

    const teams = new Set(players.map(p => p.teamId));
    for (const teamId of teams) {
      const teamPlayers = players.filter(p => p.teamId === teamId && p.acs > 0).sort((a, b) => b.acs - a.acs);
      const teamMvp = teamPlayers[0];
      if (teamMvp && teamMvp.puuid !== mvp?.puuid) {
        const existing = badges.get(teamMvp.puuid) ?? [];
        if (!existing.includes("tf")) {
          badges.set(teamMvp.puuid, [...existing, "top"]);
        }
      }
    }

    return badges;
  }, [players]);

  if (my.length === 5 && enemy.length === 5) {
    return (
      <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
        <div className="grid grid-cols-1 xl:grid-cols-2" style={{ gap: 16 }}>
          <Team label="Your Team" color={myColor} players={my} selfPuuid={selfPuuid} expandedPuuid={expandedPuuid} setExpanded={setExpanded} playerBadges={playerBadges} />
          <Team label="Enemy Team" color={enemyColor} players={enemy} selfPuuid={selfPuuid} expandedPuuid={expandedPuuid} setExpanded={setExpanded} playerBadges={playerBadges} />
        </div>
      </div>
    );
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
      {my.length > 0 && <Team label="Your Team" color={myColor} players={my} selfPuuid={selfPuuid} expandedPuuid={expandedPuuid} setExpanded={setExpanded} playerBadges={playerBadges} />}
      {enemy.length > 0 && <Team label="Enemy Team" color={enemyColor} players={enemy} selfPuuid={selfPuuid} expandedPuuid={expandedPuuid} setExpanded={setExpanded} playerBadges={playerBadges} />}
    </div>
  );
}