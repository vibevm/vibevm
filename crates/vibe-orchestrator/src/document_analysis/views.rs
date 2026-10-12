//! Immutable filtered pages/chunks on retained captures, not live trees.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#DOCUMENT-VIEW-INVALIDATION");
use super::*;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewFilter {
    Raw,
    MarkdownProjection,
    Outline { prefix: Vec<String> },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewCursor {
    identity: String,
    offset: usize,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewError {
    Location(LocationError),
    StructuredUnavailable,
    ForeignCursor,
    InvalidPageSize,
}
#[derive(Debug, Clone)]
enum ViewPayload {
    Bytes(Arc<[u8]>),
    Nodes(Vec<vibe_specdoc::selection::OutlineNode>),
}
#[derive(Debug, Clone)]
pub struct CapturedView {
    identity: String,
    revision: SourceRevision,
    basis: LocationBasis,
    filter: ViewFilter,
    payload: ViewPayload,
}
#[derive(Debug)]
pub struct ViewPage {
    pub identity: String,
    pub revision: SourceRevision,
    pub basis: LocationBasis,
    pub filter: ViewFilter,
    pub offset: usize,
    pub total: usize,
    pub bytes: Vec<u8>,
    pub nodes: Vec<vibe_specdoc::selection::OutlineNode>,
    pub next: Option<ViewCursor>,
}
impl CapturedView {
    pub fn identity(&self) -> &str {
        &self.identity
    }
    pub fn revision(&self) -> &SourceRevision {
        &self.revision
    }
    pub fn page(&self, cursor: Option<&ViewCursor>, limit: usize) -> Result<ViewPage, ViewError> {
        if limit == 0 || limit > 65536 {
            return Err(ViewError::InvalidPageSize);
        };
        let offset = match cursor {
            Some(c) if c.identity == self.identity => c.offset,
            Some(_) => return Err(ViewError::ForeignCursor),
            None => 0,
        };
        let total = match &self.payload {
            ViewPayload::Bytes(b) => b.len(),
            ViewPayload::Nodes(n) => n.len(),
        };
        if offset > total {
            return Err(ViewError::ForeignCursor);
        };
        let end = offset.saturating_add(limit).min(total);
        let (bytes, nodes) = match &self.payload {
            ViewPayload::Bytes(b) => (b[offset..end].to_vec(), vec![]),
            ViewPayload::Nodes(n) => (vec![], n[offset..end].to_vec()),
        };
        Ok(ViewPage {
            identity: self.identity.clone(),
            revision: self.revision.clone(),
            basis: self.basis.clone(),
            filter: self.filter.clone(),
            offset,
            total,
            bytes,
            nodes,
            next: (end < total).then(|| ViewCursor {
                identity: self.identity.clone(),
                offset: end,
            }),
        })
    }
}
impl AnalysisCapture {
    pub fn view(&self, id: ResourceId, filter: ViewFilter) -> Result<CapturedView, ViewError> {
        let text = match &filter {
            ViewFilter::Raw => self.raw_text(id),
            _ => self.projected_text(id),
        }
        .map_err(ViewError::Location)?;
        let payload = match &filter {
            ViewFilter::Outline { prefix } => {
                let outline = self
                    .outline(id)
                    .map_err(|_| ViewError::StructuredUnavailable)?;
                ViewPayload::Nodes(
                    outline
                        .nodes
                        .into_iter()
                        .filter(|n| n.path.starts_with(prefix))
                        .collect(),
                )
            }
            _ => ViewPayload::Bytes(Arc::from(text.raw())),
        };
        let identity = super::digest(
            format!(
                "document-view/1:{:?}:{:?}:{:?}",
                text.revision(),
                text.basis(),
                filter
            )
            .as_bytes(),
        );
        Ok(CapturedView {
            identity,
            revision: text.revision().clone(),
            basis: text.basis().clone(),
            filter,
            payload,
        })
    }
    /// Contextual diagnostics retain source revision even without a numeric
    /// parser position. Do not invent columns from flattened error prose.
    pub fn bind_diagnostic(
        &self,
        diagnostic: AnalysisDiagnostic,
    ) -> Result<RevisionDiagnostic, LocationError> {
        if diagnostic.capture != self.id {
            return Err(LocationError::RevisionMismatch);
        };
        let revision = self.source_revision(diagnostic.resource);
        Ok(RevisionDiagnostic {
            diagnostic,
            revision,
            numeric_location_unavailable: "owner did not supply an exact offset",
        })
    }
}
#[derive(Debug)]
pub struct RevisionDiagnostic {
    pub diagnostic: AnalysisDiagnostic,
    pub revision: Result<SourceRevision, LocationError>,
    pub numeric_location_unavailable: &'static str,
}
