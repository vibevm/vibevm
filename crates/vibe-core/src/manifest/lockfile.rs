//! `vibe.lock` — the project lockfile.
//!
//! Schema: `VIBEVM-SPEC.md` §7.4.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::package_ref::{PackageKind, PackageRef, VersionSpec};

use super::{read_toml, write_toml};

/// Top-level `vibe.lock` structure.
///
/// The on-disk TOML uses `[[package]]` array-of-tables. Serde's default
/// behavior flattens this when the field is named `package` and typed as
/// `Vec<LockedPackage>`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lockfile {
    pub meta: LockfileMeta,

    #[serde(
        default,
        rename = "package",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub packages: Vec<LockedPackage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LockfileMeta {
    pub generated_by: String,
    pub generated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LockedPackage {
    pub kind: PackageKind,
    pub name: String,
    pub version: semver::Version,
    pub source: String,
    pub content_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boot_snippet: Option<String>,
    #[serde(default)]
    pub files_written: Vec<PathBuf>,
}

impl Lockfile {
    pub const FILENAME: &'static str = "vibe.lock";

    pub fn empty(generated_by: impl Into<String>, generated_at: impl Into<String>) -> Self {
        Lockfile {
            meta: LockfileMeta {
                generated_by: generated_by.into(),
                generated_at: generated_at.into(),
            },
            packages: vec![],
        }
    }

    pub fn read(path: impl AsRef<Path>) -> Result<Self> {
        read_toml(path)
    }

    pub fn write(&self, path: impl AsRef<Path>) -> Result<()> {
        write_toml(path, self)
    }

    /// Find an installed package by its `<kind>:<name>` identity.
    pub fn find(&self, kind: PackageKind, name: &str) -> Option<&LockedPackage> {
        self.packages
            .iter()
            .find(|p| p.kind == kind && p.name == name)
    }

    pub fn find_mut(&mut self, kind: PackageKind, name: &str) -> Option<&mut LockedPackage> {
        self.packages
            .iter_mut()
            .find(|p| p.kind == kind && p.name == name)
    }

    /// Remove an installed package; returns the removed entry if present.
    pub fn remove(&mut self, kind: PackageKind, name: &str) -> Option<LockedPackage> {
        let idx = self
            .packages
            .iter()
            .position(|p| p.kind == kind && p.name == name)?;
        Some(self.packages.remove(idx))
    }
}

impl LockedPackage {
    /// Produce a `PackageRef` pinned to this exact installed version.
    pub fn as_package_ref(&self) -> Result<PackageRef> {
        let req = semver::VersionReq::parse(&format!("={}", self.version))
            .expect("exact version always parses");
        PackageRef::new(self.kind, self.name.clone(), VersionSpec::Req(req))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"
[meta]
generated_by = "vibe 0.1.0-dev"
generated_at = "2026-04-16T12:00:00Z"

[[package]]
kind = "flow"
name = "wal"
version = "0.3.0"
source = "git+https://example.com/reg.git#flow/wal/v0.3.0"
content_hash = "sha256:abc"
boot_snippet = "10-flow-wal.md"
files_written = [
    "spec/flows/wal/WAL-PROTOCOL.md",
    "spec/boot/10-flow-wal.md",
]

[[package]]
kind = "stack"
name = "rust-cli"
version = "0.1.0"
source = "file:///tmp/reg/stack/rust-cli/v0.1.0"
content_hash = "sha256:def"
files_written = []
"#;

    #[test]
    fn parses_spec_example() {
        let lf: Lockfile = toml::from_str(FIXTURE).unwrap();
        assert_eq!(lf.packages.len(), 2);
        let wal = lf.find(PackageKind::Flow, "wal").unwrap();
        assert_eq!(wal.version.to_string(), "0.3.0");
        assert_eq!(wal.boot_snippet.as_deref(), Some("10-flow-wal.md"));
        assert_eq!(wal.files_written.len(), 2);
    }

    #[test]
    fn roundtrip() {
        let lf: Lockfile = toml::from_str(FIXTURE).unwrap();
        let rendered = toml::to_string_pretty(&lf).unwrap();
        let back: Lockfile = toml::from_str(&rendered).unwrap();
        assert_eq!(lf, back);
    }

    #[test]
    fn empty_lockfile_serializes() {
        let lf = Lockfile::empty("vibe 0.1.0-dev", "2026-04-16T00:00:00Z");
        let rendered = toml::to_string_pretty(&lf).unwrap();
        let back: Lockfile = toml::from_str(&rendered).unwrap();
        assert_eq!(lf, back);
    }

    #[test]
    fn remove_drops_entry() {
        let mut lf: Lockfile = toml::from_str(FIXTURE).unwrap();
        assert_eq!(lf.packages.len(), 2);
        let removed = lf.remove(PackageKind::Flow, "wal").unwrap();
        assert_eq!(removed.name, "wal");
        assert_eq!(lf.packages.len(), 1);
        assert!(lf.find(PackageKind::Flow, "wal").is_none());
    }
}
