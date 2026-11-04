//! Where a run starts (`PROP-065#resume`): a fresh state, or the kept
//! `steps.json` of the work directory checked against the plan, continued
//! from its first step that is not done (or a named step). A resume whose
//! first open step is a finish step after every prepare step is done keeps
//! the whole prepare phase: the payload is already placed
//! (`PROP-064#parking`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#resume");

use std::path::Path;

use vibe_core::manifest::StepPhase;

use crate::error::{AppError, StepError};
use crate::index::StepOutcome;

use super::plan::Plan;
use super::state::{Start, StepsFile};

/// The state a run starts from.
pub(crate) struct StartPoint {
    pub state: StepsFile,
    /// Steps before this index are kept from the resumed state.
    pub first: usize,
    /// Every prepare step is kept too.
    pub finish_resume: bool,
    /// Steps the resumed state recorded `delegated`.
    pub delegated: Vec<String>,
}

/// Load and check the kept state for `start`, or begin a fresh one.
pub(crate) fn start_point(
    plan: &Plan,
    work: &Path,
    start: &Start,
    target: Option<&vibe_wire::generated::shared::ApplicationPendingTarget>,
) -> Result<StartPoint, AppError> {
    let key = plan.package.key();
    if *start == Start::Fresh && target.is_none() {
        return Ok(StartPoint {
            state: StepsFile {
                operation: None,
                package: plan.package.clone(),
                option: plan.option.clone(),
                features: plan.features.clone(),
                steps: Vec::new(),
            },
            first: 0,
            finish_resume: false,
            delegated: Vec::new(),
        });
    }
    let state = StepsFile::load(work)?.ok_or_else(|| StepError::NothingToResume {
        key: key.clone(),
        work: work.display().to_string(),
        reason: format!("no kept work directory `{}` exists", work.display()),
    })?;
    if let Some(target) = target
        && state.operation.as_ref().map(|o| &o.target) != Some(target)
    {
        return Err(StepError::ResumeMismatch {
            work: work.display().to_string(),
            reason: "the operation generation differs from the prepared checkpoint".into(),
        }
        .into());
    }
    let retained_unfinished = state.operation.as_ref().is_some_and(|o| {
        o.phase != vibe_wire::generated::shared::ApplicationPendingOperationPhase::Complete
    });
    let placed = state.operation.as_ref().is_some_and(|o| {
        matches!(
            o.phase,
            vibe_wire::generated::shared::ApplicationPendingOperationPhase::Placed
                | vibe_wire::generated::shared::ApplicationPendingOperationPhase::Publishing
                | vibe_wire::generated::shared::ApplicationPendingOperationPhase::Published
        )
    });
    let mismatch = if state.package != plan.package {
        Some(format!("it belongs to {}", state.package.key()))
    } else if state.option != plan.option {
        Some(format!(
            "it ran installer `{}`, not `{}`",
            state.option, plan.option
        ))
    } else if state.features != plan.features {
        Some(format!(
            "it ran with features [{}], not [{}]",
            state.features.join(", "),
            plan.features.join(", ")
        ))
    } else {
        None
    };
    if let Some(reason) = mismatch {
        return Err(StepError::ResumeMismatch {
            work: work.display().to_string(),
            reason,
        }
        .into());
    }
    let settled = || {
        plan.steps.iter().all(|planned| {
            matches!(
                state.get(&planned.step.id).map(|step| step.outcome),
                Some(StepOutcome::Done | StepOutcome::Skipped)
            )
        })
    };
    if *start == Start::Resume && settled() && !retained_unfinished {
        // A directory kept after success (`#work`) has nothing left to
        // continue; only a named step can be run again inside it.
        return Err(StepError::NothingToResume {
            key,
            work: work.display().to_string(),
            reason: format!(
                "every step of the kept work directory `{}` has already run",
                work.display()
            ),
        }
        .into());
    }
    let first = match start {
        Start::FromStep(id) => plan
            .steps
            .iter()
            .position(|planned| &planned.step.id == id)
            .ok_or_else(|| StepError::UnknownStep {
                option: plan.option.clone(),
                step: id.clone(),
                steps: plan
                    .steps
                    .iter()
                    .map(|planned| planned.step.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            })?,
        _ => plan
            .steps
            .iter()
            .position(|planned| {
                !matches!(
                    state.get(&planned.step.id).map(|step| step.outcome),
                    Some(StepOutcome::Done | StepOutcome::Skipped)
                )
            })
            .unwrap_or(plan.steps.len()),
    };
    let finish_resume = (placed
        || (*start == Start::Resume
            && plan
                .steps
                .get(first)
                .is_some_and(|planned| planned.step.phase == StepPhase::Finish)))
        && plan
            .steps
            .iter()
            .filter(|planned| planned.step.phase == StepPhase::Prepare)
            .all(|planned| {
                state.get(&planned.step.id).is_some_and(|step| {
                    matches!(step.outcome, StepOutcome::Done | StepOutcome::Skipped)
                })
            });
    let delegated = state
        .steps
        .iter()
        .filter(|step| *start == Start::Resume && step.outcome == StepOutcome::Delegated)
        .map(|step| step.id.clone())
        .collect();
    Ok(StartPoint {
        state,
        first,
        finish_resume,
        delegated,
    })
}
