//! The scanner's own tests: what a parsed `vibe.toml` projects onto the
//! wire, and what it refuses to project.
//!
//! File-backed submodule of [`super`] so every cell stays inside the
//! AI-Native file budget. Nothing moved but the file: each assertion here
//! is the one that stood beside the projection it tests.

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
    let body = include_bytes!("../../../fixtures/golden-flow-wal-1.0.0/vibe.toml");
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
