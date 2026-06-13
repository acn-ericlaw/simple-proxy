//! Dynamic upstream-IP discovery: run a shell command and scrape an IP out of its
//! output. Ports the JS `getVmIpAddress` (e.g. `multipass exec main ifconfig eth0`
//! with tag `inet`/index 1, or `arp -a` with tag `172.`/index 0).

use anyhow::{anyhow, bail, Context, Result};
use std::net::IpAddr;
use tokio::process::Command;

/// Run `command` through `sh -c`, then parse out the upstream IP. Runs once at startup.
pub async fn resolve(command: &str, tag: &str, index: usize) -> Result<IpAddr> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .await
        .with_context(|| format!("running discovery command {command:?}"))?;

    if output.stdout.is_empty() && !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "{}",
            stderr.lines().next().unwrap_or("discovery command failed")
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let found = parse_discovery(&stdout, tag, index)
        .ok_or_else(|| anyhow!("Unable to resolve IP address. Is VM started?"))?;
    found
        .parse::<IpAddr>()
        .with_context(|| format!("discovery produced {found:?}, which is not a valid IP address"))
}

/// Pure parse step (unit-tested): the first line whose trimmed text starts with `tag`,
/// split on whitespace, field `index`.
pub fn parse_discovery(stdout: &str, tag: &str, index: usize) -> Option<String> {
    stdout
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with(tag))
        .and_then(|line| line.split_whitespace().nth(index))
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ifconfig_inet() {
        // `multipass exec main ifconfig eth0`, tag "inet", index 1
        let out = "eth0: flags=4163<UP,BROADCAST,RUNNING,MULTICAST>  mtu 1500\n\
                   inet 192.168.64.7  netmask 255.255.255.0  broadcast 192.168.64.255\n\
                   inet6 fe80::5054:ff:fe12:3456  prefixlen 64  scopeid 0x20<link>\n";
        assert_eq!(
            parse_discovery(out, "inet", 1).as_deref(),
            Some("192.168.64.7")
        );
    }

    #[test]
    fn parses_arp_table() {
        // Windows-style `arp -a`, tag "172.", index 0
        let out = "Interface: 172.17.0.10 --- 0x5\n  \
                   172.17.0.1            00-15-5d-01-02-03     dynamic\n";
        assert_eq!(
            parse_discovery(out, "172.", 0).as_deref(),
            Some("172.17.0.1")
        );
    }

    #[test]
    fn no_matching_line_returns_none() {
        assert_eq!(parse_discovery("nothing here\n", "inet", 1), None);
    }

    #[test]
    fn index_out_of_range_returns_none() {
        assert_eq!(parse_discovery("inet 1.2.3.4\n", "inet", 9), None);
    }

    #[tokio::test]
    async fn resolve_runs_shell_and_parses() {
        // Echo a fake ifconfig line; tag "inet", index 1 -> the IP.
        let ip = resolve("printf 'inet 10.1.2.3 netmask 255.0.0.0\\n'", "inet", 1)
            .await
            .unwrap();
        assert_eq!(ip, "10.1.2.3".parse::<IpAddr>().unwrap());
    }

    #[tokio::test]
    async fn resolve_rejects_non_ip() {
        let err = resolve("printf 'inet notanip rest\\n'", "inet", 1)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("not a valid IP"));
    }
}
