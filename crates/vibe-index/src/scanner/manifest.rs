//! Parse `vibe.toml` and `subskills/<path>/vibe-subskill.toml` into
//! [`VersionEntry`](crate::types::VersionEntry) field components.
//!
//! The scanner parses through `vibe-core`'s own [`Manifest`] and
//! [`SubskillManifest`] — the very types the rest of vibevm uses — so the
//! index can never drift from the manifest schema. The pre-de-rot scanner
//! hand-duplicated a `vibe.toml` parser; nothing tied it to `vibe-core`,
//! and it rotted silently against the M1.17 / M1.18 schema churn. PROP-005
//! §3.2 / §9 item 11 record the reversal of the standalone-workspace
//! decision this dependency rests on.
//!
//! What stays duplicated is deliberate and narrow: the four-variant
//! [`PackageKind`] and `NamingConvention` in [`crate::types`] — frozen by
//! `VIBEVM-SPEC.md` §4, and needing the `Ord` + `clap::ValueEnum` that the
//! `vibe-core` originals do not derive. [`package_kind`] converts between
//! the two with a total `match`.

use std::path::Path;

use vibe_core::PackageKind as CorePackageKind;
use vibe_core::manifest::i18n::I18nDecl;
use vibe_core::manifest::{
    ActivationRules, BootCategory, BootSnippet, Compatibility, ConflictsList,
    DeliveryMode as CoreDeliveryMode, FeaturesTable, Manifest, Obsoletes, PackageMeta, Provides,
    Requires, RequiresAny, SubskillManifest,
};
use walkdir::WalkDir;

use crate::error::{Error, Result};
use crate::types::{
    BootSnippetEntry, CompatibilityEntry, ConflictsEntry, DeliveryMode, FeaturesEntry, I18nEntry,
    ObsoletesEntry, PackageKind, ProvidesEntry, RequiresAnyEntry, RequiresEntry, SubskillEntry,
};

/// Parse a `vibe.toml` byte buffer into the canonical `vibe-core`
/// [`Manifest`]. Parse / validation failures surface as
/// [`Error::Malformed`] so the scan driver records a skip note for the
/// offending package rather than aborting the whole reindex.
pub fn parse_manifest(bytes: &[u8]) -> Result<Manifest> {
    let s = std::str::from_utf8(bytes)
        .map_err(|e| Error::Malformed(format!("vibe.toml is not UTF-8: {e}")))?;
    Manifest::parse_str(s).map_err(|e| Error::Malformed(format!("vibe.toml: {e}")))
}

/// The `[package]` table — every indexable node is a publishable package.
/// A manifest without one (a plain `[project]`, a bare `[workspace]`) is
/// not an index entry.
pub fn require_package(manifest: &Manifest) -> Result<&PackageMeta> {
    manifest.package.as_ref().ok_or_else(|| {
        Error::Malformed(
            "vibe.toml carries no [package] table — not a publishable package".to_string(),
        )
    })
}

/// Map a `vibe-core` package kind onto the index's own [`PackageKind`].
/// See the module docs for why the index keeps its own enum.
pub fn package_kind(kind: CorePackageKind) -> PackageKind {
    match kind {
        CorePackageKind::Flow => PackageKind::Flow,
        CorePackageKind::Feat => PackageKind::Feat,
        CorePackageKind::Stack => PackageKind::Stack,
        CorePackageKind::Tool => PackageKind::Tool,
    }
}

pub fn compatibility_from(c: &Compatibility) -> CompatibilityEntry {
    CompatibilityEntry {
        min_vibe_version: c.min_vibe_version.clone(),
        requires_kinds: c.requires_kinds.iter().copied().map(package_kind).collect(),
    }
}

pub fn provides_from(p: &Provides) -> ProvidesEntry {
    ProvidesEntry {
        capabilities: p.capabilities.iter().map(|c| c.to_string()).collect(),
    }
}

