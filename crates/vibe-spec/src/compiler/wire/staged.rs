//! Gates 12, 13 and 14 — the STAGED half of the schedule.
//!
//! These three verdicts are replays, not shape checks: the absorption witness
//! is the exact live projection of an address-bound plan, and the link
//! witness is the exact `derive_result` replay. Re-deriving either from the
//! wire would be a second copy of a production algorithm — the very drift the
//! phase schedule exists to prevent. So the carrier is CONSTRUCTED first, and
//! each gate then calls the production law and maps its refusal to the gate
//! that owns it.
//!
//! Construction is deliberately staged BETWEEN phase 11 and gate 12. That is
//! safe precisely because construction can no longer report 12, 13 or 14
//! itself: those grammars live here and in the ordered preflight, nowhere
//! else. Each gate still runs strictly in order — the cheap wire-level
//! clauses of a gate run before its domain replay, and every clause of gate
//! 12 runs before any clause of gate 13.

use super::super::absorb::{validate_applied_absorption, validate_planned_absorption};
use super::super::assemble::validate_shape;
use super::super::ir::{AbsorptionState, ClosureIr, LaneIr, LinkState, QualificationState};
use super::super::link::validate_linked;
use super::super::pass::AnyIr;
use super::bounded::debug;
use super::{G_ABSORPTION_WITNESS, G_LINK_WITNESS_LANE, IrWireError, gate, wire};

/// Run gates 12, 13 and 14 over one constructed carrier, in that order.
pub(super) fn run(value: &wire::Ir, ir: &AnyIr) -> Result<(), IrWireError> {
    absorption(value, ir)?; // 12
    link(value, ir)?; // 13
    pass_snapshot(value)?; // 14
    Ok(())
}

/// Render a production replay error through derived `Debug`, never `Display`.
/// Several production Display implementations join attacker-controlled lists
/// before the formatter sees them; the bounded sink cannot undo that
/// allocation. Derived Debug writes fields incrementally into the cap.
fn refuse(label: &'static str, site: &str, source: impl std::fmt::Debug) -> IrWireError {
    gate(label, format!("{site}: {}", debug(source)))
}

// ── 12. the absorption witness ──────────────────────────────────────────────

/// The COMPLETE law: the wire-level count/kind clauses that make construction
/// meaningful, then the production validator — state alignment and plan mode,
/// consumed snapshots, exact meta/seed/seed-address identity, the seed node's
/// pinless spec document, and the occurrence projection (planned occurrences
/// against the live pre-absorb order; applied live order against the plan
/// filtered by `absorbed == false`).
fn absorption(value: &wire::Ir, ir: &AnyIr) -> Result<(), IrWireError> {
    if let Some(wired) = closure_of(value) {
        wire_alignment(wired)?;
    }
    let AnyIr::Closure(closure) = ir else {
        return Ok(());
    };
    match (&closure.qualification, &closure.absorption) {
        (QualificationState::Pending(_), AbsorptionState::Unplanned) => Ok(()),
        (QualificationState::Pending(_), AbsorptionState::Planned(_)) => Err(gate(
            G_ABSORPTION_WITNESS,
            "pending qualification cannot carry a planned absorption witness",
        )),
        (QualificationState::Pending(_), AbsorptionState::Applied(_)) => Err(gate(
            G_ABSORPTION_WITNESS,
            "pending qualification cannot carry an applied absorption witness",
        )),
        (QualificationState::Applied(_), AbsorptionState::Unplanned) => Err(gate(
            G_ABSORPTION_WITNESS,
            "applied qualification requires a planned or applied absorption witness",
        )),
        (QualificationState::Applied(_), AbsorptionState::Planned(plan)) => {
            if closure.pending_sources.is_some() || closure.pending_embeds.is_some() {
                return Err(gate(
                    G_ABSORPTION_WITNESS,
                    "a planned absorption carries no pending source/embed snapshot",
                ));
            }
            validate_planned_absorption(plan, closure)
                .map_err(|source| refuse(G_ABSORPTION_WITNESS, "planned absorption", source))
        }
        (QualificationState::Applied(_), AbsorptionState::Applied(_)) => {
            validate_applied_absorption(closure)
                .map_err(|source| refuse(G_ABSORPTION_WITNESS, "applied absorption", source))
        }
    }
}

/// Gate 12's wire-level clauses: any established absorption plan carries no pending
/// snapshot, and the plan aligns one witness per contribution, in order, by
/// kind. These need no construction, so they are what a malformed carrier
/// meets first.
fn wire_alignment(value: &wire::ClosureIr) -> Result<(), IrWireError> {
    if !matches!(value.absorption, wire::AbsorptionState::Unplanned(_))
        && (value.pending_sources.is_some() || value.pending_embeds.is_some())
    {
        return Err(gate(
            G_ABSORPTION_WITNESS,
            "a planned/applied absorption carries no pending source/embed snapshot",
        ));
    }
    let Some(plan) = plan_of(value) else {
        return Ok(());
    };
    if plan.contributions.len() != value.contributions.len() {
        return Err(gate(
            G_ABSORPTION_WITNESS,
            format!(
                "the absorption plan carries {} witnesses for {} contributions",
                plan.contributions.len(),
                value.contributions.len()
            ),
        ));
    }
    for (index, (witness, contribution)) in plan
        .contributions
        .iter()
        .zip(&value.contributions)
        .enumerate()
    {
        let witness_kind = match witness {
            wire::ContributionAbsorption::Normal(_) => "normal",
            wire::ContributionAbsorption::Simple(_) => "simple",
            wire::ContributionAbsorption::Elided(_) => "elided",
            wire::ContributionAbsorption::Hoisted(_) => "hoisted",
        };
        let kind = contribution_kind(contribution);
        if witness_kind != kind {
            return Err(gate(
                G_ABSORPTION_WITNESS,
                format!("plan witness {index} is `{witness_kind}` for a `{kind}` contribution"),
            ));
        }
    }
    Ok(())
}

