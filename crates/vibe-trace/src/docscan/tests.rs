//! The page scanner's contribution to a map, on a package built here.

use std::fs;

use super::*;

fn package(root: &Path, body: &str) {
    let dir = root.join("vibevm/vibespecs/model");
    fs::create_dir_all(&dir).expect("page dir");
    fs::write(
        dir.join("boot-lane.xml"),
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <spec xmlns=\"https://vibevm.org/spec/1\">\n  \
               <title id=\"root\">Boot lane</title>\n{body}\
             </spec>\n"
        ),
    )
    .expect("page");
}

/// One page, two citations: one item, two edges, and not a pin between
/// them.
#[test]
fn a_page_becomes_one_item_and_its_rules_become_unpinned_edges() {
    let tmp = tempfile::tempdir().expect("temp");
    package(
        tmp.path(),
        "  <rule ref=\"spec://org.demo/lib/common/PROP-001#A\"/>\n  \
           <rule ref=\"spec://org.demo/lib/common/PROP-001#B~r4\"/>\n",
    );
    let scanner = DocScanner::new("org.demo/lib-docs");
    let (items, edges, warnings) = scanner.scan(tmp.path(), &Config::default());

    assert_eq!(items.len(), 1, "one page, one item");
    assert_eq!(items[0].symbol, "model::boot-lane");
    assert_eq!(items[0].crateName, DOC_CRATE);
    assert_eq!(items[0].file, "vibevm/vibespecs/model/boot-lane.xml");
    assert!(items[0].fingerprint.is_none(), "a page has no token stream");

    assert_eq!(edges.len(), 2);
    for edge in &edges {
        assert!(matches!(edge.verb, EdgeVerb::Documents));
        assert!(
            edge.pinnedR.is_none(),
            "an edge with a pin can go suspect; a live citation must not"
        );
        assert_eq!(edge.fromSymbol, "model::boot-lane");
    }
    // The pin in the second attribute was dropped by the pivot, so the
    // edge names the bare address.
    assert_eq!(edges[1].uri, "spec://org.demo/lib/common/PROP-001#B");
    assert!(warnings.is_empty());
}

/// The manual's own pages are not specifications it documents, so a
/// self-address mints nothing.
#[test]
fn an_address_to_the_manuals_own_pages_mints_no_edge() {
    let tmp = tempfile::tempdir().expect("temp");
    package(
        tmp.path(),
        "  <rule ref=\"spec://org.demo/lib-docs/model/other#X\"/>\n",
    );
    let scanner = DocScanner::new("org.demo/lib-docs");
    let (items, edges, _) = scanner.scan(tmp.path(), &Config::default());
    assert!(items.is_empty() && edges.is_empty());
}

/// A tree with no pages is not a failure — a package may carry none yet.
#[test]
fn a_tree_with_no_pages_contributes_nothing_and_warns_about_nothing() {
    let tmp = tempfile::tempdir().expect("temp");
    let scanner = DocScanner::new("org.demo/lib-docs");
    let (items, edges, warnings) = scanner.scan(tmp.path(), &Config::default());
    assert!(items.is_empty() && edges.is_empty() && warnings.is_empty());
}
