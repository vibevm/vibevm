//! Semantic positions are observed by the incumbent parser, not XML spelling.
#![cfg(test)]
use super::*;

fn comment(payload: &str) -> String {
    crate::frame_generated_xml_comment(payload)
}
fn wrap(body: &str) -> String {
    format!("<spec xmlns=\"https://vibevm.org/spec/1\">{body}</spec>")
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#EMBED-GENERATED-PROVENANCE"
)]
fn equivalent_xml_layout_entities_and_named_sections_keep_semantic_events() {
    let event = comment("embed: spec://org.demo/pkg/a--b#x");
    let canonical = wrap(&format!(
        "<section id=\"s\" title=\"Section\"><p>é😀&amp;{event}tail</p></section>"
    ));
    let equivalent = format!(
        "<?xml version='1.0'?>\n<spec xmlns='https://vibevm.org/spec/1'>\n  <s title='Section'>\n    <p>  &#xE9;&#128512;&amp;{event}tail  </p>\n  </s>\n</spec>\n"
    );
    let expected = observe_annotated_xml(&canonical).unwrap();
    assert_eq!(expected, observe_annotated_xml(&equivalent).unwrap());
    assert!(matches!(
        expected.1[0].position,
        SemanticPosition::Text { offset: 3, .. }
    ));
    assert_eq!(expected.0, crate::from_xml(&canonical).unwrap());
    assert_ne!(
        generated_xml_comments(&canonical).unwrap()[0].start_line,
        generated_xml_comments(&equivalent).unwrap()[0].start_line
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#EMBED-GENERATED-PROVENANCE"
)]
fn moving_injecting_removing_comments_changes_events_even_when_document_is_equal() {
    let event = comment("embed: a");
    let original = wrap(&format!("<p>a{event}bc</p><p>second</p>"));
    let expected = observe_annotated_xml(&original).unwrap();
    for changed in [
        wrap(&format!("<p>ab{event}c</p><p>second</p>")),
        wrap(&format!("<p>abc</p>{event}<p>second</p>")),
        wrap(&format!("<p>a{event}{event}bc</p><p>second</p>")),
        wrap("<p>abc</p><p>second</p>"),
    ] {
        let actual = observe_annotated_xml(&changed).unwrap();
        assert_eq!(actual.0, expected.0);
        assert_ne!(actual.1, expected.1);
    }
    let empty_expected = observe_annotated_xml(&wrap("<p>abc</p>")).unwrap();
    let injected = observe_annotated_xml(&wrap(&format!("<p>a{event}bc</p>"))).unwrap();
    assert_eq!(empty_expected.0, injected.0);
    assert!(empty_expected.1.is_empty());
    assert_eq!(
        injected.1.len(),
        1,
        "empty expected tape cannot bypass actual comment observation"
    );
}

#[test]
fn escaped_prose_cdata_and_unreserved_comments_do_not_become_events() {
    let literal = "&lt;!-- vibe:c1 embed: counterfeit --&gt;";
    let xml = wrap(&format!(
        "<!-- ordinary --><p>{literal}</p><quote>{literal}</quote><fence><![CDATA[<!-- vibe:c1 embed: counterfeit -->]]></fence>"
    ));
    let (doc, events) = observe_annotated_xml(&xml).unwrap();
    assert_eq!(doc, crate::from_xml(&xml).unwrap());
    assert!(events.is_empty());
    assert!(generated_xml_comments(&xml).unwrap().is_empty());
    let crate::doc::Block::Fence { text, .. } = &doc.preamble[2].block else {
        panic!("fence fixture");
    };
    assert_eq!(text, "<!-- vibe:c1 embed: counterfeit -->");
}

#[test]
fn facts_list_aliases_and_quote_keep_exact_text_positions() {
    let event = comment("/embed: a");
    let generic = wrap(&format!(
        "<list ordered='false'><item><fact id='F'> α{event}β </fact></item></list>"
    ));
    let named = wrap(&format!(
        "\n<facts ordered='false'>\n  <F fact='true'>α{event}β</F>\n</facts>\n"
    ));
    assert_eq!(
        observe_annotated_xml(&generic).unwrap(),
        observe_annotated_xml(&named).unwrap()
    );
    for body in [
        format!("<p><F fact='true'>α{event}β</F></p>"),
        format!("<quote>α{event}β</quote>"),
    ] {
        let (doc, events) = observe_annotated_xml(&wrap(&body)).unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0].position,
            SemanticPosition::Text { offset: 1, .. }
        ));
        let text = match &doc.preamble[0].block {
            crate::doc::Block::Paragraph(unit) | crate::doc::Block::Quote(unit) => &unit.text,
            _ => panic!("unit fixture"),
        };
        assert_eq!(text, "αβ");
    }
}

