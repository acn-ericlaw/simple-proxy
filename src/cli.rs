//! Hand-rolled CLI parsing (no `clap`). Two subcommands:
//!
//! ```text
//! simple-proxy serve [--config <path>]
//! simple-proxy forward <src_ip:port> <dst_ip:port>
//! ```

use anyhow::{bail, Result};
use std::net::SocketAddr;
use std::path::PathBuf;

pub const USAGE: &str = "\
Usage:
  simple-proxy serve [--config <path>]      Run the config-driven daemon (default: proxy-config.json)
  simple-proxy forward <src_ip:port> <dst_ip:port>   Forward a single port pair (one-shot)";

const DEFAULT_CONFIG: &str = "proxy-config.json";

#[derive(Debug)]
pub enum Command {
    Serve {
        config: PathBuf,
    },
    Forward {
        source: SocketAddr,
        target: SocketAddr,
    },
}

/// Parse process arguments (including argv[0]) into a [`Command`].
pub fn parse<I: Iterator<Item = String>>(mut args: I) -> Result<Command> {
    let _bin = args.next();
    match args.next().as_deref() {
        Some("serve") => {
            let mut config = PathBuf::from(DEFAULT_CONFIG);
            while let Some(arg) = args.next() {
                match arg.as_str() {
                    "--config" | "-c" => {
                        config = match args.next() {
                            Some(p) => PathBuf::from(p),
                            None => bail!("--config requires a path\n\n{USAGE}"),
                        };
                    }
                    other => bail!("unexpected argument {other:?}\n\n{USAGE}"),
                }
            }
            Ok(Command::Serve { config })
        }
        Some("forward") => match (args.next(), args.next()) {
            (Some(src), Some(dst)) => Ok(Command::Forward {
                source: parse_addr(&src)?,
                target: parse_addr(&dst)?,
            }),
            _ => bail!("forward requires <src_ip:port> <dst_ip:port>\n\n{USAGE}"),
        },
        Some(other) => bail!("unknown command {other:?}\n\n{USAGE}"),
        None => bail!("{USAGE}"),
    }
}

/// Parse `ip:port` into a [`SocketAddr`]. Unlike the JS `get_address` (which split on
/// the last `:` and broke on IPv6), this uses the standard parser, so IPv6 literals
/// must be bracketed: `[::1]:22`.
fn parse_addr(s: &str) -> Result<SocketAddr> {
    s.parse::<SocketAddr>().map_err(|_| {
        anyhow::anyhow!("invalid address {s:?} (expected ip:port, e.g. 0.0.0.0:22 or [::1]:22)")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(parts: &[&str]) -> Vec<String> {
        std::iter::once("simple-proxy")
            .chain(parts.iter().copied())
            .map(String::from)
            .collect()
    }

    #[test]
    fn serve_defaults_config() {
        let cmd = parse(argv(&["serve"]).into_iter()).unwrap();
        match cmd {
            Command::Serve { config } => assert_eq!(config, PathBuf::from(DEFAULT_CONFIG)),
            _ => panic!("expected serve"),
        }
    }

    #[test]
    fn serve_with_config_flag() {
        let cmd = parse(argv(&["serve", "--config", "/etc/p.json"]).into_iter()).unwrap();
        match cmd {
            Command::Serve { config } => assert_eq!(config, PathBuf::from("/etc/p.json")),
            _ => panic!("expected serve"),
        }
    }

    #[test]
    fn forward_parses_ipv4() {
        let cmd = parse(argv(&["forward", "0.0.0.0:22", "127.0.0.1:2222"]).into_iter()).unwrap();
        match cmd {
            Command::Forward { source, target } => {
                assert_eq!(source.to_string(), "0.0.0.0:22");
                assert_eq!(target.port(), 2222);
            }
            _ => panic!("expected forward"),
        }
    }

    #[test]
    fn forward_parses_bracketed_ipv6() {
        // The original JS get_address mishandled this; ours must not.
        let cmd = parse(argv(&["forward", "[::1]:22", "[::1]:2222"]).into_iter()).unwrap();
        assert!(matches!(cmd, Command::Forward { .. }));
    }

    #[test]
    fn forward_rejects_missing_port() {
        assert!(parse(argv(&["forward", "1.2.3.4", "5.6.7.8:9"]).into_iter()).is_err());
    }

    #[test]
    fn rejects_unknown_command() {
        assert!(parse(argv(&["frobnicate"]).into_iter()).is_err());
        assert!(parse(argv(&[]).into_iter()).is_err());
    }
}
