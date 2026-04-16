//! `vibe-package.toml` — the package manifest.
//!
//! Schema: `VIBEVM-SPEC.md` §7.3.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::package_ref::{PackageKind, PackageRef, VersionSpec};

use super::{read_toml, write_toml};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageManifest {
    pub package: PackageMeta,

    #[serde(default)]
    pub compatibility: Compatibility,

    #[serde(default)]
    pub writes: WritesSection,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boot_snippet: Option<BootSnippet>,

    #[serde(default)]
    pub dependencies: PackageDependencies,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageMeta {
    pub name: String,
    pub kind: PackageKind,
    pub version: semver::Version,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Compatibility {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_vibe_version: Option<String>,

    #[serde(default)]
    pub requires_kinds: Vec<PackageKind>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WritesSection {
    #[serde(default)]
    pub files: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BootSnippet {
    /// Target filename inside `spec/boot/`, e.g. `10-flow-wal.md`.
    pub filename: String,
    /// Path to the source file inside the package directory, e.g.
    /// `boot/10-flow-wal.md`.
    pub source: PathBuf,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageDependencies {
    #[serde(default)]
    pub required: Vec<String>,

    #[serde(default)]
    pub conflicts: Vec<String>,
}

impl PackageManifest {
    pub const FILENAME: &'static str = "vibe-package.toml";

    pub fn read(path: impl AsRef<Path>) -> Result<Self> {
        read_toml(path)
    }

    pub fn write(&self, path: impl AsRef<Path>) -> Result<()> {
        write_toml(path, self)
    }

    /// Produce a `PackageRef` pinning this package to its exact version.
    pub fn as_package_ref(&self) -> Result<PackageRef> {
        let req = semver::VersionReq::parse(&format!("={}", self.package.version))
            .expect("exact version string always parses as VersionReq");
        PackageRef::new(
            self.package.kind,
            self.package.name.clone(),
            VersionSpec::Req(req),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE_WAL: &str = r#"
[package]
name = "wal"
kind = "flow"
version = "0.3.0"
authors = ["Oleg Chirukhin <oleg@example.com>"]
license = "EULA"
description = "Write-Ahead Log discipline for human-AI development sessions"
keywords = ["wal", "memory", "discipline"]

[compatibility]
min_vibe_version = "0.1.0"
requires_kinds = []

[writes]
files = [
    "spec/flows/wal/WAL-PROTOCOL.md",
    "spec/flows/wal/session-end-hook.md",
]

[boot_snippet]
filename = "10-flow-wal.md"
source = "boot/10-flow-wal.md"

[dependencies]
required = []
conflicts = []
"#;

    #[test]
    fn parses_spec_example() {
        let m: PackageManifest = toml::from_str(FIXTURE_WAL).unwrap();
        assert_eq!(m.package.kind, PackageKind::Flow);
        assert_eq!(m.package.name, "wal");
        assert_eq!(m.package.version.to_string(), "0.3.0");
        assert_eq!(m.writes.files.len(), 2);
        assert_eq!(m.boot_snippet.as_ref().unwrap().filename, "10-flow-wal.md");
    }

    #[test]
    fn roundtrips_through_toml() {
        let m: PackageManifest = toml::from_str(FIXTURE_WAL).unwrap();
        let rendered = toml::to_string_pretty(&m).unwrap();
        let back: PackageManifest = toml::from_str(&rendered).unwrap();
        assert_eq!(m, back);
    }

    #[test]
    fn as_package_ref_pins_exact_version() {
        let m: PackageManifest = toml::from_str(FIXTURE_WAL).unwrap();
        let r = m.as_package_ref().unwrap();
        assert_eq!(r.kind, PackageKind::Flow);
        assert_eq!(r.name, "wal");
        let this = semver::Version::parse("0.3.0").unwrap();
        assert!(r.version.matches(&this));
        let other = semver::Version::parse("0.3.1").unwrap();
        assert!(!r.version.matches(&other));
    }

    #[test]
    fn rejects_unknown_kind() {
        let raw = r#"
[package]
name = "wal"
kind = "widget"
version = "0.3.0"
"#;
        assert!(toml::from_str::<PackageManifest>(raw).is_err());
    }

    #[test]
    fn minimal_manifest() {
        let raw = r#"
[package]
name = "tiny"
kind = "flow"
version = "0.0.1"
"#;
        let m: PackageManifest = toml::from_str(raw).unwrap();
        assert_eq!(m.package.name, "tiny");
        assert!(m.writes.files.is_empty());
        assert!(m.boot_snippet.is_none());
        assert!(m.dependencies.required.is_empty());
    }
}
