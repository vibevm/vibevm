//! Immutable exact source provider for analysis; never falls back to disk.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#DOCUMENT-ANALYSIS-CAPTURE-LAW");
use crate::{SectionSource, SpecAddress};
use std::{collections::BTreeMap, sync::Arc};
use vibe_specdoc::{
    captured::{CapturedSyntax, StructuredView, inspect_captured},
    doc::{SpecDoc, Vocabulary},
};
#[derive(Debug, Clone)]
pub struct CapturedSpecDocument {
    pub address: SpecAddress,
    pub raw: Arc<[u8]>,
    pub syntax: CapturedSyntax,
}
#[derive(Debug)]
struct Document {
    ir: SpecDoc,
    vocabulary: Vocabulary,
    selected_version: Option<String>,
}
#[derive(Debug)]
pub struct CapturedSectionSource {
    documents: BTreeMap<String, Document>,
    refused: BTreeMap<String, String>,
}
impl CapturedSectionSource {
    pub fn new(documents: Vec<CapturedSpecDocument>) -> Result<Self, String> {
        let mut result = Self {
            documents: BTreeMap::new(),
            refused: BTreeMap::new(),
        };
        for document in documents {
            if !document.address.anchor.is_empty() || document.address.pinned_r.is_some() {
                return Err("captured document must have an unanchored logical identity".into());
            }
            let mut identity = document.address.clone();
            let selected_version = match &mut identity.authority {
                crate::Authority::Package { version, .. } => version.take(),
                crate::Authority::Host(_) => {
                    return Err("legacy host authority has no captured package coordinate".into());
                }
            };
            let key = identity.without_pin();
            SpecAddress::parse(&key).map_err(|e| e.to_string())?;
            if result.documents.contains_key(&key) || result.refused.contains_key(&key) {
                return Err(format!("ambiguous captured document {key}"));
            }
            let view = inspect_captured(&document.raw, document.syntax);
            match view.view {
                Some(StructuredView::Vibe { ir, vocabulary }) => {
                    result.documents.insert(
                        key,
                        Document {
                            ir,
                            vocabulary,
                            selected_version,
                        },
                    );
                }
                _ => {
                    result.refused.insert(
                        key,
                        format!(
                            "captured source has no admitted Vibe view: {:?}",
                            view.diagnostics
                        ),
                    );
                }
            }
        }
        Ok(result)
    }
}
impl SectionSource for CapturedSectionSource {
    fn section_text(&self, address: &SpecAddress) -> Result<String, String> {
        if let Some(revision) = address.pinned_r {
            return Err(format!(
                "captured spec-unit revision unavailable: requested r{revision}; no retained revision witness"
            ));
        }
        let mut root = address.clone();
        root.anchor.clear();
        root.pinned_r = None;
        let requested_version = match &mut root.authority {
            crate::Authority::Package { version, .. } => version.take(),
            crate::Authority::Host(_) => {
                return Err("legacy host authority has no captured package coordinate".into());
            }
        };
        let key = root.without_pin();
        if let Some(reason) = self.refused.get(&key) {
            return Err(reason.clone());
        }
        let document = self.documents.get(&key).ok_or_else(|| {
            format!(
                "source not in immutable capture: {key}; candidates: {:?}",
                self.documents.keys().collect::<Vec<_>>()
            )
        })?;
        if let Some(requested) = requested_version {
            if document.selected_version.as_ref() != Some(&requested) {
                return Err(format!(
                    "captured version mismatch: requested {requested}, selected {:?}",
                    document.selected_version
                ));
            }
        }
        let path = (!address.anchor.is_empty()).then(|| address.anchor.join("."));
        let selected = vibe_specdoc::selection::select_node(
            &document.ir,
            path.as_deref(),
            true,
            document.vocabulary,
        )
        .map_err(|e| e.to_string())?;
        Ok(vibe_specdoc::to_markdown(&selected.ir))
    }
}
#[cfg(test)]
mod m_16_d_1_cases {
    use super::*;
    fn doc(name: &str, text: &str, syntax: CapturedSyntax) -> CapturedSpecDocument {
        CapturedSpecDocument {
            address: SpecAddress::parse(&format!("spec://org.demo/pkg/{name}")).unwrap(),
            raw: Arc::from(text.as_bytes()),
            syntax,
        }
    }
    #[test]
    fn actual_builtin_closure_embed_uses_only_captured_bytes() {
        let seed = SpecAddress::parse("spec://org.demo/pkg/ROOT").unwrap();
        let source = CapturedSectionSource::new(vec![
            doc(
                "ROOT",
                "# Root {#root}\n\n#embed spec://org.demo/pkg/CHILD#child\n",
                CapturedSyntax::VibeMarkdown,
            ),
            doc(
                "CHILD",
                "# Child {#child}\n\n@fact:ONE Captured child. @status:impl/done\n",
                CapturedSyntax::VibeMarkdown,
            ),
        ])
        .unwrap();
        let result = crate::compile_static(&seed, &source).unwrap();
        assert!(result.contains("Captured child."));
        assert!(!result.contains("#embed"));
        let xml = vibe_specdoc::to_xml(
            &vibe_specdoc::from_markdown(
                "# Child {#child}\n\n@fact:ONE XML child. @status:impl/done\n",
            )
            .unwrap(),
        );
        let xml_source = CapturedSectionSource::new(vec![
            doc(
                "ROOT",
                "# Root {#root}\n\n#embed spec://org.demo/pkg/CHILD#child\n",
                CapturedSyntax::VibeMarkdown,
            ),
            doc("CHILD", &xml, CapturedSyntax::VibeXml(Vocabulary::Spec)),
        ])
        .unwrap();
        assert!(
            crate::compile_static(&seed, &xml_source)
                .unwrap()
                .contains("XML child.")
        );
    }
    #[test]
    fn revision_pin_refuses_unpinned_capture() {
        let source = CapturedSectionSource::new(vec![doc(
            "CHILD",
            "# Child {#child}\n\nRetained current bytes.\n",
            CapturedSyntax::VibeMarkdown,
        )])
        .unwrap();
        let current = SpecAddress::parse("spec://org.demo/pkg/CHILD#child").unwrap();
        assert!(
            source
                .section_text(&current)
                .unwrap()
                .contains("Retained current bytes.")
        );
        let pinned = SpecAddress::parse("spec://org.demo/pkg/CHILD#child~r999").unwrap();
        assert_eq!(pinned.pinned_r, Some(999));
        assert_eq!(
            source.section_text(&pinned),
            Err("captured spec-unit revision unavailable: requested r999; no retained revision witness".into())
        );
    }
    #[test]
    fn uncaptured_or_ordinary_source_refuses_without_fs_fallback() {
        let source = CapturedSectionSource::new(vec![doc(
            "DATA",
            "# Data\n\n@fact:FAKE Ordinary.\n",
            CapturedSyntax::Commonmark,
        )])
        .unwrap();
        assert!(
            source
                .section_text(&SpecAddress::parse("spec://org.demo/pkg/DATA").unwrap())
                .unwrap_err()
                .contains("no admitted Vibe view")
        );
        assert!(
            source
                .section_text(&SpecAddress::parse("spec://org.demo/pkg/MISSING").unwrap())
                .unwrap_err()
                .contains("not in immutable capture")
        );
        let mut pinned = doc("PINNED", "# Pinned {#root}\n", CapturedSyntax::VibeMarkdown);
        if let crate::Authority::Package { version, .. } = &mut pinned.address.authority {
            *version = Some("1.0.0".into());
        }
        let source = CapturedSectionSource::new(vec![pinned]).unwrap();
        assert!(
            source
                .section_text(&SpecAddress::parse("spec://org.demo/pkg@1.0.0/PINNED").unwrap())
                .is_ok()
        );
        assert!(
            source
                .section_text(&SpecAddress::parse("spec://org.demo/pkg@2.0.0/PINNED").unwrap())
                .unwrap_err()
                .contains("version mismatch")
        );
    }
}