// ── 13. the link witness, and a lane's bracketing ───────────────────────────

/// The COMPLETE law: the wire-level count/kind clauses, then the production
/// replay — mode, input digest, every contribution witness field in order and
/// the whole ordered occurrence stream that `derive_result` owns. For a lane
/// carrier it is the intrinsic open/node/[forced-newline]/close walk with its
/// continuous fence history, which is the same clause at the next level.
fn link(value: &wire::Ir, ir: &AnyIr) -> Result<(), IrWireError> {
    if let Some(wired) = closure_of(value) {
        wire_link_alignment(wired)?;
    }
    match ir {
        AnyIr::Closure(closure) => link_replay(closure),
        AnyIr::Lane(lane) => lane_bracketing(lane),
        _ => Ok(()),
    }
}

fn link_replay(closure: &ClosureIr) -> Result<(), IrWireError> {
    if matches!(closure.link, LinkState::Unlinked) {
        return Ok(());
    }
    validate_linked(closure).map_err(|source| refuse(G_LINK_WITNESS_LANE, "linked replay", source))
}

fn lane_bracketing(lane: &LaneIr) -> Result<(), IrWireError> {
    validate_shape(lane)
        .map(|_| ())
        .map_err(|source| refuse(G_LINK_WITNESS_LANE, "lane bracketing", source))
}

fn wire_link_alignment(value: &wire::ClosureIr) -> Result<(), IrWireError> {
    let wire::LinkState::Linked(arm) = &value.link else {
        return Ok(());
    };
    if arm.result.contributions.len() != value.contributions.len() {
        return Err(gate(
            G_LINK_WITNESS_LANE,
            format!(
                "the link result carries {} witnesses for {} contributions",
                arm.result.contributions.len(),
                value.contributions.len()
            ),
        ));
    }
    for (index, (witness, contribution)) in arm
        .result
        .contributions
        .iter()
        .zip(&value.contributions)
        .enumerate()
    {
        let witness_kind = match witness {
            wire::LinkContributionWitness::Normal(_) => "normal",
            wire::LinkContributionWitness::Simple(_) => "simple",
            wire::LinkContributionWitness::Elided(_) => "elided",
            wire::LinkContributionWitness::Hoisted(_) => "hoisted",
        };
        let kind = contribution_kind(contribution);
        if witness_kind != kind {
            return Err(gate(
                G_LINK_WITNESS_LANE,
                format!("link witness {index} is `{witness_kind}` for a `{kind}` contribution"),
            ));
        }
    }
    Ok(())
}

// ── 14. PASS/SNAPSHOT ───────────────────────────────────────────────────────

/// Each pass mints its edge kind and clears its own snapshot in one run, so an
/// edge kind and its own pending snapshot never coexist.
fn pass_snapshot(value: &wire::Ir) -> Result<(), IrWireError> {
    let Some(value) = closure_of(value) else {
        return Ok(());
    };
    let has = |kind: wire::ClosureEdgeKind| {
        value
            .edges
            .iter()
            .any(|edge| std::mem::discriminant(&edge.kind) == std::mem::discriminant(&kind))
    };
    if value.pending_embeds.is_some() && has(wire::ClosureEdgeKind::Embed) {
        return Err(gate(
            super::G_PASS_SNAPSHOT,
            "an `embed` edge exists while the embed snapshot is still pending",
        ));
    }
    if value.pending_sources.is_some() && has(wire::ClosureEdgeKind::Source) {
        return Err(gate(
            super::G_PASS_SNAPSHOT,
            "a `source` edge exists while the source snapshot is still pending",
        ));
    }
    Ok(())
}

// ── shared wire accessors ───────────────────────────────────────────────────

fn closure_of(value: &wire::Ir) -> Option<&'_ wire::ClosureIr> {
    match value {
        wire::Ir::ClosureArtifact(arm) => Some(&arm.closure),
        _ => None,
    }
}

fn plan_of(value: &wire::ClosureIr) -> Option<&'_ wire::AbsorptionPlan> {
    match &value.absorption {
        wire::AbsorptionState::Planned(arm) => Some(&arm.plan),
        wire::AbsorptionState::Applied(arm) => Some(&arm.plan),
        wire::AbsorptionState::Unplanned(_) => None,
    }
}

fn contribution_kind(value: &wire::ClosureContribution) -> &'static str {
    match value {
        wire::ClosureContribution::Normal(_) => "normal",
        wire::ClosureContribution::Simple(_) => "simple",
        wire::ClosureContribution::Elided(_) => "elided",
        wire::ClosureContribution::Hoisted(_) => "hoisted",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::link::LinkPassError;

    /// The real gate-13 error whose Display joins every candidate into one
    /// allocation. The staged refusal must use its derived Debug instead, so a
    /// multi-megabyte candidate list still yields a tightly bounded wire error.
    #[test]
    fn an_ambiguous_link_replay_refusal_is_bounded_before_display() {
        let huge = "candidate".repeat(512 * 1024);
        let source = LinkPassError::AmbiguousShortLink {
            contribution: 0,
            label: "shared".to_string(),
            candidates: vec![huge],
        };
        let rendered = refuse(G_LINK_WITNESS_LANE, "linked replay", source).to_string();
        assert!(rendered.len() < 512, "{} byte refusal", rendered.len());
        assert!(rendered.contains("AmbiguousShortLink"), "{rendered}");
        assert!(!rendered.contains(&"candidate".repeat(40)), "{rendered}");
    }
}
