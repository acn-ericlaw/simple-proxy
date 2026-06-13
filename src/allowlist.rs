//! Inbound IP allow-list. Entries are either exact IPs (`127.0.0.1`) or `x.y.z.*`
//! wildcard prefixes — kept string-based for back-compat with the JS config format
//! (not CIDR).

use std::net::IpAddr;

/// Returns true if `remote` is permitted by `authorized`.
pub fn is_authorized(remote: IpAddr, authorized: &[String]) -> bool {
    let ip = normalize(remote).to_string();
    authorized.iter().any(|entry| {
        if entry == &ip {
            return true;
        }
        match entry.strip_suffix(".*") {
            Some(prefix) => ip.starts_with(&format!("{prefix}.")),
            None => false,
        }
    })
}

/// Collapse IPv4-mapped IPv6 addresses (`::ffff:a.b.c.d`) to plain IPv4 so dotted-quad
/// allow-list entries still match if the listener ever runs dual-stack.
fn normalize(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => v6
            .to_ipv4_mapped()
            .map(IpAddr::V4)
            .unwrap_or(IpAddr::V6(v6)),
        v4 => v4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    fn list(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    fn v4(s: &str) -> IpAddr {
        IpAddr::V4(s.parse::<Ipv4Addr>().unwrap())
    }

    #[test]
    fn exact_match() {
        let allow = list(&["127.0.0.1"]);
        assert!(is_authorized(v4("127.0.0.1"), &allow));
        assert!(!is_authorized(v4("127.0.0.2"), &allow));
    }

    #[test]
    fn wildcard_prefix() {
        let allow = list(&["192.168.1.*"]);
        assert!(is_authorized(v4("192.168.1.5"), &allow));
        assert!(is_authorized(v4("192.168.1.255"), &allow));
        // Must NOT leak into the adjacent /24.
        assert!(!is_authorized(v4("192.168.10.5"), &allow));
        assert!(!is_authorized(v4("192.168.2.5"), &allow));
    }

    #[test]
    fn empty_list_rejects_all() {
        assert!(!is_authorized(v4("127.0.0.1"), &[]));
    }

    #[test]
    fn ipv4_mapped_ipv6_is_normalized() {
        let allow = list(&["192.168.1.*", "127.0.0.1"]);
        // ::ffff:192.168.1.5
        let mapped = IpAddr::V6(Ipv4Addr::new(192, 168, 1, 5).to_ipv6_mapped());
        assert!(is_authorized(mapped, &allow));
        let mapped_local = IpAddr::V6(Ipv4Addr::new(127, 0, 0, 1).to_ipv6_mapped());
        assert!(is_authorized(mapped_local, &allow));
    }

    #[test]
    fn genuine_ipv6_not_falsely_matched() {
        let allow = list(&["192.168.1.*"]);
        assert!(!is_authorized(IpAddr::V6(Ipv6Addr::LOCALHOST), &allow));
    }
}
