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

pub fn local_app_data() -> Result<PathBuf, LockfileError> {
    if let Ok(override_path) = std::env::var("VALIGHT_LOCALAPPDATA") {
        if !override_path.is_empty() {
            return Ok(PathBuf::from(override_path));
        }
    }
    std::env::var("LOCALAPPDATA")
        .map(PathBuf::from)
        .map_err(|_| LockfileError::NotFound {
            path: "LOCALAPPDATA environment variable not set".to_string(),
        })
}

pub fn lockfile_path() -> Result<PathBuf, LockfileError> {
    Ok(local_app_data()?
        .join("Riot Games")
        .join("Riot Client")
        .join("Config")
        .join("lockfile"))
}

pub fn parse_lockfile(content: &str) -> Result<Lockfile, LockfileError> {
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

pub async fn read_lockfile() -> Result<Lockfile, LockfileError> {
    let path = lockfile_path()?;

    let content = tokio::fs::read_to_string(&path).await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            LockfileError::NotFound {
                path: path.display().to_string(),
            }
        } else {
            LockfileError::Io(e)
        }
    })?;

    parse_lockfile(&content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_standard_lockfile() {
        let content = "RiotClient:12345:54321:mypassword:https";
        let parsed = parse_lockfile(content).expect("should parse valid lockfile");
        assert_eq!(parsed.port, 54321);
        assert_eq!(parsed.password, "mypassword");
        assert_eq!(parsed.basic_auth, "Basic cmlvdDpteXBhc3N3b3Jk");
    }

    #[test]
    fn parses_lockfile_with_trailing_newlines() {
        let content = "RiotClient:12345:54321:mypassword:https\r\n";
        let parsed = parse_lockfile(content).expect("should handle trailing newlines");
        assert_eq!(parsed.port, 54321);
        assert_eq!(parsed.password, "mypassword");
    }

    #[test]
    fn fails_on_incomplete_lockfile() {
        let content = "RiotClient:12345:54321";
        let err = parse_lockfile(content).unwrap_err();
        match err {
            LockfileError::BadFormat { parts } => assert_eq!(parts, 3),
            _ => panic!("expected BadFormat error"),
        }
    }

    #[test]
    fn fails_on_invalid_port() {
        let content = "RiotClient:12345:notaport:mypassword:https";
        let err = parse_lockfile(content).unwrap_err();
        match err {
            LockfileError::BadPort { raw } => assert_eq!(raw, "notaport"),
            _ => panic!("expected BadPort error"),
        }
    }
}
