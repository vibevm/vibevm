//! Pure classified inspection of retained bytes; raw input always survives.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#DOCUMENT-ANALYSIS-RAW-AND-PREVIEW");
use crate::{
    commonmark::CommonMarkDoc,
    doc::{SpecDoc, Vocabulary},
    xml_data::DataInspection,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapturedSyntax {
    VibeMarkdown,
    VibeXml(Vocabulary),
    Commonmark,
    XmlData,
    Opaque,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InspectionDiagnostic {
    InvalidUtf8,
    Markdown(crate::MarkdownConversionFailure),
    Xml(crate::Error),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PositionBasis {
    OriginalBytes,
    MarkdownProjection(&'static str),
}
#[derive(Debug)]
pub enum StructuredView {
    Vibe { ir: SpecDoc, vocabulary: Vocabulary },
    Commonmark(CommonMarkDoc),
    XmlData(DataInspection),
    Opaque,
}
#[derive(Debug)]
pub struct CapturedInspection<'a> {
    raw: &'a [u8],
    pub view: Option<StructuredView>,
    pub diagnostics: Vec<InspectionDiagnostic>,
    pub basis: PositionBasis,
}
impl CapturedInspection<'_> {
    pub fn raw(&self) -> &[u8] {
        self.raw
    }
}
pub fn inspect_captured(raw: &[u8], syntax: CapturedSyntax) -> CapturedInspection<'_> {
    let mut result = CapturedInspection {
        raw,
        view: None,
        diagnostics: vec![],
        basis: PositionBasis::OriginalBytes,
    };
    if syntax == CapturedSyntax::Opaque {
        result.view = Some(StructuredView::Opaque);
        return result;
    }
    if syntax == CapturedSyntax::XmlData {
        result.view = Some(StructuredView::XmlData(
            crate::xml_data::inspect_xml_data(raw).inspection,
        ));
        return result;
    }
    let text = match std::str::from_utf8(raw) {
        Ok(text) => text,
        Err(_) => {
            result.diagnostics.push(InspectionDiagnostic::InvalidUtf8);
            return result;
        }
    };
    match syntax {
        CapturedSyntax::Commonmark => {
            result.view = Some(StructuredView::Commonmark(CommonMarkDoc::parse(text)))
        }
        CapturedSyntax::VibeMarkdown => match crate::from_markdown_detailed(text) {
            Ok(ir) => {
                result.view = Some(StructuredView::Vibe {
                    ir,
                    vocabulary: Vocabulary::Spec,
                })
            }
            Err(error) => result
                .diagnostics
                .push(InspectionDiagnostic::Markdown(error)),
        },
        CapturedSyntax::VibeXml(vocabulary) => match crate::from_xml_with(text, vocabulary) {
            Ok(ir) => {
                result.view = Some(StructuredView::Vibe { ir, vocabulary });
                result.basis = PositionBasis::MarkdownProjection("vibe-specdoc-to-markdown/1");
            }
            Err(error) => result.diagnostics.push(InspectionDiagnostic::Xml(error)),
        },
        CapturedSyntax::XmlData | CapturedSyntax::Opaque => unreachable!(),
    }
    result
}
#[cfg(test)]
mod m_16_d_1_cases {
    use super::*;
    #[test]
    fn declared_views_preserve_inert_ordinary_and_invalid_raw() {
        let md = b"# Ordinary\n\n@fact:FAKE text @status:impl/done\n#use spec://org.demo/pkg/X\n";
        let ordinary = inspect_captured(md, CapturedSyntax::Commonmark);
        assert_eq!(ordinary.raw(), md);
        assert!(matches!(ordinary.view, Some(StructuredView::Commonmark(_))));
        let bad = b"<data><broken>";
        let xml = inspect_captured(bad, CapturedSyntax::XmlData);
        assert_eq!(xml.raw(), bad);
        assert!(matches!(
            xml.view,
            Some(StructuredView::XmlData(DataInspection::Unavailable(_)))
        ));
        let partial = b"<spec xmlns=\"https://vibevm.org/spec/1\"><p>";
        let view = inspect_captured(partial, CapturedSyntax::VibeXml(Vocabulary::Spec));
        assert_eq!(view.raw(), partial);
        assert!(view.view.is_none());
        assert!(matches!(
            &view.diagnostics[..],
            [InspectionDiagnostic::Xml(_)]
        ));
        let bytes = b"\xff\x00";
        let opaque = inspect_captured(bytes, CapturedSyntax::Opaque);
        assert_eq!(opaque.raw(), bytes);
        assert!(matches!(opaque.view, Some(StructuredView::Opaque)));
    }
    #[test]
    fn valid_vibe_inputs_use_existing_frontends() {
        let md = b"# Rule {#root}\n\n@fact:ONE One. @status:impl/done\n";
        let ir = crate::from_markdown(std::str::from_utf8(md).unwrap()).unwrap();
        let xml = crate::to_xml(&ir);
        for (raw, syntax) in [
            (md.as_slice(), CapturedSyntax::VibeMarkdown),
            (xml.as_bytes(), CapturedSyntax::VibeXml(Vocabulary::Spec)),
        ] {
            let view = inspect_captured(raw, syntax);
            assert_eq!(view.raw(), raw);
            assert!(view.diagnostics.is_empty());
            let Some(StructuredView::Vibe { ir: actual, .. }) = view.view else {
                panic!("missing Vibe view")
            };
            assert_eq!(actual, ir);
        }
    }
}
