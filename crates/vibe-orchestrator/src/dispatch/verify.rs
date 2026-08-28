//! The engine-owned verify boundary: where a COMPLETE default-phase plan
//! stops and reconciles evidence before any verify contribution runs
//! (PROP-054 `##VERIFY-CURRENT-PREFIX`, `##EVIDENCE-OUTCOME-VOCABULARY`).
//!
//! ## Why the permission is a parameter and not a chain read
//!
//! The dispatcher is entered from TWO epochs, and only one of them owns a
//! complete plan. `run_phases` dispatches the whole derived chain; the
//! post-durability install callback dispatches a `[validate, install]` plan
//! while carrying the SAME `metadata.chain` — which, for `vibe verify`'s
//! prerequisite install, already names every phase through `verify`. A
//! boundary that decided from `metadata.chain` alone would therefore fire
//! inside the install callback, reconciling before build and create had run
//! and publishing a member about a prefix that did not exist yet.
//!
//! So the permission travels as `Option<Timestamp>`: `Some` is the complete
//! epoch saying "this plan IS the whole chain, and here is the surface's
//! injected verify instant", `None` is every partial or state-blind context
//! saying "not here, whatever the chain says". The clock stays injected —
//! nothing below a surface reads time.
//!
//! ## Why rank, not string order
//!
//! "Verify-or-later" is a position in the REQUESTED chain, not a property of
//! a phase spelling: `package` is later than `verify` in one chain and absent
//! from another, and a lexical comparison would call `test` later than
//! `package`. The rank is read from the chain the run was asked for, once.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#VERIFY-CURRENT-PREFIX");

use std::sync::Arc;

use anyhow::Result;
use specmark::spec;
use vibe_lifecycle::{AgentBackend, LifecycleRun, Phase, RunMetadata};
use vibe_wire::generated::lifecycle_report::LifecycleContributionReport;
use vibe_wire::generated::shared::{EvidenceStatus, Timestamp, VerificationEvidence};

use crate::RitualPlan;
use crate::failure::{MeasuredFailure, Measurement, carry};

use super::{DispatchOutcome, MeasuredDispatch};

/// Where the completed evidence-bearing prefix ends, or `None` when this run
/// owes no reconciliation at all.
///
/// `None` means the requested chain never contained `verify`; a run asked for
/// `build` publishes no member and the boundary never fires. When the chain
/// DOES contain verify, the answer is always a real index — the first
/// execution whose phase ranks at or after verify, or `len` when no such row
/// exists. That `len` case is the point of `##VERIFY-CURRENT-PREFIX`: a
/// project with zero verify contributions still gets its member, and a
/// package/deploy request cannot skip the gate merely because verify is empty.
#[must_use]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#VERIFY-CURRENT-PREFIX")]
pub(super) fn boundary(plan: &RitualPlan, chain: &[String]) -> Option<usize> {
    let verify = rank(chain, Phase::Verify.as_str())?;
    let first_later = plan
        .executions
        .iter()
        .position(|execution| rank(chain, &execution.phase).is_some_and(|at| at >= verify));
    Some(first_later.unwrap_or(plan.executions.len()))
}

/// A phase's position in the chain this run was asked for. A spelling the
/// chain does not name ranks nowhere and is therefore never "verify-or-later":
/// it belongs to the prefix, which is the only reading that cannot let an
/// unrelated row close the gate early.
fn rank(chain: &[String], phase: &str) -> Option<usize> {
    chain.iter().position(|spelling| spelling == phase)
}

/// The boundary's fixed inputs, bound once per dispatch so firing it costs
/// one call and cannot pick up a different plan, backend or policy the second
/// time (the loop fires it before a verify row; the tail fires it when the
/// plan had none).
pub(super) struct Gate<'a> {
    /// The plan whose canonical prefix is the evidence-bearing universe.
    pub(super) plan: &'a RitualPlan,
    /// The command's ONE agent backend. The declaration replay resolves
    /// prompts through it and never asks it to complete anything, so the
    /// reconciliation spends nothing.
    pub(super) agent: &'a Arc<dyn AgentBackend>,
    /// The run this comparison is published under.
    pub(super) metadata: &'a RunMetadata,
    /// The OBSERVER's machine-document policy, read once at the same instant
    /// every other failure site reads it.
    pub(super) emit_machine_failure: bool,
}

