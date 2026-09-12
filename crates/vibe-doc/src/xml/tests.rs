//! The XML projection's own laws.

use super::*;
use crate::numbering::number_blocks;

fn page(body: &str) -> SpecDoc {
    let text = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <spec xmlns=\"https://vibevm.org/spec/1\">\n  \
           <title id=\"root\">Page</title>\n{body}\
         </spec>\n"
    );
    vibe_specdoc::from_xml_with(&text, vibe_specdoc::Vocabulary::Doc).expect("page parses")
}

fn rendered(body: &str) -> String {
    let doc = page(body);
    to_xml_numbered(&doc, &number_blocks(&doc))
}

/// The number is an attribute on the block element, spelled as the plain
/// ordinal: the padding is for a reader's margin, and this is read by a
/// machine.
#[test]
fn every_numbered_block_carries_its_ordinal_as_an_attribute() {
    let xml = rendered(
        "  <p>one</p>\n  <fence>two</fence>\n  \
           <list ordered=\"true\"><item>three</item></list>\n",
    );
    assert!(xml.contains("<p p=\"1\">one</p>"), "{xml}");
    assert!(xml.contains("<fence p=\"2\">two</fence>"), "{xml}");
    assert!(xml.contains("<list ordered=\"true\" p=\"3\">"), "{xml}");
}

/// A heading takes no number: it has an address already, and that
/// address is immutable while `pNN` lives by the current text.
#[test]
fn a_section_takes_its_anchor_and_no_number() {
    let xml = rendered("  <section id=\"s\" title=\"S\"><p>x</p></section>\n");
    assert!(xml.contains("<section id=\"s\" title=\"S\">"), "{xml}");
    assert!(!xml.contains("<section id=\"s\" title=\"S\" p="), "{xml}");
    assert!(xml.contains("<p p=\"1\">x</p>"), "{xml}");
}

/// A `rule` keeps its ADDRESS: an agent reading XML resolves the citation
/// itself, and a substituted copy is the one thing the pipeline exists to
/// avoid. No pin, ever.
#[test]
fn a_rule_keeps_its_address_and_never_a_pin() {
    let xml = rendered("  <rule ref=\"spec://org.demo/lib/g#A~r4\"/>\n");
    assert!(
        xml.contains("<rule ref=\"spec://org.demo/lib/g#A\" p=\"1\"/>"),
        "{xml}"
    );
    assert!(!xml.contains("~r"), "{xml}");
}

/// A `derived` block keeps its reference for the same reason.
#[test]
fn a_derived_block_keeps_its_reference() {
    let xml = rendered("  <derived kind=\"cli-help\" ref=\"vibe --help\"/>\n");
    assert!(
        xml.contains("<derived kind=\"cli-help\" ref=\"vibe --help\" p=\"1\"/>"),
        "{xml}"
    );
}

/// `<expect></expect>` is how the dialect says «this command prints
/// nothing», so the pair is written even when the text is empty — an
/// assertion is not an absence.
#[test]
fn an_empty_expect_is_written_as_a_pair() {
    let xml = rendered(
        "  <example id=\"e\" fixture=\"none\"><run>vibe q</run><expect></expect></example>\n",
    );
    assert!(xml.contains("<expect></expect>"), "{xml}");
    assert!(!xml.contains("<expect/>"), "{xml}");
}

/// A fact keeps its anchor and its status inside the block that carries
/// it, in the generic spelling the dialect accepts for any anchor.
#[test]
fn a_fact_keeps_its_anchor_and_status() {
    let xml = rendered(
        "  <p><A-RULE fact=\"true\" status=\"spec/done\" audience=\"user\">Text.</A-RULE></p>\n",
    );
    assert!(
        xml.contains(
            "<p p=\"1\"><fact id=\"A-RULE\" status=\"spec/done\" audience=\"user\">Text.</fact></p>"
        ),
        "{xml}"
    );
}

/// A slot's condition survives as the attribute it was written as.
#[test]
fn a_guarded_slot_keeps_its_condition() {
    let xml = rendered("  <p when=\"os:windows\">backslash</p>\n");
    assert!(xml.contains("<p when=\"os:windows\" p=\"1\">"), "{xml}");
}

/// The XML specials are escaped in text and in attributes, so a page
/// quoting markup cannot become markup.
#[test]
fn markup_a_page_quotes_stays_quoted() {
    let xml = rendered("  <fence lang=\"xml\">&lt;rule/&gt; &amp; more</fence>\n");
    assert!(xml.contains("&lt;rule/&gt; &amp; more"), "{xml}");
}

/// Without a numbering the projection carries no `p` attribute at all.
#[test]
fn without_a_numbering_no_attribute_appears() {
    let doc = page("  <p>one</p>\n");
    let xml = to_xml(&doc);
    assert!(xml.contains("<p>one</p>"), "{xml}");
    assert!(!xml.contains(" p=\""), "{xml}");
}
