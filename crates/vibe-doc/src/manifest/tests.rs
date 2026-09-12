//! Building a manifest over a real package tree.

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

fn built() -> Built {
    build(&fixture("manual"), &SpecSources::new()).expect("the fixture package builds")
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

/// A translation's card names what it adapts and how it stands for it.
#[test]
fn a_translation_names_its_source_and_its_standing() {
    let built = build(&fixture("translations/adaptation"), &SpecSources::new())
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
    let error = build(tmp.path(), &SpecSources::new()).expect_err("refused");
    assert!(error.to_string().contains("`[package].title`"), "{error}");
    assert!(
        error.to_string().contains("PROP-057#CARD-FIELDS"),
        "{error}"
    );
}