/// Flatten `[requires]` into the index entry's string lists. Registry
/// dependencies keep their `<kind>:<name>@<constraint>` form; git / path
/// / `version.var` sources — which have no single constraint string —
/// degrade to the bare `<kind>:<name>`. Both lists are sorted, so the
/// index is byte-deterministic.
pub fn requires_from(r: &Requires) -> RequiresEntry {
    let mut packages: Vec<String> = r.packages.iter().map(|p| p.to_string()).collect();
    for (kind, name) in r
        .git_packages
        .iter()
        .map(|g| (g.kind, g.name.as_str()))
        .chain(r.path_packages.iter().map(|p| (p.kind, p.name.as_str())))
        .chain(r.var_packages.iter().map(|v| (v.kind, v.name.as_str())))
    {
        packages.push(format!("{kind}:{name}"));
    }
    packages.sort();
    let mut capabilities: Vec<String> = r.capabilities.iter().map(|c| c.to_string()).collect();
    capabilities.sort();
    RequiresEntry {
        packages,
        capabilities,
    }
}

pub fn requires_any_from(list: &[RequiresAny]) -> Vec<RequiresAnyEntry> {
    list.iter()
        .map(|ra| RequiresAnyEntry {
            one_of: ra.one_of.iter().map(|p| p.to_string()).collect(),
        })
        .collect()
}

pub fn obsoletes_from(o: &Obsoletes) -> ObsoletesEntry {
    ObsoletesEntry {
        packages: o.packages.iter().map(|p| p.to_string()).collect(),
    }
}

pub fn conflicts_from(c: &ConflictsList) -> ConflictsEntry {
    ConflictsEntry {
        packages: c.packages.iter().map(|p| p.to_string()).collect(),
    }
}

pub fn features_from(f: &FeaturesTable) -> FeaturesEntry {
    FeaturesEntry {
        features: f.features.clone(),
        exclusive: f.exclusive.clone(),
    }
}

pub fn i18n_from(i: &I18nDecl) -> I18nEntry {
    I18nEntry {
        available: i.available.clone(),
        default: Some(i.canonical.clone()),
    }
}

pub fn boot_snippet_from(b: &Option<BootSnippet>) -> Option<BootSnippetEntry> {
    b.as_ref().map(|bs| BootSnippetEntry {
        source: bs.source.to_string_lossy().replace('\\', "/"),
        category: bs.category.map(boot_category_str),
    })
}

/// The kebab-case wire string for a boot category — the form `vibe-core`
/// serialises and the [`BootSnippetEntry`] records.
fn boot_category_str(c: BootCategory) -> String {
    match c {
        BootCategory::Foundation => "foundation",
        BootCategory::Flow => "flow",
        BootCategory::Stack => "stack",
        BootCategory::UserOverride => "user-override",
    }
    .to_string()
}

fn delivery_from(d: CoreDeliveryMode) -> DeliveryMode {
    match d {
        CoreDeliveryMode::Eager => DeliveryMode::Eager,
        CoreDeliveryMode::LazyPush => DeliveryMode::LazyPush,
        CoreDeliveryMode::LazyPull => DeliveryMode::LazyPull,
    }
}

// ---------------------------------------------------------------------------
// Subskill walking
// ---------------------------------------------------------------------------

