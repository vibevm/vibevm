//! `VersionEntry` — the canonical per-version index record. Schema
//! pinned in PROP-005 §2.6. Every line of `primary.jsonl` is one of
//! these; every element of a `by-name/<name>.json` candidate's
//! `versions[]` is one of these; every `POST /v1/packages` body is one
//! of these.

specmark::scope!("spec://vibevm/modules/vibe-index/PROP-005#entry");

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use semver::Version;
use serde::{Deserialize, Serialize};
use specmark::spec;
use vibe_core::Group;

use super::kinds::PackageKind;

/// Per-version index record. PROP-005 §2.6.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[spec(implements = "spec://vibevm/modules/vibe-index/PROP-005#entry", r = 1)]
pub struct VersionEntry {
    pub schema_version: u32,

    pub kind: PackageKind,
    /// Reverse-FQDN namespace qualifier (PROP-008 §2.1). With `name` it
    /// forms the package's identity — `name` is unique within a `group`,
    /// so `(group, name)` identifies a package without `kind` (PROP-008
    /// §2.2). `kind` stays on the entry as pure metadata (PROP-008 §2.3).
    pub group: Group,
    pub name: String,
    pub version: Version,

    pub content_hash: String,
    pub source_url: String,
    pub source_ref: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_commit: Option<String>,

    pub registry: String,

    /// Workspace provenance — set when the package was published from a
    /// workspace member, carrying that member's `[origin]` marker
    /// (PROP-007 §2.8, PROP-008 §2.8). `None` for a standalone publish.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_origin: Option<WorkspaceOriginEntry>,

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

    /// An entry carrying just the `(kind, group, name, version)` identity,
    /// every other field empty or placeholder — the shape index tests and
    /// doctests reach for when only identity matters. Production entries
    /// are built field-by-field from a manifest (`vibe-index add`); this
    /// is the fixture builder, public so examples need not restate the
    /// whole struct.
    pub fn minimal(
        kind: PackageKind,
        group: Group,
        name: impl Into<String>,
        version: Version,
    ) -> Self {
        VersionEntry {
            schema_version: Self::SCHEMA_VERSION,
            kind,
            group,
            name: name.into(),
            version,
            content_hash: "sha256:0".to_string(),
            source_url: String::new(),
            source_ref: String::new(),
            resolved_commit: None,
            registry: String::new(),
            workspace_origin: None,
            license: None,
            authors: Vec::new(),
            description: None,
            homepage: None,
            keywords: Vec::new(),
            describes: None,
            compatibility: CompatibilityEntry::default(),
            provides: ProvidesEntry::default(),
            requires: RequiresEntry::default(),
            requires_any: Vec::new(),
            obsoletes: ObsoletesEntry::default(),
            conflicts: ConflictsEntry::default(),
            features: FeaturesEntry::default(),
            subskills: Vec::new(),
            i18n: I18nEntry::default(),
            boot_snippet: None,
            files_count: 0,
            indexed_at: Utc::now(),
            indexed_by: "vibe-index".to_string(),
        }
    }

    /// Stable sort key `(group, name, version)` — the PROP-008 §2.2
    /// identity ordering. `kind` left the key when it left identity.
    pub fn sort_key(&self) -> (&Group, &str, &Version) {
        (&self.group, self.name.as_str(), &self.version)
    }
}

/// `[origin]` projection — the provenance marker `vibe workspace publish`
/// writes into a published workspace-member copy (PROP-007 §2.8). Mirrors
/// `vibe-core::manifest::OriginSection`; surfaced in the index so the
/// registry explorer can trace a package repository back to the monorepo
/// it was generated from (PROP-008 §2.8 / §2.9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceOriginEntry {
    /// URL of the source monorepo the published copy was generated from.
    pub upstream: String,
    /// Path of the package directory within that monorepo.
    pub path: String,
    /// Monorepo commit at generation time — present when it was a git repo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    /// Tool identity that generated the copy — e.g. `vibe 0.1.0`.
    pub generated_by: String,
    /// ISO-8601 timestamp of generation.
    pub generated_at: String,
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

/// `[boot_snippet]` projection (PROP-005 §2.6). M1.18's loading model
/// (PROP-009 §2.5) retired the author-chosen `filename`; a snippet is
/// now identified by its `source` path inside the package plus an
/// ordering `category`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BootSnippetEntry {
    /// Path to the boot file inside the package, e.g. `boot/10-flow-wal.md`.
    pub source: String,
    /// Ordering band for the computed boot sequence — `foundation` /
    /// `flow` / `stack` / `user-override`. Absent when the package
    /// declares none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
}

