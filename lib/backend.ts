import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { ApiResponse } from "./types";

/**
 * Checks if running inside a Tauri environment.
 */
export function isTauri(): boolean {
  return typeof window !== "undefined" && Boolean((window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__);
}

/**
 * Opens an external URL in the default browser safely in both Tauri and web contexts.
 */
export async function openExternalUrl(url: string): Promise<void> {
  if (isTauri()) {
    try {
      await openUrl(url);
      return;
    } catch (err) {
      console.warn("[backend:openUrl] failed to open url via tauri plugin opener:", err);
    }
  }
  window.open(url, "_blank", "noopener,noreferrer");
}

/**
 * Fetches match data via Tauri IPC command `get_match` or falls back to HTTP API in browser dev.
 */
export async function fetchMatchData(force = false): Promise<ApiResponse> {
  if (isTauri()) {
    try {
      return await invoke<ApiResponse>("get_match", { force });
    } catch (err) {
      console.error("[backend:invoke] failed to get_match:", err);
      return {
        gameState: "ERROR",
        error: String(err),
      };
    }
  }

  // Browser dev mode / fallback
  const res = await fetch(force ? "/api/match?force=1" : "/api/match");
  if (!res.ok) {
    throw new Error(`HTTP error ${res.status}`);
  }
  return await res.json();
}
