//! Dependency-relation projections carried by a
//! [`VersionEntry`](super::VersionEntry): what a version is compatible
//! with, provides, requires (all of, or any of), obsoletes, and
//! conflicts with. Each mirrors a `vibe.toml` table (PROP-005 §2.6) and
//! serialises empty-omitted through its `is_empty` skip guard.

specmark::scope!("spec://vibevm/modules/vibe-index/PROP-005#entry");

use serde::{Deserialize, Serialize};

use crate::types::kinds::PackageKind;

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
