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
            (
                owner.target() == &parent.target && owner.owns_attempt(parent.row, child),
                owner.phase() != &w::ApplicationImportPhase::Complete,
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
    if active(settings, parent, &op.target)? {
        return Ok(Some(parent.clone()));
    }
    retaining_full_import(settings, state)
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

/// Complete application ancestors still retain their subtree for an unfinished
/// FULL2 cohort. Ownership is checked at every edge; legacy1 keeps its policy.
pub(super) fn retaining_full_import(
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
                "parent lineage is cyclic".into(),
            ));
        }
        owner_path(settings, &parent)?;
        match parent {
            w::ApplicationContinuationTarget::Application(p) => {
                let owner = load(
                    settings,
                    &coordinate(&p.target.key)?,
                    Some(&p.target.operation),
                )?;
                let next = owner.pending(settings)?;
                if next.target != p.target
                    || !next
                        .children
                        .iter()
                        .any(|c| c.target.as_ref() == Some(&op.target))
                {
                    return Err(failure(
                        settings,
                        &state.package,
                        "ancestor does not own this exact dependency attempt".into(),
                    ));
                }
                op = next.clone();
            }
            w::ApplicationContinuationTarget::Import(p) => {
                let owner = super::super::import::load(settings, &p.target.operation)?;
                let owns = owner.target() == &p.target && owner.owns_attempt(p.row, &op.target);
                if !owns {
                    return Err(failure(
                        settings,
                        &state.package,
                        "cohort does not own this exact root attempt".into(),
                    ));
                }
                return Ok((owner.full().is_some()
                    && owner.phase() != &w::ApplicationImportPhase::Complete)
                    .then_some(w::ApplicationContinuationTarget::Import(p)));
            }
        }
    }
    Ok(None)
}
