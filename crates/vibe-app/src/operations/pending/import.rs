//! Exact row publication acknowledgement and terminal refused-row settlement.
use super::*;
pub(crate) fn ack_import(
    settings: &SettingsRoot,
    target: &w::ApplicationPendingTarget,
) -> Result<(), AppError> {
    let package = coordinate(&target.key)?;
    let state = load(settings, &package, Some(&target.operation))?;
    let op = state
        .operation
        .as_ref()
        .ok_or_else(|| failure(settings, &package, "missing row operation".into()))?;
    let index = Index::load(settings)?;
    if op.publication_record.is_none()
        || identity::predecessor(&index, &target.key).map_err(|e| failure(settings, &package, e))?
            != op.publication_record
        || index
            .installations
            .get(&target.key)
            .is_none_or(|r| r.pending.is_some())
    {
        return Err(failure(
            settings,
            &package,
            "import row no longer matches its exact published result".into(),
        ));
    }
    Ok(())
}

/// An ordinary terminal refusal advances the cohort, unlike a delegated park.
/// If finish had published a pending row, preserve that installation under the
/// incumbent failed-finish semantics, without calling it a successful import.
pub(crate) fn finish_import_row(
    lock: &crate::MutationLock,
    settings: &SettingsRoot,
    target: &w::ApplicationPendingTarget,
) -> Result<(), AppError> {
    let package = coordinate(&target.key)?;
    let mut state = load(settings, &package, Some(&target.operation))?;
    let op = state
        .operation
        .as_mut()
        .ok_or_else(|| failure(settings, &package, "missing refused row".into()))?;
    let mut index = Index::load(settings)?;
    if index
        .installations
        .get(&target.key)
        .is_some_and(|r| r.pending.as_ref() == Some(target))
    {
        if op.publication_record.is_none()
            || identity::predecessor(&index, &target.key)
                .map_err(|e| failure(settings, &package, e))?
                != op.publication_record
        {
            return Err(failure(
                settings,
                &package,
                "refused row publication cannot be verified".into(),
            ));
        }
        let row = index.installations.get_mut(&target.key).ok_or_else(|| {
            failure(
                settings,
                &package,
                "published row disappeared before settlement".into(),
            )
        })?;
        row.pending = None;
        op.publication_record =
            Some(identity::published(row).map_err(|e| failure(settings, &package, e))?);
        state.save(&work(settings, &package))?;
        index.save_locked(
            settings,
            &crate::catalog::HomeEvent::new(crate::catalog::EventKind::Updated),
            lock,
        )?;
    }
    state.pending_mut(settings)?.phase = w::ApplicationPendingOperationPhase::Complete;
    state.save(&work(settings, &package))
}
