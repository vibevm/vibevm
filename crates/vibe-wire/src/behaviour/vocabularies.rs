//! Behaviour of the open wire vocabularies — the strings a value
//! spells on the wire, the finite set a build can name, and the laws
//! a string→value parse follows.

use std::fmt;
use std::str::FromStr;

use vibe_core::Group;

use crate::generated::shared::{NamingConvention, PackageKind};

impl PackageKind {
    /// The wire string for this kind. An `Unknown` value returns the
    /// string it arrived with, verbatim — an older build never drops
    /// or rewrites a newer writer's vocabulary (PROP-044 §4.2a).
    pub fn as_str(&self) -> &str {
        match self {
            PackageKind::Flow => "flow",
            PackageKind::Feat => "feat",
            PackageKind::Stack => "stack",
            PackageKind::Tool => "tool",
            PackageKind::Mcp => "mcp",
            PackageKind::Lang => "lang",
            PackageKind::Unknown(value) => value.as_str(),
        }
    }

    /// The kinds this build knows. The vocabulary is open — a value
    /// this build does not know still parses (as `Unknown`), so an
    /// honest "all" stopped existing the moment the dictionary
    /// opened. This is the finite set a build can NAME: help text,
    /// summary tables, CLI error messages.
    pub fn known() -> &'static [PackageKind] {
        static KNOWN: [PackageKind; 6] = [
            PackageKind::Flow,
            PackageKind::Feat,
            PackageKind::Stack,
            PackageKind::Tool,
            PackageKind::Mcp,
            PackageKind::Lang,
        ];
        &KNOWN
    }
}

impl fmt::Display for PackageKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Total by design: the vocabulary is OPEN, so every string names a
/// kind — an unfamiliar one arrives as `Unknown(s)` carrying the
/// string verbatim, exactly as the wire reader would receive it.
/// Whether an unfamiliar kind is ACCEPTABLE is the caller's law, not
/// the parser's: on the wire it is normal life (the future must be
/// tolerable there, PROP-044 §4.2a), while a CLI argument refuses
/// with a message naming the known set — the argument speaks, it does
/// not filter to silence.
impl FromStr for PackageKind {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "flow" => PackageKind::Flow,
            "feat" => PackageKind::Feat,
            "stack" => PackageKind::Stack,
            "tool" => PackageKind::Tool,
            "mcp" => PackageKind::Mcp,
            "lang" => PackageKind::Lang,
            other => PackageKind::Unknown(other.to_string()),
        })
    }
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
    /// convention. Infallible by construction: an index entry always
    /// carries a concrete `kind`, so even the legacy `kind-*`
    /// conventions have what they need.
    pub fn repo_name(&self, kind: &PackageKind, group: &Group, name: &str) -> String {
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

impl Default for NamingConvention {
    /// `fqdn` — the reverse-FQDN `<group>.<name>` shape (PROP-008
    /// §2.5). Collision-free because `(group, name)` is unique; every
    /// group-native registry uses it.
    fn default() -> Self {
        NamingConvention::Fqdn
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_kinds_round_trip_their_wire_strings() {
        for k in PackageKind::known() {
            let text = k.as_str();
            let back = PackageKind::from_str(text).unwrap();
            assert_eq!(&back, k);
        }
    }

    #[test]
    fn unfamiliar_kind_is_preserved_verbatim_by_the_parse() {
        // The parse is total because the vocabulary is open: an
        // unfamiliar string arrives as `Unknown` carrying it verbatim,
        // exactly as the wire reader would hold it. Refusing is the
        // ARGUMENT boundary's business, not the parser's.
        let parsed = PackageKind::from_str("plugin").unwrap();
        assert_eq!(parsed, PackageKind::Unknown("plugin".to_string()));
        assert_eq!(parsed.as_str(), "plugin");
        // …and the preserved value survives a read/write cycle.
        let json = serde_json::to_string(&parsed).unwrap();
        assert_eq!(json, "\"plugin\"");
        let back: PackageKind = serde_json::from_str(&json).unwrap();
        assert_eq!(back, parsed);
    }

    #[test]
    fn known_list_names_every_variant_and_nothing_else() {
        let known = PackageKind::known();
        assert_eq!(known.len(), 6);
        for k in known {
            assert!(
                !matches!(k, PackageKind::Unknown(_)),
                "`known()` must not claim an unknown value"
            );
        }
    }

    #[test]
    fn package_kind_serde_lowercase() {
        let json = serde_json::to_string(&PackageKind::Flow).unwrap();
        assert_eq!(json, "\"flow\"");
        let parsed: PackageKind = serde_json::from_str("\"feat\"").unwrap();
        assert_eq!(parsed, PackageKind::Feat);
    }

    #[test]
    fn naming_convention_serde_matches_the_registry_wire() {
        // The `repomd.json` `naming` value must read exactly as a
        // `[[registry]].naming` does — same four wire strings, the
        // explicit per-variant renames.
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
        let flow = PackageKind::Flow;
        assert_eq!(
            NamingConvention::Fqdn.repo_name(&flow, &group, "wal"),
            "org.vibevm.wal"
        );
        assert_eq!(
            NamingConvention::KindName.repo_name(&flow, &group, "wal"),
            "flow-wal"
        );
        assert_eq!(
            NamingConvention::Name.repo_name(&flow, &group, "wal"),
            "wal"
        );
        assert_eq!(
            NamingConvention::KindSlashName.repo_name(&flow, &group, "wal"),
            "flow/wal"
        );
    }
}
