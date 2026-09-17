//! Observation helpers for engine-owned phase fences.

use anyhow::Result;
use vibe_lifecycle::Phase;

use crate::RitualPlan;
use crate::ports::RunObserver;

pub(super) fn empty_phase_boundary(
    plan: &RitualPlan,
    chain: &[String],
    phase: Phase,
) -> Option<usize> {
    (plan.count_for(phase) == 0).then_some(())?;
    let phase_rank = chain
        .iter()
        .position(|candidate| candidate == phase.as_str())?;
    Some(
        plan.executions
            .iter()
            .position(|execution| {
                chain
                    .iter()
                    .position(|candidate| candidate == &execution.phase)
                    .is_some_and(|rank| rank >= phase_rank)
            })
            .unwrap_or(plan.executions.len()),
    )
}

pub(super) fn observe_empty_fence(
    observer: &dyn RunObserver,
    phase: &str,
    success: &str,
    fire: impl FnOnce() -> Result<()>,
) -> Result<()> {
    observer.observe_phase_started(phase);
    match fire() {
        Ok(()) => {
            observer.observe_phase_finished(phase, success);
            Ok(())
        }
        Err(error) => {
            observer.observe_phase_finished(phase, "fail");
            Err(error)
        }
    }
}
