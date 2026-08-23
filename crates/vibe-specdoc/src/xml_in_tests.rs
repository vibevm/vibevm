use crate::doc::{Block, SpecDoc};
use crate::{from_xml, to_xml};

const NS_ATTR: &str = "xmlns=\"https://vibevm.org/spec/1\"";

fn ok(xml: &str) -> SpecDoc {
    from_xml(xml).expect("parses")
}

#[test]
fn minimal_document_round_trips_the_empty_ir() {
    let d = ok(&format!("<spec {NS_ATTR}/>"));
    assert_eq!(d, SpecDoc::default());
}

#[test]
fn foreign_element_is_a_loud_error() {
    let err = from_xml(&format!("<spec {NS_ATTR}>\n  <bogus>hi</bogus>\n</spec>")).unwrap_err();
    assert_eq!(err.line, 2, "{}", err);
    assert!(err.message.contains("<bogus>"), "{}", err);
    assert!(err.message.contains("vocabulary is closed"), "{}", err);
}

#[test]
fn foreign_attribute_is_a_loud_error() {
    let err = from_xml(&format!(
        "<spec {NS_ATTR}>\n  <p tone=\"nice\">x</p>\n</spec>"
    ))
    .unwrap_err();
    assert!(err.message.contains("`tone`"), "{}", err);
    assert!(err.message.contains("vocabulary is closed"), "{}", err);
}

#[test]
fn named_section_foreign_attribute_is_a_loud_error() {
    let err = from_xml(&format!(
        "<spec {NS_ATTR}>\n  <three-bands title=\"Three\" tone=\"nice\"/>\n</spec>"
    ))
    .unwrap_err();
    assert!(err.message.contains("<three-bands>"), "{}", err);
    assert!(err.message.contains("`tone`"), "{}", err);
    assert!(err.message.contains("vocabulary is closed"), "{}", err);
}

#[test]
fn reserved_or_xml_prefixed_element_is_not_a_named_section() {
    for name in ["xml-section", "XMLThing"] {
        let err = from_xml(&format!(
            "<spec {NS_ATTR}>\n  <{name} title=\"Not a section\"/>\n</spec>"
        ))
        .unwrap_err();
        assert!(
            err.message.contains("unknown") || err.message.contains("no <"),
            "{err}"
        );
        assert!(err.message.contains(name), "{err}");
    }
}

#[test]
fn dtd_is_forbidden() {
    let err = from_xml(&format!(
        "<!DOCTYPE spec [\n<!ENTITY x \"y\">]>\n<spec {NS_ATTR}/>"
    ))
    .unwrap_err();
    assert!(err.message.contains("forbids DTD"), "{}", err);
}

#[test]
fn processing_instruction_is_forbidden() {
    let err = from_xml(&format!("<?xml-stylesheet href=\"x\"?>\n<spec {NS_ATTR}/>")).unwrap_err();
    assert!(err.message.contains("processing instructions"), "{}", err);
}

#[test]
fn foreign_entity_is_forbidden() {
    let err = from_xml(&format!(
        "<spec {NS_ATTR}>\n  <fence lang=\"txt\">&extern;</fence>\n</spec>"
    ))
    .unwrap_err();
    assert!(
        err.message.contains("the dialect forbids entities"),
        "{}",
        err
    );
    assert!(err.message.contains("&extern;"), "{}", err);
}

