//! `VersionEntry` — the canonical per-version index record. Schema
//! pinned in PROP-005 §2.6. Every line of `primary.jsonl` is one of
//! these; every element of a `by-name/<name>.json` candidate's
//! `versions[]` is one of these; every `POST /v1/packages` body is one
//! of these.
//!
//! The per-version projections this record carries are split by concern:
//! dependency relations live in `relations`, content and delivery in
//! `content`, and the aggregate records built over a version
//! ([`PackageEntry`], [`NameEntry`]) in `aggregate`. All three are
//! re-exported here, so every `crate::types::*` path is unchanged.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#entry");

use chrono::{DateTime, Utc};
use semver::Version;
use serde::{Deserialize, Serialize};
use specmark::spec;
use vibe_core::Group;

use super::kinds::PackageKind;

mod aggregate;
mod content;
mod relations;

pub use aggregate::{NameEntry, PackageEntry, Tombstone};
pub use content::{
    BootSnippetEntry, DeliveryMode, FeaturesEntry, I18nEntry, SubskillEntry, WorkspaceOriginEntry,
};
pub use relations::{
    CompatibilityEntry, ConflictsEntry, ObsoletesEntry, ProvidesEntry, RequiresAnyEntry,
    RequiresEntry,
};

/// Per-version index record. PROP-005 §2.6.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#entry",
    r = 1
)]
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

    /// Reader capabilities the consumer MUST have to act on this record
    /// (PROP-044 §4.5). A reader that does not know at least one listed
    /// string skips the record rather than guessing at its semantics.
    /// Unknown fields OUTSIDE this list are ignored as before — only
    /// entries here switch the reader from ignore to refuse.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub must_understand: Vec<String>,

    /// The version's withdrawal — a journal fact, not an authorial one:
    /// frozen content cannot yank itself. Absent (`false`) = live.
    #[serde(default, skip_serializing_if = "crate::types::is_false")]
    pub yanked: bool,

    /// Projected from the package manifest (PROP-044 §2a), not the
    /// registry's opinion: `false` = **snapshot** (content may flow
    /// under the same version string; a hash mismatch is NEWS), `true`
    /// = **frozen** (bytes immutable; a hash mismatch is ALARM). The
    /// transition is one-way; further work is a new version line.
    #[serde(default, skip_serializing_if = "crate::types::is_false")]
    pub frozen: bool,

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
    /// whole struct. The clock is an input like everywhere else: `at`
    /// lands in `indexed_at` verbatim, so a fixture stamped twice with
    /// one `at` is byte-identical (F2-1).
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

#[cfg(test)]
mod tests;
