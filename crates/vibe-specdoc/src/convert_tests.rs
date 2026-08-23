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
