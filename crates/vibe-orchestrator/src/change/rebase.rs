//! Reviewable origin proposal composed with existing saved-state admission.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-EXPLICIT-REBASE");
use super::*;
use crate::document_analysis::edit::{
    DocumentEditAuthority, DocumentReplacement, EditBarrier, EditError,
};
use std::{sync::Arc, time::Duration};
use vibe_core::manifest::change::{ChangeOriginRebasePlan, IntentIdentity, plan_origin_rebase};
use vibe_requirements::change::{
    AcceptedOrigin, InitialOriginBinding, OriginRebaseReview, OriginUnavailable,
    prepare_initial_origin_binding, review_origin_rebase,
};
use vibe_workspace::source_query::RetainedOriginCapture;
#[derive(Debug)]
pub enum ChangeRebaseError {
    Origin(OriginUnavailable),
    Declaration(vibe_core::manifest::change::ChangeError),
    InvalidCapturedChange,
    Identity(SavedChangeError),
    Edit(EditError),
}
#[derive(Debug)]
pub struct PreparedOriginRebase<'a> {
    review: OriginRebaseReview,
    proposal: ChangeOriginRebasePlan,
    new_intent: IntentIdentity,
    prepared: PreparedChangeEdit<'a>,
}
impl PreparedOriginRebase<'_> {
    pub fn review(&self) -> &OriginRebaseReview {
        &self.review
    }
    pub fn proposal(&self) -> &ChangeOriginRebasePlan {
        &self.proposal
    }
    pub fn new_intent(&self) -> &IntentIdentity {
        &self.new_intent
    }
    pub fn capture_id(&self) -> &str {
        self.prepared.capture_id()
    }
    pub fn admit(self, current: &AnalysisCapture, ttl: Duration) -> Result<EditBarrier, EditError> {
        self.prepared.admit(current, ttl)
    }
}
/// Pure preparation only. Explicit authority, clean subject and fresh admission
/// retain the existing B3/M16 law; this owner never acknowledges or applies.
pub fn prepare_origin_rebase<'a>(
    view: &'a CapturedChangeView<'a>,
    old: &AcceptedOrigin,
    new: Arc<RetainedOriginCapture>,
    authority: &Arc<DocumentEditAuthority>,
) -> Result<PreparedOriginRebase<'a>, ChangeRebaseError> {
    let review = review_origin_rebase(old, new).map_err(ChangeRebaseError::Origin)?;
    let (proposal, new_intent, prepared) = prepare_proposal(
        view,
        old.declaration(),
        review.next_origin().declaration(),
        authority,
    )?;
    Ok(PreparedOriginRebase {
        review,
        proposal,
        new_intent,
        prepared,
    })
}
#[derive(Debug)]
pub struct PreparedInitialOriginBinding<'a> {
    binding: InitialOriginBinding,
    proposal: ChangeOriginRebasePlan,
    new_intent: IntentIdentity,
    prepared: PreparedChangeEdit<'a>,
}
impl PreparedInitialOriginBinding<'_> {
    pub fn binding(&self) -> &InitialOriginBinding {
        &self.binding
    }
    pub fn proposal(&self) -> &ChangeOriginRebasePlan {
        &self.proposal
    }
    pub fn new_intent(&self) -> &IntentIdentity {
        &self.new_intent
    }
    pub fn admit(self, current: &AnalysisCapture, ttl: Duration) -> Result<EditBarrier, EditError> {
        self.prepared.admit(current, ttl)
    }
}
pub fn prepare_initial_change_origin_binding<'a>(
    view: &'a CapturedChangeView<'a>,
    index: usize,
    retained: Option<Arc<RetainedOriginCapture>>,
    authority: &Arc<DocumentEditAuthority>,
) -> Result<PreparedInitialOriginBinding<'a>, ChangeRebaseError> {
    let analysis = view
        .analysis()
        .map_err(|_| ChangeRebaseError::InvalidCapturedChange)?;
    let origin = analysis
        .manifest
        .origins()
        .get(index)
        .ok_or(ChangeRebaseError::InvalidCapturedChange)?;
    let binding =
        prepare_initial_origin_binding(origin, retained).map_err(ChangeRebaseError::Origin)?;
    let (proposal, new_intent, prepared) = prepare_proposal(
        view,
        binding.previous(),
        binding.next_origin().declaration(),
        authority,
    )?;
    Ok(PreparedInitialOriginBinding {
        binding,
        proposal,
        new_intent,
        prepared,
    })
}
fn prepare_proposal<'a>(
    view: &'a CapturedChangeView<'a>,
    expected: &vibe_core::manifest::change::ChangeOrigin,
    next: &vibe_core::manifest::change::ChangeOrigin,
    authority: &Arc<DocumentEditAuthority>,
) -> Result<
    (
        ChangeOriginRebasePlan,
        IntentIdentity,
        PreparedChangeEdit<'a>,
    ),
    ChangeRebaseError,
> {
    let analysis = view
        .analysis()
        .map_err(|_| ChangeRebaseError::InvalidCapturedChange)?;
    let node = *view
        .subject
        .first()
        .ok_or(ChangeRebaseError::InvalidCapturedChange)?;
    let manifest = view
        .capture
        .manifest(node)
        .map_err(|_| ChangeRebaseError::InvalidCapturedChange)?;
    let directory = analysis.manifest.directory();
    let spec_root = directory
        .parent()
        .and_then(Path::parent)
        .ok_or(ChangeRebaseError::InvalidCapturedChange)?;
    let context = ChangeContext::for_node(
        &manifest,
        analysis.manifest.selected_node(),
        Some(spec_root),
    )
    .map_err(ChangeRebaseError::Declaration)?;
    let change = *view
        .writable
        .first()
        .ok_or(ChangeRebaseError::InvalidCapturedChange)?;
    let text = std::str::from_utf8(
        view.raw(change)
            .ok_or(ChangeRebaseError::InvalidCapturedChange)?,
    )
    .map_err(|_| ChangeRebaseError::InvalidCapturedChange)?;
    let proposal = plan_origin_rebase(text, &context, directory, expected, next)
        .map_err(ChangeRebaseError::Declaration)?;
    let mut inputs = vec![];
    let node_parent = Path::new(
        view.capture
            .resource(node)
            .ok_or(ChangeRebaseError::InvalidCapturedChange)?
            .path(),
    )
    .parent()
    .unwrap_or(Path::new(""));
    for doc in proposal.checked.documents().iter().filter(|d| d.input) {
        let relative = directory.join(&doc.path);
        let full = node_parent.join(&relative);
        let (_, resource) = view
            .capture
            .resources()
            .find(|(_, r)| Path::new(r.path()) == full)
            .ok_or(ChangeRebaseError::InvalidCapturedChange)?;
        inputs.push((
            relative,
            resource
                .raw()
                .ok_or(ChangeRebaseError::InvalidCapturedChange)?,
        ));
    }
    let borrowed: Vec<_> = inputs.iter().map(|(p, b)| (p.as_path(), *b)).collect();
    let checked = analyze_change_bytes(&proposal.after, &context, directory, &borrowed)
        .map_err(ChangeRebaseError::Identity)?;
    let prepared = prepare_change_edit(
        view,
        authority,
        vec![DocumentReplacement {
            resource: change,
            bytes: Arc::from(proposal.after.as_bytes()),
        }],
    )
    .map_err(ChangeRebaseError::Edit)?;
    Ok((proposal, checked.identities.intent, prepared))
}
