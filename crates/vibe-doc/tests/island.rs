//! The island golden — one fixture package, one rendered page, pinned
//! byte for byte.
//!
//! A golden is the only test that shows a reviewer what a renderer
//! actually emits. The unit tests in `html::tests` state the laws a
//! golden cannot — that an island carries no script, that a citation
//! carries no revision — and this one states the shape: every block of
//! the documentation genre, once, in one file somebody can read.
//!
//! Refresh it with `VIBE_DOC_BLESS=1 cargo test -p vibe-doc --test
//! island`, and read the diff before committing: a golden that moves
//! without a reason is a renderer that changed without one.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use vibe_doc::build;
use vibe_doc::citations::{RuleText, Source};
use vibe_doc::content::{Content, ExampleBody};
use vibe_doc::html;
use vibe_doc::numbering::Numbering;
use vibe_doc::pages;

fn fixture_package() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixture/manual")
}

fn golden_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name)
}

/// What the pipeline would have fetched for this page: one rule that
/// resolves, one that does not, one generated block and one borrowed
/// example. The second rule is deliberately absent — an island has to
/// show an unresolved citation honestly, and a golden that never
/// contains one cannot prove it.
///
/// The rule that does resolve runs past ten words on purpose. A cited
/// rule folds to one line described in its own words, and a rule short
/// enough to be its own summary shows the edition's generic line instead
/// (`##READER-RULE-FOLDED`) — so a golden whose only resolved rule was
/// short would pin the fallback twice and the derived description never.
///
/// `page` is the address the borrowed example is stored under: ids are
/// unique per page, so a bundle is keyed by page and then by id.
fn content(page: &str) -> Content {
    let mut rules = BTreeMap::new();
    rules.insert(
        "spec://com.example/subject/common/PROP-001#A-RULE".to_owned(),
        RuleText {
            uri: "spec://com.example/subject/common/PROP-001#A-RULE".to_owned(),
            anchor: "A-RULE".to_owned(),
            text: "A package **MUST** declare its `kind`, and the kind decides which shape \
                   a build reads it as."
                .to_owned(),
            lang: "en".to_owned(),
            source: Source::Checkout,
            path: PathBuf::from("common/PROP-001.xml"),
        },
    );
    let mut derived = BTreeMap::new();
    derived.insert(
        Content::derived_key(vibe_specdoc::doc::DerivedKind::CliHelp, "vibe list --help"),
        "Usage: vibe list [OPTIONS]\n\nOptions:\n      --json  Machine-readable output\n"
            .to_owned(),
    );
    let examples = BTreeMap::from([(
        page.to_owned(),
        BTreeMap::from([(
            "version".to_owned(),
            ExampleBody {
                run: "vibe --version".to_owned(),
                expect: "vibe 1.0.0".to_owned(),
                ..ExampleBody::default()
            },
        )]),
    )]);
    Content {
        rules,
        derived,
        examples,
        glossary: Some(fixture_glossary()),
        ..Content::new()
    }
}

/// The fixture package's one page: its address inside the package and its
/// document. The address is read from the page rather than written down
/// beside it, because it is the key the borrowed example is stored under
/// and two spellings of it would be two chances to disagree.
fn read_fixture_page() -> pages::Page {
    let set = pages::read_package(&fixture_package()).expect("the fixture package reads");
    assert!(
        set.unreadable.is_empty(),
        "the fixture page must parse: {:?}",
        set.unreadable
    );
    // Two pages now: the one every block rides on, and the one the package
    // declares as its GLOSSARY, whose entries the first one links. Only the
    // first is pinned as a golden — the second is there so a term link has
    // an entry to resolve against (`##GLOSSARY-DECLARED`).
    assert_eq!(set.pages.len(), 2, "the page and the glossary it links");
    let page = set
        .pages
        .iter()
        .find(|p| p.rel == "guide/every-block.xml")
        .expect("the golden's page");
    page.clone()
}

