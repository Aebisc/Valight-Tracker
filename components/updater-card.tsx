"use client";

import { useEffect, useState, useCallback } from "react";

type UpdateStatus = "idle" | "available" | "downloading" | "installing" | "error";

interface AppUpdate {
  currentVersion: string;
  version: string;
  body: string;
  url: string;
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`;
}

export default function UpdaterCard() {
  const [update, setUpdate] = useState<AppUpdate | null>(null);
  const [status, setStatus] = useState<UpdateStatus>("idle");
  const [dismissed, setDismissed] = useState(false);
  const [totalBytes, setTotalBytes] = useState(0);
  const [downloadedBytes, setDownloadedBytes] = useState(0);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  const checkForUpdate = useCallback(async () => {
    try {
      const res = await fetch("/api/update", { cache: "no-store" });
      if (!res.ok) return;
      const data = await res.json();

      if (data.available && data.version && data.url) {
        setUpdate({
          currentVersion: data.currentVersion,
          version: data.version,
          body: data.notes || "Bug fixes and performance improvements.",
          url: data.url,
        });
        setStatus("available");
      }
    } catch (err) {
      console.warn("Silent update check could not reach release endpoint:", err);
    }
  }, []);

  useEffect(() => {
    // Delay check slightly to let main window and sidecar complete startup
    const timer = setTimeout(() => {
      checkForUpdate();
    }, 2000);

    // Periodically re-check every 30 minutes
    const interval = setInterval(checkForUpdate, 30 * 60 * 1000);

    return () => {
      clearTimeout(timer);
      clearInterval(interval);
    };
  }, [checkForUpdate]);

  const handleUpdate = async () => {
    if (!update?.url) return;

    setStatus("downloading");
    setErrorMessage(null);
    setDownloadedBytes(0);
    setTotalBytes(0);

    try {
      const response = await fetch("/api/update", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ url: update.url }),
      });

      if (!response.ok || !response.body) {
        throw new Error(`Download failed with status ${response.status}`);
      }

      const reader = response.body.getReader();
      const decoder = new TextDecoder();
      let buffer = "";

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        buffer += decoder.decode(value, { stream: true });
        const lines = buffer.split("\n\n");
        buffer = lines.pop() ?? "";

        for (const line of lines) {
          const trimmed = line.trim();
          if (!trimmed.startsWith("data:")) continue;
          try {
            const payload = JSON.parse(trimmed.replace(/^data:\s*/, ""));
            if (payload.event === "Started") {
              setTotalBytes(payload.total || 0);
            } else if (payload.event === "Progress") {
              setDownloadedBytes(payload.downloaded || 0);
              if (payload.total) setTotalBytes(payload.total);
            } else if (payload.event === "Finished") {
              setStatus("installing");
            } else if (payload.event === "Error") {
              throw new Error(payload.message || "Error during update download");
            }
          } catch {
            // Ignore parse errors on partial stream chunks
          }
        }
      }
    } catch (err: any) {
      console.error("Update failed:", err);
      setStatus("error");
      setErrorMessage(
        err instanceof Error ? err.message : "Failed to download and install the update."
      );
    }
  };

  if (dismissed || status === "idle" || !update) {
    return null;
  }

  const progressPercent =
    totalBytes > 0 ? Math.min(100, Math.round((downloadedBytes / totalBytes) * 100)) : null;

  return (
    <div
      role="dialog"
      aria-label="Application Update Available"
      style={{
        position: "fixed",
        bottom: 24,
        right: 24,
        zIndex: 100,
        width: 380,
        maxWidth: "calc(100vw - 48px)",
        borderRadius: 16,
        background: "var(--surface-2, rgba(20, 20, 26, 0.85))",
        backdropFilter: "blur(24px)",
        WebkitBackdropFilter: "blur(24px)",
        border: "1px solid var(--border-accent, rgba(99, 102, 241, 0.3))",
        boxShadow: "0 12px 40px rgba(0, 0, 0, 0.6), 0 0 0 1px rgba(255, 255, 255, 0.05)",
        overflow: "hidden",
        display: "flex",
        flexDirection: "column",
        gap: 0,
      }}
    >
      {/* Top Banner Accent Line */}
      <div
        style={{
          height: 3,
          width: "100%",
          background: "linear-gradient(90deg, var(--accent, #6366f1), #a855f7)",
        }}
      />

      <div style={{ padding: "18px 20px" }}>
        {/* Header */}
        <div
          style={{
            display: "flex",
            alignItems: "flex-start",
            justifyContent: "space-between",
            gap: 12,
            marginBottom: 12,
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
            <div
              style={{
                width: 36,
                height: 36,
                borderRadius: 10,
                background: "var(--accent-soft, rgba(99, 102, 241, 0.15))",
                border: "1px solid var(--border-accent, rgba(99, 102, 241, 0.25))",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                color: "var(--accent, #6366f1)",
                flexShrink: 0,
              }}
            >
              <svg
                width="18"
                height="18"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                <polyline points="7 10 12 15 17 10" />
                <line x1="12" y1="15" x2="12" y2="3" />
              </svg>
            </div>
            <div>
              <div
                style={{
                  fontSize: 15,
                  fontWeight: 600,
                  color: "var(--ink, #e8e8ec)",
                  letterSpacing: "-0.2px",
                }}
              >
                Update Available
              </div>
              <div
                style={{
                  fontSize: 12,
                  color: "var(--ink-dim, rgba(160, 160, 180, 0.55))",
                  marginTop: 2,
                  display: "flex",
                  alignItems: "center",
                  gap: 6,
                }}
              >
                <span>v{update.currentVersion}</span>
                <span>→</span>
                <span
                  style={{
                    color: "var(--up, #4ade80)",
                    fontWeight: 600,
                  }}
                >
                  v{update.version}
                </span>
              </div>
            </div>
          </div>

          {status === "available" && (
            <button
              onClick={() => setDismissed(true)}
              aria-label="Dismiss update notification"
              style={{
                background: "transparent",
                border: "none",
                color: "var(--ink-dim, rgba(160, 160, 180, 0.55))",
                cursor: "pointer",
                padding: 4,
                borderRadius: 6,
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.color = "var(--ink, #fff)";
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.color = "var(--ink-dim, rgba(160, 160, 180, 0.55))";
              }}
            >
              <svg
                width="16"
                height="16"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <line x1="18" y1="6" x2="6" y2="18" />
                <line x1="6" y1="6" x2="18" y2="18" />
              </svg>
            </button>
          )}
        </div>

        {/* Release Notes */}
        {update.body && status === "available" && (
          <div
            style={{
              maxHeight: 120,
              overflowY: "auto",
              padding: "8px 12px",
              borderRadius: 8,
              background: "var(--surface-0, rgba(255, 255, 255, 0.03))",
              border: "1px solid var(--border, rgba(255, 255, 255, 0.06))",
              fontSize: 12,
              lineHeight: 1.5,
              color: "var(--ink-sub, rgba(232, 232, 236, 0.72))",
              whiteSpace: "pre-wrap",
              wordBreak: "break-word",
              marginBottom: 16,
            }}
          >
            {update.body}
          </div>
        )}

        {/* Downloading State */}
        {status === "downloading" && (
          <div style={{ marginTop: 8, marginBottom: 14 }}>
            <div
              style={{
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
                fontSize: 12,
                color: "var(--ink-sub, rgba(232, 232, 236, 0.72))",
                marginBottom: 8,
              }}
            >
              <span>Downloading update...</span>
              <span style={{ fontVariantNumeric: "tabular-nums", fontWeight: 500 }}>
                {progressPercent !== null
                  ? `${progressPercent}% (${formatBytes(downloadedBytes)} / ${formatBytes(totalBytes)})`
                  : formatBytes(downloadedBytes)}
              </span>
            </div>

            {/* Progress Bar Track */}
            <div
              style={{
                width: "100%",
                height: 6,
                borderRadius: 3,
                background: "var(--surface-3, rgba(255, 255, 255, 0.1))",
                overflow: "hidden",
                position: "relative",
              }}
            >
              <div
                style={{
                  height: "100%",
                  width: progressPercent !== null ? `${progressPercent}%` : "100%",
                  background: "linear-gradient(90deg, var(--accent, #6366f1), #a855f7)",
                  borderRadius: 3,
                  transition: "width 0.2s ease",
                  animation: progressPercent === null ? "pulse 1.5s infinite" : "none",
                }}
              />
            </div>
          </div>
        )}

        {/* Installing / Restarting State */}
        {status === "installing" && (
          <div
            style={{
              padding: "12px 0",
              display: "flex",
              alignItems: "center",
              gap: 10,
              fontSize: 13,
              color: "var(--ink-sub, rgba(232, 232, 236, 0.8))",
            }}
          >
            <div
              style={{
                width: 16,
                height: 16,
                border: "2px solid var(--accent, #6366f1)",
                borderTopColor: "transparent",
                borderRadius: "50%",
                animation: "spin 1s linear infinite",
              }}
            />
            <span>Installing update and restarting...</span>
          </div>
        )}

        {/* Error State */}
        {status === "error" && (
          <div
            style={{
              padding: "10px 12px",
              borderRadius: 8,
              background: "rgba(232, 125, 125, 0.1)",
              border: "1px solid rgba(232, 125, 125, 0.25)",
              color: "var(--red, #e87d7d)",
              fontSize: 12,
              lineHeight: 1.4,
              marginBottom: 14,
            }}
          >
            <strong>Update Error:</strong> {errorMessage ?? "An unexpected error occurred."}
          </div>
        )}

        {/* Action Buttons */}
        {status === "available" && (
          <div style={{ display: "flex", gap: 10, justifyContent: "flex-end" }}>
            <button
              onClick={() => setDismissed(true)}
              style={{
                padding: "8px 14px",
                borderRadius: 8,
                fontSize: 12,
                fontWeight: 500,
                background: "transparent",
                border: "1px solid var(--border, rgba(255, 255, 255, 0.1))",
                color: "var(--ink-muted, rgba(200, 200, 212, 0.7))",
                cursor: "pointer",
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.background = "var(--surface-1, rgba(255, 255, 255, 0.05))";
                e.currentTarget.style.color = "var(--ink, #fff)";
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.background = "transparent";
                e.currentTarget.style.color = "var(--ink-muted, rgba(200, 200, 212, 0.7))";
              }}
            >
              Later
            </button>
            <button
              onClick={handleUpdate}
              style={{
                padding: "8px 16px",
                borderRadius: 8,
                fontSize: 12,
                fontWeight: 600,
                background: "var(--accent, #6366f1)",
                color: "#ffffff",
                border: "none",
                cursor: "pointer",
                boxShadow: "0 2px 10px rgba(99, 102, 241, 0.35)",
                display: "inline-flex",
                alignItems: "center",
                gap: 6,
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.opacity = "0.9";
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.opacity = "1";
              }}
            >
              <svg
                width="14"
                height="14"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                <polyline points="7 10 12 15 17 10" />
                <line x1="12" y1="15" x2="12" y2="3" />
              </svg>
              Update Now
            </button>
          </div>
        )}

        {status === "error" && (
          <div style={{ display: "flex", gap: 10, justifyContent: "flex-end" }}>
            <button
              onClick={() => setDismissed(true)}
              style={{
                padding: "8px 14px",
                borderRadius: 8,
                fontSize: 12,
                background: "transparent",
                border: "1px solid var(--border, rgba(255, 255, 255, 0.1))",
                color: "var(--ink-muted, rgba(200, 200, 212, 0.7))",
                cursor: "pointer",
              }}
            >
              Dismiss
            </button>
            <button
              onClick={handleUpdate}
              style={{
                padding: "8px 16px",
                borderRadius: 8,
                fontSize: 12,
                fontWeight: 600,
                background: "var(--accent, #6366f1)",
                color: "#ffffff",
                border: "none",
                cursor: "pointer",
              }}
            >
              Retry
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
