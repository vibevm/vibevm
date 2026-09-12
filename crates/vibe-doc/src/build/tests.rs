//! What a build writes, and where — on a package made here, so the
//! layout is asserted against the address map rather than against one
//! manual's shape.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{TimeZone, Utc};

use super::*;

const PAGE: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<spec xmlns=\"https://vibevm.org/spec/1\">\n  \
  <title id=\"root\">Boot lane</title>\n  \
  <status stage=\"doc\" state=\"work\" audience=\"user\"/>\n  \
  <p>The boot lane is what a session reads first.</p>\n\
</spec>\n";

fn package(tmp: &Path) -> PathBuf {
    let dir = tmp.join("docs");
    fs::create_dir_all(dir.join("vibevm/vibespecs/model")).unwrap();
    fs::write(
        dir.join("vibe.toml"),
        "[package]\nname = \"thing-docs\"\ngroup = \"com.example\"\nkind = \"doc\"\n\
         version = \"0.2.0\"\ntitle = \"Thing Manual\"\nabstract = \"\"\"\nWhat it covers.\n\"\"\"\n\
         [i18n]\ncanonical = \"en\"\n\
         [[documents]]\npackage = \"com.example/thing\"\nversion = \"^1.0\"\n",
    )
    .unwrap();
    fs::write(dir.join("vibevm/vibespecs/model/boot-lane.xml"), PAGE).unwrap();
    dir
}

fn options(format: Format) -> Options {
    Options {
        format,
        base: "/doc/".to_string(),
        manifest: manifest::Options::at(Utc.with_ymd_and_hms(2026, 9, 12, 0, 0, 0).unwrap()),
        derived: BTreeMap::new(),
    }
}

fn paths(built: &Built) -> Vec<String> {
    built.files.iter().map(|f| f.path.clone()).collect()
}

/// The whole build in one case: the page at the address the site map
/// gives it, the machine files beside it, and a placeholder for every
/// role the package declared nothing for.
#[test]
fn a_build_writes_the_address_map_the_site_declares() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = package(tmp.path());
    let built = build(&dir, &SpecSources::new(), &options(Format::Html)).unwrap();

    let paths = paths(&built);
    assert!(
        paths.contains(&"com.example/thing-docs/0.2.0/model/boot-lane/index.html".to_string()),
        "{paths:?}"
    );
    for machine in ["manifest.json", "llms.txt", "llms-full.txt"] {
        assert!(
            paths.contains(&machine.to_string()),
            "{machine} in {paths:?}"
        );
    }
    assert_eq!(built.generated_roles, vec!["icon", "banner", "preview"]);
    assert_eq!(
        paths.iter().filter(|p| p.starts_with("media/")).count(),
        3,
        "{paths:?}"
    );
    assert!(built.unreadable.is_empty());
}

/// A projection lies BESIDE the page's directory as a file — the shape
/// `##SITE-TRAILING-SLASH` fixes, and the reason a reader can ask for
/// `…/boot-lane.md` next to `…/boot-lane/`.
#[test]
fn the_projections_lie_beside_the_page_rather_than_inside_it() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = package(tmp.path());
    for (format, expected) in [
        (
            Format::Md,
            "com.example/thing-docs/0.2.0/model/boot-lane.md",
        ),
        (
            Format::Xml,
            "com.example/thing-docs/0.2.0/model/boot-lane.xml",
        ),
    ] {
        let built = build(&dir, &SpecSources::new(), &options(format)).unwrap();
        assert!(paths(&built).contains(&expected.to_string()), "{expected}");
    }
}

/// A build is a function of the tree and the two instants it is given.
/// The site's render cache and any `--check` over a build rest on it.
#[test]
fn two_builds_of_one_tree_are_the_same_bytes() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = package(tmp.path());
    let first = build(&dir, &SpecSources::new(), &options(Format::Html)).unwrap();
    let second = build(&dir, &SpecSources::new(), &options(Format::Html)).unwrap();
    assert_eq!(first.files, second.files);
}

/// What a build writes is what `write` puts on disk, at the same
/// addresses.
#[test]
fn writing_a_build_creates_the_tree_it_describes() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = package(tmp.path());
    let built = build(&dir, &SpecSources::new(), &options(Format::Html)).unwrap();
    let out = tmp.path().join("site");
    write(&built, &out).unwrap();
    for file in &built.files {
        assert_eq!(
            fs::read(out.join(&file.path)).unwrap(),
            file.bytes,
            "{}",
            file.path
        );
    }
}

/// Every projection carries the same block numbers, so a human quoting
/// `p02` from the island and an agent quoting `p02` from the Markdown
/// quote one block (R-26).
#[test]
fn one_page_carries_one_set_of_numbers_in_every_projection() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = package(tmp.path());
    let (set, content) = content(&dir, &SpecSources::new(), "/doc/", BTreeMap::new()).unwrap();
    let page = &set.pages[0];
    assert!(render_page(page, &content, Format::Html).contains("p01"));
    assert!(render_page(page, &content, Format::Md).contains("[p01]"));
    assert!(render_page(page, &content, Format::Xml).contains("p=\"1\""));
}

/// `--lang` states which language the caller wants, and a package is
/// only ever one. The refusal names where the other one lives, which is
/// the whole value of asking.
#[test]
fn asking_for_another_language_names_the_package_that_holds_it() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = package(tmp.path());
    expect_language(&dir, "en").expect("the package is English");
    let e = expect_language(&dir, "ru").expect_err("it is not Russian");
    assert!(e.to_string().contains("thing-docs-ru"), "{e}");
}
