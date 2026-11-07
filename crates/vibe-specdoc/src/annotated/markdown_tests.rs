use crate::{MarkdownCommentPlacement, SemanticPosition, project_annotated_xml_to_markdown};

fn wrap(body: &str) -> String {
    format!("<spec xmlns='https://vibevm.org/spec/1'>{body}</spec>")
}

#[test]
fn scalar_mapping_preserves_old_observer_positions_and_unicode() {
    let xml = wrap("<p>é😀<!-- vibe:c1 embed: x -->&amp;tail</p>");
    let (document, events) = crate::observe_annotated_xml(&xml).unwrap();
    assert_eq!(
        events[0].position,
        SemanticPosition::Text { node: 2, offset: 2 }
    );
    let (plain, comments) = project_annotated_xml_to_markdown(&xml)
        .unwrap()
        .into_parts();
    assert_eq!(plain, crate::to_markdown(&document));
    assert_eq!(&plain[..comments[0].byte_offset], "é😀");
    assert_eq!(comments[0].placement, MarkdownCommentPlacement::Inline);
}

#[test]
fn repeated_equal_text_maps_by_semantic_node_not_spelling() {
    let xml = wrap("<p>same</p><p>sa<!-- vibe:c1 embed: x -->me</p>");
    let (plain, comments) = project_annotated_xml_to_markdown(&xml)
        .unwrap()
        .into_parts();
    assert_eq!(plain, "same\n\nsame\n\n");
    assert_eq!(comments[0].byte_offset, 8);
}

#[test]
fn container_and_coincident_empty_events_keep_order() {
    let xml = wrap(
        "<!-- vibe:c1 embed: outer --><!-- vibe:c1 embed: empty --><!-- vibe:c1 /embed: empty --><section id='s' title='S'><p>x</p></section><!-- vibe:c1 /embed: outer -->",
    );
    let (plain, comments) = project_annotated_xml_to_markdown(&xml)
        .unwrap()
        .into_parts();
    assert_eq!(comments.len(), 4);
    assert_eq!(
        comments
            .iter()
            .take(3)
            .map(|event| event.byte_offset)
            .collect::<Vec<_>>(),
        vec![0, 0, 0]
    );
    assert_eq!(comments[3].byte_offset, plain.len());
    assert_eq!(comments[1].payload, "embed: empty");
    assert_eq!(comments[2].payload, "/embed: empty");
}

#[test]
fn quote_list_and_table_scalar_maps_follow_the_actual_writer() {
    for (body, expected_prefix) in [
        (
            "<quote>first\n<!-- vibe:c1 embed: x -->second</quote>",
            "> first\n> ",
        ),
        (
            "<quote>first&#13;\n<!-- vibe:c1 embed: x -->second</quote>",
            "> first\n> ",
        ),
        (
            "<list ordered='false'><item>a</item><item>b<!-- vibe:c1 embed: x -->c</item></list>",
            "- a\n- b",
        ),
        (
            "<table><tr><td>H</td></tr><tr><td>b<!-- vibe:c1 embed: x -->c</td></tr></table>",
            "| H |\n| --- |\n| b",
        ),
    ] {
        let (plain, comments) = project_annotated_xml_to_markdown(&wrap(body))
            .unwrap()
            .into_parts();
        assert_eq!(&plain[..comments[0].byte_offset], expected_prefix);
    }
}

#[test]
fn literal_and_cdata_controls_are_not_events() {
    let xml = wrap(
        "<p>&lt;!-- vibe:c1 embed: literal --&gt;</p><fence><![CDATA[<!-- vibe:c1 embed: code -->]]></fence>",
    );
    let (plain, comments) = project_annotated_xml_to_markdown(&xml)
        .unwrap()
        .into_parts();
    assert!(comments.is_empty());
    assert!(plain.contains("<!-- vibe:c1 embed: literal -->"));
    assert!(plain.contains("```\n<!-- vibe:c1 embed: code -->\n```"));
}
