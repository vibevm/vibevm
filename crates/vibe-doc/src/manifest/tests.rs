//! Building a manifest over a real package tree.

use chrono::{TimeZone, Utc};
use vibe_wire::generated::doc_manifest::{DocumentationStatus, PageGenre, TranslationStatus};

use super::*;

/// The fixture documentation package the island golden also renders — a
/// real tree, because a package is its own source tree (PROP-024) and a
/// golden taken from a tree is one somebody can open and correct.
pub(crate) fn fixture(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixture")
        .join(name)
}

pub(crate) fn rendered_at() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, 12, 9, 0, 0)
        .single()
        .expect("instant")
}

fn built() -> Built {
    build(
        &fixture("manual"),
        &SpecSources::new(),
        &Options::at(rendered_at()),
    )
    .expect("the fixture package builds")
}

/// The card is the manifest's, taken from the package's own declaration:
/// the coordinate a site address is built from, the title a person reads,
/// the publisher beside it, the abstract, and the language.
#[test]
fn the_card_is_read_out_of_the_packages_own_manifest() {
    let m = built().manifest;
    assert_eq!(m.schema_version, SCHEMA_VERSION);
    assert_eq!(m.package.group.as_str(), "com.example.docs");
    assert_eq!(m.package.name, "fixture-manual");
    assert_eq!(m.package.version.to_string(), "0.1.0");
    assert_eq!(m.package.publisher, "com.example.docs");
    assert_eq!(m.package.title, "Fixture Manual");
    assert_eq!(m.package.lang, "en");
    assert!(m.package.abstract_.starts_with("What it covers:"));
    assert!(m.package.description.is_some());
}

/// The card names where its three images are served from, and the
/// addresses are the ones the build actually writes (X-042).
///
/// Two claims, and the second is the one that matters: the shell shows
/// what this names and computes nothing, so a manifest whose addresses
/// disagreed with the build's files would be a picture that silently
/// stops loading. The comparison is against the build's own output rather
/// than against a spelling typed here, which is the only way the two can
/// be checked to agree instead of merely both looking plausible.
#[test]
fn the_card_names_the_addresses_the_build_writes_its_images_at() {
    let card = built().manifest.package;
    let media = card
        .media
        .expect("every build of this pipeline writes the addresses");
    for address in [&media.icon, &media.banner, &media.preview] {
        assert!(
            address.starts_with("media/"),
            "`{address}` is not published where the build puts images"
        );
    }
    // Three roles, three distinct files: a preview is never a crop of the
    // banner, so it is never the same address either.
    let named = std::collections::BTreeSet::from([
        media.icon.clone(),
        media.banner.clone(),
        media.preview.clone(),
    ]);
    assert_eq!(named.len(), 3);

    let built = crate::build::build(
        &fixture("manual"),
        &SpecSources::new(),
        &crate::build::Options {
            format: crate::build::Format::Html,
            base: crate::content::SITE_BASE.to_string(),
            manifest: Options::at(rendered_at()),
            derived: std::collections::BTreeMap::new(),
        },
    )
    .expect("the fixture builds");
    let written: std::collections::BTreeSet<String> = built
        .files
        .iter()
        .filter(|file| file.path.starts_with("media/"))
        .map(|file| file.path.clone())
        .collect();
    assert_eq!(
        named, written,
        "the manifest names files the build did not write"
    );
}

/// The subject is the one `[[documents]]` names, with the constraint it
/// names it under. No source holds it here, so only this documentation's
/// own edge is known — which is what community means.
#[test]
fn the_subject_and_its_status_come_from_the_two_edges() {
    let m = built().manifest;
    assert_eq!(m.package.subjects.len(), 1);
    assert_eq!(m.package.subjects[0].package, "com.example/subject");
    assert_eq!(m.package.subjects[0].version, "^1.0");
    assert_eq!(m.package.subjects[0].status, DocumentationStatus::Community);
    assert_eq!(m.package.status, DocumentationStatus::Community);
}

/// A package that declares no `[translates]` is a source documentation,
/// and its absence is what says so — there is no second flag to disagree
/// with it.
#[test]
fn a_source_documentation_carries_no_translation_block() {
    assert!(built().manifest.package.translation.is_none());
}

