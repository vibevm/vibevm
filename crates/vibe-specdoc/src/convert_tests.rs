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
