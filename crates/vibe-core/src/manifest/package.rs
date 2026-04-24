//! `vibe-package.toml` — the package manifest.
//!
//! Schema: `VIBEVM-SPEC.md` §7.3. The capability-based dependency vocabulary
//! (`[provides]` / `[requires]` / `[[requires_any]]` / `[obsoletes]` /
//! `[conflicts]`) is defined in [PROP-002 §2.9](../../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#capability).
//!
//! Legacy M0 / M1.1 compact form — `[dependencies] required = [...] conflicts =
//! [...]` — is still accepted on parse: values migrate transparently into
//! `requires.packages` / `conflicts.packages` via [`PackageManifest::normalize_legacy_deps`],
//! which is called from [`PackageManifest::read`]. On the next write the
//! manifest round-trips in modern form; the `[dependencies]` section
//! disappears.
//!
//! Rationale for the migration: empty-deps packages (every live v0.1.0 flow
//! today) round-trip unchanged because `PackageDependencies::is_empty()` is
//! true for them, and the modern serializer skips empty sections too.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::capability_ref::CapabilityRef;
use crate::error::Result;
use crate::package_ref::{PackageKind, PackageRef, VersionSpec};

use super::{read_toml, write_toml};

/// The package manifest — `vibe-package.toml` inside a package directory.
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

    /// Capabilities this package advertises. Consumers reference them via
    /// `[requires].capabilities` and `[[requires_any]]`.
    #[serde(default, skip_serializing_if = "Provides::is_empty")]
    pub provides: Provides,

    /// Packages and capabilities this package requires. Resolved transitively
    /// by the depsolver at install time.
    #[serde(default, skip_serializing_if = "Requires::is_empty")]
    pub requires: Requires,

    /// Disjunctive requirements — any `one_of` list must be satisfied by at
    /// least one of its entries. Each `[[requires_any]]` is an independent
    /// disjunction.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires_any: Vec<RequiresAny>,

    /// Packages this package supersedes. On upgrade, the solver treats an
    /// installed `obsoletes` target as evidence to remove it.
    #[serde(default, skip_serializing_if = "Obsoletes::is_empty")]
    pub obsoletes: Obsoletes,

    /// Direct exclusion — these cannot coexist with this package.
    #[serde(default, skip_serializing_if = "ConflictsList::is_empty")]
    pub conflicts: ConflictsList,

    /// Legacy v1 compact form — accepted for back-compat; migrated into
    /// `requires.packages` / `conflicts.packages` by
    /// [`PackageManifest::normalize_legacy_deps`]. After normalization this
    /// field is empty; serialization skips it.
    #[serde(default, skip_serializing_if = "PackageDependencies::is_empty")]
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

/// `[provides]` — capabilities this package advertises.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provides {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capabilities: Vec<CapabilityRef>,
}

impl Provides {
    pub fn is_empty(&self) -> bool {
        self.capabilities.is_empty()
    }
}

/// `[requires]` — concrete package pkgrefs plus capability requirements.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Requires {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub packages: Vec<PackageRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capabilities: Vec<CapabilityRef>,
}

impl Requires {
    pub fn is_empty(&self) -> bool {
        self.packages.is_empty() && self.capabilities.is_empty()
    }
}

/// `[[requires_any]]` — one entry per independent disjunction; `one_of` must
/// be satisfied by at least one of its alternatives.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequiresAny {
    pub one_of: Vec<PackageRef>,
}

/// `[obsoletes]` — packages this one supersedes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Obsoletes {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub packages: Vec<PackageRef>,
}

impl Obsoletes {
    pub fn is_empty(&self) -> bool {
        self.packages.is_empty()
    }
}

/// `[conflicts]` — packages that cannot coexist with this one.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConflictsList {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub packages: Vec<PackageRef>,
}

impl ConflictsList {
    pub fn is_empty(&self) -> bool {
        self.packages.is_empty()
    }
}

/// Legacy `[dependencies]` section — v1 compact form.
///
/// Kept on [`PackageManifest`] purely for backwards-compatible parsing. It
/// is emptied out in [`PackageManifest::normalize_legacy_deps`] and the
/// serializer skips it, so round-tripping a v1 manifest produces a v2 one.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageDependencies {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required: Vec<PackageRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conflicts: Vec<PackageRef>,
}

