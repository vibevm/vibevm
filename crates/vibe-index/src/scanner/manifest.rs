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
//! What stays converted is narrow: `vibe-core`'s closed eight-variant
//! [`PackageKind`](vibe_core::PackageKind) against the index's open wire
//! vocabulary ([`crate::types::PackageKind`] — a re-export of the
//! generated type, `Unknown(String)` and all). The manifest side is
//! closed because `vibe.toml` is written by this build's own tooling;
//! the wire side is open because a registry serves the future.
//! [`package_kind`] converts between the two with a total `match`, and
//! `FromStr` on the open side preserves an unfamiliar string verbatim
//! for the wire to carry.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");

use std::path::Path;

use specmark::spec;
use vibe_core::PackageKind as CorePackageKind;
use vibe_core::manifest::i18n::I18nDecl;
use vibe_core::manifest::{
    ActivationRules, BootCategory, BootSnippet, Compatibility, ConflictsList,
    DeliveryMode as CoreDeliveryMode, DocumentationDecl, DocumentsDecl, EmbeddedSourceDecl,
    EmbeddedSourceKind, FeaturesTable, Manifest, MediaDecl, Obsoletes, OriginSection, PackageMeta,
    Provides, Requires, RequiresAny, SubskillManifest, TranslatesDecl,
};
use walkdir::WalkDir;

use crate::error::{Error, Result};
use crate::types::{
    BootSnippetEntry, CompatibilityEntry, ConflictsEntry, DeliveryMode, DocumentationEntry,
    DocumentsEntry, EmbeddedSourceEntry, FeaturesEntry, I18nEntry, MediaEntry, ObsoletesEntry,
    PackageKind, ProvidesEntry, RequiresAnyEntry, RequiresEntry, SubskillEntry, TranslatesEntry,
    WorkspaceOriginEntry,
};

/// Parse a `vibe.toml` byte buffer into the canonical `vibe-core`
/// [`Manifest`]. Parse / validation failures surface as
/// [`Error::Malformed`] so the scan driver records a skip note for the
/// offending package rather than aborting the whole reindex.
#[spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#entry",
    r = 1
)]
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
        CorePackageKind::Mcp => PackageKind::Mcp,
        CorePackageKind::Lang => PackageKind::Lang,
        CorePackageKind::Doc => PackageKind::Doc,
        CorePackageKind::App => PackageKind::App,
    }
}

/// Every projection builder normalises emptiness to absence: the
/// writer never emits a present-but-empty section (`"provides": {}`),
/// so an empty projection becomes `None` at the source rather than at
/// each call site.
pub fn compatibility_from(c: &Compatibility) -> Option<CompatibilityEntry> {
    let entry = CompatibilityEntry {
        min_vibe_version: c.min_vibe_version.clone(),
        requires_kinds: c.requires_kinds.iter().copied().map(package_kind).collect(),
    };
    (!entry.is_empty()).then_some(entry)
}

pub fn provides_from(p: &Provides) -> Option<ProvidesEntry> {
    let entry = ProvidesEntry {
        capabilities: p.capabilities.iter().map(|c| c.to_string()).collect(),
    };
    (!entry.is_empty()).then_some(entry)
}

/// Flatten `[requires]` into the index entry's string lists. Registry
/// dependencies keep their `<group>/<name>@<constraint>` form; git / path
/// / `version.var` sources — which have no single constraint string —
/// degrade to the bare `<group>/<name>`. Both lists are sorted, so the
/// index is byte-deterministic.
pub fn requires_from(r: &Requires) -> Option<RequiresEntry> {
    let mut packages: Vec<String> = r.packages.iter().map(|p| p.to_string()).collect();
    for (group, name) in r
        .git_packages
        .iter()
        .map(|g| (&g.group, g.name.as_str()))
        .chain(r.path_packages.iter().map(|p| (&p.group, p.name.as_str())))
        .chain(r.var_packages.iter().map(|v| (&v.group, v.name.as_str())))
    {
        packages.push(format!("{group}/{name}"));
    }
    packages.sort();
    let mut capabilities: Vec<String> = r.capabilities.iter().map(|c| c.to_string()).collect();
    capabilities.sort();
    let entry = RequiresEntry {
        packages,
        capabilities,
    };
    (!entry.is_empty()).then_some(entry)
}

