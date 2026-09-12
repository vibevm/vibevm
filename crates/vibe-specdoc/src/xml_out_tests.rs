//! The writer's own tests: the bytes it emits, element by element.
//!
//! Sibling test cell of [`crate::xml_out`], in this crate's flat
//! `<module>_tests` shape, so both stay inside the AI-Native file budget.
//! Nothing moved but the file.

use crate::xml_out::anchor_is_elementable;
use crate::{from_markdown, to_xml};

/// The shape test: a small document emits exactly the dialect's
/// canonical form — pinned byte-for-byte, because this is the format
/// contract every golden file rests on.
#[test]
fn canonical_form_is_pinned() {
    let d = from_markdown(
        "# T {#t}\n\n<status stage=\"spec\" state=\"work\"/>\n\n\
         @fact:A One. @status:impl/done\n\nplain paragraph\n",
    )
    .expect("parses");
    let xml = to_xml(&d);
    assert_eq!(
        xml,
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <spec xmlns=\"https://vibevm.org/spec/1\">\n  \
         <title id=\"t\">T</title>\n  \
         <status stage=\"spec\" state=\"work\"/>\n  \
         <p><A fact=\"true\" status=\"impl/done\">One.</A></p>\n  \
         <p>plain paragraph</p>\n\
         </spec>\n"
    );
}

#[test]
fn specials_are_escaped_and_round_trip() {
    let d = from_markdown("# T {#t}\n\n@fact:A `a < b` & `c > d`. @impl/done\n").unwrap();
    let xml = to_xml(&d);
    assert!(xml.contains("&lt;"), "{xml}");
    let back = crate::from_xml(&xml).unwrap();
    assert_eq!(back, d);
}

#[test]
fn named_sections_and_generic_fallbacks_are_pinned() {
    let d = from_markdown(
        "# T {#root}\n\n\
         ## Named {#three-bands}\n\nbody\n\n\
         ## Leading digit {#2-fast}\n\nbody\n\n\
         ## Structural word {#table}\n\nbody\n",
    )
    .expect("parses");
    let xml = to_xml(&d);
    assert!(xml.contains("<three-bands title=\"Named\">"), "{xml}");
    assert!(
        xml.contains("<section id=\"2-fast\" title=\"Leading digit\">"),
        "{xml}"
    );
    assert!(
        xml.contains("<section id=\"table\" title=\"Structural word\">"),
        "{xml}"
    );
    assert_eq!(crate::from_xml(&xml).expect("reads both forms"), d);
    assert_eq!(to_xml(&crate::from_xml(&xml).unwrap()), xml);
}

#[test]
fn named_facts_and_generic_vocabulary_fallback_are_pinned() {
    let d = from_markdown(
        "# T {#root}\n\n\
         @fact:THE-LAW named @status:impl/done\n\n\
         @fact:table fallback @status:spec/work\n",
    )
    .expect("parses");
    let xml = to_xml(&d);
    assert!(
        xml.contains("<THE-LAW fact=\"true\" status=\"impl/done\">named</THE-LAW>"),
        "{xml}"
    );
    assert!(
        xml.contains("<fact id=\"table\" status=\"spec/work\">fallback</fact>"),
        "{xml}"
    );
    assert_eq!(crate::from_xml(&xml).expect("reads both fact forms"), d);
    assert_eq!(to_xml(&crate::from_xml(&xml).unwrap()), xml);
}

#[test]
fn all_fact_lists_use_facts_while_mixed_lists_keep_item_carriers() {
    let all_fact = from_markdown(
        "# T {#root}\n\n\
         - @fact:FIRST first @status:impl/done\n\
         - @fact:SECOND second @status:spec/work\n",
    )
    .expect("all-fact list parses");
    let facts_xml = to_xml(&all_fact);
    assert!(
        facts_xml.contains("<facts ordered=\"false\">"),
        "{facts_xml}"
    );
    assert!(!facts_xml.contains("<item>"), "{facts_xml}");
    assert_eq!(
        crate::from_xml(&facts_xml).expect("facts read back"),
        all_fact
    );
    assert_eq!(to_xml(&crate::from_xml(&facts_xml).unwrap()), facts_xml);

    let mixed = from_markdown(
        "# T {#root}\n\n\
         - @fact:FIRST first @status:impl/done\n\
         - plain item\n",
    )
    .expect("mixed list parses");
    let list_xml = to_xml(&mixed);
    assert!(list_xml.contains("<list ordered=\"false\">"), "{list_xml}");
    assert!(list_xml.contains("<item>"), "{list_xml}");
    assert!(!list_xml.contains("<facts"), "{list_xml}");
    assert_eq!(crate::from_xml(&list_xml).expect("list reads back"), mixed);
    assert_eq!(to_xml(&crate::from_xml(&list_xml).unwrap()), list_xml);
}

#[test]
fn elementable_anchor_predicate_is_the_format_boundary() {
    for anchor in ["three-bands", "_private", "a.b-c_1"] {
        assert!(anchor_is_elementable(anchor), "{anchor}");
    }
    for anchor in [
        "",
        "2-fast",
        "table",
        "facts",
        "section",
        "xml-section",
        "XMLThing",
        "has space",
        "éclair",
    ] {
        assert!(!anchor_is_elementable(anchor), "{anchor}");
    }
    // The documentation genre does NOT enter this blacklist
    // (##DOC-VOCAB-DISCRIMINATOR). Were it to, a section anchored
    // `#example` would emit as `<section id="example">` in a
    // documentation package and as `<example title=…>` everywhere
    // else, and one IR would have two spellings. The discriminator
    // carries that weight instead: a section always has `title=`, a
    // documentation block never does.
    for anchor in ["example", "rule", "derived", "note", "figure", "prompt"] {
        assert!(
            anchor_is_elementable(anchor),
            "{anchor}: the genre's names stay elementable"
        );
    }
}
