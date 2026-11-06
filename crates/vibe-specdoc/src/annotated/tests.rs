use super::*;

fn event(line_gap: usize, payload: &str) -> SourceComment {
    SourceComment {
        line_gap,
        payload: payload.into(),
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#EMBED-GENERATED-PROVENANCE"
)]
fn paragraph_boundaries_preserve_exact_semantics_without_inserting_whitespace() {
    let text = "# T {#t}\nbefore\ninside & <literal>\nafter\n";
    let projection = from_markdown_annotated(
        text,
        &[
            event(2, "embed: spec://org.test/a#x"),
            event(3, "/embed: spec://org.test/a#x"),
        ],
    )
    .unwrap();
    let xml = projection.render();
    assert_eq!(
        crate::from_xml(&xml).unwrap(),
        crate::from_markdown(text).unwrap()
    );
    assert!(
        xml.contains(&format!(
            "before\n{}inside",
            crate::frame_generated_xml_comment("embed: spec://org.test/a#x")
        )),
        "{xml}"
    );
    assert!(
        xml.contains(&format!(
            "&lt;literal&gt;\n{}after",
            crate::frame_generated_xml_comment("/embed: spec://org.test/a#x")
        )),
        "{xml}"
    );
    projection.observe(&xml).unwrap();
    let shifted = xml.replace(
        &crate::frame_generated_xml_comment("embed: spec://org.test/a#x"),
        "",
    );
    assert!(projection.observe(&shifted).is_err());
}

#[test]
fn structural_nested_and_empty_boundaries_are_not_comment_layout() {
    let text = "# T {#t}\n\n## A {#a}\nfirst\n\n### B {#b}\nsecond\n";
    let projection = from_markdown_annotated(
        text,
        &[
            event(2, "embed: outer"),
            event(5, "embed: inner"),
            event(7, "/embed: inner"),
            event(7, "/embed: outer"),
            event(7, "embed: empty"),
            event(7, "/embed: empty"),
        ],
    )
    .unwrap();
    let xml = projection.render();
    assert_eq!(
        crate::from_xml(&xml).unwrap(),
        crate::from_markdown(text).unwrap()
    );
    projection.observe(&xml).unwrap();
    assert_eq!(xml.matches("vibe:c1").count(), 6);
    let empty = from_markdown_annotated("", &[event(0, "embed: empty"), event(0, "/embed: empty")])
        .unwrap();
    empty.observe(&empty.render()).unwrap();
}

#[test]
fn literal_prose_comments_and_fences_never_mint_typed_events() {
    let text = "# T\n\n`<!-- embed: inline -->`\n\n<!-- embed: standalone -->\n\n```xml\n<!-- embed: fenced -->\n```\n\n&lt;!-- embed: escaped --&gt;\n";
    let projection = from_markdown_annotated(text, &[]).unwrap();
    assert_eq!(
        projection.render(),
        crate::to_xml(&crate::from_markdown(text).unwrap())
    );
    assert_eq!(projection.render().matches("vibe:c1").count(), 0);
    let injected = projection.render().replacen(
        "</spec>",
        &format!(
            "{}</spec>",
            crate::frame_generated_xml_comment("embed: injected")
        ),
        1,
    );
    assert!(projection.observe(&injected).is_err());
}

#[test]
fn extraction_maps_quote_fact_and_multibyte_text_through_existing_grammar() {
    for text in [
        "# T\n> first\n> Γ second\n> last\n",
        "# T\n@fact:X first\nΓ second\nlast @status:impl/done\n",
        "# T\n- first\n  Γ second\n  last\n",
    ] {
        let projected =
            from_markdown_annotated(text, &[event(2, "embed: x--y"), event(3, "/embed: x--y")])
                .unwrap();
        projected.observe(&projected.render()).unwrap();
        assert_eq!(
            crate::from_xml(&projected.render()).unwrap(),
            crate::from_markdown(text).unwrap()
        );
    }
}