pub fn requires_any_from(list: &[RequiresAny]) -> Vec<RequiresAnyEntry> {
    list.iter()
        .map(|ra| RequiresAnyEntry {
            one_of: ra.one_of.iter().map(|p| p.to_string()).collect(),
        })
        .collect()
}

pub fn obsoletes_from(o: &Obsoletes) -> Option<ObsoletesEntry> {
    let entry = ObsoletesEntry {
        packages: o.packages.iter().map(|p| p.to_string()).collect(),
    };
    (!entry.is_empty()).then_some(entry)
}

pub fn conflicts_from(c: &ConflictsList) -> Option<ConflictsEntry> {
    let entry = ConflictsEntry {
        packages: c.packages.iter().map(|p| p.to_string()).collect(),
    };
    (!entry.is_empty()).then_some(entry)
}

pub fn features_from(f: &FeaturesTable) -> Option<FeaturesEntry> {
    let entry = FeaturesEntry {
        features: f.features.clone(),
        exclusive: f.exclusive.clone(),
    };
    (!entry.is_empty()).then_some(entry)
}

pub fn i18n_from(i: &I18nDecl) -> Option<I18nEntry> {
    let entry = I18nEntry {
        available: i.available.clone(),
        default: Some(i.canonical.clone()),
    };
    (!entry.is_empty()).then_some(entry)
}

/// Project `[[documents]]` — the subjects a `doc` package documents
/// (PROP-057 `##REL-INDEX-FIELDS`). The list rides into the index so
/// that «who documents X» is a fold over `primary.jsonl` and never a
/// download (`##REL-REVERSE-QUERIES-SITE-SIDE`); an empty list is
/// absence, like every sibling here.
pub fn documents_from(list: &[DocumentsDecl]) -> Vec<DocumentsEntry> {
    list.iter()
        .map(|d| DocumentsEntry {
            package: d.package.clone(),
            version: d.version.clone(),
        })
        .collect()
}

/// Project `[documentation]` — the other end of the same edge, written
/// by the SUBJECT and legal in a package of any kind. Officiality is the
/// convergence of the two ends, computed at render time, so nothing here
/// is a stored flag (PROP-057 `##REL-NO-OFFICIAL-FLAG`).
pub fn documentation_from(d: &Option<DocumentationDecl>) -> Option<DocumentationEntry> {
    let entry = DocumentationEntry {
        primary: d.as_ref().and_then(|d| d.primary.clone()),
        official: d.as_ref().map(|d| d.official.clone()).unwrap_or_default(),
    };
    (!entry.is_empty()).then_some(entry)
}

/// Project `[translates]` — the source this adaptation follows. The
/// source stores no list of its translations, so this edge is the only
/// one there is and the language selector is built by folding it
/// (PROP-057 `##LOC-NO-TRANSLATIONS-TABLE`).
pub fn translates_from(t: &Option<TranslatesDecl>) -> Option<TranslatesEntry> {
    t.as_ref().map(|t| TranslatesEntry {
        package: t.package.clone(),
        version: t.version.clone(),
    })
}

/// Project `[media]` — the card's images as package-relative paths.
/// Separators are normalised to `/` for the same reason `boot_snippet`
/// normalises its own: the wire is one path grammar, not the scanning
/// host's.
pub fn media_from(m: &Option<MediaDecl>) -> Option<MediaEntry> {
    let entry = MediaEntry {
        icon: m.as_ref().and_then(|m| m.icon.as_deref()).map(wire_path),
        banner: m.as_ref().and_then(|m| m.banner.as_deref()).map(wire_path),
        preview: m.as_ref().and_then(|m| m.preview.as_deref()).map(wire_path),
    };
    (!entry.is_empty()).then_some(entry)
}

