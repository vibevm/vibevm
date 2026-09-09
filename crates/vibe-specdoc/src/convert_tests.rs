use crate::convert::classify_projection;
use crate::doc::{SpecDoc, Title};
use crate::{Conversion, Direction, convert, from_markdown, to_xml};

const CANONICAL_MD: &str = "# T {#t}\n\n@fact:A body @status:impl/done\n\n";

#[test]
fn byte_stable_markdown_to_xml() {
    let result = convert(CANONICAL_MD, Direction::ToXml).expect("convert canonical Markdown");
    assert!(
        matches!(result, Conversion::ByteStable { .. }),
        "{result:?}"
    );
}

#[test]
fn byte_stable_xml_to_markdown() {
    let xml = to_xml(&from_markdown(CANONICAL_MD).expect("parse canonical Markdown"));
    let result = convert(&xml, Direction::ToMarkdown).expect("convert canonical XML");
    assert!(matches!(result, Conversion::ByteStable { .. }));
}

#[test]
fn markdown_comment_is_reported_as_content_loss() {
    let source = "# T {#t}\n\n<!-- REVIEW: preserve this -->\n\n@fact:A body @status:impl/done\n";
    let result = convert(source, Direction::ToXml).expect("convert commented Markdown");
    match result {
        Conversion::IrStableLoss { loss, .. } => {
            assert!(loss.contains("-<!-- REVIEW: preserve this -->"), "{loss}");
        }
        other => panic!("expected IR-stable loss, got {other:?}"),
    }
}

#[test]
fn xml_comment_is_reported_as_content_loss() {
    let canonical = to_xml(&from_markdown(CANONICAL_MD).expect("parse canonical Markdown"));
    let source = canonical.replacen("<spec", "<!-- REVIEW: preserve XML -->\n<spec", 1);
    let result = convert(&source, Direction::ToMarkdown).expect("convert commented XML");
    match result {
        Conversion::IrStableLoss { loss, .. } => {
            assert!(loss.contains("-<!-- REVIEW: preserve XML -->"), "{loss}");
        }
        other => panic!("expected IR-stable loss, got {other:?}"),
    }
}

#[test]
fn distinct_round_trip_ir_is_class_three() {
    let original = SpecDoc::default();
    let divergent = SpecDoc {
        title: Some(Title {
            text: "changed".to_string(),
            id: None,
        }),
        ..SpecDoc::default()
    };
    let result = classify_projection(
        "",
        "",
        &original,
        &divergent,
        "target".to_string(),
        from_markdown,
    );
    assert!(matches!(result, Conversion::IrDivergent { .. }));
}

#[test]
fn malformed_source_error_keeps_its_position() {
    let error = convert("<spec><foreign/></spec>", Direction::ToMarkdown)
        .expect_err("foreign element must fail");
    assert!(error.line > 0, "error must carry a line: {error}");
    assert!(error.to_string().contains("line "), "{error}");
}

#[test]
fn markdown_status_comment_with_inline_code_round_trips_verbatim() {
    let source = "# T {#t}\n\n\
                  <status stage=\"impl\" state=\"work\" \
                  comment=\"shipped with `471e3b1b`\"/>\n\n";
    let parsed = from_markdown(source).expect("parse Markdown status");
    let xml = to_xml(&parsed);
    let projected = crate::to_markdown(&crate::from_xml(&xml).expect("parse projected XML"));
    assert_eq!(projected, source);
}

#[test]
fn fragment_wrapper_beside_unit_status_is_not_ir_divergent() {
    let source = "@fact:X <status stage=\"spec\" state=\"void\">Retired.\
                  </status> @status:spec/void\n";
    let result = convert(source, Direction::ToXml).expect("convert fragment status");
    assert!(
        !matches!(result, Conversion::IrDivergent { .. }),
        "{result:?}"
    );
}

#[test]
fn multiline_inline_code_pipe_is_not_a_table_on_reparse() {
    let source = "CLI `session begin|end|show\n|ls` stays inline.\n";
    let result = convert(source, Direction::ToXml).expect("convert multiline inline code");
    assert!(
        !matches!(result, Conversion::IrDivergent { .. }),
        "{result:?}"
    );
}

#[test]
fn table_cell_code_span_pipe_survives_full_trip() {
    let source = "| `a | b` |\n";
    let result = convert(source, Direction::ToXml).expect("convert table cell code span");
    assert!(
        !matches!(result, Conversion::IrDivergent { .. }),
        "{result:?}"
    );
}

#[test]
fn table_cell_escaped_code_span_pipe_survives_full_trip() {
    let source = "| `a \\| b` |\n";
    let result = convert(source, Direction::ToXml).expect("convert escaped table cell code span");
    assert!(
        !matches!(result, Conversion::IrDivergent { .. }),
        "{result:?}"
    );
}

#[test]
fn requirements_survive_both_pivot_polygons_in_canonical_order() {
    let md = "# T {#t}\n\n\
              @fact:A body @requires:external,verification,specification <status stage=\"test\" state=\"done\" ref=\"proof:1\"/>\n";
    let ir = from_markdown(md).expect("requirements Markdown");
    let fact = match &ir.preamble[0] {
        crate::doc::Block::Paragraph(unit) => unit.fact.as_ref().unwrap(),
        other => panic!("{other:?}"),
    };
    assert_eq!(
        fact.requirements.as_ref().unwrap().to_csv(),
        "specification,verification,external"
    );
    assert_eq!(
        fact.status.as_ref().unwrap().r#ref.as_deref(),
        Some("proof:1")
    );

    let xml = to_xml(&ir);
    assert!(
        xml.contains(
            "status=\"test/done\" requires=\"specification,verification,external\" ref=\"proof:1\""
        ),
        "{xml}"
    );
    let from_xml = crate::from_xml(&xml).expect("XML reparse");
    assert_eq!(from_xml, ir);
    let emitted_md = crate::to_markdown(&from_xml);
    assert!(
        emitted_md.contains("@requires:specification,verification,external <status"),
        "{emitted_md}"
    );
    assert_eq!(from_markdown(&emitted_md).unwrap(), ir);
    assert_eq!(to_xml(&from_markdown(&emitted_md).unwrap()), xml);
}

