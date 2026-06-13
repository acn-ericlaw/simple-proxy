//! Config file model + loading. The on-disk JSON schema is kept compatible with the
//! original Node.js tool's `proxy-config.json` so existing files keep working.

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::net::IpAddr;
use std::path::Path;
use std::time::Duration;

const DEFAULT_IDLE_SECS: u64 = 1800; // 30 minutes, matching the JS IDLE_TIMEOUT

/// One `source_ports[i] -> target:target_ports[i]` daemon configuration.
#[derive(Debug, Deserialize)]
pub struct Config {
    /// Resolve the target IP at startup by running a shell command. Optional —
    /// omit it and set `target_ip` instead for a static upstream.
    #[serde(default)]
    pub discovery: Option<Discovery>,
    /// Static upstream IP, used when `discovery` is absent.
    #[serde(default)]
    pub target_ip: Option<String>,
    pub source_ports: Vec<u16>,
    pub target_ports: Vec<u16>,
    /// Inbound IP allow-list: exact IPs and `x.y.z.*` wildcard prefixes.
    #[serde(default)]
    pub authorized: Vec<String>,
    /// If a connect to this target port times out, exit(1) so a process manager restarts us.
    #[serde(default)]
    pub restart: Option<u16>,
    /// Per-connection idle timeout in seconds (default 1800). Exposed mainly for tests.
    #[serde(default)]
    pub idle_timeout_secs: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct Discovery {
    pub command: String,
    pub tag: String,
    pub index: usize,
}

impl Config {
    pub fn load(path: &Path) -> Result<Config> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("reading config from {}", path.display()))?;
        Self::from_json(&text).with_context(|| format!("loading {}", path.display()))
    }

    pub fn from_json(text: &str) -> Result<Config> {
        let cfg: Config = serde_json::from_str(text)?;
        cfg.validate()?;
        Ok(cfg)
    }

    fn validate(&self) -> Result<()> {
        if self.source_ports.is_empty() {
            bail!("Invalid proxy-config.json: no source_ports configured");
        }
        if self.source_ports.len() != self.target_ports.len() {
            bail!(
                "Invalid proxy-config.json: source_ports and target_ports must have equal length"
            );
        }
        match (&self.discovery, &self.target_ip) {
            (Some(_), Some(_)) => {
                bail!("Invalid proxy-config.json: set either \"discovery\" or \"target_ip\", not both")
            }
            (None, None) => {
                bail!("Invalid proxy-config.json: set one of \"discovery\" or \"target_ip\"")
            }
            (None, Some(ip)) => {
                ip.parse::<IpAddr>()
                    .with_context(|| format!("target_ip {ip:?} is not a valid IP address"))?;
            }
            (Some(_), None) => {}
        }
        Ok(())
    }

    pub fn idle_timeout(&self) -> Duration {
        Duration::from_secs(self.idle_timeout_secs.unwrap_or(DEFAULT_IDLE_SECS))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_discovery_config() {
        let cfg = Config::from_json(
            r#"{
                "discovery": { "command": "multipass exec main ifconfig eth0", "tag": "inet", "index": 1 },
                "source_ports": [22], "target_ports": [22],
                "authorized": ["192.168.1.*", "127.0.0.1"], "restart": 22
            }"#,
        )
        .unwrap();
        assert!(cfg.discovery.is_some());
        assert_eq!(cfg.restart, Some(22));
        assert_eq!(cfg.idle_timeout(), Duration::from_secs(1800));
    }

    #[test]
    fn parses_static_target_ip_config() {
        let cfg = Config::from_json(
            r#"{ "target_ip": "10.0.0.5", "source_ports": [8080], "target_ports": [80] }"#,
        )
        .unwrap();
        assert!(cfg.discovery.is_none());
        assert_eq!(cfg.target_ip.as_deref(), Some("10.0.0.5"));
        assert!(cfg.authorized.is_empty());
    }

    #[test]
    fn rejects_mismatched_port_lengths() {
        let err = Config::from_json(
            r#"{ "target_ip": "10.0.0.5", "source_ports": [1, 2], "target_ports": [1] }"#,
        )
        .unwrap_err();
        assert!(err.to_string().contains("equal length"));
    }

    #[test]
    fn rejects_both_discovery_and_target_ip() {
        let err = Config::from_json(
            r#"{ "discovery": {"command":"x","tag":"y","index":0}, "target_ip": "10.0.0.5",
                 "source_ports": [1], "target_ports": [1] }"#,
        )
        .unwrap_err();
        assert!(err.to_string().contains("not both"));
    }

    #[test]
    fn rejects_neither_discovery_nor_target_ip() {
        let err = Config::from_json(r#"{ "source_ports": [1], "target_ports": [1] }"#).unwrap_err();
        assert!(err.to_string().contains("one of"));
    }

    #[test]
    fn rejects_bad_static_ip() {
        let err = Config::from_json(
            r#"{ "target_ip": "not-an-ip", "source_ports": [1], "target_ports": [1] }"#,
        )
        .unwrap_err();
        assert!(err.to_string().contains("valid IP"));
    }
}
