//! Unit tests for [`super`], out-of-line per the file-length budget.
//!
//! The scaffolded text is checked by PARSING it, never by matching
//! strings: a manifest a scaffold writes and `vibe-core` then refuses
//! would be the worst possible first experience of a kind, and only the
//! real parser can rule that out.

use std::fs;

use tempfile::tempdir;
use vibe_core::PackageKind;
use vibe_core::manifest::Manifest;

use super::*;

fn fields() -> ProjectFields {
    ProjectFields {
        name: "acme-docs".to_string(),
        version: "0.1.0".to_string(),
        authors: vec!["A. Author".to_string()],
        license: "UPL-1.0".to_string(),
        description: "The Acme manual".to_string(),
        format: "normal".to_string(),
    }
}

fn quiet() -> output::Context {
    output::Context::from_flags(true, false, None, true, crate::cli::AgentModeArg::Auto)
        .quiet_child()
}

/// The scaffolded manifest parses, validates, and is a `doc` package
/// with the card its kind requires — the three things `Manifest::read`
/// would refuse it for.
#[test]
fn the_scaffolded_manifest_parses_and_validates() {
    let project = tempdir().unwrap();
    let pkg_dir = project.path().join("pkg");
    create_doc_package(
        &quiet(),
        project.path(),
        &pkg_dir,
        "org.acme",
        "acme-docs",
        &fields(),
        None,
    )
    .unwrap();

    let manifest = Manifest::read(pkg_dir.join(Manifest::FILENAME))
        .expect("the scaffolded manifest must parse and validate");
    let package = manifest.package.expect("a [package] table");
    assert_eq!(package.kind, PackageKind::Doc);
    assert!(package.title.is_some(), "a doc package owes a title");
    assert!(package.abstract_text.is_some(), "and an abstract");
    assert_eq!(manifest.documents.len(), 1, "and at least one subject");
    // The language is `[i18n].canonical`; `lang` is refused by name, so
    // a scaffold that wrote it would produce an unparseable manifest.
    assert_eq!(manifest.i18n.canonical, "en");
}

/// The four questions are in the file, as a comment — an author who
/// answers them has written the abstract, while one handed a
/// lorem-ipsum paragraph has written nothing and does not know it.
#[test]
fn the_abstract_carries_the_four_questions() {
    let project = tempdir().unwrap();
    let pkg_dir = project.path().join("pkg");
    create_doc_package(
        &quiet(),
        project.path(),
        &pkg_dir,
        "org.acme",
        "acme-docs",
        &fields(),
        None,
    )
    .unwrap();
    let text = fs::read_to_string(pkg_dir.join(Manifest::FILENAME)).unwrap();
    for question in [
        "What does this documentation cover?",
        "Who is it for?",
        "What does it assume the reader already knows?",
        "What does it deliberately leave out?",
    ] {
        assert!(text.contains(question), "missing `{question}`:\n{text}");
    }
}

/// Everything the scaffold owes, and nothing that belongs to another
/// kind: a doc package has no boot lane at all.
#[test]
fn the_tree_has_the_pages_the_card_and_no_boot_lane() {
    let project = tempdir().unwrap();
    let pkg_dir = project.path().join("pkg");
    create_doc_package(
        &quiet(),
        project.path(),
        &pkg_dir,
        "org.acme",
        "acme-docs",
        &fields(),
        None,
    )
    .unwrap();

    for owed in ["vibe.toml", "README.md", "specmap.toml", "media/.gitkeep"] {
        assert!(pkg_dir.join(owed).is_file(), "missing `{owed}`");
    }
    assert!(
        pkg_dir
            .join(vibe_core::layout::current_specs_root())
            .join("README.xml")
            .is_file(),
        "the example page is the shape an author copies"
    );
    assert!(
        !pkg_dir.join(vibe_core::layout::current_boot_dir()).exists(),
        "documentation never enters a boot lane"
    );

    let specmap = fs::read_to_string(pkg_dir.join("specmap.toml")).unwrap();
    let parsed: toml::Table = specmap.parse().expect("the specmap policy parses");
    assert_eq!(
        parsed["spec_roots"].as_array().unwrap().len(),
        1,
        "the pages are the one spec root"
    );
    assert!(
        parsed["scan_roots"].as_array().unwrap().is_empty(),
        "a documentation package ships no code to tag"
    );
}