/// One page, and everything about it taken from the page.
#[test]
fn the_page_row_is_the_page() {
    let m = built().manifest;
    assert_eq!(m.pages.len(), 1);
    let page = &m.pages[0];
    assert_eq!(page.path, "guide/every-block.xml");
    assert_eq!(page.title, "Every block once");
    // It carries two `prompt` blocks, so it is a task page.
    assert_eq!(page.genre, PageGenre::Task);
    assert_eq!(page.audiences, vec![Audience::User, Audience::Dev]);
    assert!(page.anchors.starts_with(&["root".to_owned()]));
    assert!(page.summary.starts_with("This page uses every block"));
    assert!(page.reading_time_min >= 1);
    assert_eq!(page.reviewed_at, None);
}

/// The card's audiences are the union of its pages' — derived from the
/// markup, never declared (`##CARD-NO-AUDIENCE-DECLARATION`).
#[test]
fn the_cards_audiences_are_the_pages_audiences() {
    assert_eq!(
        built().manifest.package.audiences,
        vec![Audience::User, Audience::Dev]
    );
}

/// The clock is an input. Two builds of one tree at one instant produce
/// the same bytes, which is what makes a site build cacheable.
#[test]
fn two_builds_of_one_tree_at_one_instant_are_the_same_bytes() {
    assert_eq!(to_json(&built().manifest), to_json(&built().manifest));
}

/// The manifest is a projection, not a check: it carries no publication
/// date for a package read out of a tree, because a package read out of a
/// tree has none.
#[test]
fn a_package_read_from_a_tree_has_no_publication_date() {
    assert!(built().manifest.package.published_at.is_none());
    let when = rendered_at();
    let published = build(
        &fixture("manual"),
        &SpecSources::new(),
        &Options::at(when).published(when),
    )
    .expect("builds");
    assert_eq!(published.manifest.package.published_at, Some(when));
}

/// A page that has been read aloud carries the date the package recorded,
/// lifted to midnight UTC — the one staleness signal the project keeps.
#[test]
fn a_recorded_read_aloud_date_reaches_the_page_row() {
    let built = build(
        &fixture("translations/source"),
        &SpecSources::new(),
        &Options::at(rendered_at()),
    )
    .expect("builds");
    let read: Vec<Option<String>> = built
        .manifest
        .pages
        .iter()
        .map(|p| p.reviewed_at.map(|d| d.to_rfc3339()))
        .collect();
    assert!(
        read.contains(&Some("2026-09-11T00:00:00+00:00".to_owned())),
        "{read:?}"
    );
    assert!(read.contains(&None), "the other page was never read aloud");
}

/// A translation's card names what it adapts and how it stands for it.
#[test]
fn a_translation_names_its_source_and_its_standing() {
    let built = build(
        &fixture("translations/adaptation"),
        &SpecSources::new(),
        &Options::at(rendered_at()),
    )
    .expect("the adaptation builds");
    let translation = built
        .manifest
        .package
        .translation
        .expect("an adaptation declares `[translates]`");
    assert_eq!(translation.package, "com.example.docs/pair");
    assert_eq!(translation.status, TranslationStatus::Official);
    assert_eq!(built.manifest.package.lang, "ru");
}

/// A card without the fields a `doc` package must declare is named as a
/// manifest defect, with the file that carries it.
#[test]
fn a_package_with_no_card_is_refused_by_name() {
    let tmp = tempfile::tempdir().expect("temp");
    std::fs::write(
        tmp.path().join("vibe.toml"),
        "[package]\nname = \"x\"\ngroup = \"org.demo\"\nkind = \"doc\"\nversion = \"0.1.0\"\n",
    )
    .expect("write");
    let error =
        build(tmp.path(), &SpecSources::new(), &Options::at(rendered_at())).expect_err("refused");
    assert!(error.to_string().contains("`[package].title`"), "{error}");
    assert!(
        error.to_string().contains("PROP-057#CARD-FIELDS"),
        "{error}"
    );
}
