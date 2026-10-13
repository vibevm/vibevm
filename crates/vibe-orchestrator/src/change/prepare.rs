//! Saved admission composition only; no barrier acknowledgement or apply.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-SAVED-PREPARATION");
use super::*;
use crate::document_analysis::{
    ResourceDisposition,
    edit::{DocumentEditAuthority, DocumentReplacement, EditBarrier, EditError, EditPlan},
};
use std::{sync::Arc, time::Duration};
pub struct PreparedChangeEdit<'a> {
    view: &'a CapturedChangeView<'a>,
    plan: EditPlan,
}
impl std::fmt::Debug for PreparedChangeEdit<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedChangeEdit")
            .field("capture", &self.view.capture.id())
            .field("plan", &self.plan.id())
            .finish()
    }
}
pub fn prepare_change_edit<'a>(
    view: &'a CapturedChangeView<'a>,
    authority: &Arc<DocumentEditAuthority>,
    replacements: Vec<DocumentReplacement>,
) -> Result<PreparedChangeEdit<'a>, EditError> {
    if view.analysis.is_err() {
        return Err(EditError::InvalidPlan);
    };
    for id in &view.subject {
        let row = view.capture.resource(*id).ok_or(EditError::InvalidPlan)?;
        let disk = row.disk_observation().ok_or(EditError::SaveRequired)?;
        if row.raw() != Some(disk.bytes.as_slice())
            || !matches!(
                row.disposition(),
                ResourceDisposition::Disk | ResourceDisposition::Replaced
            )
        {
            return Err(EditError::SaveRequired);
        };
    }
    if replacements
        .iter()
        .any(|r| !view.writable.contains(&r.resource))
    {
        return Err(EditError::AuthorityRefused);
    };
    let plan = authority.prepare(view.capture, replacements)?;
    Ok(PreparedChangeEdit { view, plan })
}
impl PreparedChangeEdit<'_> {
    pub fn capture_id(&self) -> &str {
        self.view.capture.id()
    }
    /// Fresh capture is explicit caller work; this adapter reopens nothing.
    /// Returned incumbent barrier still needs all acknowledgements. B3 does
    /// not acknowledge/apply it or publish execution/closure output.
    pub fn admit(self, current: &AnalysisCapture, ttl: Duration) -> Result<EditBarrier, EditError> {
        if current.context() != self.view.capture.context()
            || current.client() != self.view.capture.client()
            || current.id() == self.view.capture.id()
        {
            return Err(EditError::Stale);
        };
        for id in &self.view.subject {
            let old = self.view.capture.resource(*id).ok_or(EditError::Stale)?;
            let new = current.resource(*id).ok_or(EditError::Stale)?;
            if old.disk_observation() != new.disk_observation()
                || old.buffer_version() != new.buffer_version()
                || old.raw() != new.raw()
                || old.disposition() != new.disposition()
            {
                return Err(EditError::Stale);
            };
        }
        self.plan.barrier(ttl)
    }
}
