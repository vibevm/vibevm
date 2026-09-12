//! Stable text before text that moves.

use super::*;
use crate::pages::Page;

fn page(rel: &str, body: &str) -> Page {
    let text = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <spec xmlns=\"https://vibevm.org/spec/1\">\n  \
           <title id=\"root\">T</title>\n{body}\
         </spec>\n"
    );
    Page {
        rel: rel.to_owned(),
        path: std::path::PathBuf::from(rel),
        doc: vibe_specdoc::from_xml_with(&text, vibe_specdoc::Vocabulary::Doc).expect("parses"),
    }
}

fn row(path: &str) -> DocPage {
    DocPage {
        path: path.to_owned(),
        title: String::new(),
        genre: vibe_wire::generated::doc_manifest::PageGenre::Concept,
        audiences: Vec::new(),
        anchors: Vec::new(),
        summary: String::new(),
        reading_time_min: 1,
        reviewed_at: None,
    }
}

/// The gradient: prose and quoted rules first, examples next, a page
/// generated from the product last. A change at the top re-prices
/// everything below it, so the top must be what changes least.
#[test]
fn pages_are_ordered_from_prose_to_generated_reference() {
    let pages = vec![
        page(
            "reference/commands.xml",
            "  <derived kind=\"cli-help\" ref=\"vibe list --help\"/>\n",
        ),
        page(
            "start/first.xml",
            "  <p>one</p>\n  \
               <example id=\"e\" fixture=\"none\"><run>vibe --version</run>\
               <expect>vibe 1.0.0</expect></example>\n",
        ),
        page(
            "model/versions.xml",
            "  <p>one</p>\n  <rule ref=\"spec://org.demo/lib/common/PROP-001#A\"/>\n",
        ),
    ];
    let mut rows = vec![
        row("reference/commands.xml"),
        row("start/first.xml"),
        row("model/versions.xml"),
    ];
    order(&mut rows, &pages);
    assert_eq!(
        rows.iter().map(|r| r.path.as_str()).collect::<Vec<_>>(),
        vec![
            "model/versions.xml",
            "start/first.xml",
            "reference/commands.xml"
        ]
    );
}

/// A quoted rule is the slow end of the gradient, not the fast one:
/// specifications change by amendment and tombstone, and a page dense
/// with citations is the most stable kind there is.
#[test]
fn a_quoted_rule_does_not_count_as_the_products_text() {
    let doc = page(
        "a.xml",
        "  <rule ref=\"spec://org.demo/lib/common/PROP-001#A\"/>\n  \
           <rule ref=\"spec://org.demo/lib/common/PROP-001#B\"/>\n",
    )
    .doc;
    assert_eq!(mutability(&doc), (0, 2));
}

/// A translation's borrowed example is the product's text just as the
/// source's own example is: the output it shows is checked against the
/// binary, once, on the source.
#[test]
fn a_borrowed_example_counts_like_the_example_it_borrows() {
    let doc = page("a.xml", "  <example ref=\"e\"/>\n  <p>prose</p>\n").doc;
    assert_eq!(mutability(&doc), (1, 2));
}

/// Ties break on the address, so the order is total and two runs over an
/// unchanged package write the same bytes.
#[test]
fn pages_that_tie_are_ordered_by_their_address() {
    let pages = vec![page("b.xml", "  <p>x</p>\n"), page("a.xml", "  <p>y</p>\n")];
    let mut rows = vec![row("b.xml"), row("a.xml")];
    order(&mut rows, &pages);
    assert_eq!(
        rows.iter().map(|r| r.path.as_str()).collect::<Vec<_>>(),
        vec!["a.xml", "b.xml"]
    );
}

/// A page with no blocks divides by nothing, and is the most stable text
/// there is rather than a panic.
#[test]
fn a_page_with_no_blocks_is_pure_stability() {
    let pages = vec![
        page("empty.xml", ""),
        page(
            "generated.xml",
            "  <derived kind=\"cli-help\" ref=\"vibe list --help\"/>\n",
        ),
    ];
    let mut rows = vec![row("generated.xml"), row("empty.xml")];
    order(&mut rows, &pages);
    assert_eq!(rows[0].path, "empty.xml");
}
