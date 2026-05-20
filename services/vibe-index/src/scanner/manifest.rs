//! Parse `vibe.toml` (and `subskills/<path>/vibe-subskill.toml`)
//! into [`VersionEntry`] field components. Mirrors the relevant subset
//! of `vibe-core::manifest::Manifest`. PROP-005 §3.2 explained
//! the duplicate-not-import trade-off; the parity test `tests/
//! content_hash_parity.rs` plus integration tests under
//! `tests/scanner_e2e.rs` lock the byte shape against the reference.

use std::collections::BTreeMap;
use std::path::Path;

use semver::Version;
use serde::Deserialize;
use walkdir::WalkDir;

use crate::error::{Error, Result};
use crate::types::{
    BootSnippetEntry, CompatibilityEntry, ConflictsEntry, DeliveryMode, FeaturesEntry, I18nEntry,
    ObsoletesEntry, PackageKind, ProvidesEntry, RequiresAnyEntry, RequiresEntry, SubskillEntry,
};

/// Subset of `vibe.toml` we care about for the index.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawManifest {
    pub package: RawPackage,

    #[serde(default)]
    pub compatibility: RawCompatibility,

    #[serde(default)]
    pub provides: RawProvides,

    #[serde(default)]
    pub requires: RawRequires,

    #[serde(default)]
    pub requires_any: Vec<RawRequiresAny>,

    #[serde(default)]
    pub obsoletes: RawObsoletes,

    #[serde(default)]
    pub conflicts: RawConflicts,

    /// `[writes]` — captured opaquely because file-paths the package
    /// would land on disk do not bear on the index entry. Slice 11
    /// docs cover why this stays out of the index.
    #[serde(default)]
    pub writes: Option<toml::Value>,

    /// Carried as a generic value-bag; we split into named features + the `exclusive` group during conversion.
    #[serde(default)]
    pub features: BTreeMap<String, toml::Value>,

    #[serde(default)]
    pub i18n: RawI18n,

    #[serde(default)]
    pub boot_snippet: Option<RawBootSnippet>,

    /// Legacy v1 compact dependencies block — accepted on parse, not
    /// surfaced into the index entry (modern manifests carry the
    /// requires/conflicts split). Anything else here means the raw
    /// manifest is in legacy form; callers either migrate or skip.
    #[serde(default)]
    pub dependencies: Option<toml::Value>,

    /// `[target."context(...)".dependencies]` blocks per PROP-003 §2.6.1.
    /// Captured opaquely; not surfaced into the index entry by slice 3.
    #[serde(default, rename = "target")]
    pub conditional_targets: Option<toml::Value>,
}

#[derive(Debug, Deserialize)]
pub struct RawPackage {
    pub name: String,
    pub kind: PackageKind,
    pub version: Version,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub describes: Option<String>,
}


#[derive(Debug, Default, Deserialize)]
pub struct RawCompatibility {
    #[serde(default)]
    pub min_vibe_version: Option<String>,
    #[serde(default)]
    pub requires_kinds: Vec<PackageKind>,
}

