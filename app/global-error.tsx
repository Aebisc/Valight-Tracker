"use client";

import { useEffect } from "react";

export default function GlobalError({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  useEffect(() => {
    console.error("Global error:", error);
  }, [error]);

  return (
    <html lang="en">
      <body
        style={{
          margin: 0,
          padding: 0,
          minHeight: "100vh",
          background: "#0f0f0f",
          color: "#e8e8ec",
          fontFamily: "system-ui, sans-serif",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
        }}
      >
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            justifyContent: "center",
            gap: 20,
            textAlign: "center",
            padding: "0 24px",
          }}
        >
          <div
            style={{
              width: 64,
              height: 64,
              borderRadius: 12,
              background: "rgba(255,255,255,0.07)",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
            }}
          >
            <svg
              width="28"
              height="28"
              viewBox="0 0 24 24"
              fill="none"
              stroke="#f87171"
              strokeWidth="1.5"
              strokeLinecap="round"
              strokeLinejoin="round"
              aria-hidden="true"
            >
              <circle cx="12" cy="12" r="10" />
              <line x1="12" y1="8" x2="12" y2="12" />
              <line x1="12" y1="16" x2="12.01" y2="16" />
            </svg>
          </div>
          <h2
            style={{
              fontSize: 22,
              fontWeight: 700,
              letterSpacing: "-0.3px",
            }}
          >
            Something went wrong
          </h2>
          <p
            style={{
              fontSize: 13,
              color: "rgba(200,200,212,0.5)",
              maxWidth: 360,
            }}
          >
            A critical error occurred. Try again or restart the app.
          </p>
          <button
            onClick={reset}
            style={{
              padding: "10px 24px",
              borderRadius: 8,
              background: "#6366f1",
              color: "#fff",
              fontSize: 13,
              fontWeight: 600,
              border: "none",
              cursor: "pointer",
              transition: "opacity 0.2s ease",
            }}
            onMouseEnter={(e) => { e.currentTarget.style.opacity = "0.85"; }}
            onMouseLeave={(e) => { e.currentTarget.style.opacity = "1"; }}
          >
            Try again
          </button>
        </div>
      </body>
    </html>
  );
}
