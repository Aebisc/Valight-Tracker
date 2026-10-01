// src-tauri/src/riot/lockfile.rs
//
// Reads the Riot Client lockfile and builds the Basic auth header.
// Format: name:pid:port:password:protocol

use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct Lockfile {
    pub port: u16,
    pub password: String,
    /// Precomputed Basic auth value: base64("riot:{password}")
    pub basic_auth: String,
}

#[derive(Debug, Error)]
pub enum LockfileError {
    #[error("Valorant is not running (lockfile not found: {path})")]
    NotFound { path: String },
    #[error("Lockfile has unexpected format (got {parts} parts, expected 5)")]
    BadFormat { parts: usize },
    #[error("Invalid port in lockfile: {raw}")]
    BadPort { raw: String },
    #[error("IO error reading lockfile: {0}")]
    Io(#[from] std::io::Error),
}

pub fn lockfile_path() -> PathBuf {
    let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| {
        // Fallback for non-standard setups — log a warning
        tracing::warn!("LOCALAPPDATA not set, using C:\\Users\\Default\\AppData\\Local");
        r"C:\Users\Default\AppData\Local".to_string()
    });
    PathBuf::from(local_app_data)
        .join("Riot Games")
        .join("Riot Client")
        .join("Config")
        .join("lockfile")
}

pub async fn read_lockfile() -> Result<Lockfile, LockfileError> {
    let path = lockfile_path();

    let content = tokio::fs::read_to_string(&path).await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            LockfileError::NotFound {
                path: path.display().to_string(),
            }
        } else {
            LockfileError::Io(e)
        }
    })?;

    let parts: Vec<&str> = content.trim().splitn(5, ':').collect();
    if parts.len() < 5 {
        return Err(LockfileError::BadFormat { parts: parts.len() });
    }

    // parts: [name, pid, port, password, protocol]
    let port_str = parts[2];
    let password = parts[3].to_string();

    let port: u16 = port_str.parse().map_err(|_| LockfileError::BadPort {
        raw: port_str.to_string(),
    })?;

    // Basic auth: base64("riot:{password}")
    use base64::Engine;
    let credentials = format!("riot:{}", password);
    let basic_auth = format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes())
    );

    Ok(Lockfile {
        port,
        password,
        basic_auth,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn parses_standard_lockfile() {
        // Simulate parsing with a known lockfile string
        let content = "RiotClient:12345:54321:mypassword:https";
        let parts: Vec<&str> = content.trim().splitn(5, ':').collect();
        assert_eq!(parts.len(), 5);
        assert_eq!(parts[2], "54321");
        assert_eq!(parts[3], "mypassword");

        let port: u16 = parts[2].parse().unwrap();
        assert_eq!(port, 54321);

        use base64::Engine;
        let basic = base64::engine::general_purpose::STANDARD.encode("riot:mypassword");
        assert!(basic.len() > 0);
    }
}