impl PackageDependencies {
    pub fn is_empty(&self) -> bool {
        self.required.is_empty() && self.conflicts.is_empty()
    }
}

impl PackageManifest {
    pub const FILENAME: &'static str = "vibe-package.toml";

    /// Read a manifest from disk and migrate any legacy v1 `[dependencies]`
    /// section into the modern fields. Callers always see a manifest in
    /// modern form regardless of which form was on disk.
    pub fn read(path: impl AsRef<Path>) -> Result<Self> {
        let mut m: PackageManifest = read_toml(path)?;
        m.normalize_legacy_deps();
        Ok(m)
    }

    /// Write a manifest to disk. Always serializes the modern form —
    /// `[dependencies]` is omitted even if it was non-empty before
    /// [`Self::normalize_legacy_deps`] was called.
    pub fn write(&self, path: impl AsRef<Path>) -> Result<()> {
        write_toml(path, self)
    }

    /// Migrate any `[dependencies]` entries into `requires.packages` and
    /// `conflicts.packages`. Idempotent — a no-op after the first call. Safe
    /// to call even on a modern manifest (just returns immediately).
    pub fn normalize_legacy_deps(&mut self) {
        if self.dependencies.is_empty() {
            return;
        }
        let legacy = std::mem::take(&mut self.dependencies);
        for r in legacy.required {
            self.requires.packages.push(r);
        }
        for c in legacy.conflicts {
            self.conflicts.packages.push(c);
        }
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

    const FIXTURE_MODERN: &str = r#"
[package]
name = "welcome-page"
kind = "feat"
version = "0.3.0"
authors = ["Oleg Chirukhin <oleg@example.com>"]
license = "EULA"
description = "Welcome page demo feat"
keywords = ["welcome", "demo"]

[compatibility]
min_vibe_version = "0.1.0"
requires_kinds = []

[writes]
files = [
    "spec/feats/welcome-page/SPEC.md",
]

[boot_snippet]
filename = "40-feat-welcome-page.md"
source = "boot/40-feat-welcome-page.md"

[provides]
capabilities = ["ui:landing-page@0.3.0", "auth:oauth-callback"]

[requires]
packages = ["flow:atomic-commits@^0.1", "stack:rust-cli@^0.1"]
capabilities = ["db:any@>=1.0"]

[[requires_any]]
one_of = ["stack:rust-cli@^0.1", "stack:rust-axum@^0.2"]

[obsoletes]
packages = ["feat:welcome-page-legacy"]

[conflicts]
packages = ["feat:welcome-page-legacy-v2"]
"#;

    const FIXTURE_LEGACY: &str = r#"
[package]
name = "wal"
kind = "flow"
version = "0.3.0"
authors = ["Oleg Chirukhin <oleg@example.com>"]
license = "EULA"
description = "WAL"
keywords = ["wal"]

[compatibility]
min_vibe_version = "0.1.0"
requires_kinds = []

[writes]
files = [
    "spec/flows/wal/WAL-PROTOCOL.md",
]

[boot_snippet]
filename = "10-flow-wal.md"
source = "boot/10-flow-wal.md"

[dependencies]
required = ["flow:atomic-commits@^0.1"]
conflicts = ["flow:legacy-wal"]
"#;

    const FIXTURE_MINIMAL: &str = r#"
[package]
name = "tiny"
kind = "flow"
version = "0.0.1"
"#;

    #[test]
    fn parses_modern_form() {
        let m: PackageManifest = toml::from_str(FIXTURE_MODERN).unwrap();
        assert_eq!(m.package.kind, PackageKind::Feat);
        assert_eq!(m.package.name, "welcome-page");
        assert_eq!(m.provides.capabilities.len(), 2);
        assert_eq!(m.provides.capabilities[0].qualified(), "ui:landing-page");
        assert_eq!(m.requires.packages.len(), 2);
        assert_eq!(m.requires.packages[0].qualified_name(), "flow:atomic-commits");
        assert_eq!(m.requires.capabilities.len(), 1);
        assert_eq!(m.requires_any.len(), 1);
        assert_eq!(m.requires_any[0].one_of.len(), 2);
        assert_eq!(m.obsoletes.packages.len(), 1);
        assert_eq!(m.conflicts.packages.len(), 1);
        // Legacy section absent.
        assert!(m.dependencies.is_empty());
    }

    #[test]
    fn migrates_legacy_dependencies_section() {
        let mut m: PackageManifest = toml::from_str(FIXTURE_LEGACY).unwrap();
        // Before normalization: deps populated, requires empty.
        assert_eq!(m.dependencies.required.len(), 1);
        assert_eq!(m.dependencies.conflicts.len(), 1);
        assert!(m.requires.is_empty());
        assert!(m.conflicts.is_empty());

        m.normalize_legacy_deps();

        // After: deps empty, requires/conflicts populated.
        assert!(m.dependencies.is_empty());
        assert_eq!(m.requires.packages.len(), 1);
        assert_eq!(
            m.requires.packages[0].qualified_name(),
            "flow:atomic-commits"
        );
        assert_eq!(m.conflicts.packages.len(), 1);
        assert_eq!(
            m.conflicts.packages[0].qualified_name(),
            "flow:legacy-wal"
        );
    }

    #[test]
    fn normalize_is_idempotent() {
        let mut m: PackageManifest = toml::from_str(FIXTURE_LEGACY).unwrap();
        m.normalize_legacy_deps();
        let snapshot = m.clone();
        m.normalize_legacy_deps();
        assert_eq!(m, snapshot);
    }

    #[test]
    fn modern_form_roundtrips_unchanged() {
        let m: PackageManifest = toml::from_str(FIXTURE_MODERN).unwrap();
        let rendered = toml::to_string_pretty(&m).unwrap();
        let back: PackageManifest = toml::from_str(&rendered).unwrap();
        assert_eq!(m, back);
    }

    #[test]
    fn legacy_form_roundtrips_into_modern() {
        let mut m: PackageManifest = toml::from_str(FIXTURE_LEGACY).unwrap();
        m.normalize_legacy_deps();
        let rendered = toml::to_string_pretty(&m).unwrap();
        // After normalization + write, the legacy `[dependencies]` table is gone.
        assert!(!rendered.contains("[dependencies]"));
        assert!(rendered.contains("[requires]"));
        assert!(rendered.contains("[conflicts]"));
        // And a re-read is byte-identical to the already-normalized state.
        let back: PackageManifest = toml::from_str(&rendered).unwrap();
        assert_eq!(m, back);
    }

    #[test]
    fn parses_minimal_manifest() {
        let m: PackageManifest = toml::from_str(FIXTURE_MINIMAL).unwrap();
        assert_eq!(m.package.name, "tiny");
        assert!(m.writes.files.is_empty());
        assert!(m.boot_snippet.is_none());
        assert!(m.provides.is_empty());
        assert!(m.requires.is_empty());
        assert!(m.requires_any.is_empty());
        assert!(m.obsoletes.is_empty());
        assert!(m.conflicts.is_empty());
        assert!(m.dependencies.is_empty());
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
    fn rejects_unknown_top_level_field() {
        let raw = r#"
[package]
name = "wal"
kind = "flow"
version = "0.3.0"

[bogus]
value = 1
"#;
        assert!(toml::from_str::<PackageManifest>(raw).is_err());
    }

    #[test]
    fn as_package_ref_pins_exact_version() {
        let m: PackageManifest = toml::from_str(FIXTURE_MODERN).unwrap();
        let r = m.as_package_ref().unwrap();
        assert_eq!(r.kind, PackageKind::Feat);
        assert_eq!(r.name, "welcome-page");
        let this = semver::Version::parse("0.3.0").unwrap();
        assert!(r.version.matches(&this));
        let other = semver::Version::parse("0.3.1").unwrap();
        assert!(!r.version.matches(&other));
    }

    #[test]
    fn rejects_invalid_pkgref_in_requires() {
        let raw = r#"
[package]
name = "foo"
kind = "flow"
version = "0.1.0"

[requires]
packages = ["not-a-valid-pkgref"]
"#;
        let err = toml::from_str::<PackageManifest>(raw).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("invalid package reference") || msg.contains("missing `:`"));
    }

    #[test]
    fn rejects_invalid_capability_in_requires() {
        let raw = r#"
[package]
name = "foo"
kind = "flow"
version = "0.1.0"

[requires]
capabilities = ["no-colon-here"]
"#;
        let err = toml::from_str::<PackageManifest>(raw).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("invalid capability reference") || msg.contains("missing `:`"));
    }
}
