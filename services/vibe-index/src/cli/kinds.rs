//! Package-kind enum — a deliberate duplicate of the four-kind enum
//! that lives in `vibe-core`. PROP-005 §3.2 explains the trade-off:
//! standalone redistribution beats workspace re-use here, so we keep a
//! local copy and parity-test it (slice 3) against `vibe-core` to
//! catch divergence at CI time.

use std::fmt;
use std::str::FromStr;

use clap::ValueEnum;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, ValueEnum)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
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
    fn package_kind_round_trips() {
        for k in [
            PackageKind::Flow,
            PackageKind::Feat,
            PackageKind::Stack,
            PackageKind::Tool,
        ] {
            assert_eq!(<PackageKind as FromStr>::from_str(k.as_str()).unwrap(), k);
        }
    }

    #[test]
    fn package_kind_rejects_unknown() {
        assert!(<PackageKind as FromStr>::from_str("plugin").is_err());
        assert!(<PackageKind as FromStr>::from_str("").is_err());
    }
}
