//! Finishing a slot run `vibe reinstall` still owes — as a VALUE.
//!
//! `vibe reinstall --force` is the mode that re-fetches and therefore reaches
//! changed slot callbacks, so it is the mode that can park. The handoff names
//! the plain `vibe reinstall` base verb, which means the plain verb must
//! actually be able to finish it: rebuild the locked world, select exactly the
//! persisted targets, and run the post-install continuation.
//!
//! Nothing here selects an identity, re-reads a manifest, emits a document or
//! disposes of a plan preview. It used to do the first two — a second
//! `run_identity` inside the helper, and a second `Manifest::read` of the file
//! the workspace already carries — which meant a resume could allocate against
//! a different identity than the command that invoked it. The outer session's
//! metadata and prepared tree are handed in instead, and the outcome travels
//! back for the one funnel to render.
//!
//! ## Why a failure is CARRIED here and not simply propagated
//!
//! A resume runs its own slot rows in its own lifecycle. When one of them
//! fails, the rows already produced — the earlier successful contribution, the
//! delegated row it satisfied — exist nowhere else: the forced path's own
//! lifecycle is a different run and never saw them, and the plain path has no
//! lifecycle at all. So the typed [`ResumeOutcome::Failed`] is turned into a
//! measured Reinstall draft right here, with the historical emission policy
//! (silent while tracing is off) and the ORIGINAL error object.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#REF-AGENT-RESUME");

use anyhow::Result;
use vibe_install::{InstallProgress, InstallSlotLifecycle, SlotLifecycleReport};
use vibe_lifecycle::{LifecycleLease, RunMetadata};
use vibe_workspace::Workspace;

use crate::commands::compile_trace::{RegisteredReportDraft, carry_measured};
use crate::commands::install::{
    InstallDisposition, MeasuredFailure, Measurement, ResumeOutcome, ResumeRequest,
    resume_slot_continuation,
};
use crate::output;

use super::draft::{ReinstallDraft, ReinstallIdentity};

/// What servicing a continuation produced.
#[derive(Debug)]
pub(super) struct Serviced {
    pub(super) progress: InstallProgress,
    pub(super) rows: Vec<SlotLifecycleReport>,
    /// `Some` when the resumed run parked AGAIN — a later declared row taking
    /// the handoff the satisfied one released.
    pub(super) parked: Option<vibe_lifecycle::Delegation>,
}

pub(super) struct Request<'a> {
    pub(super) identity: &'a ReinstallIdentity,
    /// The tree this continuation runs over. Its selected node's manifest is
    /// DERIVED from it by the resume itself — reinstall's operational host —
    /// never passed beside it, so the two cannot come from different moments.
    pub(super) workspace: &'a Workspace,
    pub(super) metadata: &'a RunMetadata,
    /// The command's mutation lease: the resumed slot run is rebuilt on the
    /// caller's ONE acquisition — a resume never reacquires.
    pub(super) lease: &'a std::sync::Arc<LifecycleLease>,
    /// What the CALLER's own pass did. A forced reinstall hands in the run's
    /// real completed progress; a plain one has nothing of its own yet.
    pub(super) progress: InstallProgress,
    /// The command's ONE agent backend: a continuation finishes a run somebody
    /// else began and must serve its agent rows from the same seam.
    pub(super) agent: std::sync::Arc<dyn vibe_lifecycle::AgentBackend>,
}

/// A run whose rows this command's CURRENT pass owns.
///
/// Two operations, and no more, because two are all the forced path needs and
/// a wider surface would let a caller read the lifecycle for something else:
/// whether the run still owes slot work, and the ONE destructive take of its
/// rows.
///
/// It exists so the forced path can be driven by a fake. The previous shape
/// passed `Option<&InstallSlotLifecycle>` and selected `Some(lifecycle)` at one
/// call site — a selection no unit test could reach, so mutating it to `None`
/// silently deleted forced ownership and stayed green.
pub(super) trait CurrentRows {
    /// Whether the live run still owes a continuation.
    fn owes_slot_work(&self) -> bool;

    /// The ONE destructive take of this pass's rows.
    fn take_rows(&self) -> Vec<SlotLifecycleReport>;
}

impl CurrentRows for InstallSlotLifecycle {
    fn owes_slot_work(&self) -> bool {
        Self::owes_slot_work(self)
    }

    fn take_rows(&self) -> Vec<SlotLifecycleReport> {
        self.take_reports().unwrap_or_default()
    }
}

/// Service a continuation only when the live run still owes one.
///
/// The gate matters on the forced path: an apply can finish without ever
/// revisiting a live slot-scoped park, because an unchanged slot raises no
/// payload event and the post-install plan is then empty. Clearing the
/// continuation without servicing it in that state forgets a target the run
/// promised to finish.
///
/// The FORCED selector, and the only one: it takes the live run by value-of-
/// trait and hands its lazy take straight into the ownership seam, so there is
/// no `Some`/`None` choice left for a mutation to flip.
pub(super) fn service_if_owed(
    ctx: &output::Context,
    current: &impl CurrentRows,
    request: Request<'_>,
) -> Result<Option<Serviced>> {
    let observer = crate::commands::install::CliInstallObserver::new(ctx, None);
    let agent = request.agent.clone();
    forced_continuation(current, request.identity, || {
        resume_slot_continuation(&observer, &agent, resume_request(request))
    })
}

