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

use vibe_doc::citations::{RuleText, Source};
use vibe_doc::content::{Content, ExampleBody};
use vibe_doc::html;
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
fn content() -> Content {
    let mut rules = BTreeMap::new();
    rules.insert(
        "spec://com.example/subject/common/PROP-001#A-RULE".to_owned(),
        RuleText {
            uri: "spec://com.example/subject/common/PROP-001#A-RULE".to_owned(),
            anchor: "A-RULE".to_owned(),
            text: "A package **MUST** declare its `kind`.".to_owned(),
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
    let mut examples = BTreeMap::new();
    examples.insert(
        "version".to_owned(),
        ExampleBody {
            run: "vibe --version".to_owned(),
            expect: "vibe 1.0.0".to_owned(),
            ..ExampleBody::default()
        },
    );
    Content {
        rules,
        derived,
        examples,
        ..Content::new()
    }
}

fn read_fixture_page() -> vibe_specdoc::doc::SpecDoc {
    let set = pages::read_package(&fixture_package()).expect("the fixture package reads");
    assert!(
        set.unreadable.is_empty(),
        "the fixture page must parse: {:?}",
        set.unreadable
    );
    assert_eq!(set.pages.len(), 1, "one page, so one golden");
    assert_eq!(set.pages[0].rel, "guide/every-block.xml");
    set.pages[0].doc.clone()
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
    let doc = read_fixture_page();
    assert_golden("guide-every-block.html", &html::to_html(&doc, &content()));
}

/// The island's own contract, restated on the golden's subject: no
/// executable, no style, no page furniture. The shell owns all three,
/// and the content security policy depends on this holding.
#[test]
fn the_island_carries_no_script_no_style_and_no_page_furniture() {
    let island = html::to_html(&read_fixture_page(), &content());
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
    let doc = read_fixture_page();
    assert_eq!(
        html::to_html(&doc, &content()),
        html::to_html(&doc, &content())
    );
}
