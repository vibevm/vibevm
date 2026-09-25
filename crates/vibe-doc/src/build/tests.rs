//! What a build writes, and where — on a package made here, so the
//! layout is asserted against the address map rather than against one
//! manual's shape.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{TimeZone, Utc};

use super::*;
use crate::manifest::tests::fixture;

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

#[test]
fn reconciled_writes_reuse_equal_files_and_remove_stale_outputs() {
    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("site");
    fs::create_dir_all(&out).unwrap();
    fs::write(out.join("stale.txt"), "old").unwrap();
    let first = Built {
        files: vec![BuiltFile {
            path: "page/index.html".into(),
            bytes: b"one".to_vec(),
        }],
        ..Built::default()
    };
    let written = write_reconciled(&first, &out).unwrap();
    assert_eq!(written.written, 1);
    assert_eq!(written.removed, 1);
    assert!(!out.join("stale.txt").exists());

    let reused = write_reconciled(&first, &out).unwrap();
    assert_eq!(reused.unchanged, 1);
    assert_eq!(reused.written, 0);

    let changed = Built {
        files: vec![BuiltFile {
            path: "page/index.html".into(),
            bytes: b"two".to_vec(),
        }],
        ..Built::default()
    };
    let rewritten = write_reconciled(&changed, &out).unwrap();
    assert_eq!(rewritten.written, 1);
    assert_eq!(fs::read(out.join("page/index.html")).unwrap(), b"two");
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

/// Where every page of the borrowed-example fixture's adaptation is
/// written, from the address map.
const BORROWED: &str = "com.example.docs/borrowed-ru/0.1.0/";

/// The world that reaches the borrowed-example fixture's source: the
/// checkout arm answers for the coordinate the adaptation adapts.
fn borrowed_world() -> SpecSources {
    SpecSources::for_checkout(
        fixture("borrowed/source"),
        Some("com.example.docs"),
        "borrowed",
    )
}

/// One build of the borrowed-example fixture's adaptation, as text by
/// output address.
fn borrowed_build(sources: &SpecSources, format: Format) -> BTreeMap<String, String> {
    build(&fixture("borrowed/adaptation"), sources, &options(format))
        .expect("the fixture builds")
        .files
        .iter()
        .map(|file| {
            (
                file.path.clone(),
                String::from_utf8_lossy(&file.bytes).into_owned(),
            )
        })
        .collect()
}

/// The defect B-176 named, on the fixture built to catch it: both pages of
/// the adaptation write the same `<example ref="demo"/>`, and each must
/// come back carrying ITS OWN source page's command. A bundle keyed by
/// bare id would give them both the first page's, and report no gap.
#[test]
fn each_page_of_a_translation_borrows_from_the_source_page_at_its_address() {
    let files = borrowed_build(&borrowed_world(), Format::Html);
    let first = &files[&format!("{BORROWED}guide/first/index.html")];
    let second = &files[&format!("{BORROWED}guide/second/index.html")];

    assert!(first.contains("data-example-ref=\"demo\""), "{first}");
    assert!(first.contains("vibe --version"), "{first}");
    assert!(first.contains("vibe 1.0.0"), "{first}");
    assert!(!first.contains("vibe list"), "the second page's: {first}");
    assert!(!first.contains("data-unresolved"), "{first}");

    assert!(second.contains("data-example-ref=\"demo\""), "{second}");
    assert!(second.contains("vibe list"), "{second}");
    assert!(second.contains("no packages"), "{second}");
    // Every field of a body travels, not only the command: the language,
    // the failing exit code and the stderr the source spelled.
    assert!(second.contains("class=\"language-ps1\""), "{second}");
    assert!(second.contains("data-exit=\"2\""), "{second}");
    assert!(second.contains("error: nothing is installed"), "{second}");
    assert!(
        !second.contains("vibe --version"),
        "the first page's: {second}"
    );
    assert!(!second.contains("data-unresolved"), "{second}");
}

/// The Markdown carries the source's fences rather than the note that says
/// they will be copied — the note is what an unresolved reference reads as.
#[test]
fn the_markdown_of_a_translation_carries_the_fences_it_borrowed() {
    let files = borrowed_build(&borrowed_world(), Format::Md);
    let first = &files[&format!("{BORROWED}guide/first.md")];
    let second = &files[&format!("{BORROWED}guide/second.md")];

    assert!(first.contains("```sh\nvibe --version\n```"), "{first}");
    assert!(first.contains("```output\nvibe 1.0.0\n```"), "{first}");
    assert!(!first.contains("copied from the source page"), "{first}");

    assert!(second.contains("```ps1\nvibe list\n```"), "{second}");
    assert!(second.contains("```output\nno packages\n```"), "{second}");
    assert!(
        second.contains("```stderr\nerror: nothing is installed\n```"),
        "{second}"
    );
    assert!(!second.contains("vibe --version"), "{second}");
    assert!(!second.contains("copied from the source page"), "{second}");
}

/// The XML projection keeps the ADDRESS by its own contract: it is the
/// form a tool resolves itself, and a copied body there would be the
/// stored copy the whole design exists to avoid.
#[test]
fn the_xml_projection_of_a_translation_keeps_the_reference() {
    let files = borrowed_build(&borrowed_world(), Format::Xml);
    let first = &files[&format!("{BORROWED}guide/first.xml")];
    assert!(first.contains("<example ref=\"demo\""), "{first}");
    assert!(!first.contains("<run>"), "{first}");
}

/// A source this build cannot reach is a marked GAP, not a failure. A
/// renderer that aborted here would report one defect per run, and which
/// gaps are errors is `vibe doc check --translations`' question.
#[test]
fn a_source_this_build_cannot_reach_leaves_a_marked_gap_and_still_builds() {
    let files = borrowed_build(&SpecSources::new(), Format::Html);
    let first = &files[&format!("{BORROWED}guide/first/index.html")];
    assert!(first.contains("data-unresolved=\"true\""), "{first}");
    assert!(!first.contains("vibe --version"), "{first}");
}

/// A source documentation borrows nothing: it authors its examples, and
/// the bundle a build hands its renderers is empty.
#[test]
fn a_source_documentation_carries_no_borrowed_examples() {
    let (_, content) = content(
        &fixture("borrowed/source"),
        &borrowed_world(),
        "/doc/",
        BTreeMap::new(),
    )
    .unwrap();
    assert!(content.examples.is_empty(), "{:?}", content.examples);
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
