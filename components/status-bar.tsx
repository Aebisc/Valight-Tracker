import packageJson from "@/package.json";

const appVersion = `v${packageJson.version}`;

export default function StatusBar() {
  return (
    <footer className="status-bar-footer" role="contentinfo">
      <div className="status-bar-border" />

      <div className="status-bar-bg status-bar-inner">
        <div className="status-bar-content">
          <span className="t-label status-bar-dim status-bar-mono">
            {appVersion}
          </span>

          <div className="status-bar-credits">
            <span className="t-label">
              <a
                href="https://github.com/Aebisc/Light-Valorant-Tracker"
                target="_blank"
                rel="noopener noreferrer"
                className="footer-link"
              >
                Aebisc
              </a>
            </span>

          </div>
        </div>
      </div>

      <style>{`
        .status-bar-footer {
          width: 100%;
          margin-top: auto;
        }
        .status-bar-border {
          height: 1px;
          background: linear-gradient(90deg, transparent 0%, var(--accent) 20%, var(--accent) 50%, var(--accent) 80%, transparent 100%);
          opacity: 0.15;
        }
        .status-bar-inner {
          width: 100%;
          padding: 10px 32px;
        }
        .status-bar-content {
          max-width: 1400px;
          margin: 0 auto;
          display: flex;
          align-items: center;
          justify-content: space-between;
          gap: 20px;
          flex-wrap: wrap;
        }
        .status-bar-dim {
          opacity: 0.3;
          font-size: 11px;
          user-select: none;
          letter-spacing: 0.02em;
        }
        .status-bar-mono {
          letter-spacing: 0.05em;
          font-family: var(--font-mono, monospace);
        }
        .status-bar-credits {
          display: flex;
          align-items: center;
          gap: 14px;
          flex-wrap: wrap;
          justify-content: center;
        }
      `}</style>
    </footer>
  );
}
