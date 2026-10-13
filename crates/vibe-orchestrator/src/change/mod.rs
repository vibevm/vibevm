//! Change analysis/preparation over the incumbent immutable editor capture.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-CAPTURED-ANALYSIS");
use crate::document_analysis::{AnalysisCapture, LocationBasis, ResourceId, SourceRevision};
use std::path::{Path, PathBuf};
use vibe_core::manifest::change::{ChangeContext, ChangeId, ChangeManifest};
use vibe_requirements::change::{ParsedCapturedChange, SavedChangeError, analyze_change_bytes};
mod full_pass;
pub use full_pass::{ChangeFullPassPlan, FullPassPreparationError, plan_change_full_pass};
mod prepare;
pub mod query;
mod rebase;
pub mod scaffold;
pub use prepare::{PreparedChangeEdit, prepare_change_edit};
pub use rebase::{ChangeRebaseError, PreparedOriginRebase, prepare_origin_rebase};
pub use rebase::{PreparedInitialOriginBinding, prepare_initial_change_origin_binding};
#[derive(Debug)]
pub enum ChangeViewError {
    Context,
    Encoding,
    MissingInput(String),
    Declaration(vibe_core::manifest::change::ChangeError),
    Identity(SavedChangeError),
}
#[derive(Debug)]
pub struct ChangeDiagnostic {
    pub resource: Option<ResourceId>,
    pub revision: Option<SourceRevision>,
    pub basis: LocationBasis,
    pub numeric_position_unavailable: bool,
    pub error: ChangeViewError,
}
#[derive(Debug)]
pub struct CapturedChangeView<'a> {
    capture: &'a AnalysisCapture,
    directory: PathBuf,
    raw_ids: Vec<ResourceId>,
    subject: Vec<ResourceId>,
    writable: Vec<ResourceId>,
    analysis: Result<ParsedCapturedChange, ChangeDiagnostic>,
}
impl CapturedChangeView<'_> {
    pub fn capture_id(&self) -> &str {
        self.capture.id()
    }
    pub fn analysis(&self) -> Result<&ParsedCapturedChange, &ChangeDiagnostic> {
        self.analysis.as_ref()
    }
    pub fn raw(&self, id: ResourceId) -> Option<&[u8]> {
        self.raw_ids
            .contains(&id)
            .then(|| self.capture.resource(id)?.raw())
            .flatten()
    }
    pub fn revisions(&self) -> impl Iterator<Item = SourceRevision> + '_ {
        self.raw_ids
            .iter()
            .filter_map(|id| self.capture.source_revision(*id).ok())
    }
    pub fn directory(&self) -> &Path {
        &self.directory
    }
    pub fn location_basis(&self) -> LocationBasis {
        LocationBasis::OriginalBytes
    }
}
fn diagnostic(
    capture: &AnalysisCapture,
    id: Option<ResourceId>,
    error: ChangeViewError,
) -> ChangeDiagnostic {
    ChangeDiagnostic {
        resource: id,
        revision: id.and_then(|id| capture.source_revision(id).ok()),
        basis: LocationBasis::OriginalBytes,
        numeric_position_unavailable: true,
        error,
    }
}
/// Host-composed node resource/configured relative spec root. No root/path in
/// these values grants read or write authority beyond the supplied capture.
pub fn analyze_change<'a>(
    capture: &'a AnalysisCapture,
    node: ResourceId,
    spec_root: Option<&Path>,
    id: &str,
) -> CapturedChangeView<'a> {
    let mut view = CapturedChangeView {
        capture,
        directory: PathBuf::new(),
        raw_ids: vec![node],
        subject: vec![node],
        writable: vec![],
        analysis: Err(diagnostic(capture, Some(node), ChangeViewError::Context)),
    };
    let result = (|| {
        let manifest = capture
            .manifest(node)
            .map_err(|_| diagnostic(capture, Some(node), ChangeViewError::Context))?;
        let node_path = capture
            .resource(node)
            .ok_or_else(|| diagnostic(capture, Some(node), ChangeViewError::Context))?
            .path();
        let parent = Path::new(node_path).parent().unwrap_or(Path::new(""));
        let selected = if parent.as_os_str().is_empty() {
            "."
        } else {
            parent
                .to_str()
                .ok_or_else(|| diagnostic(capture, Some(node), ChangeViewError::Context))?
        };
        let context = ChangeContext::for_node(&manifest, selected, spec_root)
            .map_err(|e| diagnostic(capture, Some(node), ChangeViewError::Declaration(e)))?;
        let id = ChangeId::parse(id)
            .map_err(|e| diagnostic(capture, None, ChangeViewError::Declaration(e)))?;
        let relative = context.directory(&id);
        view.directory = parent.join(&relative);
        view.raw_ids.extend(
            capture
                .resources()
                .filter(|(_, r)| Path::new(r.path()).starts_with(&view.directory))
                .map(|(id, _)| id),
        );
        let manifest_path = view.directory.join("change.toml");
        let (change_id, resource) = capture
            .resources()
            .find(|(_, r)| Path::new(r.path()) == manifest_path)
            .ok_or_else(|| {
                diagnostic(
                    capture,
                    None,
                    ChangeViewError::MissingInput("change.toml".into()),
                )
            })?;
        view.subject.push(change_id);
        view.writable.push(change_id);
        if resource.encoding() == crate::document_analysis::ByteEncoding::Opaque {
            return Err(diagnostic(
                capture,
                Some(change_id),
                ChangeViewError::Encoding,
            ));
        }
        let raw = resource.raw().ok_or_else(|| {
            diagnostic(
                capture,
                Some(change_id),
                ChangeViewError::MissingInput("change.toml".into()),
            )
        })?;
        let text = std::str::from_utf8(raw)
            .map_err(|_| diagnostic(capture, Some(change_id), ChangeViewError::Encoding))?;
        let parsed = ChangeManifest::parse_str(text, &context, &relative)
            .map_err(|e| diagnostic(capture, Some(change_id), ChangeViewError::Declaration(e)))?;
        let mut inputs = vec![];
        for doc in parsed.documents().iter().filter(|d| d.input) {
            let relative_path = relative.join(&doc.path);
            let full_path = parent.join(&relative_path);
            let (id, row) = capture
                .resources()
                .find(|(_, r)| Path::new(r.path()) == full_path)
                .ok_or_else(|| {
                    diagnostic(
                        capture,
                        None,
                        ChangeViewError::MissingInput(doc.path.clone()),
                    )
                })?;
            let bytes = row.raw().ok_or_else(|| {
                diagnostic(
                    capture,
                    Some(id),
                    ChangeViewError::MissingInput(doc.path.clone()),
                )
            })?;
            view.subject.push(id);
            view.writable.push(id);
            inputs.push((relative_path, bytes));
        }
        let borrowed: Vec<_> = inputs.iter().map(|(p, b)| (p.as_path(), *b)).collect();
        analyze_change_bytes(text, &context, &relative, &borrowed)
            .map_err(|e| diagnostic(capture, Some(change_id), ChangeViewError::Identity(e)))
    })();
    view.analysis = result;
    view
}
#[cfg(test)]
mod m_02_b_3_cases;
#[cfg(test)]
mod m_02_c_1_cases;

#[cfg(test)]
mod m_03_a_2_cases;

#[cfg(test)]
mod full_pass_fixture;