/// The forced path's gate and take, over anything that can answer the two
/// questions — so the reds drive it with a counting fake.
fn forced_continuation(
    current: &impl CurrentRows,
    identity: &ReinstallIdentity,
    resume: impl FnOnce() -> Result<ResumeOutcome>,
) -> Result<Option<Serviced>> {
    if !current.owes_slot_work() {
        // Nothing owed: the resume never runs, and the rows stay with the run
        // for the caller's own fallback.
        return Ok(None);
    }
    owned_continuation(identity, resume, || current.take_rows())
}

/// `Ok(None)` when nothing is owed, so the ordinary path proceeds untouched.
///
/// Plain reinstall has no current pass at all: it regenerates boot and nothing
/// else, so its prefix is EXPLICITLY empty rather than an absent option.
pub(super) fn service(ctx: &output::Context, request: Request<'_>) -> Result<Option<Serviced>> {
    let identity = request.identity;
    let observer = crate::commands::install::CliInstallObserver::new(ctx, None);
    let agent = request.agent.clone();
    owned_continuation(
        identity,
        || resume_slot_continuation(&observer, &agent, resume_request(request)),
        Vec::new,
    )
}

/// The one `ResumeRequest` both wrappers build, from the values handed in.
fn resume_request(request: Request<'_>) -> ResumeRequest<'_> {
    ResumeRequest {
        project_root: &request.workspace.root,
        workspace: request.workspace,
        metadata: request.metadata,
        lease: request.lease,
        disposition: InstallDisposition::Fresh,
        progress: request.progress,
        // A reinstall report carries no resolved count; this path only
        // finishes a run that already resolved.
        packages_resolved: 0,
    }
}

/// The ownership seam: resume, then match, then take — in that order, with both
/// dependencies injected so the order is provable.
///
/// The take is DESTRUCTIVE and therefore lazy. It runs on the two arms that own
/// a value and nowhere else: `Nothing` and an ordinary `Err` still owe their
/// rows to the caller's own failure fallback.
fn owned_continuation(
    identity: &ReinstallIdentity,
    resume: impl FnOnce() -> Result<ResumeOutcome>,
    take_current: impl FnOnce() -> Vec<SlotLifecycleReport>,
) -> Result<Option<Serviced>> {
    let resumed = match crate::commands::install::own_resume(resume, take_current)? {
        ResumeOutcome::Nothing => return Ok(None),
        ResumeOutcome::Completed(resumed) => *resumed,
        // Measured: the rows the resume really ran, behind whatever the current
        // pass ran, with the durable progress it inherited, in THIS command's
        // registered root — and the original error object, unformatted, on its
        // way to the exit code.
        ResumeOutcome::Failed(failure) => return Err(carry_failure(identity, failure)),
    };
    // The handoff was validated by `own_resume`, BEFORE it took the current
    // rows — see that seam for why the order is load-bearing. There is nothing
    // fallible left on this arm.
    let run = resumed.run;
    Ok(Some(Serviced {
        progress: run.progress,
        rows: run.slot_reports,
        parked: run.parked,
    }))
}

/// Turn a measured resume failure into this command's registered root.
///
/// Pure, and separate from the seam above, so the three things that have to be
/// right can be proved without a live lifecycle: the rows survive IN ORDER, the
/// draft is the reinstall family with the selected identity, and the emission
/// policy is the historical one — silence while tracing is off, observable the
/// moment tracing is requested.
fn carry_failure(identity: &ReinstallIdentity, failure: MeasuredFailure) -> anyhow::Error {
    let (progress, reports) = match failure.evidence {
        Measurement::Slot {
            progress, reports, ..
        }
        | Measurement::InstallBarrier {
            progress, reports, ..
        } => (*progress, reports),
        // A reinstall continuation measures slot work and nothing else; a
        // lifecycle measurement here would be an internal contradiction, and
        // reporting an empty run is honest rather than inventing rows.
        Measurement::Lifecycle { .. } => (InstallProgress::default(), Vec::new()),
    };
    // `carry_measured` supplies `emit_when_trace_disabled: false`: a failure on
    // this path has never emitted a document, and adding one now would be a new
    // root on an old path. Requested tracing is what makes it observable.
    carry_measured(failure.original, || {
        RegisteredReportDraft::Reinstall(Box::new(ReinstallDraft::failed(
            identity, progress, reports,
        )))
    })
}

#[cfg(test)]
#[path = "continuation/tests.rs"]
mod tests;
