//! Positions retain source and projection identities; no inferred XML map.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#DOCUMENT-EDITOR-POSITIONS");
use super::*;
use vibe_spec::editor_positions;
pub use vibe_spec::editor_positions::{
    EditorEncoding, EditorPosition, PositionError, negotiate_encoding,
};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRevision {
    pub resource: ResourceId,
    pub capture: String,
    pub buffer_version: Option<u64>,
    pub source_encoding: ByteEncoding,
    pub source_digest: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocationBasis {
    OriginalBytes,
    MarkdownProjection {
        recipe: &'static str,
        digest: String,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorLocation {
    pub revision: SourceRevision,
    pub basis: LocationBasis,
    pub encoding: EditorEncoding,
    pub offset: usize,
    pub position: EditorPosition,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocationError {
    UnknownResource,
    MissingBytes,
    EncodingUnavailable,
    ProjectionUnavailable,
    PositionUnavailable,
    ReverseMapUnavailable,
    RevisionMismatch,
    Position(PositionError),
}
#[derive(Debug)]
pub struct CapturedText {
    revision: SourceRevision,
    basis: LocationBasis,
    bytes: Arc<[u8]>,
    character_encoding_available: bool,
}
impl CapturedText {
    pub fn revision(&self) -> &SourceRevision {
        &self.revision
    }
    pub fn basis(&self) -> &LocationBasis {
        &self.basis
    }
    pub fn raw(&self) -> &[u8] {
        &self.bytes
    }
    pub fn location(
        &self,
        offset: usize,
        encoding: EditorEncoding,
    ) -> Result<EditorLocation, LocationError> {
        if !self.character_encoding_available {
            return Err(LocationError::EncodingUnavailable);
        }
        let position = editor_positions::byte_to_position(&self.bytes, offset, encoding)
            .map_err(LocationError::Position)?;
        Ok(EditorLocation {
            revision: self.revision.clone(),
            basis: self.basis.clone(),
            encoding,
            offset,
            position,
        })
    }
    pub fn offset(
        &self,
        revision: &SourceRevision,
        basis: &LocationBasis,
        position: EditorPosition,
        encoding: EditorEncoding,
    ) -> Result<usize, LocationError> {
        if revision != &self.revision || basis != &self.basis {
            return Err(LocationError::RevisionMismatch);
        }
        if !self.character_encoding_available {
            return Err(LocationError::EncodingUnavailable);
        }
        editor_positions::position_to_byte(&self.bytes, position, encoding)
            .map_err(LocationError::Position)
    }
    pub fn original_offset(&self, location: &EditorLocation) -> Result<usize, LocationError> {
        if location.revision != self.revision || location.basis != self.basis {
            return Err(LocationError::RevisionMismatch);
        }
        // Verify the complete location rather than trusting a caller offset.
        if self.location(location.offset, location.encoding)? != *location {
            return Err(LocationError::RevisionMismatch);
        }
        match self.basis {
            LocationBasis::OriginalBytes => Ok(location.offset),
            _ => Err(LocationError::ReverseMapUnavailable),
        }
    }
}
impl AnalysisCapture {
    pub fn source_revision(&self, id: ResourceId) -> Result<SourceRevision, LocationError> {
        let r = self.resource(id).ok_or(LocationError::UnknownResource)?;
        Ok(SourceRevision {
            resource: id,
            capture: self.id.clone(),
            buffer_version: r.buffer_version(),
            source_encoding: r.encoding(),
            source_digest: r.content_digest().ok_or(LocationError::MissingBytes)?,
        })
    }
    pub fn raw_text(&self, id: ResourceId) -> Result<CapturedText, LocationError> {
        let revision = self.source_revision(id)?;
        let r = self.resource(id).unwrap();
        Ok(CapturedText {
            revision,
            basis: LocationBasis::OriginalBytes,
            bytes: r.bytes.clone().unwrap(),
            character_encoding_available: r.encoding() == ByteEncoding::Utf8,
        })
    }
    /// Canonical whole-document projection used by shared selection/outline.
    /// Retaining the projection does not create a reverse source map.
    pub fn projected_text(&self, id: ResourceId) -> Result<CapturedText, LocationError> {
        let revision = self.source_revision(id)?;
        let preview = self
            .inspect(id)
            .map_err(|_| LocationError::ProjectionUnavailable)?;
        let Some(vibe_specdoc::captured::StructuredView::Vibe { ir, .. }) = preview.inspection.view
        else {
            return Err(LocationError::ProjectionUnavailable);
        };
        let bytes: Arc<[u8]> = Arc::from(vibe_specdoc::to_markdown(&ir).into_bytes());
        Ok(CapturedText {
            revision,
            basis: LocationBasis::MarkdownProjection {
                recipe: "vibe-specdoc-to-markdown/1",
                digest: super::digest(&bytes),
            },
            bytes,
            character_encoding_available: true,
        })
    }
}

#[derive(Debug)]
pub struct ParserLocationDiagnostic {
    pub revision: SourceRevision,
    pub basis: LocationBasis,
    pub message: String,
    /// The incumbent parser supplies a one-based line, not an exact column.
    /// This is explicitly a line-start locator, never a guessed token span.
    pub line_start: Result<EditorLocation, LocationError>,
}
impl AnalysisCapture {
    pub fn parser_diagnostics(
        &self,
        id: ResourceId,
        encoding: EditorEncoding,
    ) -> Result<Vec<ParserLocationDiagnostic>, LocationError> {
        use vibe_specdoc::{MarkdownConversionFailure, captured::InspectionDiagnostic};
        let text = self.raw_text(id)?;
        let preview = self
            .inspect(id)
            .map_err(|_| LocationError::ProjectionUnavailable)?;
        let mut rows = vec![];
        for diagnostic in preview.inspection.diagnostics {
            let lines: Vec<(usize, String)> = match diagnostic {
                InspectionDiagnostic::InvalidUtf8 => vec![(0, "invalid UTF8".into())],
                InspectionDiagnostic::Xml(e) => vec![(e.line, e.message)],
                InspectionDiagnostic::Markdown(MarkdownConversionFailure::PivotAdapter(e)) => {
                    vec![(e.line, e.message)]
                }
                InspectionDiagnostic::Markdown(MarkdownConversionFailure::Issues(issues)) => {
                    issues.into_iter().map(|i| (i.line, i.message)).collect()
                }
            };
            for (line, message) in lines {
                let line_start = if line == 0 {
                    Err(LocationError::PositionUnavailable)
                } else {
                    text.offset(
                        text.revision(),
                        text.basis(),
                        EditorPosition {
                            line: line - 1,
                            character: 0,
                        },
                        encoding,
                    )
                    .and_then(|offset| text.location(offset, encoding))
                };
                rows.push(ParserLocationDiagnostic {
                    revision: text.revision().clone(),
                    basis: text.basis().clone(),
                    message,
                    line_start,
                });
            }
        }
        Ok(rows)
    }
}
