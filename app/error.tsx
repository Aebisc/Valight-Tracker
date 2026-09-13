"use client";

import { useEffect } from "react";

export default function Error({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  useEffect(() => {
    console.error("Unhandled error:", error);
  }, [error]);

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
        height: "70vh",
        gap: 20,
        textAlign: "center",
        padding: "0 24px",
      }}
    >
      <div
        style={{
          width: 64,
          height: 64,
          borderRadius: "var(--radius-lg, 12px)",
          background: "var(--surface-2, rgba(255,255,255,0.07))",
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
          stroke="var(--down, #f87171)"
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
          color: "var(--ink, #e8e8ec)",
        }}
      >
        Something went wrong
      </h2>
      <p
        style={{
          fontSize: 13,
          color: "var(--ink-muted, rgba(200,200,212,0.5))",
          maxWidth: 360,
        }}
      >
        An unexpected error occurred. Try again or restart the app.
      </p>
      <button
        onClick={reset}
        style={{
          padding: "10px 24px",
          borderRadius: "var(--radius-md, 8px)",
          background: "var(--accent, #6366f1)",
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
  );
}
