//! Content and delivery projections carried by a
//! [`VersionEntry`](super::VersionEntry): workspace provenance, the
//! feature table, sub-skill delivery, i18n availability, and the boot
//! snippet. Each mirrors a `vibe.toml` table (PROP-005 §2.6).

specmark::scope!("spec://vibevm/modules/vibe-index/PROP-005#entry");

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

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
