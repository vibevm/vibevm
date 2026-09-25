//! One fixture page, three projections, ONE set of numbers.
//!
//! This is the law A2.22 exists for: a human reading the island quotes
//! `p12`, an agent reading the `.md` quotes `p12`, a tool walking the
//! `.xml` reads `p="12"`, and all three mean the same block. The test
//! renders all three from one document and compares the sets of numbers,
//! not the spellings — each projection spells the number the way its own
//! medium wants it.
//!
//! The goldens beside it pin the shape. Refresh them with
//! `VIBE_DOC_BLESS=1 cargo test -p vibe-doc --test projections`, and read
//! the diff: a golden that moves without a reason is a renderer that
//! changed without one.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use vibe_doc::citations::{RuleText, Source};
use vibe_doc::content::{Content, ExampleBody};
use vibe_doc::numbering::{Numbering, number_blocks};
use vibe_doc::{html, md, pages, xml};

/// What the pipeline would have fetched for this page. The projections
/// are pinned WITH it, because the interesting bytes are the resolved
/// ones: the rule's text in the island and the Markdown, and the address
/// the XML deliberately keeps instead.
///
/// `page` is the address the borrowed example is stored under: ids are
/// unique per page, so a bundle is keyed by page and then by id.
fn content(page: &str) -> Content {
    let uri = "spec://com.example/subject/common/PROP-001#A-RULE";
    let mut rules = BTreeMap::new();
    rules.insert(
        uri.to_owned(),
        RuleText {
            uri: uri.to_owned(),
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
        "Usage: vibe list [OPTIONS]\n".to_owned(),
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
        ..Content::new()
    }
}

/// The fixture package's one page: its address inside the package and its
/// document. The address is read from the page, because it is what the
/// two substituting backends resolve a borrowed example against.
fn fixture_page() -> pages::Page {
    let package = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixture/manual");
    let set = pages::read_package(&package).expect("the fixture package reads");
    assert!(set.unreadable.is_empty(), "{:?}", set.unreadable);
    set.pages[0].clone()
}

fn assert_golden(name: &str, got: &str) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name);
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
    assert_eq!(want, got, "the projection moved from `{}`", path.display());
}

/// Every number the HTML island carries, from its `data-p` attributes.
fn numbers_in_html(text: &str) -> BTreeSet<u32> {
    collect(text, "data-p=\"", '"')
}

/// Every number the Markdown carries, from its `[pNN]` labels.
fn numbers_in_md(text: &str) -> BTreeSet<u32> {
    let mut out = BTreeSet::new();
    let mut at = 0usize;
    while let Some(hit) = text[at..].find("[p") {
        let start = at + hit + 2;
        let end = text[start..]
            .find(']')
            .map(|e| start + e)
            .unwrap_or(text.len());
        at = end;
        if let Ok(n) = text[start..end].parse::<u32>() {
            out.insert(n);
        }
    }
    out
}

/// Every number the XML carries, from its `p="N"` attributes.
fn numbers_in_xml(text: &str) -> BTreeSet<u32> {
    collect(text, " p=\"", '"')
}

fn collect(text: &str, needle: &str, end: char) -> BTreeSet<u32> {
    let mut out = BTreeSet::new();
    let mut at = 0usize;
    while let Some(hit) = text[at..].find(needle) {
        let start = at + hit + needle.len();
        let stop = text[start..]
            .find(end)
            .map(|e| start + e)
            .unwrap_or(text.len());
        at = stop;
        if let Ok(n) = text[start..stop].parse::<u32>() {
            out.insert(n);
        }
    }
    out
}

/// The law: three projections, one set of numbers, and the set is
/// exactly `1..=len` with no gaps and no repeats.
#[test]
fn the_three_projections_carry_one_set_of_numbers() {
    let page = fixture_page();
    let doc = &page.doc;
    let numbering = number_blocks(doc);
    let content = content(&page.rel);

    let island = html::to_html_numbered(doc, &page.rel, &content, &numbering);
    let markdown = md::to_markdown_numbered(doc, &page.rel, &content, &numbering);
    let dialect = xml::to_xml_numbered(doc, &numbering);

    let expected: BTreeSet<u32> = (1..=numbering.len() as u32).collect();
    assert_eq!(numbers_in_html(&island), expected, "the island");
    assert_eq!(numbers_in_md(&markdown), expected, "the markdown");
    assert_eq!(numbers_in_xml(&dialect), expected, "the xml");
}

/// A second render is the same render: numbering is a pure function of
/// the document, so `#p12` cannot move because somebody rebuilt.
#[test]
fn a_second_render_carries_the_same_numbers() {
    let page = fixture_page();
    let doc = &page.doc;
    let content = content(&page.rel);
    let first = number_blocks(doc);
    let second = number_blocks(doc);
    assert_eq!(first, second);
    assert_eq!(
        html::to_html_numbered(doc, &page.rel, &content, &first),
        html::to_html_numbered(doc, &page.rel, &content, &second)
    );
    assert_eq!(
        md::to_markdown_numbered(doc, &page.rel, &content, &first),
        md::to_markdown_numbered(doc, &page.rel, &content, &second)
    );
    assert_eq!(
        xml::to_xml_numbered(doc, &first),
        xml::to_xml_numbered(doc, &second)
    );
}

/// Footnotes are apparatus, not flow: the section takes no numbers in any
/// projection, and the numbers before it are untouched by its presence.
#[test]
fn the_footnotes_section_is_numbered_in_no_projection() {
    let page = fixture_page();
    let doc = &page.doc;
    let numbering = number_blocks(doc);
    let footnotes = doc
        .sections
        .iter()
        .position(|s| s.id.as_deref() == Some("footnotes"))
        .expect("the fixture carries a footnotes section");
    let path = vibe_doc::numbering::BlockPath::new(vec![footnotes as u16], 0);
    assert_eq!(numbering.get(&path), None);

    let markdown = md::to_markdown_numbered(doc, &page.rel, &content(&page.rel), &numbering);
    let apparatus = markdown
        .split("## Footnotes")
        .nth(1)
        .expect("the footnotes heading is in the projection");
    assert!(
        !apparatus.contains("[p"),
        "a footnote block took a number:\n{apparatus}"
    );
}

/// The shape of each projection, pinned.
#[test]
fn the_three_projections_are_pinned() {
    let page = fixture_page();
    let doc = &page.doc;
    let numbering = number_blocks(doc);
    let content = content(&page.rel);
    assert_golden(
        "guide-every-block.numbered.html",
        &html::to_html_numbered(doc, &page.rel, &content, &numbering),
    );
    assert_golden(
        "guide-every-block.md",
        &md::to_markdown_numbered(doc, &page.rel, &content, &numbering),
    );
    assert_golden(
        "guide-every-block.xml",
        &xml::to_xml_numbered(doc, &numbering),
    );
}

/// Without a numbering the three projections carry no numbers at all —
/// the neutral element costs a caller no branch.
#[test]
fn the_neutral_numbering_leaves_every_projection_bare() {
    let page = fixture_page();
    let doc = &page.doc;
    let content = content(&page.rel);
    let none = Numbering::none();
    assert!(numbers_in_html(&html::to_html_numbered(doc, &page.rel, &content, &none)).is_empty());
    assert!(numbers_in_md(&md::to_markdown_numbered(doc, &page.rel, &content, &none)).is_empty());
    assert!(numbers_in_xml(&xml::to_xml_numbered(doc, &none)).is_empty());
}
