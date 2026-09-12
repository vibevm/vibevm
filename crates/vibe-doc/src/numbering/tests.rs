//! The numbering laws.

use vibe_specdoc::doc::DerivedKind;

use super::*;

fn page(body: &str) -> SpecDoc {
    let text = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <spec xmlns=\"https://vibevm.org/spec/1\">\n  \
           <title id=\"root\">Page</title>\n{body}\
         </spec>\n"
    );
    vibe_specdoc::from_xml_with(&text, vibe_specdoc::Vocabulary::Doc).expect("page parses")
}

/// Document order is preamble, then each section's own blocks, then its
/// subsections — the same walk the pivot's two writers and the host's
/// fact collector already take.
#[test]
fn numbers_follow_document_order_through_nested_sections() {
    let doc = page(
        "  <p>preamble</p>\n  \
           <section id=\"a\" title=\"A\">\n    <p>a one</p>\n    <p>a two</p>\n    \
             <section id=\"b\" title=\"B\">\n      <p>b one</p>\n    </section>\n  \
           </section>\n  \
           <section id=\"c\" title=\"C\">\n    <p>c one</p>\n  </section>\n",
    );
    let n = number_blocks(&doc);
    assert_eq!(n.len(), 5);
    assert_eq!(n.get(&BlockPath::new(vec![], 0)), Some(1));
    assert_eq!(n.get(&BlockPath::new(vec![0], 0)), Some(2));
    assert_eq!(n.get(&BlockPath::new(vec![0], 1)), Some(3));
    assert_eq!(n.get(&BlockPath::new(vec![0, 0], 0)), Some(4));
    assert_eq!(n.get(&BlockPath::new(vec![1], 0)), Some(5));
}

/// Every flow block counts, whatever its kind; a heading does not, and
/// neither do the pieces inside a block.
#[test]
fn every_flow_block_counts_and_headings_and_inner_pieces_do_not() {
    let doc = page(
        "  <p>one</p>\n  \
           <list ordered=\"false\"><item>a</item><item>b</item></list>\n  \
           <table><tr><td>x</td><td>y</td></tr></table>\n  \
           <fence>code</fence>\n  \
           <quote>q</quote>\n  \
           <rule ref=\"spec://org.demo/lib/g#R\"/>\n  \
           <derived kind=\"cli-help\" ref=\"vibe --help\"/>\n  \
           <note kind=\"tip\">t</note>\n  \
           <figure src=\"a.svg\" alt=\"a\"><caption>c</caption></figure>\n  \
           <example id=\"e\" fixture=\"none\"><run>r</run><expect>o</expect></example>\n  \
           <prompt id=\"p\" assert=\"none\">do it</prompt>\n  \
           <section id=\"s\" title=\"A heading is not a block\"><p>two</p></section>\n",
    );
    let n = number_blocks(&doc);
    // Eleven preamble blocks plus one in the section; a two-item list is
    // ONE block and a two-cell row is ONE block.
    assert_eq!(n.len(), 12);
    assert_eq!(n.get(&BlockPath::new(vec![0], 0)), Some(12));
}

/// Footnotes are apparatus, not flow.
#[test]
fn the_footnotes_section_is_not_numbered() {
    let doc = page(
        "  <p>one</p>\n  \
           <section id=\"footnotes\" title=\"Footnotes\">\n    <p>[^1]: a note</p>\n  </section>\n  \
           <section id=\"after\" title=\"After\">\n    <p>two</p>\n  </section>\n",
    );
    let n = number_blocks(&doc);
    assert_eq!(n.len(), 2, "the footnote paragraph takes no number");
    assert_eq!(n.get(&BlockPath::new(vec![0], 0)), None);
    assert_eq!(n.get(&BlockPath::new(vec![1], 0)), Some(2));
}

/// A conditional slot is counted like any other: `p12` has to mean the
/// same block in the Windows build and in the Linux build, and gaps in
/// what one build shows are the accepted price.
#[test]
fn a_conditional_block_is_counted_before_any_filtering() {
    let doc = page(
        "  <p>one</p>\n  <p when=\"os:windows\">windows</p>\n  \
           <p when=\"os:linux\">linux</p>\n  <p>four</p>\n",
    );
    let n = number_blocks(&doc);
    assert_eq!(n.len(), 4);
    assert_eq!(n.get(&BlockPath::new(vec![], 3)), Some(4));
}

/// Expanding `derived` replaces one block with one block, so the
/// numbering is identical before and after — which is what makes the
/// norm's order («expand, then number») free to obey.
#[test]
fn expanding_derived_does_not_move_a_single_number() {
    let doc =
        page("  <p>one</p>\n  <derived kind=\"cli-help\" ref=\"vibe --help\"/>\n  <p>three</p>\n");
    let mut content = Content::new();
    content.derived.insert(
        Content::derived_key(DerivedKind::CliHelp, "vibe --help"),
        "Usage: vibe".to_owned(),
    );
    let expanded = expand_derived(&doc, &content);
    assert_eq!(number_blocks(&doc), number_blocks(&expanded));
    assert!(matches!(
        expanded.preamble[1].block,
        vibe_specdoc::doc::Block::Fence { .. }
    ));
}

/// A `derived` block this build could not generate stays a `derived`
/// block, so a backend can mark it unresolved instead of rendering an
/// empty fence that reads as a command with no output.
#[test]
fn an_ungenerated_derived_block_is_left_for_the_backend_to_mark() {
    let doc = page("  <derived kind=\"cli-help\" ref=\"vibe --help\"/>\n");
    let expanded = expand_derived(&doc, &Content::new());
    assert!(matches!(
        expanded.preamble[0].block,
        vibe_specdoc::doc::Block::Derived { .. }
    ));
}

/// The number reads back to its block: «see p12» has to resolve, and a
/// translation checks itself block by block against it.
#[test]
fn a_number_reads_back_to_the_block_it_names() {
    let doc = page("  <p>one</p>\n  <section id=\"s\" title=\"S\"><p>two</p></section>\n");
    let n = number_blocks(&doc);
    assert_eq!(n.path_of(1), Some(&BlockPath::new(vec![], 0)));
    assert_eq!(n.path_of(2), Some(&BlockPath::new(vec![0], 0)));
    assert_eq!(n.path_of(3), None);
    assert_eq!(n.path_of(0), None, "numbers start at one");
}

/// The margin lines up: two digits with a leading zero below ten.
#[test]
fn the_label_is_two_digits_with_a_leading_zero() {
    assert_eq!(Numbering::spell(7), "p07");
    assert_eq!(Numbering::spell(12), "p12");
    assert_eq!(Numbering::spell(120), "p120");
    assert_eq!(Numbering::digits(7), "07");
}

/// An empty numbering is a value a backend can take without a branch.
#[test]
fn the_empty_numbering_answers_nothing_for_every_block() {
    let n = Numbering::none();
    assert!(n.is_empty());
    assert_eq!(n.get(&BlockPath::new(vec![], 0)), None);
    assert_eq!(n.label(&BlockPath::new(vec![], 0)), None);
}

/// Numbering is a pure function: two calls on one document agree, and a
/// re-render therefore carries the same numbers.
#[test]
fn two_numberings_of_one_document_agree() {
    let doc = page("  <p>one</p>\n  <p>two</p>\n");
    assert_eq!(number_blocks(&doc), number_blocks(&doc));
}
