//! Verified mixed-owner parent relations; application targets remain package-specific.
use super::*;

pub(crate) fn owner_path(
    settings: &SettingsRoot,
    parent: &w::ApplicationContinuationTarget,
) -> Result<PathBuf, AppError> {
    let (home, path) = match parent {
        w::ApplicationContinuationTarget::Application(p) => (
            p.target.home.as_str(),
            work(settings, &coordinate(&p.target.key)?),
        ),
        w::ApplicationContinuationTarget::Import(p) => (
            p.target.home.as_str(),
            super::super::import::store::path(settings, &p.target.operation)?,
        ),
    };
    if home != actual_home(settings)? {
        return Err(StepError::ResumeMismatch {
            work: path.display().to_string(),
            reason: "retaining owner belongs to another actual home".into(),
        }
        .into());
    }
    Ok(path)
}
pub(crate) fn continuation_command(target: &w::ApplicationContinuationTarget) -> String {
    match target {
        w::ApplicationContinuationTarget::Application(t) => command(&t.target),
        w::ApplicationContinuationTarget::Import(t) => super::super::import::command(&t.target),
    }
}
fn active(
    settings: &SettingsRoot,
    parent: &w::ApplicationContinuationTarget,
    child: &w::ApplicationPendingTarget,
) -> Result<bool, AppError> {
    owner_path(settings, parent)?;
    let package = coordinate(&child.key)?;
    let (owns, active) = match parent {
        w::ApplicationContinuationTarget::Application(parent) => {
            let state = load(
                settings,
                &coordinate(&parent.target.key)?,
                Some(&parent.target.operation),
            )?;
            let op = state.pending(settings)?.clone();
            (
                op.target == parent.target
                    && op.children.iter().any(|b| b.target.as_ref() == Some(child)),
                op.phase != w::ApplicationPendingOperationPhase::Complete,
            )
        }
        w::ApplicationContinuationTarget::Import(parent) => {
            let owner = super::super::import::load(settings, &parent.target.operation)?;
            let row = owner.cohort.rows.get(parent.row as usize);
            let relation =
                owner.cohort.cursor == parent.row && owner.cohort.current.as_ref() == Some(child)
                    || owner.cohort.results.iter().any(|r| {
                        r.row_index == Some(parent.row) && r.target.as_ref() == Some(child)
                    });
            (
                owner.target == parent.target
                    && row.is_some_and(|r| r.key == child.key)
                    && relation,
                owner.phase != w::ApplicationImportPhase::Complete,
            )
        }
    };
    if !owns {
        return Err(failure(
            settings,
            &package,
            "retained parent does not own this exact row/attempt".into(),
        ));
    }
    Ok(active)
}
pub(crate) fn retained_for(
    settings: &SettingsRoot,
    state: &StepsFile,
) -> Result<Option<w::ApplicationContinuationTarget>, AppError> {
    let Some(op) = &state.operation else {
        return Ok(None);
    };
    let Some(parent) = &op.parent else {
        return Ok(None);
    };
    Ok(active(settings, parent, &op.target)?.then(|| parent.clone()))
}
/// Only verified forward/backward relations widen a hosted continuation.
pub(crate) fn continuation_target(
    settings: &SettingsRoot,
    state: &StepsFile,
) -> Result<Option<w::ApplicationContinuationTarget>, AppError> {
    let Some(mut op) = state.operation.clone() else {
        return Ok(None);
    };
    let mut seen = std::collections::BTreeSet::new();
    while let Some(parent) = op.parent.clone() {
        if !seen.insert((op.target.key.clone(), op.target.operation.clone())) {
            return Err(failure(
                settings,
                &state.package,
                "pending parent chain is cyclic".into(),
            ));
        }
        if !active(settings, &parent, &op.target)? {
            return Err(failure(
                settings,
                &state.package,
                "retained parent is already settled".into(),
            ));
        }
        match parent {
            w::ApplicationContinuationTarget::Import(_) => return Ok(Some(parent)),
            w::ApplicationContinuationTarget::Application(parent) => {
                op = load(
                    settings,
                    &coordinate(&parent.target.key)?,
                    Some(&parent.target.operation),
                )?
                .pending(settings)?
                .clone();
            }
        }
    }
    Ok(Some(application_reference(&op.target)))
}
