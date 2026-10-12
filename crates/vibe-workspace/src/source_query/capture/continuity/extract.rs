//! Classified extraction from one retained selected-world observation.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-070#CLASSIFIED-SOURCE-EXTRACTION");

use std::ops::Range;

use vibe_core::manifest::{
    SelectedSource, SourceDescriptor, SourceInterpretation, select_source_rows,
};
use vibe_safefs::StableFileSnapshot;
use vibe_specdoc::doc::{SpecDoc, Vocabulary};
use vibe_specdoc::selection::{NodeSelection, SelectionError};

use super::membership::{MaterializedVibeMedia, SourceStrength};
mod export;
use super::{CaptureOwner, SourceQueryUnavailable, WitnessedSourceWorld};
pub use export::{CommonMarkExtraction, SpecOutline};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceEncoding {
    Utf8,
    Bytes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanBasis {
    CapturedBytes,
    EmittedMarkdown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepresentationWitness {
    pub encoding: SourceEncoding,
    pub recipe: String,
    pub basis: SpanBasis,
    /// Exact offsets within this named representation, not another source.
    pub span: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedDescriptorWitness {
    pub identity: String,
    pub declaration_index: usize,
    pub descriptor: SourceDescriptor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractionProvenance {
    pub output_path: String,
    pub source_path: String,
    pub manifest_sha256: String,
    pub output_sha256: String,
    pub descriptor: Option<SelectedDescriptorWitness>,
    pub materialized_media: Option<String>,
    /// A local record claim, not a canonical replay/derivation proof.
    pub recorded_converter_recipe: Option<String>,
    pub strength: SourceStrength,
}

#[derive(Debug)]
pub enum StructuredView {
    Vibe {
        document: SpecDoc,
        vocabulary: Vocabulary,
    },
    CommonMark(vibe_specdoc::commonmark::CommonMarkDoc),
    XmlData(vibe_specdoc::xml_data::DataInspection),
    NotRequested,
    Unavailable(String),
}

#[derive(Debug)]
pub struct ExtractedSource<'a> {
    pub capture: &'a StableFileSnapshot,
    pub provenance: ExtractionProvenance,
    pub representation: RepresentationWitness,
    pub view: StructuredView,
}

impl ExtractedSource<'_> {
    pub fn raw(&self) -> &[u8] {
        &self.capture.bytes
    }
}

#[derive(Debug)]
pub struct SpecExtraction<'a> {
    pub capture: &'a StableFileSnapshot,
    pub provenance: ExtractionProvenance,
    pub selection: NodeSelection,
    pub markdown: String,
    pub representation: RepresentationWitness,
}

#[derive(Debug, thiserror::Error)]
pub enum ExtractionError {
    #[error("source export `{requested}` unavailable: {reason}; candidates: {candidates:?}")]
    Export {
        requested: String,
        candidates: Vec<String>,
        reason: String,
    },
    #[error(transparent)]
    World(#[from] SourceQueryUnavailable),
    #[error("source descriptor selection is unavailable: {0}")]
    Descriptor(String),
    #[error("selected source has no declared Vibe view: {0}")]
    VibeUnavailable(String),
    #[error(transparent)]
    Node(#[from] SelectionError),
}

impl WitnessedSourceWorld {
    /// Read only an output already captured in this owner world. Optional
    /// parsing failure leaves the exact raw observation in the result.
    pub fn extract_source(
        &self,
        owner: Option<(&str, &str)>,
        output: &str,
    ) -> Result<ExtractedSource<'_>, ExtractionError> {
        let prepared = self.revalidate_current()?;
        let key = owner.map_or(CaptureOwner::SelfNode, |(group, name)| {
            CaptureOwner::Dependency {
                group: group.to_owned(),
                name: name.to_owned(),
            }
        });
        let captured = prepared
            .captured
            .get(&key)
            .ok_or_else(|| SourceQueryUnavailable::MissingPreparedCapture(key.label()))?;
        let observation = prepared.captured_output(owner, output)?;
        let manifest = captured.captured_manifest()?;
        let row = captured
            .descriptor
            .as_ref()
            .and_then(|map| map.output_row(output));
        let source_path = row.map_or(output, |row| row.source.as_str());
        let selection = if let Some(package) = &manifest.package {
            package.select_source(source_path)
        } else if let Some(project) = &manifest.project {
            let coordinate = project
                .group
                .as_ref()
                .map(|group| format!("{group}/{}", project.name))
                .unwrap_or_else(|| "self".to_owned());
            select_source_rows(&coordinate, &project.sources, source_path)
        } else {
            return Err(ExtractionError::Descriptor(
                "owner has no authored source context".into(),
            ));
        }
        .map_err(|error| ExtractionError::Descriptor(format!("{error:?}")))?;
        let descriptor = match selection {
            SelectedSource::Unclassified { .. } => None,
            SelectedSource::Descriptor {
                identity,
                declaration_index,
                descriptor,
            } => Some(SelectedDescriptorWitness {
                identity,
                declaration_index,
                descriptor: descriptor.clone(),
            }),
            SelectedSource::NeedsInput {
                identity,
                declaration_indices,
            } => {
                return Err(ExtractionError::Descriptor(format!(
                    "{identity}: ambiguous rows {declaration_indices:?}"
                )));
            }
        };
        let interpretation = descriptor
            .as_ref()
            .map(|witness| witness.descriptor.interpretation());
        if row.is_some_and(|row| row.interpretation != interpretation) {
            return Err(ExtractionError::Descriptor(
                "captured map and authored interpretation differ".into(),
            ));
        }
        let mut media = descriptor
            .as_ref()
            .map(|witness| witness.descriptor.media().to_owned());
        let mut recorded_recipe = None;
        let mut strength = SourceStrength::Observed;
        if let Some((group, name)) = owner {
            if let Some(recorded) = self.recorded_output(group, name, output)? {
                strength = SourceStrength::RecordConsistent;
                recorded_recipe = recorded.converter_recipe;
                if let Some(actual) = recorded.media {
                    media = Some(
                        match actual {
                            MaterializedVibeMedia::Markdown => "text/markdown",
                            MaterializedVibeMedia::Xml => "application/xml",
                        }
                        .into(),
                    );
                }
            }
        }
        let view = structured_view(&observation.bytes, interpretation, media.as_deref());
        self.revalidate_current()?;
        Ok(ExtractedSource {
            capture: observation,
            provenance: ExtractionProvenance {
                output_path: output.to_owned(),
                source_path: source_path.to_owned(),
                manifest_sha256: captured.manifest.sha256.clone(),
                output_sha256: observation.sha256.clone(),
                descriptor,
                materialized_media: media,
                recorded_converter_recipe: recorded_recipe,
                strength,
            },
            representation: RepresentationWitness {
                encoding: if std::str::from_utf8(&observation.bytes).is_ok() {
                    SourceEncoding::Utf8
                } else {
                    SourceEncoding::Bytes
                },
                recipe: "captured-bytes/1".into(),
                basis: SpanBasis::CapturedBytes,
                span: 0..observation.bytes.len(),
            },
            view,
        })
    }

    /// The pure node semantics used by the eventual read_spec adapters and
    /// compiler embeds. This is a selected-world API, not a new discovery route.
    pub fn read_spec(
        &self,
        owner: Option<(&str, &str)>,
        output: &str,
        anchor: Option<&str>,
        children: bool,
    ) -> Result<SpecExtraction<'_>, ExtractionError> {
        let source = self.extract_source(owner, output)?;
        let (document, vocabulary) = match source.view {
            StructuredView::Vibe {
                document,
                vocabulary,
            } => (document, vocabulary),
            StructuredView::Unavailable(reason) => {
                return Err(ExtractionError::VibeUnavailable(reason));
            }
            _ => {
                return Err(ExtractionError::VibeUnavailable(
                    "ordinary or unclassified resource".into(),
                ));
            }
        };
        let selection =
            vibe_specdoc::selection::select_node(&document, anchor, children, vocabulary)?;
        let markdown = vibe_specdoc::to_markdown(&selection.ir);
        self.revalidate_current()?;
        Ok(SpecExtraction {
            capture: source.capture,
            provenance: source.provenance,
            selection,
            representation: RepresentationWitness {
                encoding: SourceEncoding::Utf8,
                recipe: format!(
                    "{}+{}",
                    vibe_specdoc::CONVERTER_RECIPE,
                    vibe_specdoc::selection::SELECTION_RECIPE
                ),
                basis: SpanBasis::EmittedMarkdown,
                span: 0..markdown.len(),
            },
            markdown,
        })
    }
}

fn structured_view(
    bytes: &[u8],
    interpretation: Option<SourceInterpretation>,
    media: Option<&str>,
) -> StructuredView {
    use SourceInterpretation as Kind;
    if interpretation == Some(Kind::XmlData) {
        return StructuredView::XmlData(vibe_specdoc::xml_data::inspect_xml_data(bytes).inspection);
    }
    if matches!(interpretation, None | Some(Kind::Opaque)) {
        return StructuredView::NotRequested;
    }
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(_) => {
            return StructuredView::Unavailable("selected structured source is not UTF-8".into());
        }
    };
    match (interpretation, media) {
        (Some(Kind::Commonmark), Some("text/markdown")) => {
            StructuredView::CommonMark(vibe_specdoc::commonmark::CommonMarkDoc::parse(text))
        }
        (Some(Kind::VibeSpec), Some("text/markdown")) => {
            vibe_view(vibe_specdoc::from_markdown(text), Vocabulary::Spec)
        }
        (Some(Kind::VibeSpec), Some("application/xml")) => {
            vibe_view(vibe_specdoc::from_xml(text), Vocabulary::Spec)
        }
        (Some(Kind::VibeDoc), Some("text/markdown")) => StructuredView::Unavailable(
            "documentation Markdown is a one-way projection; captured XML pivot required".into(),
        ),
        (Some(Kind::VibeDoc), Some("application/xml")) => vibe_view(
            vibe_specdoc::from_xml_with(text, Vocabulary::Doc),
            Vocabulary::Doc,
        ),
        _ => StructuredView::Unavailable(
            "selected descriptor has no compatible structured reader".into(),
        ),
    }
}

fn vibe_view(parsed: vibe_specdoc::Result<SpecDoc>, vocabulary: Vocabulary) -> StructuredView {
    match parsed {
        Ok(document) => StructuredView::Vibe {
            document,
            vocabulary,
        },
        Err(error) => StructuredView::Unavailable(error.to_string()),
    }
}

#[cfg(test)]
mod factory_cases;
#[cfg(test)]
mod m_01_d_1_cases;
