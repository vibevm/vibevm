//! Node-identity detection (plan D19, invariant I7).
//!
//! The stable key is the OS machine id; hostname and addresses are
//! human-facing labels (never identity — DHCP, multiple NICs, VPNs).
//! Detection degrades loudly: a hostname-derived id carries the `host:`
//! prefix so a reader can tell a first-class id from a fallback.

use fractality_core::node::NodeIdentity;

specmark::scope!("spec://fractality/PROP-001#invariants");

/// Detects this machine's identity. Infallible by design — every source
/// has a documented fallback.
pub fn detect() -> NodeIdentity {
    let hostname = hostname();
    NodeIdentity {
        node_id: machine_id().unwrap_or_else(|| format!("host:{hostname}")),
        hostname,
        os: std::env::consts::OS.to_owned(),
        addresses: primary_address().into_iter().collect(),
    }
}

fn hostname() -> String {
    sysinfo::System::host_name()
        .or_else(|| {
            std::env::var("COMPUTERNAME")
                .ok()
                .or_else(|| std::env::var("HOSTNAME").ok())
        })
        .unwrap_or_else(|| "unknown-host".to_owned())
}

/// `MachineGuid` on Windows.
#[cfg(windows)]
fn machine_id() -> Option<String> {
    let hklm = winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE);
    let key = hklm.open_subkey("SOFTWARE\\Microsoft\\Cryptography").ok()?;
    let guid: String = key.get_value("MachineGuid").ok()?;
    let guid = guid.trim().to_owned();
    (!guid.is_empty()).then_some(guid)
}

/// `/etc/machine-id` on Linux (macOS support arrives with the CI matrix,
/// DEF-8; the hostname fallback keeps it honest until then).
#[cfg(unix)]
fn machine_id() -> Option<String> {
    let text = std::fs::read_to_string("/etc/machine-id").ok()?;
    let id = text.trim().to_owned();
    (!id.is_empty()).then_some(id)
}

/// Best-effort primary route address: a UDP socket "connected" to a
/// public address reveals which local address the OS would route from.
/// No packet is sent. Failure means no label, never an error.
fn primary_address() -> Option<String> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    Some(socket.local_addr().ok()?.ip().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detection_yields_a_nonempty_stable_id() {
        let node = detect();
        assert!(!node.node_id.trim().is_empty());
        assert!(!node.hostname.trim().is_empty());
        assert_eq!(node.os, std::env::consts::OS);
    }
}
