"use client";

import {
  createContext,
  useContext,
  useCallback,
  useState,
  useEffect,
  useRef,
  type ReactNode,
} from "react";

type ToastType = "info" | "success" | "error" | "warning";

interface Toast {
  id: number;
  message: string;
  type: ToastType;
  createdAt: number;
  exiting: boolean;
}

interface ToastContextValue {
  toast: (message: string, type?: ToastType) => void;
}

const ToastContext = createContext<ToastContextValue | null>(null);

export function useToast(): ToastContextValue {
  const ctx = useContext(ToastContext);
  if (!ctx) throw new Error("useToast must be used within <ToastProvider>");
  return ctx;
}

const MAX_TOASTS = 3;
const AUTO_DISMISS_MS = 4000;
const EXIT_DURATION_MS = 300;

const BORDER_COLORS: Record<ToastType, string> = {
  success: "var(--green, #34d399)",
  error: "var(--red, #e87d7d)",
  warning: "var(--amber, #fbbf24)",
  info: "var(--accent, #ff4655)",
};

const PROGRESS_COLORS: Record<ToastType, string> = {
  success: "var(--green, #34d399)",
  error: "var(--red, #e87d7d)",
  warning: "var(--amber, #fbbf24)",
  info: "var(--accent, #ff4655)",
};

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<Toast[]>([]);
  const idRef = useRef(0);
  const timersRef = useRef<Map<number, ReturnType<typeof setTimeout>>>(new Map());

  useEffect(() => {
    const timers = timersRef.current;
    return () => {
      timers.forEach((timer) => clearTimeout(timer));
      timers.clear();
    };
  }, []);

  const dismiss = useCallback((id: number) => {
    setToasts((prev) =>
      prev.map((t) => (t.id === id ? { ...t, exiting: true } : t))
    );
    const existing = timersRef.current.get(id);
    if (existing) {
      clearTimeout(existing);
      timersRef.current.delete(id);
    }
    const exitTimer = setTimeout(() => {
      setToasts((prev) => prev.filter((t) => t.id !== id));
      timersRef.current.delete(-id);
    }, EXIT_DURATION_MS);
    timersRef.current.set(-id, exitTimer);
  }, []);

  const toast = useCallback(
    (message: string, type: ToastType = "info") => {
      const id = ++idRef.current;
      setToasts((prev) => {
        const next = [...prev, { id, message, type, createdAt: Date.now(), exiting: false }];
        const active = next.filter((t) => !t.exiting);
        if (active.length > MAX_TOASTS) {
          const oldest = active[0];
          oldest.exiting = true;
          const oldTimer = timersRef.current.get(oldest.id);
          if (oldTimer) {
            clearTimeout(oldTimer);
            timersRef.current.delete(oldest.id);
          }
          const evictTimer = setTimeout(() => {
            setToasts((p) => p.filter((t) => t.id !== oldest.id));
            timersRef.current.delete(-oldest.id);
          }, EXIT_DURATION_MS);
          timersRef.current.set(-oldest.id, evictTimer);
        }
        return next;
      });

      const timer = setTimeout(() => dismiss(id), AUTO_DISMISS_MS);
      timersRef.current.set(id, timer);
    },
    [dismiss]
  );

  return (
    <ToastContext.Provider value={{ toast }}>
      {children}
      <ToastContainer toasts={toasts} onDismiss={dismiss} />
    </ToastContext.Provider>
  );
}

function ToastContainer({
  toasts,
  onDismiss,
}: {
  toasts: Toast[];
  onDismiss: (id: number) => void;
}) {
  if (toasts.length === 0) return null;

  return (
    <div
      style={{
        position: "fixed",
        bottom: 20,
        right: 20,
        zIndex: "var(--z-toast, 200)" as unknown as number,
        display: "flex",
        flexDirection: "column",
        gap: 8,
        pointerEvents: "none",
        paddingBottom: "env(safe-area-inset-bottom, 0px)",
        backdropFilter: "blur(4px)",
        WebkitBackdropFilter: "blur(4px)",
      }}
    >
      {toasts.map((t) => (
        <ToastCard key={t.id} toast={t} onDismiss={onDismiss} />
      ))}
    </div>
  );
}

function ToastCard({
  toast: t,
  onDismiss,
}: {
  toast: Toast;
  onDismiss: (id: number) => void;
}) {
  const [mounted, setMounted] = useState(false);

  useEffect(() => {
    const raf = requestAnimationFrame(() => setMounted(true));
    return () => cancelAnimationFrame(raf);
  }, []);

  const borderColor = BORDER_COLORS[t.type];
  const progressColor = PROGRESS_COLORS[t.type];

  return (
    <div
      role={t.type === "error" ? "alert" : "status"}
      aria-live={t.type === "error" ? "assertive" : "polite"}
      style={{
        pointerEvents: "auto",
        position: "relative",
        overflow: "hidden",
        minWidth: 280,
        maxWidth: 380,
        background: "var(--surface-1, rgba(255,255,255,0.035))",
        backdropFilter: "blur(24px)",
        WebkitBackdropFilter: "blur(24px)",
        border: "1px solid var(--border, rgba(255,255,255,0.07))",
        borderLeft: `3px solid ${borderColor}`,
        borderRadius: "var(--radius-lg)",
        boxShadow:
          "0 4px 24px rgba(0,0,0,0.4), 0 0 0 1px rgba(255,255,255,0.04)",
        transform: mounted && !t.exiting
          ? "translateX(0)"
          : "translateX(calc(100% + 24px))",
        opacity: mounted && !t.exiting ? 1 : 0,
        transition: `transform ${EXIT_DURATION_MS}ms cubic-bezier(0.16, 1, 0.3, 1), opacity ${EXIT_DURATION_MS}ms ease`,
      }}
    >
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          gap: 12,
          padding: "10px 12px 10px 14px",
        }}
      >
        <span
          style={{
            fontFamily: "var(--sans, 'DM Sans', sans-serif)",
            fontSize: 13,
            fontWeight: 500,
            color: "var(--ink-sub, rgba(230,230,242,0.72))",
            lineHeight: 1.3,
          }}
        >
          {t.message}
        </span>
        <button
          onClick={() => onDismiss(t.id)}
          style={{
            position: "relative",
            flexShrink: 0,
            width: 20,
            height: 20,
            minWidth: 44,
            minHeight: 44,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            background: "transparent",
            border: "none",
            borderRadius: 4,
            color: "var(--ink-faint, rgba(200,200,220,0.28))",
            cursor: "pointer",
            padding: 12,
            margin: -12,
            transition: "color 0.15s ease, background 0.15s ease",
          }}
          onMouseEnter={(e) => {
            e.currentTarget.style.color = "var(--ink-sub)";
            e.currentTarget.style.background = "var(--surface-2)";
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.color = "var(--ink-faint)";
            e.currentTarget.style.background = "transparent";
          }}
          aria-label="Dismiss"
        >
          <svg
            width="12"
            height="12"
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
      </div>

      <div
        style={{
          position: "absolute",
          bottom: 0,
          left: 0,
          right: 0,
          height: 2,
          background: "var(--surface-0, rgba(255,255,255,0.02))",
        }}
      >
        <div
          style={{
            height: "100%",
            background: progressColor,
            opacity: 0.5,
            borderRadius: 1,
            animation: mounted && !t.exiting
              ? `toast-progress ${AUTO_DISMISS_MS}ms linear forwards`
              : "none",
          }}
        />
      </div>

    </div>
  );
}