#[test]
fn xml_named_and_generic_requirements_share_one_ir() {
    let named = "<spec xmlns=\"https://vibevm.org/spec/1\"><p><LAW fact=\"true\" status=\"spec/done\" requires=\"plan,specification\">body</LAW></p></spec>";
    let generic = "<spec xmlns=\"https://vibevm.org/spec/1\"><p><fact id=\"LAW\" status=\"spec/done\" requires=\"plan,specification\">body</fact></p></spec>";
    let named_ir = crate::from_xml(named).unwrap();
    let generic_ir = crate::from_xml(generic).unwrap();
    assert_eq!(named_ir, generic_ir);
    let canonical = to_xml(&named_ir);
    assert!(
        canonical.contains("requires=\"specification,plan\""),
        "{canonical}"
    );
    assert_eq!(crate::from_xml(&canonical).unwrap(), named_ir);
}

#[test]
fn xml_requirements_fail_closed() {
    let cases = [
        ("requires=\"\" status=\"spec/done\"", "list is empty"),
        ("requires=\"plan,\" status=\"spec/done\"", "empty member"),
        ("requires=\"plan,plan\" status=\"spec/done\"", "duplicate"),
        (
            "requires=\"spaceship\" status=\"spec/done\"",
            "unknown required",
        ),
        ("requires=\"plan\"", "needs a `status`"),
        ("requires=\"plan\" status=\"spec/void\"", "void fact"),
        (
            "requires=\"external\" status=\"impl/done\"",
            "non-empty fact `ref`",
        ),
        (
            "requires=\"external\" status=\"impl/done\" ref=\"\"",
            "non-empty fact `ref`",
        ),
    ];
    for (attrs, needle) in cases {
        let xml = format!(
            "<spec xmlns=\"https://vibevm.org/spec/1\"><p><fact id=\"A\" {attrs}>body</fact></p></spec>"
        );
        let error = crate::from_xml(&xml).expect_err(attrs);
        assert!(error.to_string().contains(needle), "{attrs}: {error}");
    }

    let non_fact = "<spec xmlns=\"https://vibevm.org/spec/1\"><p requires=\"plan\">body</p></spec>";
    let error = crate::from_xml(non_fact).expect_err("non-fact requires");
    assert!(
        error.to_string().contains("no `requires` attribute"),
        "{error}"
    );

    let standalone = "<spec xmlns=\"https://vibevm.org/spec/1\"><status stage=\"spec\" state=\"done\" requires=\"plan\"/></spec>";
    let error = crate::from_xml(standalone).expect_err("status cannot carry requires");
    assert!(
        error.to_string().contains("no `requires` attribute"),
        "{error}"
    );

    let duplicate_ref = "<spec xmlns=\"https://vibevm.org/spec/1\"><p><fact id=\"A\" status=\"impl/done\" requires=\"external\" ref=\"one\" ref=\"two\">body</fact></p></spec>";
    assert!(crate::from_xml(duplicate_ref).is_err());
}

#[test]
fn every_markdown_carrier_round_trips_requirements() {
    let md = "# T {#t}\n\n\
              @fact:PARA paragraph @requires:verification,specification @status:test/done\n\n\
              - @fact:ITEM item @requires:verification,specification @status:test/done\n\n\
              | H |\n|---|\n| @fact:CELL cell @requires:verification,specification @status:test/done |\n\n\
              > @fact:QUOTE quote @requires:verification,specification @status:test/done\n\n\
              @fact/code:CODE typed @requires:verification,specification @status:test/done\n\n\
              ```rust\nassert!(true);\n```\n";
    let ir = from_markdown(md).expect("all carriers parse");
    let xml = to_xml(&ir);
    let xml_ir = crate::from_xml(&xml).expect("all carriers parse as XML");
    assert_eq!(xml_ir, ir);
    let emitted = crate::to_markdown(&xml_ir);
    assert_eq!(from_markdown(&emitted).unwrap(), ir);
    assert_eq!(
        emitted
            .matches("@requires:specification,verification")
            .count(),
        5
    );
}

#[test]
fn same_line_table_cells_keep_their_own_status_ref_and_requirements() {
    let md = "| A | B |\n|---|---|\n\
              | @fact:LEFT left @requires:specification @status:spec/done | \
              @fact:RIGHT right @requires:external <status stage=\"impl\" state=\"done\" ref=\"proof:right\"/> |\n";
    let ir = from_markdown(md).expect("two independently marked cells");
    let xml = to_xml(&ir);
    assert!(
        xml.contains("<LEFT fact=\"true\" status=\"spec/done\" requires=\"specification\""),
        "{xml}"
    );
    assert!(
        xml.contains(
            "<RIGHT fact=\"true\" status=\"impl/done\" requires=\"external\" ref=\"proof:right\""
        ),
        "{xml}"
    );
    assert_eq!(crate::from_xml(&xml).expect("XML reparse"), ir);
}
