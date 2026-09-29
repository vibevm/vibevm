//! Node identity and filesystem-scope identity (plan D19, invariant I7).
//!
//! Every agent can learn **which machine it runs on** and **where its
//! filesystem comes from**. Node identity is a stable machine id plus
//! human-facing labels (hostname, addresses — labels, never identity:
//! DHCP, multiple NICs and VPNs disqualify raw IPs as keys). Scope
//! identity is behavioral: two parties share a filesystem scope **iff
//! they read the same live rendezvous beacon** stamped at the scope
//! root — a proof that sidesteps every platform quirk; mount metadata is
//! corroboration and diagnostics only.
//!
//! This module holds the wire/file **types**; detection (registry reads,
//! hostname, beacon stamping) lives in mission-control, which owns the
//! scopes.

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::ids::ScopeId;

specmark::scope!("spec://fractality/PROP-001#invariants");

/// Identity of the machine a fractality process runs on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeIdentity {
    /// Stable machine id: `MachineGuid` (Windows), `/etc/machine-id`
    /// (Linux), `IOPlatformUUID` (macOS). Falls back to a
    /// hostname-derived id (`host:<hostname>`) when the platform source
    /// is unreadable — degraded, and marked so by the prefix.
    pub node_id: String,
    pub hostname: String,
    /// `std::env::consts::OS` at detection time.
    pub os: String,
    /// Human-facing labels only (D19: raw IPs are labels, not identity).
    #[serde(default)]
    pub addresses: Vec<String>,
}

/// One filesystem scope as mission-control advertises it
/// (`GET /v0/node`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScopeInfo {
    pub id: ScopeId,
    /// Scope root on this node. Paths inside FileRefs are relative to
    /// this root; the root itself is node-local knowledge.
    pub root: Utf8PathBuf,
    /// The live beacon nonce (rotates; equality of nonces read by two
    /// parties is the scope-sharing proof).
    pub nonce: String,
    pub issued_ts_ms: u64,
}

/// The rendezvous beacon file stamped at a scope root
/// (`<root>/.fractality-fsid`). TOML on disk; rotated by
/// mission-control (v0.1: on every MC start).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScopeBeacon {
    /// Beacon schema; this build writes and reads `1`.
    pub schema: u32,
    pub scope_id: ScopeId,
    /// Random nonce minted at stamping; the liveness proof.
    pub nonce: String,
    /// The mission-control instance that stamped it.
    pub mc_id: String,
    pub issued_ts_ms: u64,
}

impl ScopeBeacon {
    /// File name of the beacon inside its scope root.
    pub const FILE_NAME: &'static str = ".fractality-fsid";

    pub fn to_toml_string(&self) -> Result<String, CoreError> {
        Ok(toml::to_string_pretty(self)?)
    }

    pub fn from_toml_str(text: &str) -> Result<Self, CoreError> {
        Ok(toml::from_str(text)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beacon_round_trips_through_toml() {
        let b = ScopeBeacon {
            schema: 1,
            scope_id: ScopeId::new("01ARZ3NDEKTSV4RRFFQ69G5FAV"),
            nonce: "01BX5ZZKBKACTAV9WEVGEMMVRY".into(),
            mc_id: "mc-01ARZ3NDEKTSV4RRFFQ69G5FAV".into(),
            issued_ts_ms: 1_751_800_000_000,
        };
        let text = b.to_toml_string().expect("renders");
        let back = ScopeBeacon::from_toml_str(&text).expect("parses");
        assert_eq!(b, back);
    }

    #[test]
    fn beacon_rejects_unknown_fields() {
        let text = "schema = 1\nscope_id = \"s\"\nnonce = \"n\"\nmc_id = \"m\"\nissued_ts_ms = 1\nextra = true\n";
        assert!(ScopeBeacon::from_toml_str(text).is_err());
    }
}