#[derive(Debug, Default, Deserialize)]
pub struct RawProvides {
    #[serde(default)]
    pub capabilities: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct RawRequires {
    #[serde(default)]
    pub packages: Vec<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct RawRequiresAny {
    pub one_of: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct RawObsoletes {
    #[serde(default)]
    pub packages: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct RawConflicts {
    #[serde(default)]
    pub packages: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct RawI18n {
    #[serde(default)]
    pub available: Vec<String>,
    #[serde(default)]
    pub default: Option<String>,
    #[serde(default)]
    pub preferred: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct RawBootSnippet {
    pub filename: String,
    #[serde(default)]
    pub source: Option<String>,
}

pub fn parse_manifest(bytes: &[u8]) -> Result<RawManifest> {
    let s = std::str::from_utf8(bytes)
        .map_err(|e| Error::Malformed(format!("vibe.toml is not UTF-8: {e}")))?;
    toml::from_str::<RawManifest>(s)
        .map_err(|e| Error::Malformed(format!("vibe.toml: {e}")))
}

/// Convert `RawManifest` features into the [`FeaturesEntry`] shape —
/// pulling out the `exclusive` sub-table, leaving every other key as
/// a feature with a string-list activation.
pub fn features_from_raw(raw: &BTreeMap<String, toml::Value>) -> Result<FeaturesEntry> {
    let mut features = BTreeMap::new();
    let mut exclusive = BTreeMap::new();
    for (k, v) in raw {
        if k == "exclusive" {
            let table: BTreeMap<String, Vec<String>> = v.clone().try_into().map_err(|e| {
                Error::Malformed(format!("`features.exclusive` is malformed: {e}"))
            })?;
            exclusive = table;
            continue;
        }
        let arr: Vec<String> = v.clone().try_into().map_err(|e| {
            Error::Malformed(format!("`features.{k}` is not a string list: {e}"))
        })?;
        features.insert(k.clone(), arr);
    }
    Ok(FeaturesEntry {
        features,
        exclusive,
    })
}

pub fn provides_from_raw(raw: &RawProvides) -> ProvidesEntry {
    ProvidesEntry {
        capabilities: raw.capabilities.clone(),
    }
}

pub fn requires_from_raw(raw: &RawRequires) -> RequiresEntry {
    RequiresEntry {
        packages: raw.packages.clone(),
        capabilities: raw.capabilities.clone(),
    }
}

pub fn requires_any_from_raw(raw: &[RawRequiresAny]) -> Vec<RequiresAnyEntry> {
    raw.iter()
        .map(|r| RequiresAnyEntry {
            one_of: r.one_of.clone(),
        })
        .collect()
}

pub fn obsoletes_from_raw(raw: &RawObsoletes) -> ObsoletesEntry {
    ObsoletesEntry {
        packages: raw.packages.clone(),
    }
}

pub fn conflicts_from_raw(raw: &RawConflicts) -> ConflictsEntry {
    ConflictsEntry {
        packages: raw.packages.clone(),
    }
}

pub fn compatibility_from_raw(raw: &RawCompatibility) -> CompatibilityEntry {
    CompatibilityEntry {
        min_vibe_version: raw.min_vibe_version.clone(),
        requires_kinds: raw.requires_kinds.clone(),
    }
}

pub fn i18n_from_raw(raw: &RawI18n) -> I18nEntry {
    I18nEntry {
        available: raw.available.clone(),
        default: raw.default.clone(),
    }
}

pub fn boot_snippet_from_raw(raw: &Option<RawBootSnippet>) -> Option<BootSnippetEntry> {
    raw.as_ref().map(|b| BootSnippetEntry {
        filename: b.filename.clone(),
    })
}

// ---------------------------------------------------------------------------
// Subskill walking
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawSubskill {
    #[serde(default)]
    pub subskill: RawSubskillMeta,
    #[serde(default)]
    pub activation: RawActivation,
    #[serde(default)]
    pub recommends: Option<toml::Value>,
    #[serde(default)]
    pub conflicts: Option<toml::Value>,
    #[serde(default)]
    pub content: Option<toml::Value>,
}

#[derive(Debug, Default, Deserialize)]
pub struct RawSubskillMeta {
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub delivery: Option<String>,
    #[serde(default)]
    pub describes: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct RawActivation {
    #[serde(default)]
    pub manual: Option<bool>,
    #[serde(default)]
    pub if_present: Option<toml::Value>,
    #[serde(default)]
    pub if_provides: Option<toml::Value>,
    #[serde(default)]
    pub if_files: Option<toml::Value>,
    #[serde(default)]
    pub if_command: Option<toml::Value>,
    #[serde(default)]
    pub if_env: Option<toml::Value>,
    #[serde(default)]
    pub if_describes_match: Option<toml::Value>,
    #[serde(default)]
    pub if_language: Option<toml::Value>,
}

/// Walk `<pkg_root>/subskills/<path>/vibe-subskill.toml`. Subskills
/// without a manifest are ignored; malformed manifests surface as an
/// error so authoring bugs are loud at index time.
pub fn collect_subskills(pkg_root: &Path) -> Result<Vec<SubskillEntry>> {
    let subdir = pkg_root.join("subskills");
    if !subdir.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in WalkDir::new(&subdir).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        if entry.file_name() != "vibe-subskill.toml" {
            continue;
        }
        let bytes = std::fs::read(entry.path()).map_err(|e| Error::Io {
            path: entry.path().to_path_buf(),
            message: e.to_string(),
        })?;
        let raw: RawSubskill = toml::from_str(
            std::str::from_utf8(&bytes)
                .map_err(|e| Error::Malformed(format!("vibe-subskill.toml is not UTF-8: {e}")))?,
        )
        .map_err(|e| {
            Error::Malformed(format!(
                "{}: {e}",
                entry.path().strip_prefix(pkg_root).unwrap_or(entry.path()).display()
            ))
        })?;

        let path = match raw.subskill.path.clone() {
            Some(p) => p,
            None => entry
                .path()
                .parent()
                .and_then(|p| p.strip_prefix(&subdir).ok())
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default(),
        };
        let delivery = match raw.subskill.delivery.as_deref() {
            Some("eager") => DeliveryMode::Eager,
            Some("lazy-push") => DeliveryMode::LazyPush,
            Some("lazy-pull") => DeliveryMode::LazyPull,
            Some(other) => {
                return Err(Error::Malformed(format!(
                    "subskill `{path}` has unknown delivery mode `{other}`"
                )));
            }
            None => DeliveryMode::Eager,
        };
        out.push(SubskillEntry {
            path,
            delivery,
            describes: raw.subskill.describes.clone(),
            description: raw.subskill.description.clone(),
            channels: declared_channels(&raw.activation),
        });
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

fn declared_channels(a: &RawActivation) -> Vec<String> {
    let mut ch = Vec::new();
    if a.manual.unwrap_or(false) {
        ch.push("manual".into());
    }
    if a.if_present.is_some() {
        ch.push("if_present".into());
    }
    if a.if_provides.is_some() {
        ch.push("if_provides".into());
    }
    if a.if_files.is_some() {
        ch.push("if_files".into());
    }
    if a.if_command.is_some() {
        ch.push("if_command".into());
    }
    if a.if_env.is_some() {
        ch.push("if_env".into());
    }
    if a.if_describes_match.is_some() {
        ch.push("if_describes_match".into());
    }
    if a.if_language.is_some() {
        ch.push("if_language".into());
    }
    ch
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_manifest_minimal() {
        let body = br#"
[package]
name = "wal"
kind = "flow"
version = "0.1.0"
"#;
        let m = parse_manifest(body).unwrap();
        assert_eq!(m.package.name, "wal");
        assert_eq!(m.package.kind, PackageKind::Flow);
        assert_eq!(m.package.version.to_string(), "0.1.0");
    }

    #[test]
    fn parse_manifest_with_provides_requires() {
        let body = br#"
[package]
name = "welcome"
kind = "feat"
version = "0.3.0"

[provides]
capabilities = ["ui:landing-page@0.3.0"]

[requires]
packages = ["flow:wal@^0.1"]
capabilities = ["db:any@>=1.0"]

[[requires_any]]
one_of = ["stack:rust-cli@^0.1", "stack:rust-axum@^0.2"]
"#;
        let m = parse_manifest(body).unwrap();
        assert_eq!(m.provides.capabilities.len(), 1);
        assert_eq!(m.requires.packages.len(), 1);
        assert_eq!(m.requires.capabilities.len(), 1);
        assert_eq!(m.requires_any.len(), 1);
    }

    #[test]
    fn features_pulls_out_exclusive_table() {
        let body = br#"
[package]
name = "x"
kind = "flow"
version = "0.1.0"

[features]
default = ["a"]
a = []
b = ["subskill:x/y"]

[features.exclusive]
group = ["a", "b"]
"#;
        let m = parse_manifest(body).unwrap();
        let f = features_from_raw(&m.features).unwrap();
        assert!(f.features.contains_key("default"));
        assert!(f.features.contains_key("a"));
        assert!(f.features.contains_key("b"));
        assert_eq!(f.exclusive.get("group").unwrap(), &vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn parses_real_fixture() {
        let body = include_bytes!("../../fixtures/golden-flow-wal-0.1.0/vibe.toml");
        let m = parse_manifest(body).unwrap();
        assert_eq!(m.package.name, "wal");
        assert_eq!(m.package.kind, PackageKind::Flow);
        assert_eq!(m.package.license, Some("EULA".into()));
    }
}
