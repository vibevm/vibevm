//! Public conversion preserves literal quotations without manufacturing anchors.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-facts/PROP-043#PARSE-FACT-ANCHORS");
use vibe_specdoc::{from_markdown, from_xml, to_markdown, to_xml};

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-facts/PROP-043#PARSE-FACT-ANCHORS"
)]
fn quoted_anchor_survives_public_conversion_and_xml_projection() {
    let source = "# H {#h}\n\n@fact:SAME definition\n\n@fact:OTHER see `@fact:SAME` and ``@fact/code:SAME``\n\n| Example |\n| --- |\n| @fact:CELL `@fact:SAME` |\n";
    let doc = from_markdown(source).expect("code-span quotations are not definitions");
    let xml = to_xml(&doc);
    assert_eq!(from_xml(&xml).unwrap(), doc);
    let projected = to_markdown(&doc);
    assert!(projected.contains("`@fact:SAME`"));
    assert!(projected.contains("``@fact/code:SAME``"));
    assert_eq!(from_markdown(&projected).unwrap(), doc);
    assert_eq!(xml.matches("<SAME ").count(), 1, "{xml}");
}

#[test]
fn public_conversion_keeps_real_duplicate_refusal() {
    let error = from_markdown("# H {#h}\n\n@fact:SAME one\n\n@fact:SAME two\n").unwrap_err();
    assert!(error.to_string().contains("defined twice"));
}
