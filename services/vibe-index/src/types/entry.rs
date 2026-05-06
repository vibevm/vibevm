//! `VersionEntry` — the canonical per-version index record. Schema
//! pinned in PROP-005 §2.6. Every line of `primary.jsonl` is one of
//! these; every element of `by-name/<kind>/<name>.json::versions[]` is
//! one of these; every `POST /v1/packages` body is one of these.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use semver::Version;
use serde::{Deserialize, Serialize};

use super::kinds::PackageKind;

/// Per-version index record. PROP-005 §2.6.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VersionEntry {
    pub schema_version: u32,

    pub kind: PackageKind,
    pub name: String,
    pub version: Version,

    pub content_hash: String,
    pub source_url: String,
    pub source_ref: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_commit: Option<String>,

    pub registry: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authors: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,

    /// PURL of the upstream library this package documents.
    /// Stored as opaque string; structured parsing happens in the
    /// resolver, not in the index itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub describes: Option<String>,

    #[serde(default, skip_serializing_if = "CompatibilityEntry::is_empty")]
    pub compatibility: CompatibilityEntry,

    #[serde(default, skip_serializing_if = "ProvidesEntry::is_empty")]
    pub provides: ProvidesEntry,

    #[serde(default, skip_serializing_if = "RequiresEntry::is_empty")]
    pub requires: RequiresEntry,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires_any: Vec<RequiresAnyEntry>,

    #[serde(default, skip_serializing_if = "ObsoletesEntry::is_empty")]
    pub obsoletes: ObsoletesEntry,

    #[serde(default, skip_serializing_if = "ConflictsEntry::is_empty")]
    pub conflicts: ConflictsEntry,

    #[serde(default, skip_serializing_if = "FeaturesEntry::is_empty")]
    pub features: FeaturesEntry,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subskills: Vec<SubskillEntry>,

    #[serde(default, skip_serializing_if = "I18nEntry::is_empty")]
    pub i18n: I18nEntry,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boot_snippet: Option<BootSnippetEntry>,

    pub files_count: u32,

    pub indexed_at: DateTime<Utc>,
    pub indexed_by: String,
}

impl VersionEntry {
    pub const SCHEMA_VERSION: u32 = 1;

