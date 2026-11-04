//! Bound old-version cleanup after successful replacement, in the same steps2 owner.
use super::super::pending;
use super::*;
use crate::steps::StepsFile;
use crate::{Installation, SettingsRoot};
use vibe_wire::generated::shared as w;

pub(super) fn bind(
    ctx: &OperationContext<'_>,
    index: &Index,
    prior: &Installation,
    next: &str,
    purge: bool,
) -> Result<Vec<w::ApplicationPendingRetirement>, AppError> {
    let previous = prior.package.key();
    let mut rows = Vec::new();
    for (key, record) in &index.installations {
        if key == next
            || record.slot != prior.slot
            || !(record.lifecycle.superseded_by.is_some() || key == &previous)
            || key == &previous && !purge
        {
            continue;
        }
        if let Some(target) = &record.pending {
            return Err(pending::failure(
                ctx.settings,
                &record.package,
                format!(
                    "old version has unfinished work; continue `{}` before retiring it",
                    pending::command(target)
                ),
            ));
        }
        let mut expected = record.clone();
        if key == &previous {
            expected.lifecycle.hidden = true;
            expected.lifecycle.superseded_by = Some(next.into());
        }
        rows.push(w::ApplicationPendingRetirement {
            key: key.clone(),
            expected_record: pending::record_identity(&expected)
                .map_err(|e| pending::failure(ctx.settings, &record.package, e))?,
            state: w::ApplicationPendingRetirementState::Queued,
        });
    }
    Ok(rows)
}
fn uncertain(settings: &SettingsRoot, state: &StepsFile, key: &str) -> AppError {
    pending::failure(
        settings,
        &state.package,
        format!(
            "removal of `{key}` may have executed; it cannot be replayed or called successful automatically. Inspect the old installation and its uninstall log; repair explicitly with the existing uninstall command if needed, then abandon this work with `vibe app work {} --clean` before a fresh request",
            state.package.key()
        ),
    )
}
fn read(
    ctx: &OperationContext<'_>,
    target: &w::ApplicationPendingTarget,
) -> Result<StepsFile, AppError> {
    pending::load(
        ctx.settings,
        &pending::coordinate(&target.key)?,
        Some(&target.operation),
    )
}
fn check_current(
    ctx: &OperationContext<'_>,
    state: &StepsFile,
    index: &Index,
) -> Result<Installation, AppError> {
    let op = state.pending(ctx.settings)?;
    let record = index
        .installations
        .get(&op.target.key)
        .cloned()
        .ok_or_else(|| AppError::not_installed(ctx.settings, &op.target.key))?;
    if op.publication_record.as_ref()
        != Some(
            &pending::record_identity(&record)
                .map_err(|e| pending::failure(ctx.settings, &state.package, e))?,
        )
    {
        return Err(pending::failure(
            ctx.settings,
            &state.package,
            "the published replacement changed before old-version cleanup".into(),
        ));
    }
    if op.request.purge_previous
        && matches!(record.location, Location::Owned { .. })
        && op.predecessor.as_ref().is_some_and(|key| {
            key != &op.target.key && !op.retirements.iter().any(|r| &r.key == key)
        })
    {
        return Err(pending::failure(
            ctx.settings,
            &state.package,
            "requested previous-version retirement has no bound obligation".into(),
        ));
    }
    ensure_available(&op.target.key, &record, &Storage::load(ctx.settings)?)?;
    Ok(record)
}
fn set_state(
    ctx: &OperationContext<'_>,
    target: &w::ApplicationPendingTarget,
    position: usize,
    next: w::ApplicationPendingRetirementState,
) -> Result<(), AppError> {
    let mut state = read(ctx, target)?;
    let package = state.package.clone();
    let row = state
        .pending_mut(ctx.settings)?
        .retirements
        .get_mut(position)
        .ok_or_else(|| {
            pending::failure(ctx.settings, &package, "retirement index changed".into())
        })?;
    row.state = next;
    state.save(&pending::work(ctx.settings, &state.package))
}
/// Admission and drift are checked for the whole known list before any new victim
/// is attempted. The removal owner calls the boundary after its own preflight.
pub(in crate::operations) fn run(
    lock: &crate::MutationLock,
    ctx: &OperationContext<'_>,
    target: &w::ApplicationPendingTarget,
) -> Result<(), AppError> {
    let state = read(ctx, target)?;
    let op = state.pending(ctx.settings)?;
    let index = Index::load(ctx.settings)?;
    let current = check_current(ctx, &state, &index)?;
    let storage = Storage::load(ctx.settings)?;
    if op.retirements.is_empty() {
        return Err(pending::failure(
            ctx.settings,
            &state.package,
            "retirement stage has no bound cleanup list".into(),
        ));
    }
    if state.steps.iter().any(|step| {
        matches!(
            step.outcome,
            crate::StepOutcome::Delegated | crate::StepOutcome::Interrupted
        )
    }) {
        return Err(pending::failure(
            ctx.settings,
            &state.package,
            "replacement setup has not settled before retirement".into(),
        ));
    }
    if op.phase != w::ApplicationPendingOperationPhase::Retiring {
        pending::phase(
            ctx.settings,
            target,
            w::ApplicationPendingOperationPhase::Retiring,
        )?;
    }
    for row in &op.retirements {
        match row.state {
            w::ApplicationPendingRetirementState::Removing => {
                return Err(uncertain(ctx.settings, &state, &row.key));
            }
            w::ApplicationPendingRetirementState::Removed => {
                if index.installations.contains_key(&row.key) {
                    return Err(pending::failure(
                        ctx.settings,
                        &state.package,
                        format!(
                            "already removed key `{}` now exists again; it is not this retirement attempt",
                            row.key
                        ),
                    ));
                }
            }
            w::ApplicationPendingRetirementState::Queued => {
                let victim = index.installations.get(&row.key).ok_or_else(|| {
                    pending::failure(
                        ctx.settings,
                        &state.package,
                        format!(
                            "queued old version `{}` disappeared before its removal",
                            row.key
                        ),
                    )
                })?;
                if victim.slot != current.slot
                    || victim.lifecycle.superseded_by.is_none()
                    || victim.pending.is_some()
                    || pending::record_identity(victim)
                        .map_err(|e| pending::failure(ctx.settings, &state.package, e))?
                        != row.expected_record
                {
                    return Err(pending::failure(
                        ctx.settings,
                        &state.package,
                        format!("queued old version `{}` changed before removal", row.key),
                    ));
                }
                ensure_available(&row.key, victim, &storage)?;
            }
        }
    }
    for (position, row) in op.retirements.iter().enumerate() {
        if row.state == w::ApplicationPendingRetirementState::Removed {
            continue;
        }
        let boundary = || {
            set_state(
                ctx,
                target,
                position,
                w::ApplicationPendingRetirementState::Removing,
            )
        };
        remove::remove_with_boundary(lock, ctx, &row.key, false, Some(&boundary))?;
        set_state(
            ctx,
            target,
            position,
            w::ApplicationPendingRetirementState::Removed,
        )?;
    }
    Ok(())
}
/// Clear only this operation's marker after every bound removal settled. The
/// expected final record is stored before index save, including the marker change.
pub(super) fn clear_pending(
    lock: &crate::MutationLock,
    ctx: &OperationContext<'_>,
    target: &w::ApplicationPendingTarget,
) -> Result<(), AppError> {
    let state = read(ctx, target)?;
    let op = state.pending(ctx.settings)?;
    if op
        .retirements
        .iter()
        .any(|r| r.state != w::ApplicationPendingRetirementState::Removed)
    {
        return Err(pending::failure(
            ctx.settings,
            &state.package,
            "old-version cleanup is still outstanding".into(),
        ));
    }
    let mut index = Index::load(ctx.settings)?;
    let mut record = check_current(ctx, &state, &index)?;
    if record.pending.as_ref().is_some_and(|p| p != target) {
        return Err(pending::failure(
            ctx.settings,
            &state.package,
            "replacement has a different pending operation".into(),
        ));
    }
    if record.pending.is_some() {
        record.pending = None;
        pending::publishing(ctx.settings, target, &record)?;
        index.installations.insert(target.key.clone(), record);
        index.save_locked(
            ctx.settings,
            &crate::catalog::HomeEvent::about(
                crate::catalog::EventKind::Updated,
                target.key.clone(),
            ),
            lock,
        )?;
    }
    pending::phase(
        ctx.settings,
        target,
        w::ApplicationPendingOperationPhase::Published,
    )
}
pub(in crate::operations) fn resume(
    lock: &crate::MutationLock,
    ctx: &OperationContext<'_>,
    state: &StepsFile,
) -> Result<Outcome, AppError> {
    let op = state.pending(ctx.settings)?;
    let target = &op.target;
    if op.phase == w::ApplicationPendingOperationPhase::Publishing {
        // A crash between writing the final expected identity and clearing its
        // index marker is recoverable only when that marker is the sole change.
        let mut index = Index::load(ctx.settings)?;
        let mut current = index
            .installations
            .get(&target.key)
            .cloned()
            .ok_or_else(|| AppError::not_installed(ctx.settings, &target.key))?;
        if current.pending.as_ref().is_some_and(|p| p != target) {
            return Err(pending::failure(
                ctx.settings,
                &state.package,
                "another operation owns the pending marker".into(),
            ));
        }
        current.pending = None;
        if op.publication_record.as_ref()
            != Some(
                &pending::record_identity(&current)
                    .map_err(|e| pending::failure(ctx.settings, &state.package, e))?,
            )
        {
            return Err(pending::failure(
                ctx.settings,
                &state.package,
                "final pending-marker publication differs from its exact expected record".into(),
            ));
        }
        for row in &op.retirements {
            if row.state != w::ApplicationPendingRetirementState::Removed
                || index.installations.contains_key(&row.key)
            {
                return Err(pending::failure(
                    ctx.settings,
                    &state.package,
                    "finalization has an unsettled or reappeared retirement".into(),
                ));
            }
        }
        index.installations.insert(target.key.clone(), current);
        index.save_locked(
            ctx.settings,
            &crate::catalog::HomeEvent::about(
                crate::catalog::EventKind::Updated,
                target.key.clone(),
            ),
            lock,
        )?;
        pending::phase(
            ctx.settings,
            target,
            w::ApplicationPendingOperationPhase::Published,
        )?;
    } else {
        run(lock, ctx, target)?;
        clear_pending(lock, ctx, target)?;
    }
    let index = Index::load(ctx.settings)?;
    let record = index
        .installations
        .get(&target.key)
        .ok_or_else(|| AppError::not_installed(ctx.settings, &target.key))?;
    let outcome = Outcome {
        command: Command::Update,
        slot: record.slot.clone(),
        key: target.key.clone(),
        method: record.method,
        app_root: crate::launch::app_root(record, &Storage::load(ctx.settings)?).ok(),
        slot_default: index.default_of(&record.slot).map(str::to_owned),
        message: format!(
            "updated {}; completed its bound old-version cleanup",
            target.key
        ),
        work: op
            .request
            .keep_work
            .then(|| pending::work(ctx.settings, &state.package)),
    };
    pending::complete(lock, ctx.settings, target)?;
    Ok(outcome)
}

/// This stage runs only recorded cleanup/finalization, never installer setup.
pub(in crate::operations) fn cleanup_only(op: &w::ApplicationPendingOperation) -> bool {
    op.kind == w::ApplicationPendingOperationKind::Update
        && (op.phase == w::ApplicationPendingOperationPhase::Retiring
            || matches!(
                op.phase,
                w::ApplicationPendingOperationPhase::Publishing
                    | w::ApplicationPendingOperationPhase::Published
            ) && !op.retirements.is_empty()
                && op
                    .retirements
                    .iter()
                    .all(|r| r.state == w::ApplicationPendingRetirementState::Removed))
}
