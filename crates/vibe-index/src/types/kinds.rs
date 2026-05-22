//! Package-kind enum — a deliberate duplicate of the four-kind enum
//! that lives in `vibe-core`. PROP-005 §3.2 explains the trade-off:
//! standalone redistribution beats workspace re-use here, so we keep a
//! local copy and parity-test it (slice 3) against `vibe-core` to
//! catch divergence at CI time.

use std::fmt;
use std::str::FromStr;

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, ValueEnum, Serialize, Deserialize,
)]
#[serde(rename_all = "lowercase")]
#[value(rename_all = "kebab-case")]
pub enum PackageKind {
    Flow,
    Feat,
    Stack,
    Tool,
}

impl PackageKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            PackageKind::Flow => "flow",
            PackageKind::Feat => "feat",
            PackageKind::Stack => "stack",
            PackageKind::Tool => "tool",
        }
    }

    pub fn all() -> &'static [PackageKind] {
        &[
            PackageKind::Flow,
            PackageKind::Feat,
            PackageKind::Stack,
            PackageKind::Tool,
        ]
    }
}

impl fmt::Display for PackageKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for PackageKind {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "flow" => Ok(PackageKind::Flow),
            "feat" => Ok(PackageKind::Feat),
            "stack" => Ok(PackageKind::Stack),
            "tool" => Ok(PackageKind::Tool),
            other => Err(format!(
                "unknown package kind `{other}` — expected one of: flow, feat, stack, tool"
            )),
        }
    }
}

/// Naming convention used to map `<kind>:<name>` to a repo name under
/// the registry's org root. Mirrors `vibe-core::manifest::NamingConvention`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[value(rename_all = "kebab-case")]
pub enum NamingConvention {
    /// `flow:wal` → `<org>/flow-wal` (default for GitHub-shape orgs).
    KindName,
    /// `flow:wal` → `<org>/wal` (default for orgs that provision repos
    /// without a kind prefix — e.g. `vibespecs` on GitVerse).
    Name,
}

impl NamingConvention {
    pub fn as_str(&self) -> &'static str {
        match self {
            NamingConvention::KindName => "kind-name",
            NamingConvention::Name => "name",
        }
    }

    pub fn repo_name(&self, kind: PackageKind, name: &str) -> String {
        match self {
            NamingConvention::KindName => format!("{kind}-{name}"),
            NamingConvention::Name => name.to_string(),
        }
    }
}

impl fmt::Display for NamingConvention {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_kind_round_trips_string() {
        for k in PackageKind::all() {
            assert_eq!(<PackageKind as FromStr>::from_str(k.as_str()).unwrap(), *k);
        }
    }

    #[test]
    fn package_kind_rejects_unknown() {
        assert!(<PackageKind as FromStr>::from_str("plugin").is_err());
        assert!(<PackageKind as FromStr>::from_str("").is_err());
    }

    #[test]
    fn package_kind_serde_lowercase() {
        let json = serde_json::to_string(&PackageKind::Flow).unwrap();
        assert_eq!(json, "\"flow\"");
        let parsed: PackageKind = serde_json::from_str("\"feat\"").unwrap();
        assert_eq!(parsed, PackageKind::Feat);
    }

    #[test]
    fn naming_convention_serde_kebab() {
        let json = serde_json::to_string(&NamingConvention::KindName).unwrap();
        assert_eq!(json, "\"kind-name\"");
        let parsed: NamingConvention = serde_json::from_str("\"name\"").unwrap();
        assert_eq!(parsed, NamingConvention::Name);
    }

    #[test]
    fn repo_name_composes_kind_name() {
        assert_eq!(
            NamingConvention::KindName.repo_name(PackageKind::Flow, "wal"),
            "flow-wal"
        );
        assert_eq!(
            NamingConvention::Name.repo_name(PackageKind::Flow, "wal"),
            "wal"
        );
    }
}
