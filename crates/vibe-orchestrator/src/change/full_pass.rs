//! Selected full-pass preparation. Execution carriage is deliberately absent.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-FULL-PASS-PLAN");
use super::CapturedChangeView;
use crate::document_analysis::ResourceDisposition;
use vibe_core::manifest::change::{NormativeInput, SelectedFullPass};

#[derive(Debug, thiserror::Error)]
pub enum FullPassPreparationError {
    #[error("change analysis is unavailable")]
    AnalysisUnavailable,
    #[error("saved recaptured inputs are required")]
    SaveRequired,
    #[error("retained change input is unavailable: {0}")]
    MissingInput(String),
    #[error(transparent)]
    Declaration(#[from] vibe_core::manifest::change::ChangeError),
}
/// Immutable template, not a lifecycle lease, machine grant or writer.
#[derive(Debug, Clone)]
pub struct ChangeFullPassPlan {
    capture: String,
    selected: SelectedFullPass,
}
impl ChangeFullPassPlan {
    pub fn capture_id(&self) -> &str {
        &self.capture
    }
    pub fn selection(&self) -> &SelectedFullPass {
        &self.selected
    }
    pub fn execution_carriage(&self) -> &'static str {
        "unavailable:m16-change-full-pass"
    }
    pub fn unbound_inputs(&self) -> [&'static str; 3] {
        ["resolution", "declaration", "target"]
    }
}
/// Parse only the same held inputs as the captured change analysis. This does
/// not read live disk, reserve a run, acknowledge an editor or dispatch work.
pub fn plan_change_full_pass(
    view: &CapturedChangeView<'_>,
    work: &str,
) -> Result<ChangeFullPassPlan, FullPassPreparationError> {
    let parsed = view
        .analysis()
        .map_err(|_| FullPassPreparationError::AnalysisUnavailable)?;
    for id in &view.subject {
        let row = view
            .capture
            .resource(*id)
            .ok_or(FullPassPreparationError::AnalysisUnavailable)?;
        let disk = row
            .disk_observation()
            .ok_or(FullPassPreparationError::SaveRequired)?;
        if row.raw() != Some(disk.bytes.as_slice())
            || !matches!(
                row.disposition(),
                ResourceDisposition::Disk | ResourceDisposition::Replaced
            )
        {
            return Err(FullPassPreparationError::SaveRequired);
        }
    }
    let mut normative = vec![];
    for doc in parsed.manifest.documents().iter().filter(|d| d.input) {
        let path = view.directory().join(&doc.path);
        let (_, row) = view
            .capture
            .resources()
            .find(|(_, r)| std::path::Path::new(r.path()) == path)
            .ok_or_else(|| FullPassPreparationError::MissingInput(doc.path.clone()))?;
        let raw = row
            .raw()
            .ok_or_else(|| FullPassPreparationError::MissingInput(doc.path.clone()))?;
        if doc.normative {
            normative.push(NormativeInput {
                path: &doc.path,
                bytes: raw,
            });
        }
    }
    let selected =
        SelectedFullPass::select(&parsed.manifest, &normative, &parsed.identities.graph, work)?;
    Ok(ChangeFullPassPlan {
        capture: view.capture_id().into(),
        selected,
    })
}
