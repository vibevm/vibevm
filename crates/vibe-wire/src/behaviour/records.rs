//! Behaviour of the catalog records and the aggregates built over
//! them — the schema constant, the identity-only fixture builder, the
//! identity sort key, and the `finalise` passes that make a written
//! record byte-deterministic from its data regardless of insertion
//! order.

use chrono::{DateTime, Utc};
use semver::Version;
use vibe_core::Group;

use crate::generated::index::e1::by_name::{NameEntry, PackageEntry};
use crate::generated::shared::{PackageKind, VersionEntry};

impl VersionEntry {
    pub const SCHEMA_VERSION: u32 = 1;

    /// An entry carrying just the `(kind, group, name, version)`
    /// identity, every other field empty or placeholder — the shape
    /// index tests and doctests reach for when only identity matters.
    /// Production entries are built field-by-field from a manifest
    /// (`vibe-index add`); this is the fixture builder, public so
    /// examples need not restate the whole struct. The clock is an
    /// input like everywhere else: `at` lands in `indexed_at`
    /// verbatim, so a fixture stamped twice with one `at` is
    /// byte-identical (F2-1). Empty projections are absence, not
    /// present-but-empty: the writer never emits `"provides": {}`.
    pub fn minimal(
        kind: PackageKind,
        group: Group,
        name: impl Into<String>,
        version: Version,
        at: DateTime<Utc>,
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
            compatibility: None,
            provides: None,
            requires: None,
            requires_any: Vec::new(),
            obsoletes: None,
            conflicts: None,
            features: None,
            subskills: Vec::new(),
            i18n: None,
            boot_snippet: None,
            files_count: 0,
            must_understand: Vec::new(),
            yanked: false,
            frozen: false,
            indexed_at: at,
            indexed_by: "vibe-index".to_string(),
        }
    }

    /// Stable sort key `(group, name, version)` — the PROP-008 §2.2
    /// identity ordering. `kind` left the key when it left identity.
    pub fn sort_key(&self) -> (&Group, &str, &Version) {
        (&self.group, self.name.as_str(), &self.version)
    }
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

impl NameEntry {
    pub fn new(name: impl Into<String>, indexed_at: DateTime<Utc>) -> Self {
        NameEntry {
            name: name.into(),
            indexed_at,
            packages: Vec::new(),
            tombstone: None,
        }
    }

    /// Sort the candidate packages by `group` and stamp `indexed_at`
    /// with the freshest candidate's, so the on-disk file is
    /// byte-deterministic from its data regardless of insertion order.
    pub fn finalise(&mut self) {
        self.packages.sort_by(|a, b| a.group.cmp(&b.group));
        if let Some(latest) = self.packages.iter().map(|p| p.indexed_at).max() {
            self.indexed_at = latest;
        }
    }
}