impl Gate<'_> {
    /// Reconcile, attach, and stop if the comparison says so.
    ///
    /// The member is attached to the outcome AND to the accumulator before the
    /// stop decision, so a matched identity survives whatever fails later —
    /// a verify handler, a state write, a checkpoint — and a stop carries the
    /// exact comparison outward on the ordinary measured-failure carrier.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#EVIDENCE-WIRE-AND-SURFACES")]
    pub(super) fn fire(
        &self,
        run: &mut LifecycleRun,
        prefix_end: usize,
        observed_at: Timestamp,
        outcome: &mut DispatchOutcome,
        measured: &mut MeasuredDispatch,
    ) -> Result<()> {
        let member = run.reconcile_verification(
            &self.plan.executions[..prefix_end],
            self.agent.as_ref(),
            observed_at,
        )?;
        outcome.verification = Some(member.clone());
        measured.verification = Some(member.clone());
        if stops(&member.status) {
            return Err(self.stop(member, outcome.reports.clone()));
        }
        Ok(())
    }

    /// The stale/missing/unstable stop, on the carrier every other measured
    /// failure already travels on.
    ///
    /// No new error family and no new report family: an ordinary
    /// [`MeasuredFailure`] carrying the rows that really ran and the exact
    /// member the comparison produced, from which the surfaces choose their
    /// family exactly as they do for a failed handler.
    ///
    /// The emission policy is the OBSERVER's, deliberately — the same one a
    /// failed handler transition uses, and never `carry_once`'s historical
    /// silence. `vibe verify --json` must return this member
    /// (`##EVIDENCE-WIRE-AND-SURFACES`), and a silent site would emit no root
    /// at all.
    fn stop(
        &self,
        evidence: VerificationEvidence,
        rows: Vec<LifecycleContributionReport>,
    ) -> anyhow::Error {
        let status = wire_spelling(&evidence.status);
        let original = anyhow::anyhow!(
            "verification evidence is `{status}` for run `{}` (governed by \
             spec://org.vibevm.core/vibevm/common/PROP-054#EVIDENCE-OUTCOME-VOCABULARY; \
             fix: read this report's `verification` member, rerun the phases whose declared \
             inputs or artifacts changed, then invoke verify again)",
            evidence.run.run_id,
        )
        .context(format!(
            "phase `{}` stopped before any later lifecycle contribution",
            Phase::Verify.as_str()
        ));
        carry(MeasuredFailure {
            original,
            evidence: Measurement::Lifecycle {
                rows,
                stopped_phase: Phase::Verify.as_str().to_string(),
                requested: self.metadata.requested.clone(),
                chain: self.metadata.chain.clone(),
                verification: Some(Box::new(evidence)),
            },
            emit_machine_failure: self.emit_machine_failure,
        })
    }
}

/// Whether this comparison stops verify before its contributions run.
///
/// `matched` and `unavailable` continue: an honestly undeclared project keeps
/// today's empty verify posture, and VibeVM does not invent the policy that
/// project did not declare. The other three are a real mismatch and stop.
#[must_use]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#EVIDENCE-OUTCOME-VOCABULARY")]
const fn stops(status: &EvidenceStatus) -> bool {
    match status {
        EvidenceStatus::Matched | EvidenceStatus::Unavailable => false,
        EvidenceStatus::Stale | EvidenceStatus::Missing | EvidenceStatus::Unstable => true,
    }
}

/// The exact wire spelling of a status, for the one sentence that names it.
/// Read from the enum rather than a `Debug` rendering, so the operator's error
/// and the member they are told to read cannot disagree.
const fn wire_spelling(status: &EvidenceStatus) -> &'static str {
    match status {
        EvidenceStatus::Matched => "matched",
        EvidenceStatus::Missing => "missing",
        EvidenceStatus::Stale => "stale",
        EvidenceStatus::Unavailable => "unavailable",
        EvidenceStatus::Unstable => "unstable",
    }
}

#[cfg(test)]
#[path = "verify/tests.rs"]
mod tests;