/// One package — every indexed version of a single `(group, name)`
/// identity (PROP-008 §2.2). A [`NameEntry`] holds the candidate set:
/// every `PackageEntry` that shares one bare `name`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[spec(implements = "spec://vibevm/modules/vibe-index/PROP-005#entry", r = 1)]
pub struct PackageEntry {
    pub group: Group,
    pub name: String,
    pub indexed_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_stable: Option<Version>,
    pub versions: Vec<VersionEntry>,
}

impl PackageEntry {
    pub fn new(group: Group, name: impl Into<String>, indexed_at: DateTime<Utc>) -> Self {
        PackageEntry {
            group,
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

/// `by-name/<name>.json` — the candidate set for one bare package name
/// (PROP-008 §2.8). A single HTTP GET per registry yields every package
/// sharing the short name `<name>`, each carrying its own `group`; this
/// is what makes CLI short-name resolution (PROP-008 §2.6) one round-trip
/// per registry and lets a collision (PROP-008 §2.7) be detected at once.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[spec(implements = "spec://vibevm/modules/vibe-index/PROP-005#entry", r = 1)]
pub struct NameEntry {
    pub name: String,
    pub indexed_at: DateTime<Utc>,
    /// One entry per `group` that publishes a package called `name`,
    /// sorted by `group`. A length greater than one is a short-name
    /// collision (PROP-008 §2.7).
    pub packages: Vec<PackageEntry>,
}

impl NameEntry {
    pub fn new(name: impl Into<String>, indexed_at: DateTime<Utc>) -> Self {
        NameEntry {
            name: name.into(),
            indexed_at,
            packages: Vec::new(),
        }
    }

    /// Sort the candidate packages by `group` and stamp `indexed_at` with
    /// the freshest candidate's, so the on-disk file is byte-deterministic
    /// from its data regardless of insertion order.
    pub fn finalise(&mut self) {
        self.packages.sort_by(|a, b| a.group.cmp(&b.group));
        if let Some(latest) = self.packages.iter().map(|p| p.indexed_at).max() {
            self.indexed_at = latest;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use specmark::verifies;

    fn sample_entry() -> VersionEntry {
        VersionEntry {
            schema_version: VersionEntry::SCHEMA_VERSION,
            kind: PackageKind::Flow,
            group: Group::parse("org.vibevm").unwrap(),
            name: "wal".into(),
            version: "0.1.0".parse().unwrap(),
            content_hash: "sha256:0000".into(),
            source_url: "https://example.invalid/flow-wal.git".into(),
            source_ref: "v0.1.0".into(),
            resolved_commit: Some("abc123".into()),
            registry: "vibespecs".into(),
            workspace_origin: None,
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
                source: "boot/10-flow-wal.md".into(),
                category: Some("flow".into()),
            }),
            files_count: 5,
            indexed_at: DateTime::parse_from_rfc3339("2026-05-06T12:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            indexed_by: "vibe-index 0.1.0-dev".into(),
        }
    }

    #[test]
    #[verifies("spec://vibevm/modules/vibe-index/PROP-005#entry", r = 1)]
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
            Group::parse("org.vibevm").unwrap(),
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

    #[test]
    fn workspace_origin_round_trips_through_json() {
        let mut v = sample_entry();
        v.workspace_origin = Some(WorkspaceOriginEntry {
            upstream: "https://github.com/you/monorepo".into(),
            path: "packages/flow-wal".into(),
            commit: Some("abc123".into()),
            generated_by: "vibe 0.1.0".into(),
            generated_at: "2026-05-20T00:00:00Z".into(),
        });
        let json = serde_json::to_string(&v).unwrap();
        assert!(json.contains("workspace_origin"));
        let back: VersionEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(v, back);
    }

    #[test]
    fn name_entry_finalise_sorts_candidates_by_group() {
        let now = DateTime::parse_from_rfc3339("2026-05-06T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let mut ne = NameEntry::new("wal", now);
        ne.packages.push(PackageEntry::new(
            Group::parse("org.vibevm").unwrap(),
            "wal",
            now,
        ));
        ne.packages.push(PackageEntry::new(
            Group::parse("com.acme").unwrap(),
            "wal",
            now,
        ));
        ne.finalise();
        assert_eq!(ne.packages[0].group.as_str(), "com.acme");
        assert_eq!(ne.packages[1].group.as_str(), "org.vibevm");
        let json = serde_json::to_string(&ne).unwrap();
        let back: NameEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(ne, back);
    }

    #[test]
    fn sort_key_orders_by_group_then_name_then_version() {
        let mut a = sample_entry();
        a.group = Group::parse("com.acme").unwrap();
        let b = sample_entry(); // org.vibevm
        // com.acme sorts before org.vibevm regardless of name.
        assert!(a.sort_key() < b.sort_key());
    }
}