#[test]
fn predefined_entities_and_char_refs_resolve() {
    let d = ok(&format!(
        "<spec {NS_ATTR}>\n  <fence lang=\"txt\">a &amp; b &lt; c &#65;</fence>\n</spec>"
    ));
    match &d.preamble[0] {
        Block::Fence { text, .. } => assert_eq!(text, "a & b < c A"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn cdata_is_legal_only_in_fence() {
    ok(&format!(
        "<spec {NS_ATTR}>\n  <fence><![CDATA[`raw` < & >]]></fence>\n</spec>"
    ));
    let err = from_xml(&format!(
        "<spec {NS_ATTR}>\n  <p><![CDATA[x]]></p>\n</spec>"
    ))
    .unwrap_err();
    assert!(
        err.message.contains("CDATA is allowed only inside <fence>"),
        "{}",
        err
    );
}

#[test]
fn duplicate_id_uses_the_progress_core_message() {
    let err = from_xml(&format!(
        "<spec {NS_ATTR}>\n  \
         <p><fact id=\"SAME\" status=\"impl/done\">one</fact></p>\n  \
         <p><fact id=\"SAME\" status=\"impl/done\">two</fact></p>\n</spec>"
    ))
    .unwrap_err();
    assert!(
        err.message
            .contains("fact id `@fact:SAME` is defined twice in this file: lines 2 and 3"),
        "{}",
        err
    );
}

#[test]
fn ids_use_the_shared_markdown_anchor_grammar() {
    let err = from_xml(&format!(
        "<spec {NS_ATTR}><p><fact id=\"not an id\">x</fact></p></spec>"
    ))
    .unwrap_err();
    assert!(
        err.message.contains("progress-core anchor grammar"),
        "{err}"
    );
}

#[test]
fn named_fact_reads_and_is_the_canonical_writer_form() {
    let d = ok(&format!(
        "<spec {NS_ATTR}><p><THE-LAW fact=\"true\" status=\"impl/done\">one law</THE-LAW></p></spec>"
    ));
    match &d.preamble[0] {
        Block::Paragraph(unit) => {
            assert_eq!(unit.text, "one law");
            assert_eq!(
                unit.fact.as_ref().and_then(|fact| fact.id.as_deref()),
                Some("THE-LAW")
            );
        }
        other => panic!("{other:?}"),
    }
    assert!(to_xml(&d).contains("<THE-LAW fact=\"true\" status=\"impl/done\">one law</THE-LAW>"));
}

#[test]
fn fact_discriminator_accepts_only_true() {
    for value in ["false", "yes"] {
        let err = from_xml(&format!(
            "<spec {NS_ATTR}><p><CLAIM fact=\"{value}\">x</CLAIM></p></spec>"
        ))
        .unwrap_err();
        assert!(err.message.contains("must be \"true\""), "{err}");
        assert!(err.message.contains(value), "{err}");
    }
}

#[test]
fn unknown_unit_element_without_fact_discriminator_stays_unknown() {
    let err = from_xml(&format!("<spec {NS_ATTR}><p><CLAIM>x</CLAIM></p></spec>")).unwrap_err();
    assert!(err.message.contains("no <CLAIM> element"), "{err}");
}

#[test]
fn named_fact_name_must_be_elementable_and_a_valid_fact_id() {
    let invalid_id = from_xml(&format!(
        "<spec {NS_ATTR}><p><bad.id fact=\"true\">x</bad.id></p></spec>"
    ))
    .unwrap_err();
    assert!(
        invalid_id.message.contains("progress-core anchor grammar"),
        "{invalid_id}"
    );

    let reserved = from_xml(&format!(
        "<spec {NS_ATTR}><p><table fact=\"true\">x</table></p></spec>"
    ))
    .unwrap_err();
    assert!(reserved.message.contains("not elementable"), "{reserved}");
    assert!(
        reserved.message.contains("<fact id=\"table\">"),
        "{reserved}"
    );
}

#[test]
fn generic_fact_form_remains_readable() {
    let d = ok(&format!(
        "<spec {NS_ATTR}><p><fact id=\"table\" status=\"impl/done\">fallback</fact></p></spec>"
    ));
    assert!(to_xml(&d).contains("<fact id=\"table\" status=\"impl/done\">fallback</fact>"));
}

#[test]
fn fence_binding_must_be_adjacent() {
    let err = from_xml(&format!(
        "<spec {NS_ATTR}>\n  <section title=\"S\">\n    \
         <p><fact id=\"X\" status=\"impl/done\">claim</fact></p>\n    \
         <p>in between</p>\n    \
         <fence lang=\"bash\" fact=\"X\">true</fence>\n  </section>\n</spec>"
    ))
    .unwrap_err();
    assert!(
        err.message
            .contains("must name the fact of the immediately"),
        "{}",
        err
    );
}

#[test]
fn idless_fact_needs_the_cell_exemption() {
    let err = from_xml(&format!(
        "<spec {NS_ATTR}>\n  <p><fact status=\"impl/done\">no id</fact></p>\n</spec>"
    ))
    .unwrap_err();
    assert!(err.message.contains("cell exemption"), "{}", err);
}

#[test]
fn list_requires_ordered_true_or_false() {
    let err = from_xml(&format!(
        "<spec {NS_ATTR}>\n  <list ordered=\"yes\"><item>x</item></list>\n</spec>"
    ))
    .unwrap_err();
    assert!(err.message.contains("`ordered`"), "{}", err);
}

#[test]
fn canonical_named_facts_group_round_trips_byte_for_byte() {
    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <spec {NS_ATTR}>\n  \
         <facts ordered=\"false\">\n    \
         <FIRST fact=\"true\" status=\"impl/done\">first</FIRST>\n    \
         <SECOND fact=\"true\" status=\"spec/work\">second</SECOND>\n  \
         </facts>\n\
         </spec>\n"
    );
    let doc = ok(&xml);
    match &doc.preamble[0] {
        Block::List { ordered, items } => {
            assert!(!ordered);
            assert_eq!(items.len(), 2);
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(to_xml(&doc), xml);
}

#[test]
fn generic_fact_is_legal_inside_facts_group() {
    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <spec {NS_ATTR}>\n  \
         <facts ordered=\"true\">\n    \
         <fact id=\"table\" status=\"impl/done\">fallback</fact>\n  \
         </facts>\n\
         </spec>\n"
    );
    let doc = ok(&xml);
    assert_eq!(to_xml(&doc), xml);
}

#[test]
fn legacy_all_fact_list_normalizes_to_facts_group() {
    let doc = ok(&format!(
        "<spec {NS_ATTR}>\n  <list ordered=\"false\">\n    <item><OLD fact=\"true\" status=\"impl/done\">old</OLD></item>\n  </list>\n</spec>"
    ));
    let normalized = to_xml(&doc);
    assert!(
        normalized.contains("<facts ordered=\"false\">"),
        "{normalized}"
    );
    assert!(!normalized.contains("<item>"), "{normalized}");
}

#[test]
fn item_is_rejected_inside_facts_group() {
    let err = from_xml(&format!(
        "<spec {NS_ATTR}><facts ordered=\"false\"><item>x</item></facts></spec>"
    ))
    .unwrap_err();
    assert!(err.message.contains("mixed lists stay in <list>"), "{err}");
}

#[test]
fn bare_text_is_rejected_inside_facts_group() {
    let err = from_xml(&format!(
        "<spec {NS_ATTR}><facts ordered=\"false\">bare text</facts></spec>"
    ))
    .unwrap_err();
    assert!(err.message.contains("text lives inside"), "{err}");
}

#[test]
fn empty_facts_group_is_rejected() {
    let err = from_xml(&format!(
        "<spec {NS_ATTR}><facts ordered=\"false\"/></spec>"
    ))
    .unwrap_err();
    assert!(err.message.contains("needs at least one fact"), "{err}");
}

#[test]
fn facts_group_requires_ordered_attribute() {
    let err = from_xml(&format!(
        "<spec {NS_ATTR}><facts><ONLY fact=\"true\">x</ONLY></facts></spec>"
    ))
    .unwrap_err();
    assert!(err.message.contains("requires an `ordered`"), "{err}");
}

#[test]
fn status_needs_the_pair_spelled_correctly() {
    let err = from_xml(&format!(
        "<spec {NS_ATTR}>\n  <p><fact id=\"X\" status=\"impl\">t</fact></p>\n</spec>"
    ))
    .unwrap_err();
    assert!(err.message.contains("`<stage>/<state>`"), "{}", err);

    let err = from_xml(&format!(
        "<spec {NS_ATTR}>\n  <p><fact id=\"X\" status=\"impl/donn\">t</fact></p>\n</spec>"
    ))
    .unwrap_err();
    assert!(err.message.contains("did you mean `done`"), "{}", err);
}

#[test]
fn status_text_must_fit_the_markdown_element_spelling() {
    let err = from_xml(&format!(
        "<spec {NS_ATTR}><status stage=\"spec\" state=\"work\" comment=\"a &quot;quote&quot;\"/></spec>"
    ))
    .unwrap_err();
    assert!(err.message.contains("not Markdown-expressible"), "{err}");
}

#[test]
fn empty_p_is_rejected_but_empty_td_is_a_cell() {
    let err = from_xml(&format!("<spec {NS_ATTR}>\n  <p></p>\n</spec>")).unwrap_err();
    assert!(err.message.contains("cannot express"), "{}", err);
    ok(&format!(
        "<spec {NS_ATTR}>\n  <table><tr><td>a</td><td/></tr></table>\n</spec>"
    ));
}

#[test]
fn empty_fact_with_an_id_is_a_valid_unit() {
    let d = ok(&format!(
        "<spec {NS_ATTR}>\n  <p><fact id=\"EMPTY\"/></p>\n</spec>"
    ));
    match &d.preamble[0] {
        Block::Paragraph(unit) => {
            assert_eq!(unit.text, "");
            assert_eq!(
                unit.fact.as_ref().and_then(|f| f.id.as_deref()),
                Some("EMPTY")
            );
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn table_cell_code_span_pipes_are_accepted() {
    for text in ["`a | b`", "`a \\| b`"] {
        let document = ok(&format!(
            "<spec {NS_ATTR}>\n  <table><tr><td>{text}</td></tr></table>\n</spec>"
        ));
        assert!(to_xml(&document).contains(&format!("<td>{text}</td>")));
    }
}

#[test]
fn table_cell_bare_pipe_and_newline_are_rejected() {
    for text in ["a | b", "a\nb"] {
        let err = from_xml(&format!(
            "<spec {NS_ATTR}>\n  <table><tr><td>{text}</td></tr></table>\n</spec>"
        ))
        .unwrap_err();
        assert!(err.message.contains("cannot hold `|`"), "{err}");
    }
}

#[test]
fn xml_order_and_heading_depth_stay_markdown_expressible() {
    let reordered = from_xml(&format!(
        "<spec {NS_ATTR}><section title=\"child\"/><p>late</p></spec>"
    ))
    .unwrap_err();
    assert!(reordered.message.contains("cannot follow"), "{reordered}");

    let too_deep = from_xml(&format!(
        "<spec {NS_ATTR}><section title=\"2\"><section title=\"3\"><section title=\"4\"><section title=\"5\"><section title=\"6\"><section title=\"7\"/></section></section></section></section></section></spec>"
    ))
    .unwrap_err();
    assert!(too_deep.message.contains("H6"), "{too_deep}");
}
