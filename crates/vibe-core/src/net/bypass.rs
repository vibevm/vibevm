//! The `no_proxy` bypass list (`PROP-068#proxy`).

use std::net::IpAddr;

/// Hosts that connect directly instead of through the proxy: domain
/// suffixes (`corp.example` and `.corp.example` both cover the domain and
/// every host below it; `*.corp.example` likewise), IP addresses, CIDR
/// ranges (`10.0.0.0/8`, `fd00::/8`) and `*` (everything). Loopback —
/// `localhost`, `*.localhost`, `127.0.0.0/8`, `::1` — always bypasses, so a
/// local server is never sent to a corporate proxy. A `host:port` entry
/// matches the host; unparseable entries are ignored, never widened.
///
/// ```
/// use vibe_core::net::Bypass;
/// let bypass = Bypass::parse(".corp.example, git.internal, 10.0.0.0/8, 192.168.1.7");
/// assert!(bypass.matches("corp.example"));
/// assert!(bypass.matches("nexus.corp.example"));
/// assert!(!bypass.matches("corp.example.attacker.test"));
/// assert!(!bypass.matches("evilcorp.example"));
/// assert!(bypass.matches("GIT.internal"));
/// assert!(bypass.matches("10.20.30.40"));
/// assert!(bypass.matches("192.168.1.7"));
/// assert!(!bypass.matches("192.168.1.8"));
/// assert!(!bypass.matches("github.com"));
/// // Loopback is never proxied, whatever the list says.
/// let empty = Bypass::parse("");
/// assert!(empty.matches("127.0.0.1") && empty.matches("localhost") && empty.matches("::1"));
/// assert!(empty.matches("[::1]"));
/// assert!(Bypass::parse("*").matches("github.com"));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Bypass {
    rules: Vec<Rule>,
    all: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Rule {
    /// Lowercase domain without leading dots: matches itself and below.
    Domain(String),
    Ip(IpAddr),
    Cidr(IpAddr, u8),
}

impl Bypass {
    /// Parse a comma- or whitespace-separated list.
    pub fn parse(list: &str) -> Self {
        let mut bypass = Bypass::default();
        for entry in list
            .split(|c: char| c == ',' || c.is_whitespace())
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
        {
            if entry == "*" {
                bypass.all = true;
                continue;
            }
            if let Some((address, bits)) = entry.split_once('/') {
                if let (Ok(address), Ok(bits)) = (address.parse::<IpAddr>(), bits.parse::<u8>()) {
                    let max = if address.is_ipv4() { 32 } else { 128 };
                    if bits <= max {
                        bypass.rules.push(Rule::Cidr(address, bits));
                    }
                }
                continue;
            }
            let bare = entry.trim_start_matches('[');
            let bare = bare.split(']').next().unwrap_or(bare);
            if let Ok(address) = bare.parse::<IpAddr>() {
                bypass.rules.push(Rule::Ip(address));
                continue;
            }
            let host = entry.rsplit_once(':').map_or(entry, |(host, port)| {
                if port.chars().all(|c| c.is_ascii_digit()) {
                    host
                } else {
                    entry
                }
            });
            let domain = host
                .trim_start_matches("*.")
                .trim_start_matches('.')
                .trim_end_matches('.')
                .to_ascii_lowercase();
            if !domain.is_empty() && !domain.contains(['/', '@', '*']) {
                bypass.rules.push(Rule::Domain(domain));
            }
        }
        bypass
    }

    /// Whether `host` (a URL host: a name, an IPv4 address or an IPv6
    /// address with or without brackets) connects directly.
    pub fn matches(&self, host: &str) -> bool {
        let host = host.trim_start_matches('[').trim_end_matches(']');
        let host = host.trim_end_matches('.').to_ascii_lowercase();
        if let Ok(address) = host.parse::<IpAddr>() {
            if address.is_loopback() || self.all {
                return true;
            }
            return self.rules.iter().any(|rule| match rule {
                Rule::Ip(ip) => *ip == address,
                Rule::Cidr(network, bits) => in_cidr(address, *network, *bits),
                Rule::Domain(_) => false,
            });
        }
        if host == "localhost" || host.ends_with(".localhost") || self.all {
            return true;
        }
        self.rules.iter().any(|rule| match rule {
            Rule::Domain(domain) => {
                host == *domain
                    || host
                        .strip_suffix(domain.as_str())
                        .is_some_and(|prefix| prefix.ends_with('.'))
            }
            Rule::Ip(_) | Rule::Cidr(..) => false,
        })
    }
}

fn in_cidr(address: IpAddr, network: IpAddr, bits: u8) -> bool {
    match (address, network) {
        (IpAddr::V4(a), IpAddr::V4(n)) => {
            let mask = u32::MAX.checked_shl(32 - u32::from(bits)).unwrap_or(0);
            u32::from(a) & mask == u32::from(n) & mask
        }
        (IpAddr::V6(a), IpAddr::V6(n)) => {
            let mask = u128::MAX.checked_shl(128 - u32::from(bits)).unwrap_or(0);
            u128::from(a) & mask == u128::from(n) & mask
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cidr_edges_and_ports() {
        let bypass = Bypass::parse("10.1.0.0/16 fd00::/8 0.0.0.0/0x nexus.corp.example:8443");
        assert!(bypass.matches("10.1.255.255"));
        assert!(!bypass.matches("10.2.0.0"));
        assert!(bypass.matches("fd12::1"));
        assert!(!bypass.matches("fe80::1"));
        assert!(bypass.matches("nexus.corp.example"));
        // The malformed CIDR is ignored, not widened to everything.
        assert!(!bypass.matches("8.8.8.8"));
        assert!(Bypass::parse("0.0.0.0/0").matches("8.8.8.8"));
    }

    #[test]
    fn wildcards_and_trailing_dots_do_not_bypass_lookalikes() {
        let bypass = Bypass::parse("*.corp.example");
        assert!(bypass.matches("a.corp.example."));
        assert!(bypass.matches("corp.example"));
        assert!(!bypass.matches("xcorp.example"));
        assert!(!Bypass::parse("a*b").matches("ab"));
    }
}
