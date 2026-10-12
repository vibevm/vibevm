//! Authored CommonMark exports joined to the same selected held observation.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-EXPORT-SELECTOR");

use super::*;

#[derive(Debug)]
pub struct CommonMarkExtraction<'a> {
    pub capture: &'a StableFileSnapshot,
    pub provenance: ExtractionProvenance,
    pub export_name: String,
    pub heading_chain: Vec<String>,
    pub representation: RepresentationWitness,
}

impl CommonMarkExtraction<'_> {
    pub fn content(&self) -> &[u8] {
        &self.capture.bytes[self.representation.span.clone()]
    }
}

#[derive(Debug)]
pub struct SpecOutline<'a> {
    pub capture: &'a StableFileSnapshot,
    pub provenance: ExtractionProvenance,
    pub nodes: Vec<vibe_specdoc::selection::OutlineNode>,
    pub representation: RepresentationWitness,
}

impl WitnessedSourceWorld {
    /// Resolve only an authored name/file/heading row retained by this owner.
    /// A caller cannot substitute a heading path or turn the result into facts.
    pub fn read_export(
        &self,
        owner: Option<(&str, &str)>,
        name: &str,
    ) -> Result<CommonMarkExtraction<'_>, ExtractionError> {
        let prepared = self.revalidate_current()?;
        let key = owner.map_or(CaptureOwner::SelfNode, |(group, name)| {
            CaptureOwner::Dependency {
                group: group.to_owned(),
                name: name.to_owned(),
            }
        });
        let capture = prepared
            .captured
            .get(&key)
            .ok_or_else(|| SourceQueryUnavailable::MissingPreparedCapture(key.label()))?;
        let manifest = capture.captured_manifest()?;
        let package = manifest.package.as_ref().ok_or_else(|| {
            ExtractionError::Descriptor("source exports require an authored package".into())
        })?;
        let candidates = package
            .source_exports
            .iter()
            .map(|row| row.name().to_owned())
            .collect::<Vec<_>>();
        let matches = package
            .source_exports
            .iter()
            .filter(|row| row.name() == name)
            .collect::<Vec<_>>();
        let [row] = matches.as_slice() else {
            return Err(ExtractionError::Export {
                requested: name.to_owned(),
                candidates,
                reason: "missing or ambiguous authored export".into(),
            });
        };
        let selected = if let Some(map) = &capture.descriptor {
            let outputs: Vec<_> = capture
                .outputs
                .keys()
                .filter(|output| {
                    map.output_row(output)
                        .is_some_and(|mapped| mapped.source == row.file())
                })
                .cloned()
                .collect();
            let [output] = outputs.as_slice() else {
                return Err(ExtractionError::Export {
                    requested: name.to_owned(),
                    candidates,
                    reason: "missing or ambiguous captured export mapping".into(),
                });
            };
            output.clone()
        } else {
            row.file().to_owned()
        };
        let source = self.extract_source(owner, &selected)?;
        if source.provenance.source_path != row.file() {
            return Err(ExtractionError::Descriptor(
                "export and captured source differ".into(),
            ));
        }
        let StructuredView::CommonMark(document) = source.view else {
            return Err(ExtractionError::Export {
                requested: name.to_owned(),
                candidates,
                reason: "export source is not selected CommonMark".into(),
            });
        };
        let resolved = document
            .resolve_export(row.name(), row.headings())
            .map_err(|error| ExtractionError::Export {
                requested: name.to_owned(),
                candidates,
                reason: format!("{error:?}"),
            })?;
        if resolved.source_sha256 != format!("sha256:{}", source.capture.sha256)
            || source.capture.bytes.get(resolved.span.clone()).is_none()
        {
            return Err(ExtractionError::Descriptor(
                "export snapshot/span differs from held observation".into(),
            ));
        }
        self.revalidate_current()?;
        Ok(CommonMarkExtraction {
            capture: source.capture,
            provenance: source.provenance,
            export_name: row.name().to_owned(),
            heading_chain: row.headings().to_vec(),
            representation: RepresentationWitness {
                encoding: SourceEncoding::Utf8,
                recipe: "commonmark-export/1".into(),
                basis: SpanBasis::CapturedBytes,
                span: resolved.span,
            },
        })
    }

    /// Outline from captured classified IR, never an ambient/prebuilt map.
    pub fn outline_spec(
        &self,
        owner: Option<(&str, &str)>,
        output: &str,
    ) -> Result<SpecOutline<'_>, ExtractionError> {
        let source = self.extract_source(owner, output)?;
        let StructuredView::Vibe {
            document,
            vocabulary,
        } = source.view
        else {
            return Err(ExtractionError::VibeUnavailable(
                "selected source has no usable Vibe IR outline".into(),
            ));
        };
        let nodes = vibe_specdoc::selection::outline(&document, vocabulary)?;
        let size = vibe_specdoc::to_markdown(&document).len();
        self.revalidate_current()?;
        Ok(SpecOutline {
            capture: source.capture,
            provenance: source.provenance,
            nodes,
            representation: RepresentationWitness {
                encoding: SourceEncoding::Utf8,
                recipe: format!(
                    "{}+{}",
                    vibe_specdoc::CONVERTER_RECIPE,
                    vibe_specdoc::selection::SELECTION_RECIPE
                ),
                basis: SpanBasis::EmittedMarkdown,
                span: 0..size,
            },
        })
    }
}