/// Walk `<pkg_root>/subskills/<path>/vibe-subskill.toml`, parsing each
/// through `vibe-core`'s [`SubskillManifest`]. A directory without a
/// manifest is ignored; a malformed manifest surfaces as an error so the
/// authoring bug is loud at index time.
pub fn collect_subskills(pkg_root: &Path) -> Result<Vec<SubskillEntry>> {
    let subdir = pkg_root.join("subskills");
    if !subdir.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in WalkDir::new(&subdir).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() || entry.file_name() != SubskillManifest::FILENAME {
            continue;
        }
        let rel = entry
            .path()
            .strip_prefix(pkg_root)
            .unwrap_or(entry.path())
            .display()
            .to_string();
        let sm = SubskillManifest::read(entry.path())
            .map_err(|e| Error::Malformed(format!("{rel}: {e}")))?;
        out.push(SubskillEntry {
            path: sm.subskill.path.clone(),
            delivery: delivery_from(sm.subskill.delivery),
            describes: sm.subskill.describes.as_ref().map(|p| p.to_string()),
            description: sm.subskill.description.clone(),
            channels: declared_channels(&sm.activation),
        });
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

/// The activation channels a subskill declares — every non-empty
/// `[activation]` lane, surfaced in the index entry for discovery.
fn declared_channels(a: &ActivationRules) -> Vec<String> {
    let mut ch = Vec::new();
    if !a.if_present.is_empty() {
        ch.push("if_present".into());
    }
    if !a.if_provides.is_empty() {
        ch.push("if_provides".into());
    }
    if !a.if_files.is_empty() {
        ch.push("if_files".into());
    }
    if !a.if_command.is_empty() {
        ch.push("if_command".into());
    }
    if !a.if_env.is_empty() {
        ch.push("if_env".into());
    }
    if !a.if_os.is_empty() {
        ch.push("if_os".into());
    }
    if a.if_describes_match {
        ch.push("if_describes_match".into());
    }
    if !a.if_language.is_empty() {
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
        let pkg = require_package(&m).unwrap();
        assert_eq!(pkg.name, "wal");
        assert_eq!(package_kind(pkg.kind), PackageKind::Flow);
        assert_eq!(pkg.version.to_string(), "0.1.0");
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
capabilities = ["db:any@>=1.0"]

[requires.packages]
"flow:wal" = "^0.1"

[[requires_any]]
one_of = ["stack:rust-cli@^0.1", "stack:rust-axum@^0.2"]
"#;
        let m = parse_manifest(body).unwrap();
        assert_eq!(provides_from(&m.provides).capabilities.len(), 1);
        let req = requires_from(&m.requires);
        // The modern `[requires.packages]` table flattens to a
        // `<kind>:<name>@<constraint>` pkgref string.
        assert_eq!(req.packages, vec!["flow:wal@^0.1".to_string()]);
        assert_eq!(req.capabilities, vec!["db:any@>=1.0".to_string()]);
        assert_eq!(requires_any_from(&m.requires_any).len(), 1);
    }

    #[test]
    fn features_split_into_named_and_exclusive() {
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
        let f = features_from(&m.features);
        assert!(f.features.contains_key("default"));
        assert!(f.features.contains_key("a"));
        assert!(f.features.contains_key("b"));
        assert_eq!(
            f.exclusive.get("group").unwrap(),
            &vec!["a".to_string(), "b".to_string()]
        );
    }

    #[test]
    fn parses_real_fixture() {
        let body = include_bytes!("../../fixtures/golden-flow-wal-0.1.0/vibe.toml");
        let m = parse_manifest(body).unwrap();
        let pkg = require_package(&m).unwrap();
        assert_eq!(pkg.name, "wal");
        assert_eq!(package_kind(pkg.kind), PackageKind::Flow);
        assert_eq!(pkg.license, Some("EULA".into()));
    }

    #[test]
    fn boot_snippet_carries_source_and_category() {
        // M1.18 loading model: `[boot_snippet]` is `source` + `category`,
        // not the retired `filename`. `link` is a loading-model concern
        // `vibe-core` parses and the index simply does not catalogue.
        let body = br#"
[package]
name = "wal"
kind = "flow"
version = "0.1.0"

[boot_snippet]
source = "boot/10-flow-wal.md"
category = "flow"
link = "inline"
"#;
        let m = parse_manifest(body).unwrap();
        let bs = boot_snippet_from(&m.boot_snippet).expect("boot_snippet present");
        assert_eq!(bs.source, "boot/10-flow-wal.md");
        assert_eq!(bs.category.as_deref(), Some("flow"));
    }

    #[test]
    fn non_package_manifest_is_rejected() {
        // A plain `[project]` is a valid manifest but not a publishable
        // package — the scanner cannot make an index entry from it.
        let body = br#"
[project]
name = "consumer"
version = "0.1.0"
"#;
        let m = parse_manifest(body).unwrap();
        assert!(require_package(&m).is_err());
    }
}
