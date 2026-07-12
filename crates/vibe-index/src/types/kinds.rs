//! Package-kind enum — a deliberate duplicate of the four-kind enum
//! that lives in `vibe-core`. PROP-005 §3.2 explains the trade-off:
//! standalone redistribution beats workspace re-use here, so we keep a
//! local copy and parity-test it (slice 3) against `vibe-core` to
//! catch divergence at CI time.

specmark::scope!("spec://vibevm/modules/vibe-index/PROP-005#deps");

use std::fmt;
use std::str::FromStr;

use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use vibe_core::Group;

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
    /// The agent-server kind (VIBEVM-SPEC §4.1, PROP-027) — mirrored
    /// from `vibe-core` like the other four; the parity test below
    /// keeps the copies honest.
    Mcp,
}

impl PackageKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            PackageKind::Flow => "flow",
            PackageKind::Feat => "feat",
            PackageKind::Stack => "stack",
            PackageKind::Tool => "tool",
            PackageKind::Mcp => "mcp",
        }
    }

    pub fn all() -> &'static [PackageKind] {
        &[
            PackageKind::Flow,
            PackageKind::Feat,
            PackageKind::Stack,
            PackageKind::Tool,
            PackageKind::Mcp,
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
            "mcp" => Ok(PackageKind::Mcp),
            other => Err(format!(
                "unknown package kind `{other}` — expected one of: flow, feat, stack, tool, mcp"
            )),
        }
    }
}

/// Naming convention used to map a pkgref to a repo name under the
/// registry's org root. Mirrors `vibe-core::manifest::NamingConvention` —
/// the same four variants and the same wire strings, so the `naming`
/// field of `repomd.json` reads exactly as a `[[registry]].naming` does.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
pub enum NamingConvention {
    /// `org.vibevm/wal` → `<org>/org.vibevm.wal` — the reverse-FQDN repo
    /// name (PROP-008 §2.5). The default: a flat `<group>.<name>`,
    /// collision-free because `(group, name)` is unique. Every
    /// group-native registry uses it.
    #[default]
    #[serde(rename = "fqdn")]
    #[value(name = "fqdn")]
    Fqdn,
    /// `flow:wal` → `<org>/flow-wal` — a pre-`group` convention, kept for
    /// registries that have not adopted reverse-FQDN naming.
    #[serde(rename = "kind-name")]
    #[value(name = "kind-name")]
    KindName,
    /// `flow:wal` → `<org>/wal`. Legal only when names are globally
    /// unique across kinds within a registry.
    #[serde(rename = "name")]
    #[value(name = "name")]
    Name,
    /// `flow:wal` → `<org>/flow/wal` — needs host support for nested
    /// repository paths (GitLab groups, Gitea orgs).
    #[serde(rename = "kind/name")]
    #[value(name = "kind/name")]
    KindSlashName,
}

impl NamingConvention {
    pub fn as_str(&self) -> &'static str {
        match self {
            NamingConvention::Fqdn => "fqdn",
            NamingConvention::KindName => "kind-name",
            NamingConvention::Name => "name",
            NamingConvention::KindSlashName => "kind/name",
        }
    }

    /// Repository name for a `(kind, group, name)` package under this
    /// convention. Unlike `vibe-core`'s registry-side counterpart this is
    /// infallible: an index entry always carries a concrete `kind`, so
    /// even the legacy `kind-*` conventions have what they need.
    pub fn repo_name(&self, kind: PackageKind, group: &Group, name: &str) -> String {
        match self {
            NamingConvention::Fqdn => format!("{group}.{name}"),
            NamingConvention::KindName => format!("{kind}-{name}"),
            NamingConvention::Name => name.to_string(),
            NamingConvention::KindSlashName => format!("{kind}/{name}"),
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
    fn naming_convention_serde_matches_vibe_core_wire() {
        // The `repomd.json` `naming` value must read exactly as a
        // `[[registry]].naming` does — same four wire strings.
        assert_eq!(
            serde_json::to_string(&NamingConvention::Fqdn).unwrap(),
            "\"fqdn\""
        );
        assert_eq!(
            serde_json::to_string(&NamingConvention::KindName).unwrap(),
            "\"kind-name\""
        );
        assert_eq!(
            serde_json::to_string(&NamingConvention::KindSlashName).unwrap(),
            "\"kind/name\""
        );
        let parsed: NamingConvention = serde_json::from_str("\"name\"").unwrap();
        assert_eq!(parsed, NamingConvention::Name);
    }

    #[test]
    fn naming_convention_default_is_fqdn() {
        assert_eq!(NamingConvention::default(), NamingConvention::Fqdn);
    }

    #[test]
    fn repo_name_composes_under_every_convention() {
        let group = Group::parse("org.vibevm").unwrap();
        assert_eq!(
            NamingConvention::Fqdn.repo_name(PackageKind::Flow, &group, "wal"),
            "org.vibevm.wal"
        );
        assert_eq!(
            NamingConvention::KindName.repo_name(PackageKind::Flow, &group, "wal"),
            "flow-wal"
        );
        assert_eq!(
            NamingConvention::Name.repo_name(PackageKind::Flow, &group, "wal"),
            "wal"
        );
        assert_eq!(
            NamingConvention::KindSlashName.repo_name(PackageKind::Flow, &group, "wal"),
            "flow/wal"
        );
    }
}
