//! A level-0 render, composed and then read back through the pivot.
//!
//! The pivot is the test that matters. A page this module writes is a
//! page in the closed dialect or it is nothing, and the only reader
//! whose opinion counts is the one the site uses.

use std::fs;
use std::path::Path;

use super::*;

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().expect("a parent")).expect("the directory");
    fs::write(path, text).expect("writing");
}

/// A package that wrote no documentation at all: a manifest, a README,
/// a boot snippet and one specification.
fn plain(root: &Path) {
    write(
        &root.join("vibe.toml"),
        "[package]\n\
         name = \"wal\"\n\
         group = \"org.example\"\n\
         version = \"1.2.0\"\n\
         kind = \"flow\"\n\
         license = \"UPL-1.0\"\n\
         authors = [\"A Person\", \"Another\"]\n\
         keywords = [\"log\", \"recovery\"]\n\
         description = \"A write-ahead log for sessions\"\n\
         \n\
         [boot_snippet]\n\
         path = \"snippet.md\"\n\
         \n\
         [[skill]]\n\
         name = \"wal\"\n\
         path = \"skills/wal\"\n",
    );
    write(
        &root.join("README.md"),
        "# wal\n\n\
         A log you can replay. It is *small* and it has `no` dependencies.\n\n\
         ## Install\n\n\
         Run this:\n\n\
         ```sh\n\
         vibe install org.example/wal\n\
         ```\n\n\
         ## Install\n\n\
         The same heading twice, which a README is allowed to do.\n",
    );
    write(
        &root.join("vibevm/vibespecs/common/PROP-001.xml"),
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <spec xmlns=\"https://vibevm.org/spec/1\">\n  \
         <title id=\"root\">The log</title>\n  \
         <p>What the log promises.</p>\n\
         </spec>\n",
    );
}

fn composed(root: &Path, work: &Path) -> Composed {
    plain(root);
    compose(root, work).expect("a composition")
}

#[test]
fn every_composed_page_is_readable_by_the_pivot() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    let out = composed(&tmp.path().join("pkg"), &tmp.path().join("work"));
    let set = crate::pages::read_package(&out.dir).expect("a page set");
    assert!(
        set.unreadable.is_empty(),
        "the pivot refused: {:?}",
        set.unreadable
            .iter()
            .map(|u| format!("{}: {}", u.rel, u.message))
            .collect::<Vec<_>>()
    );
    // The manifest page, the README, the boot snippet and the copied
    // specification.
    assert_eq!(set.pages.len(), 4, "{:?}", page_names(&set));
}

#[test]
fn a_specification_keeps_its_own_path_because_the_path_is_the_address() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    let out = composed(&tmp.path().join("pkg"), &tmp.path().join("work"));
    assert_eq!(out.copied, 1);
    let set = crate::pages::read_package(&out.dir).expect("a page set");
    assert!(
        page_names(&set).contains(&"common/PROP-001.xml".to_string()),
        "{:?}",
        page_names(&set)
    );
}

#[test]
fn the_composed_pages_are_the_three_level_zero_names() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    let out = composed(&tmp.path().join("pkg"), &tmp.path().join("work"));
    assert_eq!(
        out.generated,
        vec![
            MANIFEST_PAGE.to_string(),
            README_PAGE.to_string(),
            BOOT_SNIPPET_PAGE.to_string()
        ]
    );
}

/// The card of a package that never wrote one still has to say
/// something, and what it says is what the package says about itself.
#[test]
fn a_package_with_no_card_gets_one_composed_from_what_it_did_declare() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    let out = composed(&tmp.path().join("pkg"), &tmp.path().join("work"));
    let manifest = fs::read_to_string(out.dir.join("vibe.toml")).expect("the manifest");
    assert!(manifest.contains("title = \"wal\""), "{manifest}");
    assert!(
        manifest.contains("abstract = \"A write-ahead log for sessions\""),
        "{manifest}"
    );
    assert!(manifest.contains("kind = \"flow\""), "{manifest}");
}

#[test]
fn a_package_with_neither_title_nor_description_is_still_described() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    let root = tmp.path().join("pkg");
    write(
        &root.join("vibe.toml"),
        "[package]\nname = \"bare\"\ngroup = \"org.example\"\nversion = \"0.1.0\"\nkind = \"lang\"\n",
    );
    let out = compose(&root, &tmp.path().join("work")).expect("a composition");
    let manifest = fs::read_to_string(out.dir.join("vibe.toml")).expect("the manifest");
    assert!(manifest.contains("title = \"bare\""), "{manifest}");
    assert!(manifest.contains("wrote no abstract"), "{manifest}");
    let set = crate::pages::read_package(&out.dir).expect("a page set");
    assert!(set.unreadable.is_empty());
}