/// A package-relative path as the wire spells it: `/` separators
/// whatever the scanning host uses.
fn wire_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

pub fn boot_snippet_from(b: &Option<BootSnippet>) -> Option<BootSnippetEntry> {
    b.as_ref().map(|bs| BootSnippetEntry {
        source: bs.source.to_string_lossy().replace('\\', "/"),
        category: bs.category.map(boot_category_str),
    })
}

/// Project immutable external-source declarations into the public catalog.
/// These are provenance rows, not dependency nodes: their independent hash
/// and upstream licence remain visibly separate from the bridge package's
/// own content hash and licence.
pub fn embedded_sources_from(sources: &[EmbeddedSourceDecl]) -> Vec<EmbeddedSourceEntry> {
    sources
        .iter()
        .map(|source| EmbeddedSourceEntry {
            name: source.name.clone(),
            kind: match source.kind {
                EmbeddedSourceKind::Git => "git".to_string(),
            },
            source_url: source.url.clone(),
            source_ref: source.ref_hint.clone(),
            resolved_commit: source.commit.clone(),
            content_hash: source.content_hash.to_string(),
            upstream_license: source.upstream_license.clone(),
            upstream_authors: source.upstream_authors.clone(),
            license_path: source.license_path.to_string_lossy().replace('\\', "/"),
            license_url: source.license_url.clone(),
        })
        .collect()
}

/// Project the `[origin]` provenance marker (PROP-007 §2.8) — present
/// only on a copy `vibe workspace publish` generated from a workspace
/// member — into the index entry's `workspace_origin` (PROP-008 §2.8).
pub fn workspace_origin_from(origin: &Option<OriginSection>) -> Option<WorkspaceOriginEntry> {
    origin.as_ref().map(|o| WorkspaceOriginEntry {
        upstream: o.upstream.clone(),
        path: o.path.clone(),
        commit: o.commit.clone(),
        generated_by: o.generated_by.clone(),
        generated_at: o.generated_at.clone(),
    })
}

