"use client";

import MatchHeader from "../components/match-header";
import PlayerTable from "../components/player-table";
import StatusBar from "../components/status-bar";
import { useMatchPolling } from "../hooks/use-match-polling";
import { useSettings } from "../hooks/use-settings";
import type { Player, MatchInfo } from "@/lib/types";
import { rankColor, rankIconUrl } from "@/lib/constants";

export default function Home() {
  const {
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
    refreshMatch: refresh,
  } = useMatchPolling();

  const {
    theme,
    toggleTheme,
  } = useSettings();

  const time = lastUpdated
    ? lastUpdated.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" })
    : "--:--:--";

  const inMatch = gameState === "PREGAME" || gameState === "INGAME";

  return (
    <div className="min-h-screen flex flex-col">
      <div style={{
        position: "fixed", top: 12, right: 16, zIndex: 50,
        display: "flex", alignItems: "center", gap: 8,
        padding: "8px 14px", borderRadius: 12,
        background: "var(--surface-2)", border: "1px solid var(--border)",
        backdropFilter: "blur(20px)", WebkitBackdropFilter: "blur(20px)",
        boxShadow: "0 4px 24px rgba(0,0,0,0.15)",
      }}>
        <button onClick={toggleTheme} className="theme-toggle" title={`Switch to ${theme === "dark" ? "light" : "dark"} mode`} aria-label="Toggle theme" aria-pressed={theme === "light"}>
          {theme === "dark" ? (
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
              <circle cx="12" cy="12" r="5" />
              <path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42" />
            </svg>
          ) : (
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
              <path d="M21 12.79A9 9 0 1111.21 3 7 7 0 0021 12.79z" />
            </svg>
          )}
        </button>
        <div style={{ width: 1, height: 16, background: "var(--border)" }} />
        <span className="t-label" style={{ fontVariantNumeric: "tabular-nums", color: "var(--ink-dim)" }}>{time}</span>
        <span role="status" aria-live="polite" className="t-label" style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
          <span
            className={connected ? "a-pulse" : ""}
            style={{ width: 6, height: 6, borderRadius: "50%", background: reconnecting ? "var(--amber, #fbbf24)" : connected ? "var(--up)" : "var(--down)", boxShadow: connected ? "0 0 6px rgba(74, 222, 128, 0.3)" : "none" }}
          />
          {reconnecting ? "Reconnecting..." : connected ? "Connected" : "Offline"}
        </span>
      </div>

      <main style={{ flex: 1, width: "100%", maxWidth: 1600, margin: "0 auto", padding: "56px 32px 56px", position: "relative", zIndex: 1 }}>
        {loading ? (
          <Splash>
            <div style={{ fontSize: 13, fontWeight: 600, letterSpacing: 2, textTransform: "uppercase", marginBottom: 28, color: "var(--ink-muted)" }}>VaLight Tracker</div>
            <div style={{ position: "relative", width: 64, height: 64, display: "flex", alignItems: "center", justifyContent: "center" }}>
              <div style={{ width: 36, height: 36, border: "2px solid var(--accent)", borderTopColor: "transparent", borderRadius: "50%" }} className="a-spin" />
            </div>
            <span className="t-label" style={{ color: "var(--ink-muted)", marginTop: 24, display: "inline-flex", alignItems: "center", gap: 0 }}>
              Connecting to Valorant<span className="a-dots" style={{ width: 18, textAlign: "left" }}></span>
            </span>
          </Splash>
        ) : gameState === "OFFLINE" ? (
          <Splash>
            <div className="card" style={{ width: 80, height: 80, display: "flex", alignItems: "center", justifyContent: "center", marginBottom: 8 }}>
              <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="var(--ink-faint)" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
              </svg>
            </div>
            <p className="t-display" style={{ fontSize: 28, marginTop: 20 }}>Waiting for Valorant</p>
            <p className="t-label" style={{ marginTop: 10, color: "var(--ink-muted)", fontSize: 13, letterSpacing: 2 }}>Launch the game to start tracking</p>
            {error && (
              <span className="tag" style={{ color: reconnecting ? "var(--amber, #fbbf24)" : "var(--accent)", borderColor: reconnecting ? "rgba(251, 191, 36, 0.25)" : "var(--border-accent)", marginTop: 14 }}>{error}</span>
            )}
          </Splash>
        ) : gameState === "MENUS" ? (
          <div className="a-enter" style={{ maxWidth: 760, margin: "0 auto", width: "100%", minHeight: "70vh", display: "flex", flexDirection: "column", justifyContent: "center", gap: 24 }}>
            <div className="card a-enter" style={{ padding: "32px 36px", display: "flex", alignItems: "center", gap: 24 }}>
              <div style={{ position: "relative", width: 64, height: 64, display: "flex", alignItems: "center", justifyContent: "center", flexShrink: 0 }}>
                <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="var(--up)" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                  <circle cx="12" cy="12" r="10" />
                  <polyline points="12 6 12 12 16 14" />
                </svg>
              </div>
              <div style={{ flex: 1, minWidth: 0 }}>
                <p className="t-title" style={{ fontSize: 24 }}>In Menus</p>
                <p className="t-label" style={{ marginTop: 6, color: "var(--ink-dim)" }}>Ready to queue</p>
              </div>
              {error && (
                <span className="tag" style={{ color: "var(--accent)", borderColor: "var(--border-accent)", fontSize: 10 }}>{error}</span>
              )}
              <button
                onClick={refresh}
                disabled={refreshing}
                className="btn-ghost"
                aria-label={refreshing ? "Refreshing match data" : "Refresh match data"}
                style={{
                  padding: "10px 18px",
                  fontSize: 12,
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
                  width="13"
                  height="13"
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

            {selfPuuid && (() => {
              const self = players.find((p) => p.puuid === selfPuuid);
              if (!self) return null;
              return <SelfCard player={self} />;
            })()}
          </div>
        ) : (
          <div className="a-enter" style={{ display: "flex", flexDirection: "column", gap: 0 }}>
            {matchInfo && (
              <div style={{ marginBottom: 20 }}>
                <MatchHeader matchInfo={matchInfo} gameState={gameState} onRefresh={refresh} refreshing={refreshing} stateStartTime={stateStartTime ?? undefined} />
              </div>
            )}

            <div className="a-d1">
              <PlayerTable players={players} isDeathmatch={matchInfo?.isDeathmatch} selfPuuid={selfPuuid} />
            </div>
          </div>
        )}
      </main>

      <StatusBar />
    </div>
  );
}

function Splash({ children }: { children: React.ReactNode }) {
  return (
    <div className="a-enter" style={{
      display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center",
      height: "70vh", gap: 0, textAlign: "center",
    }}>
      {children}
    </div>
  );
}

function SelfCard({ player }: { player?: Player }) {
  if (!player) return null;
  const rc = rankColor(player.rankName);
  return (
    <div className="card a-enter" style={{ padding: "18px 22px", display: "flex", alignItems: "center", gap: 18, flexWrap: "wrap" }}>
      {player.agentId && (
        <img
          src={`https://media.valorant-api.com/agents/${player.agentId}/displayicon.png`}
          alt={player.agentName}
          width={42} height={42}
          style={{ borderRadius: 8, border: "1px solid var(--border)" }}
        />
      )}

      <div style={{ minWidth: 0 }}>
        <span className="t-title" style={{ fontSize: 16 }}>{player.name}</span>
        <span className="t-label" style={{ marginLeft: 6, color: "var(--ink-faint)" }}>#{player.tag}</span>
        <div className="t-micro" style={{ marginTop: 4, color: "var(--ink-dim)" }}>Level {player.accountLevel}</div>
      </div>

      <div style={{ width: 1, height: 28, background: "var(--border)", flexShrink: 0 }} />

      <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
        <img
          src={rankIconUrl(player.rank)}
          alt={player.rankName}
          width={36} height={36}
          style={{ filter: "drop-shadow(0 0 4px rgba(255,255,255,0.1))", objectFit: "contain" }}
        />
        <div>
          <span className="t-title" style={{ color: rc, fontSize: 14 }}>{player.rankName}</span>
          <div className="t-mono" style={{ marginTop: 2, fontSize: 11 }}>{player.rr} RR</div>
        </div>
      </div>

      <div style={{ width: 1, height: 28, background: "var(--border)", flexShrink: 0 }} />

      <div>
        <div className="t-micro" style={{ color: "var(--ink-dim)", marginBottom: 4 }}>This Act</div>
        <span className="t-mono" style={{ fontSize: 12 }}>
          {player.currentSeasonWins}W / {player.currentSeasonGames}G
        </span>
      </div>
    </div>
  );
}