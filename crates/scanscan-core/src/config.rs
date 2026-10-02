use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, Result};

/// Runtime configuration for the core daemon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    /// Directory holding the catalog, CAS blocks, snapshots and journal.
    pub data_dir: PathBuf,
    /// Unix-domain socket the daemon listens on.
    pub socket: PathBuf,
    /// HTTP bind address advertised to the server (informational for the core).
    pub bind: String,
    /// Default roots to scan when none are supplied.
    pub roots: Vec<String>,
    /// Whether Docker integration is enabled.
    pub docker_enabled: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("./data"),
            socket: PathBuf::from("/tmp/scanscan.sock"),
            bind: "127.0.0.1:8080".to_string(),
            roots: Vec::new(),
            docker_enabled: false,
        }
    }
}

impl Config {
    /// Build a configuration from `SCANSCAN_*` environment variables.
    pub fn from_env() -> Result<Self> {
        let mut cfg = Config::default();

        if let Ok(dir) = std::env::var("SCANSCAN_DATA_DIR") {
            if dir.trim().is_empty() {
                return Err(CoreError::Config("SCANSCAN_DATA_DIR is empty".into()));
            }
            cfg.data_dir = PathBuf::from(dir);
        }
        if let Ok(sock) = std::env::var("SCANSCAN_SOCKET") {
            if !sock.trim().is_empty() {
                cfg.socket = PathBuf::from(sock);
            }
        }
        if let Ok(bind) = std::env::var("SCANSCAN_BIND") {
            if !bind.trim().is_empty() {
                cfg.bind = bind;
            }
        }
        if let Ok(roots) = std::env::var("SCANSCAN_ROOTS") {
            cfg.roots = roots
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect();
        }
        if let Ok(docker) = std::env::var("SCANSCAN_DOCKER") {
            cfg.docker_enabled = matches!(docker.as_str(), "1" | "true" | "yes" | "on");
        }

        Ok(cfg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_sane() {
        let cfg = Config::default();
        assert_eq!(cfg.data_dir, PathBuf::from("./data"));
        assert!(!cfg.docker_enabled);
    }

    #[test]
    fn docker_flag_parses_truthy_values() {
        for value in ["1", "true", "yes", "on"] {
            std::env::set_var("SCANSCAN_DOCKER", value);
            assert!(Config::from_env().unwrap().docker_enabled, "{value}");
        }
        std::env::set_var("SCANSCAN_DOCKER", "false");
        assert!(!Config::from_env().unwrap().docker_enabled);
        std::env::remove_var("SCANSCAN_DOCKER");
    }
}