/// Re-running the scaffold over a package under edit keeps every file.
#[test]
fn a_second_run_keeps_what_is_already_there() {
    let project = tempdir().unwrap();
    let pkg_dir = project.path().join("pkg");
    let args = ("org.acme", "acme-docs");
    create_doc_package(
        &quiet(),
        project.path(),
        &pkg_dir,
        args.0,
        args.1,
        &fields(),
        None,
    )
    .unwrap();
    let readme = pkg_dir.join("README.md");
    fs::write(&readme, "# mine\n").unwrap();

    let outcomes = create_doc_package(
        &quiet(),
        project.path(),
        &pkg_dir,
        args.0,
        args.1,
        &fields(),
        None,
    )
    .unwrap();
    assert!(
        outcomes.iter().all(|o| matches!(o.action, Action::Kept)),
        "a re-run creates nothing: {outcomes:?}"
    );
    assert_eq!(fs::read_to_string(&readme).unwrap(), "# mine\n");
}

/// Plant a source documentation in the project's own package root.
fn plant_source(project_root: &std::path::Path) {
    let dir = project_root
        .join(vibe_core::layout::current_packages_root())
        .join("org.acme")
        .join("acme-docs")
        .join("v0.3.1");
    let pages = dir.join(vibe_core::layout::current_specs_root());
    fs::create_dir_all(pages.join("start")).unwrap();
    fs::write(
        dir.join(Manifest::FILENAME),
        r#"[package]
group = "org.acme"
name = "acme-docs"
kind = "doc"
version = "0.3.1"
title = "The Acme Manual"
abstract = "Four answers."

[[documents]]
package = "org.acme/widget"
version = "^2.0"
"#,
    )
    .unwrap();
    fs::write(pages.join("index.xml"), "<spec/>\n").unwrap();
    fs::write(pages.join("start/install.xml"), "<spec>install</spec>\n").unwrap();
}

/// A translation mirrors its source file for file, copies the source's
/// subjects verbatim, and names its own language — all three of which
/// the gate later checks.
#[test]
fn a_translation_mirrors_its_source_and_copies_its_subjects() {
    let project = tempdir().unwrap();
    plant_source(project.path());
    let source = resolve_translation(project.path(), "org.acme/acme-docs", "acme-docs-ru")
        .expect("the source is readable in the project's own package root");
    assert_eq!(source.language, "ru");
    assert_eq!(
        source.constraint, "^0.3",
        "the constraint follows the source's own version line"
    );

    let pkg_dir = project.path().join("ru");
    create_doc_package(
        &quiet(),
        project.path(),
        &pkg_dir,
        "org.acme",
        "acme-docs-ru",
        &fields(),
        Some(&source),
    )
    .unwrap();

    let manifest = Manifest::read(pkg_dir.join(Manifest::FILENAME))
        .expect("a scaffolded translation must parse and validate");
    assert_eq!(manifest.i18n.canonical, "ru");
    // The language suffix in the package name is the convention, not
    // part of what a reader is shown.
    assert_eq!(
        manifest.package.as_ref().unwrap().title.as_deref(),
        Some("Acme Docs (ru)")
    );
    let translates = manifest.translates.expect("the source edge");
    assert_eq!(translates.package, "org.acme/acme-docs");
    assert_eq!(translates.version, "^0.3");
    assert_eq!(manifest.documents.len(), 1);
    assert_eq!(manifest.documents[0].package, "org.acme/widget");
    assert_eq!(manifest.documents[0].version, "^2.0");

    let pages = pkg_dir.join(vibe_core::layout::current_specs_root());
    assert!(pages.join("index.xml").is_file(), "the tree is mirrored");
    assert!(
        pages.join("start/install.xml").is_file(),
        "including nested pages — the same paths, file for file"
    );
    assert!(
        !pages.join("README.xml").exists(),
        "a mirror carries the source's pages, not an invented one"
    );
}

/// The language comes from the name, and a name that does not follow the
/// `<docname>-<lang>` convention is refused by teaching it rather than
/// by guessing.
#[test]
fn a_name_that_names_no_language_is_refused_with_the_convention() {
    let project = tempdir().unwrap();
    plant_source(project.path());
    let err = resolve_translation(project.path(), "org.acme/acme-docs", "acme-manual")
        .expect_err("a name that hides the language cannot be scaffolded");
    let message = format!("{err:#}");
    assert!(message.contains("acme-docs-<lang>"), "{message}");
    assert!(message.contains("LOC-OFFICIAL-TRANSLATION"), "{message}");
}

/// A source that is on no disk here is not a scaffold this command can
/// honestly write, and the refusal names the remedy.
#[test]
fn an_unreachable_source_names_the_warm_up() {
    let project = tempdir().unwrap();
    let err = resolve_translation(project.path(), "org.acme/acme-docs", "acme-docs-ru")
        .expect_err("nothing to mirror");
    let message = format!("{err:#}");
    assert!(
        message.contains("vibe cache add org.acme/acme-docs"),
        "{message}"
    );
}