/// The host's root is a `[project]` and the site addresses it like every
/// other version.
#[test]
fn a_project_is_addressed_the_way_a_package_is() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    let root = tmp.path().join("host");
    write(
        &root.join("vibe.toml"),
        "[project]\nname = \"vibevm\"\ngroup = \"org.vibevm.core\"\nversion = \"1.0.0\"\n",
    );
    let out = compose(&root, &tmp.path().join("work")).expect("a composition");
    let manifest = fs::read_to_string(out.dir.join("vibe.toml")).expect("the manifest");
    assert!(manifest.contains("name = \"vibevm\""), "{manifest}");
    assert!(
        manifest.contains("group = \"org.vibevm.core\""),
        "{manifest}"
    );
}

/// An authored page is always the better page.
#[test]
fn an_authored_page_under_a_reserved_name_is_kept() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    let root = tmp.path().join("pkg");
    plain(&root);
    write(
        &root.join("vibevm/vibespecs").join(README_PAGE),
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <spec xmlns=\"https://vibevm.org/spec/1\">\n  \
         <title id=\"root\">Mine</title>\n\
         </spec>\n",
    );
    let out = compose(&root, &tmp.path().join("work")).expect("a composition");
    assert!(!out.generated.contains(&README_PAGE.to_string()));
    assert!(
        out.notes.iter().any(|n| n.contains(README_PAGE)),
        "{:?}",
        out.notes
    );
    let kept = fs::read_to_string(out.dir.join("vibevm/vibespecs").join(README_PAGE))
        .expect("the authored page");
    assert!(kept.contains("Mine"), "{kept}");
}

/// The relation tables are the author's statements and a level-0 render
/// carries them unchanged, because officiality is computed from them.
#[test]
fn the_relation_tables_are_carried_across_unchanged() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    let root = tmp.path().join("pkg");
    write(
        &root.join("vibe.toml"),
        "[package]\n\
         name = \"wal-docs\"\n\
         group = \"org.example\"\n\
         version = \"0.1.0\"\n\
         kind = \"doc\"\n\
         title = \"The wal manual\"\n\
         abstract = \"How to run a log.\"\n\
         \n\
         [i18n]\n\
         canonical = \"ru\"\n\
         \n\
         [[documents]]\n\
         package = \"org.example/wal\"\n\
         version = \"^1.0\"\n\
         \n\
         [translates]\n\
         package = \"org.example/wal-docs\"\n\
         version = \"^0.1\"\n",
    );
    let out = compose(&root, &tmp.path().join("work")).expect("a composition");
    let manifest = fs::read_to_string(out.dir.join("vibe.toml")).expect("the manifest");
    assert!(manifest.contains("[[documents]]"), "{manifest}");
    assert!(manifest.contains("org.example/wal"), "{manifest}");
    assert!(manifest.contains("[translates]"), "{manifest}");
    assert!(manifest.contains("canonical = \"ru\""), "{manifest}");
}

/// A composition carries the package's current bytes and nothing else.
#[test]
fn a_page_from_an_earlier_composition_does_not_survive_into_the_next() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    let root = tmp.path().join("pkg");
    let work = tmp.path().join("work");
    plain(&root);
    compose(&root, &work).expect("the first composition");
    fs::remove_file(root.join("vibevm/vibespecs/common/PROP-001.xml")).expect("the removal");
    let out = compose(&root, &work).expect("the second composition");
    assert_eq!(out.copied, 0);
    let set = crate::pages::read_package(&out.dir).expect("a page set");
    assert!(
        !page_names(&set).contains(&"common/PROP-001.xml".to_string()),
        "{:?}",
        page_names(&set)
    );
}

/// A declared image that is not there costs the role a picture, never
/// the package its render.
#[test]
fn a_declared_image_that_is_missing_is_a_note_and_not_a_refusal() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    let root = tmp.path().join("pkg");
    write(
        &root.join("vibe.toml"),
        "[package]\n\
         name = \"wal\"\ngroup = \"org.example\"\nversion = \"1.0.0\"\nkind = \"flow\"\n\
         \n[media]\nicon = \"media/icon.png\"\n",
    );
    let out = compose(&root, &tmp.path().join("work")).expect("a composition");
    assert!(
        out.notes.iter().any(|n| n.contains("media/icon.png")),
        "{:?}",
        out.notes
    );
}

fn page_names(set: &crate::pages::PageSet) -> Vec<String> {
    set.pages.iter().map(|p| p.rel.clone()).collect()
}