#[test]
fn nested_empty_coincident_events_preserve_order_and_fence_verbatim_offsets() {
    let payloads = [
        "embed: outer",
        "embed: inner",
        "/embed: inner",
        "/embed: outer",
    ];
    let together: String = payloads.iter().map(|p| comment(p)).collect();
    let empty = wrap(&format!("<fence>{together}</fence>"));
    let (doc, events) = observe_annotated_xml(&empty).unwrap();
    assert_eq!(doc, crate::from_xml(&wrap("<fence/>")).unwrap());
    assert_eq!(
        events
            .iter()
            .map(|e| e.payload.as_str())
            .collect::<Vec<_>>(),
        payloads
    );
    assert!(events.iter().all(|e| e.position == events[0].position));
    assert!(matches!(
        events[0].position,
        SemanticPosition::Text { offset: 0, .. }
    ));
    let event = comment("embed: x");
    let cdata = wrap(&format!("<fence><![CDATA[ α]]>{event}&#x1F600; </fence>"));
    let text = wrap(&format!("<fence> α{event}😀 </fence>"));
    assert_eq!(
        observe_annotated_xml(&cdata).unwrap(),
        observe_annotated_xml(&text).unwrap()
    );
    assert!(matches!(
        observe_annotated_xml(&text).unwrap().1[0].position,
        SemanticPosition::Text { offset: 2, .. }
    ));
}

#[test]
fn container_gaps_are_indent_independent_and_distinct_from_leaf_positions() {
    let event = comment("embed: x");
    let tight = wrap(&format!("<p>one</p>{event}<p>two</p>"));
    let spaced = wrap(&format!("\n <p>one</p>\n  {event}\n <p>two</p>\n"));
    let expected = observe_annotated_xml(&tight).unwrap();
    assert_eq!(expected, observe_annotated_xml(&spaced).unwrap());
    assert!(matches!(
        expected.1[0].position,
        SemanticPosition::ChildGap { child: 1, .. }
    ));
    let inside = observe_annotated_xml(&wrap(&format!("<p>one</p><p>{event}two</p>"))).unwrap();
    assert_eq!(inside.0, expected.0);
    assert_ne!(inside.1, expected.1);
}

#[test]
fn actual_reserved_malformed_comments_refuse_only_in_opt_in_observation() {
    for invalid in [
        "<!-- vibe:c2 embed: x -->",
        "<!-- vibe:c1 %2d -->",
        "<!--vibe:c1 embed: x-->",
    ] {
        let xml = wrap(&format!("<p>a{invalid}b</p>"));
        assert!(
            crate::from_xml(&xml).is_ok(),
            "ordinary reader compatibility"
        );
        let error = observe_annotated_xml(&xml).unwrap_err();
        assert!(
            error.message.contains("EMBED-GENERATED-PROVENANCE"),
            "{error}"
        );
    }
    for xml in [
        "<!DOCTYPE spec><spec xmlns='https://vibevm.org/spec/1'/>",
        "<spec xmlns='https://vibevm.org/spec/1'><p>&foreign;</p></spec>",
    ] {
        assert!(crate::from_xml(xml).is_err());
        assert!(observe_annotated_xml(xml).is_err());
    }
}

#[test]
fn before_after_document_and_multiline_physical_locations_are_observed() {
    let before = comment("embed: before\ncontinued");
    let after = comment("/embed: after");
    let xml = format!("{before}\n{}\n{after}", wrap("<p>x</p>"));
    let (_, events) = observe_annotated_xml(&xml).unwrap();
    assert_eq!(events[0].position, SemanticPosition::BeforeDocument);
    assert_eq!(events[1].position, SemanticPosition::AfterDocument);
    let locations = generated_xml_comments(&xml).unwrap();
    assert_eq!((locations[0].start_line, locations[0].end_line), (1, 2));
    assert_eq!((locations[1].start_line, locations[1].end_line), (4, 4));
}