/// The glossary the fixture package declares, read the way a build reads it.
///
/// Read from the package rather than written down here: the golden shows a
/// term link carrying its entry, and an entry spelled twice would let the
/// attribute and the definition below it disagree.
fn fixture_glossary() -> vibe_doc::glossary::Glossary {
    let package = fixture_package();
    let set = pages::read_package(&package).expect("the fixture package reads");
    vibe_doc::glossary::read(&package, &set)
        .expect("the manifest reads")
        .expect("the fixture declares its glossary")
}

/// The island the golden pins: the fixture page at its own address, with
/// everything the pipeline would have fetched, and no block numbers.
fn island() -> String {
    let page = read_fixture_page();
    html::to_html_numbered(
        &page.doc,
        &page.rel,
        &content(&page.rel),
        &Numbering::none(),
    )
}

/// Compare against the golden, or write it when the author asked.
fn assert_golden(name: &str, got: &str) {
    let path = golden_path(name);
    if std::env::var_os("VIBE_DOC_BLESS").is_some() {
        std::fs::create_dir_all(path.parent().unwrap_or(Path::new("."))).expect("golden dir");
        std::fs::write(&path, got).expect("write golden");
        return;
    }
    let want = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| {
            panic!(
                "golden `{}` is missing ({e}); bless it with VIBE_DOC_BLESS=1",
                path.display()
            )
        })
        .replace("\r\n", "\n");
    if want != got {
        panic!(
            "the island moved from its golden `{}`\n{}",
            path.display(),
            first_difference(&want, got)
        );
    }
}

/// The first line where two texts part, named — so a moved golden
/// reports WHERE it moved and not merely that it did.
fn first_difference(want: &str, got: &str) -> String {
    let (w, g): (Vec<&str>, Vec<&str>) = (want.lines().collect(), got.lines().collect());
    for i in 0..w.len().max(g.len()) {
        let (a, b) = (w.get(i).copied(), g.get(i).copied());
        if a != b {
            return format!(
                "  line {}\n  golden: {}\n  render: {}",
                i + 1,
                a.unwrap_or("<end of file>"),
                b.unwrap_or("<end of file>")
            );
        }
    }
    "  the texts differ only in their trailing newline".to_owned()
}

/// Every block of the genre, once, rendered as an island.
#[test]
fn the_island_of_the_fixture_page_is_pinned() {
    assert_golden("guide-every-block.html", &island());
}

/// The island's own contract, restated on the golden's subject: no
/// executable, no style, no page furniture. The shell owns all three,
/// and the content security policy depends on this holding.
#[test]
fn the_island_carries_no_script_no_style_and_no_page_furniture() {
    let island = island();
    for forbidden in ["<script", "<style", "<html", "<head", "<body", "onclick="] {
        assert!(
            !island.contains(forbidden),
            "an island must not contain `{forbidden}`"
        );
    }
}

/// Rendering is a pure function of the page and the bundle.
#[test]
fn two_renders_of_the_fixture_page_are_the_same_bytes() {
    assert_eq!(island(), island());
}

/// The gap census counts exactly the blocks the island marks.
///
/// The island's `data-unresolved` is what a reader and a shell actually
/// see, so it is the definition and the census is the thing that has to
/// agree with it. A number in a build's summary that the page in front of
/// somebody does not corroborate is worse than no number: it would be
/// read, and believed.
///
/// Two bundles, because one proves nothing. With everything fetched this
/// page carries the single gap the fixture keeps on purpose — the rule
/// the bundle deliberately omits — and with nothing fetched it carries
/// one of every kind there is.
#[test]
fn the_gap_census_counts_the_blocks_the_island_marks() {
    let page = read_fixture_page();

    assert_eq!(
        build::unresolved(&page, &content(&page.rel)),
        build::Unresolved {
            examples: 0,
            rules: 1,
            derived: 0,
        }
    );
    assert_eq!(
        build::unresolved(&page, &Content::new()),
        build::Unresolved {
            examples: 1,
            rules: 2,
            derived: 1,
        }
    );

    for bundle in [content(&page.rel), Content::new()] {
        let island = build::render_page(&page, &bundle, build::Format::Html);
        assert_eq!(
            build::unresolved(&page, &bundle).total(),
            island.matches("data-unresolved=\"true\"").count(),
            "{island}"
        );
    }
}