/// The kebab-case wire string for a boot category — the form `vibe-core`
/// serialises and the [`BootSnippetEntry`] records.
fn boot_category_str(c: BootCategory) -> String {
    match c {
        BootCategory::Foundation => "foundation",
        BootCategory::Flow => "flow",
        BootCategory::Stack => "stack",
        BootCategory::Tool => "tool",
        BootCategory::App => "app",
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
    use crate::types::VersionEntry;

    #[test]
    fn parse_manifest_minimal() {
        let body = br#"
[package]
group = "org.vibevm"
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
    fn bridge_source_provenance_projects_without_upstream_bytes() {
        let body = br#"
[package]
group = "org.example"
name = "upstream-tool"
kind = "tool"
version = "1.0.0"
bridge = true
license = "UPL-1.0"

[[embedded_source]]
name = "upstream"
kind = "git"
url = "https://github.com/example/upstream.git"
commit = "0123456789abcdef0123456789abcdef01234567"
content_hash = "sha256-tree/1:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
ref_hint = "refs/tags/v1.2.3"
upstream_license = "MIT"
upstream_authors = ["Example Contributors"]
license_path = "LICENSE"
license_url = "https://github.com/example/upstream/blob/0123456789abcdef0123456789abcdef01234567/LICENSE"
"#;
        let manifest = parse_manifest(body).unwrap();
        assert!(require_package(&manifest).unwrap().bridge);
        let sources = embedded_sources_from(&manifest.embedded_sources);
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].kind, "git");
        assert_eq!(sources[0].upstream_license, "MIT");
        assert_eq!(sources[0].upstream_authors, ["Example Contributors"]);
        assert_eq!(sources[0].license_path, "LICENSE");
    }

    #[test]
    fn parse_manifest_with_provides_requires() {
        let body = br#"
[package]
group = "org.vibevm"
name = "welcome"
kind = "feat"
version = "0.3.0"

[provides]
capabilities = ["ui:landing-page@0.3.0"]

[requires]
capabilities = ["db:any@>=1.0"]

[requires.packages]
"org.vibevm/wal" = "^0.1"

[[requires_any]]
one_of = ["org.vibevm/rust-cli@^0.1", "org.vibevm/rust-axum@^0.2"]
"#;
        let m = parse_manifest(body).unwrap();
        assert_eq!(
            provides_from(&m.provides)
                .map(|p| p.capabilities.len())
                .unwrap_or(0),
            1
        );
        let req = requires_from(&m.requires).unwrap();
        // The modern `[requires.packages]` table flattens to a
        // `<group>/<name>@<constraint>` pkgref string.
        assert_eq!(req.packages, vec!["org.vibevm/wal@^0.1".to_string()]);
        assert_eq!(req.capabilities, vec!["db:any@>=1.0".to_string()]);
        assert_eq!(requires_any_from(&m.requires_any).len(), 1);
    }

    #[test]
    fn features_split_into_named_and_exclusive() {
        let body = br#"
[package]
group = "org.vibevm"
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
        let f = features_from(&m.features).unwrap();
        assert!(f.features.contains_key("default"));
        assert!(f.features.contains_key("a"));
        assert!(f.features.contains_key("b"));
        assert_eq!(
            f.exclusive.get("group").unwrap(),
            &vec!["a".to_string(), "b".to_string()]
        );
    }

    /// The writer's law: an empty projection is ABSENCE, not a
    /// present-but-empty section. A manifest whose `[provides]` /
    /// `[requires]` tables exist but carry nothing must produce an
    /// entry whose wire form has NO key for them — `"provides": {}` is
    /// a shape this writer never emits. The negative control in the
    /// same test pins the other edge: a non-empty table DOES reach the
    /// wire, so the absence above is normalisation, not a dropped
    /// field.
    #[test]
    fn empty_projection_tables_never_reach_the_wire() {
        let empty_tables = br#"
[package]
group = "org.vibevm"
name = "wal"
kind = "flow"
version = "0.1.0"

[provides]
capabilities = []

[requires]
capabilities = []
"#;
        let m = parse_manifest(empty_tables).unwrap();
        let mut v = VersionEntry::minimal(
            PackageKind::Flow,
            m.package.as_ref().unwrap().group.clone(),
            "wal",
            "0.1.0".parse().unwrap(),
            chrono::Utc::now(),
        );
        v.provides = provides_from(&m.provides);
        v.requires = requires_from(&m.requires);
        let json = serde_json::to_string(&v).unwrap();
        assert!(!json.contains("\"provides\""), "{json}");
        assert!(!json.contains("\"requires\""), "{json}");

        let filled = br#"
[package]
group = "org.vibevm"
name = "wal"
kind = "flow"
version = "0.1.0"

[provides]
capabilities = ["ui:landing-page"]
"#;
        let m = parse_manifest(filled).unwrap();
        v.provides = provides_from(&m.provides);
        let json = serde_json::to_string(&v).unwrap();
        assert!(json.contains("\"provides\""), "{json}");
    }

    /// A `doc` package's manifest carries the card and both directions
    /// of the documentation edge into its index entry, so the site and
    /// the local reader answer «who documents X» and «which languages
    /// are there» by folding `primary.jsonl` instead of downloading
    /// packages (PROP-057 `##REL-INDEX-FIELDS`,
    /// `##REL-REVERSE-QUERIES-SITE-SIDE`).
    #[test]
    fn a_doc_manifest_projects_its_card_and_its_relations() {
        // A raw `str`, not a byte string: the title of a Russian
        // adaptation is the realistic case, and a byte-string literal
        // cannot hold it.
        let body = r#"
[package]
group = "org.vibevm"
name = "vibevm-docs-ru"
kind = "doc"
version = "0.1.0"
title = "Руководство VibeVM"
abstract = "Что покрывает, для кого, что предполагает известным."

[i18n]
canonical = "ru"

[[documents]]
package = "org.vibevm.core/vibevm"
version = "^0.1"

[documentation]
primary = "org.vibevm/vibevm-docs"
official = ["org.vibevm/vibevm-tutorials"]

[translates]
package = "org.vibevm/vibevm-docs"
version = "^0.1"

[media]
icon = "media/icon.png"
banner = 'media\banner.jpg'
"#;
        let m = parse_manifest(body.as_bytes()).unwrap();
        let pkg = require_package(&m).unwrap();
        assert_eq!(package_kind(pkg.kind), PackageKind::Doc);
        assert_eq!(pkg.title.as_deref(), Some("Руководство VibeVM"));
        assert!(pkg.abstract_text.is_some());

        let documents = documents_from(&m.documents);
        assert_eq!(documents.len(), 1);
        assert_eq!(documents[0].package, "org.vibevm.core/vibevm");
        assert_eq!(documents[0].version, "^0.1");

        let documentation = documentation_from(&m.documentation).expect("the subject pointer");
        assert_eq!(
            documentation.primary.as_deref(),
            Some("org.vibevm/vibevm-docs")
        );
        assert_eq!(documentation.official, vec!["org.vibevm/vibevm-tutorials"]);

        let translates = translates_from(&m.translates).expect("the source edge");
        assert_eq!(translates.package, "org.vibevm/vibevm-docs");

        // The language of a documentation is `[i18n].canonical`, never a
        // field of its own (PROP-057 `##LOC-LANGUAGE-FIELD`), and it
        // already had a home in the entry.
        let i18n = i18n_from(&m.i18n).expect("the canonical locale");
        assert_eq!(i18n.default.as_deref(), Some("ru"));

        // Separators are the wire's, not the scanning host's.
        let media = media_from(&m.media).expect("the card's images");
        assert_eq!(media.icon.as_deref(), Some("media/icon.png"));
        assert_eq!(media.banner.as_deref(), Some("media/banner.jpg"));
        assert!(media.preview.is_none());
    }

    /// The same law the sibling projections obey: an empty table is
    /// absence on the wire, never `{}` or `[]`. A package of an ordinary
    /// kind declares none of this and its entry says nothing about it.
    #[test]
    fn a_package_without_documentation_says_nothing_about_it() {
        let body = br#"
[package]
group = "org.vibevm"
name = "wal"
kind = "flow"
version = "0.1.0"

[documentation]
"#;
        let m = parse_manifest(body).unwrap();
        let mut v = VersionEntry::minimal(
            PackageKind::Flow,
            m.package.as_ref().unwrap().group.clone(),
            "wal",
            "0.1.0".parse().unwrap(),
            chrono::Utc::now(),
        );
        v.documents = documents_from(&m.documents);
        v.documentation = documentation_from(&m.documentation);
        v.translates = translates_from(&m.translates);
        v.media = media_from(&m.media);
        let json = serde_json::to_string(&v).unwrap();
        for key in ["documents", "documentation", "translates", "media"] {
            assert!(!json.contains(&format!("\"{key}\"")), "{key}: {json}");
        }
    }

    #[test]
    fn parses_real_fixture() {
        let body = include_bytes!("../../fixtures/golden-flow-wal-1.0.0/vibe.toml");
        let m = parse_manifest(body).unwrap();
        let pkg = require_package(&m).unwrap();
        assert_eq!(pkg.name, "golden-pkg");
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
group = "org.vibevm"
name = "wal"
kind = "flow"
version = "0.1.0"

[boot_snippet]
source = "boot/10-flow-wal.md"
category = "flow"
link = "static"
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
