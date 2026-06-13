//! Aggregate records built over [`VersionEntry`](super::VersionEntry):
//! [`PackageEntry`] gathers every indexed version of one `(group, name)`
//! identity (PROP-008 §2.2); [`NameEntry`] gathers every `PackageEntry`
//! that shares one bare `name` — the `by-name/<name>.json` candidate set
//! that makes short-name resolution one round-trip per registry.

specmark::scope!("spec://vibevm/modules/vibe-index/PROP-005#entry");

use chrono::{DateTime, Utc};
use semver::Version;
use serde::{Deserialize, Serialize};
use specmark::spec;
use vibe_core::Group;

use super::VersionEntry;

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