    /// Stable sort key (kind, name, version).
    pub fn sort_key(&self) -> (PackageKind, &str, &Version) {
        (self.kind, self.name.as_str(), &self.version)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompatibilityEntry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_vibe_version: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires_kinds: Vec<PackageKind>,
}

impl CompatibilityEntry {
    pub fn is_empty(&self) -> bool {
        self.min_vibe_version.is_none() && self.requires_kinds.is_empty()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProvidesEntry {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capabilities: Vec<String>,
}

impl ProvidesEntry {
    pub fn is_empty(&self) -> bool {
        self.capabilities.is_empty()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequiresEntry {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub packages: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capabilities: Vec<String>,
}

impl RequiresEntry {
    pub fn is_empty(&self) -> bool {
        self.packages.is_empty() && self.capabilities.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequiresAnyEntry {
    pub one_of: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObsoletesEntry {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub packages: Vec<String>,
}

impl ObsoletesEntry {
    pub fn is_empty(&self) -> bool {
        self.packages.is_empty()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConflictsEntry {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub packages: Vec<String>,
}

impl ConflictsEntry {
    pub fn is_empty(&self) -> bool {
        self.packages.is_empty()
    }
}

/// Mirror of `vibe-core::manifest::FeaturesTable` — feature names map
/// to activation lists; `exclusive` is the at-most-one-of named-group
/// table.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeaturesEntry {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub features: BTreeMap<String, Vec<String>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub exclusive: BTreeMap<String, Vec<String>>,
}

impl FeaturesEntry {
    pub fn is_empty(&self) -> bool {
        self.features.is_empty() && self.exclusive.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeliveryMode {
    Eager,
    LazyPush,
    LazyPull,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubskillEntry {
    pub path: String,
    pub delivery: DeliveryMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub describes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub channels: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct I18nEntry {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub available: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
}

impl I18nEntry {
    pub fn is_empty(&self) -> bool {
        self.available.is_empty() && self.default.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BootSnippetEntry {
    pub filename: String,
}

/// Aggregated record stored on disk in `by-name/<kind>/<name>.json`.
/// Wraps a `Vec<VersionEntry>` plus per-package metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageEntry {
    pub kind: PackageKind,
    pub name: String,
    pub indexed_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_stable: Option<Version>,
    pub versions: Vec<VersionEntry>,
}

impl PackageEntry {
    pub fn new(kind: PackageKind, name: impl Into<String>, indexed_at: DateTime<Utc>) -> Self {
        PackageEntry {
            kind,
            name: name.into(),
            indexed_at,
            latest_stable: None,
            versions: Vec::new(),
        }
    }

    /// Sort versions ascending and recompute `latest_stable`.
    pub fn finalise(&mut self) {
        self.versions.sort_by(|a, b| a.version.cmp(&b.version));
        self.latest_stable = self
            .versions
            .iter()
            .filter(|v| v.version.pre.is_empty())
            .map(|v| v.version.clone())
            .next_back();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entry() -> VersionEntry {
        VersionEntry {
            schema_version: VersionEntry::SCHEMA_VERSION,
            kind: PackageKind::Flow,
            name: "wal".into(),
            version: "0.1.0".parse().unwrap(),
            content_hash: "sha256:0000".into(),
            source_url: "https://example.invalid/flow-wal.git".into(),
            source_ref: "v0.1.0".into(),
            resolved_commit: Some("abc123".into()),
            registry: "vibespecs".into(),
            license: Some("EULA".into()),
            authors: vec!["Oleg".into()],
            description: Some("WAL discipline".into()),
            homepage: None,
            keywords: vec!["wal".into()],
            describes: None,
            compatibility: CompatibilityEntry::default(),
            provides: ProvidesEntry::default(),
            requires: RequiresEntry::default(),
            requires_any: vec![],
            obsoletes: ObsoletesEntry::default(),
            conflicts: ConflictsEntry::default(),
            features: FeaturesEntry::default(),
            subskills: vec![],
            i18n: I18nEntry::default(),
            boot_snippet: Some(BootSnippetEntry {
                filename: "10-flow-wal.md".into(),
            }),
            files_count: 5,
            indexed_at: DateTime::parse_from_rfc3339("2026-05-06T12:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            indexed_by: "vibe-index 0.1.0-dev".into(),
        }
    }

    #[test]
    fn version_entry_round_trips_through_json() {
        let v = sample_entry();
        let json = serde_json::to_string(&v).unwrap();
        let back: VersionEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(v, back);
    }

    #[test]
    fn empty_subsections_are_omitted() {
        let v = sample_entry();
        let json = serde_json::to_string(&v).unwrap();
        assert!(!json.contains("provides"));
        assert!(!json.contains("requires_any"));
        assert!(!json.contains("subskills"));
    }

    #[test]
    fn package_entry_finalise_picks_latest_stable() {
        let mut p = PackageEntry::new(
            PackageKind::Flow,
            "wal",
            DateTime::parse_from_rfc3339("2026-05-06T12:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
        );
        let mut v1 = sample_entry();
        v1.version = "0.1.0".parse().unwrap();
        let mut v2 = sample_entry();
        v2.version = "0.2.0".parse().unwrap();
        let mut v_pre = sample_entry();
        v_pre.version = "0.3.0-rc.1".parse().unwrap();
        p.versions.push(v2);
        p.versions.push(v1);
        p.versions.push(v_pre);
        p.finalise();
        assert_eq!(p.latest_stable.as_ref().unwrap().to_string(), "0.2.0");
        // versions sorted ascending
        assert_eq!(p.versions[0].version.to_string(), "0.1.0");
        assert_eq!(p.versions[1].version.to_string(), "0.2.0");
        assert_eq!(p.versions[2].version.to_string(), "0.3.0-rc.1");
    }

    #[test]
    fn delivery_mode_serde_kebab() {
        let v = serde_json::to_string(&DeliveryMode::LazyPush).unwrap();
        assert_eq!(v, "\"lazy-push\"");
        let parsed: DeliveryMode = serde_json::from_str("\"lazy-pull\"").unwrap();
        assert_eq!(parsed, DeliveryMode::LazyPull);
    }
}
