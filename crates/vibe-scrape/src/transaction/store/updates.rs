fn require_same_transaction(old: &Journal, new: &Journal) -> Result<(), TransactionError> {
    if old.schema != new.schema
        || old.project_key != new.project_key
        || old.transaction_id != new.transaction_id
        || old.mode != new.mode
        || old.plan_id != new.plan_id
        || old.project_display_root != new.project_display_root
        || old.canonical_plan != new.canonical_plan
        || old.verification_workspace != new.verification_workspace
        || old.execution != new.execution
        || old.snapshots != new.snapshots
    {
        return Err(store_error(
            "journal persistence attempted to change immutable transaction intent",
        ));
    }
    Ok(())
}

fn validate_journal_update(
    old: &Journal,
    new: &Journal,
    snapshot_observation: SnapshotActiveObservation,
) -> Result<(), TransactionError> {
    validate_snapshot_update(old, new, snapshot_observation)?;
    validate_state_update(old, new)?;
    validate_step_update(old, new)?;
    validate_cleanup_wal_update(old, new)?;

    if !is_prefix(&old.verification, &new.verification) {
        return Err(store_error(
            "journal verification evidence is not append-only",
        ));
    }
    if !is_prefix(&old.events, &new.events) {
        return Err(store_error("journal events are not append-only"));
    }
    if !is_prefix(&old.actual_mutations, &new.actual_mutations) {
        return Err(store_error("actual mutation evidence is not append-only"));
    }
    for (index, evidence) in new.actual_mutations.iter().enumerate() {
        if new.actual_mutations[..index]
            .iter()
            .any(|prior| prior.id == evidence.id && prior.direction == evidence.direction)
        {
            return Err(store_error(
                "actual mutation evidence repeats an id/direction",
            ));
        }
    }
    if old
        .settlement_intent
        .is_some_and(|intent| new.settlement_intent != Some(intent))
    {
        return Err(store_error(
            "durable settlement intent changed or disappeared",
        ));
    }
    if old
        .delivered_tree
        .as_ref()
        .is_some_and(|tree| new.delivered_tree.as_ref() != Some(tree))
    {
        return Err(store_error("delivered-tree proof changed or disappeared"));
    }
    if old
        .candidate_name
        .as_ref()
        .is_some_and(|name| new.candidate_name.as_ref() != Some(name))
        || old
            .quarantine_name
            .as_ref()
            .is_some_and(|name| new.quarantine_name.as_ref() != Some(name))
        || old
            .owned_tree_token
            .as_ref()
            .is_some_and(|token| new.owned_tree_token.as_ref() != Some(token))
    {
        return Err(store_error(
            "journaled owned-tree name or ownership token changed or disappeared",
        ));
    }
    match (&old.report, &new.report) {
        (None, _) | (Some(_), Some(_)) if report_update_is_legal(&old.report, &new.report) => {}
        _ => {
            return Err(store_error(
                "embedded transaction report regressed or changed evidence",
            ));
        }
    }
    Ok(())
}

fn validate_snapshot_update(
    old: &Journal,
    new: &Journal,
    observation: SnapshotActiveObservation,
) -> Result<(), TransactionError> {
    if old.snapshots_persisted == new.snapshots_persisted
        && old.snapshot_active == new.snapshot_active
    {
        return Ok(());
    }
    match (old.snapshot_active, new.snapshot_active) {
        (None, Some(active))
            if old.snapshots_persisted == new.snapshots_persisted
                && active == old.snapshots_persisted
                && observation == SnapshotActiveObservation::None =>
        {
            Ok(())
        }
        (Some(active), None)
            if new.snapshots_persisted == old.snapshots_persisted + 1
                && active == old.snapshots_persisted
                && observation == SnapshotActiveObservation::ExactPresent =>
        {
            Ok(())
        }
        _ => Err(store_error(
            "snapshot checkpoint regressed, skipped intent/data, or advanced without exact data",
        )),
    }
}

fn validate_state_update(old: &Journal, new: &Journal) -> Result<(), TransactionError> {
    use TransactionState as State;

    if old.state == new.state {
        return Ok(());
    }
    let ordinary = match (&old.state, &new.state, old.mode) {
        (State::Preparing, State::Prepared, _) => true,
        (State::Prepared, State::Candidate, TransactionMode::Export)
        | (State::Candidate, State::PublishedPendingVerify, TransactionMode::Export) => true,
        (State::Prepared, State::BeforePassed, TransactionMode::InPlace)
        | (State::BeforePassed, State::Mutating, TransactionMode::InPlace)
        | (State::Mutating, State::ContractBoundary(_), TransactionMode::InPlace) => true,
        (State::ContractBoundary(old_action), State::ContractBoundary(new_action), _)
            if old_action == new_action =>
        {
            true
        }
        (State::PublishedPendingVerify, State::Verified, TransactionMode::Export)
        | (State::ContractBoundary(_), State::Verified, TransactionMode::InPlace)
        | (State::Verified, State::CleanupPending, _)
        | (State::CleanupPending, State::Complete, _)
        | (State::RollingBack, State::RolledBack | State::RollbackFailed, _)
        | (State::RolledBack, State::Complete, _) => true,
        (
            State::Prepared | State::Candidate | State::PublishedPendingVerify,
            State::RollingBack,
            TransactionMode::Export,
        )
        | (
            State::Prepared | State::BeforePassed | State::Mutating | State::ContractBoundary(_),
            State::RollingBack,
            TransactionMode::InPlace,
        ) => true,
        _ => false,
    };
    let terminal_refusal = new.state == State::Complete
        && new
            .report
            .as_ref()
            .is_some_and(|report| report.outcome == Outcome::Refused)
        && matches!(old.state, State::Preparing | State::Prepared);
    if ordinary || terminal_refusal {
        Ok(())
    } else {
        Err(store_error(format!(
            "illegal durable transaction state transition {:?} -> {:?}",
            old.state, new.state
        )))
    }
}

fn validate_step_update(old: &Journal, new: &Journal) -> Result<(), TransactionError> {
    if new.completed_steps < old.completed_steps
        || new.completed_steps > old.completed_steps.saturating_add(1)
    {
        return Err(store_error(
            "completed mutation prefix regressed or skipped a step",
        ));
    }
    match (old.active_step, new.active_step) {
        (None, Some(active))
            if new.completed_steps == old.completed_steps && active == old.completed_steps => {}
        (Some(old_active), Some(new_active))
            if old_active == new_active && new.completed_steps == old.completed_steps => {}
        (Some(active), None)
            if active == old.completed_steps
                && new.completed_steps == old.completed_steps.saturating_add(1) => {}
        (None, None) if new.completed_steps == old.completed_steps => {}
        _ => {
            return Err(store_error(
                "mutation step checkpoint regressed, skipped intent, or advanced out of order",
            ));
        }
    }
    for (prior, next) in old.mutation_progress.iter().zip(&new.mutation_progress) {
        if prior.id != next.id
            || prior.kind != next.kind
            || !mutation_status_update_is_legal(prior.status, next.status)
            || (next.status == MutationStatus::NoMutation
                && next.kind
                    != PlannedMutationKind::InPlace(MutationKind::ContractExternalPreserve))
        {
            return Err(store_error(format!(
                "mutation progress for `{}` regressed or skipped a durable intent",
                prior.id
            )));
        }
    }
    Ok(())
}

fn mutation_status_update_is_legal(old: MutationStatus, new: MutationStatus) -> bool {
    old == new
        || matches!(
            (old, new),
            (
                MutationStatus::Planned,
                MutationStatus::ApplyIntent | MutationStatus::NoMutation
            ) | (MutationStatus::ApplyIntent, MutationStatus::Applied)
                | (MutationStatus::Applied, MutationStatus::RollbackIntent)
                | (MutationStatus::RollbackIntent, MutationStatus::RolledBack)
        )
}

fn validate_cleanup_wal_update(old: &Journal, new: &Journal) -> Result<(), TransactionError> {
    match (&old.cleanup_wal, &new.cleanup_wal) {
        (None, None) => Ok(()),
        (None, Some(wal)) => {
            if wal.completed.is_empty() && wal.active.is_none() {
                Ok(())
            } else {
                Err(store_error(
                    "cleanup WAL did not start at an empty durable prefix",
                ))
            }
        }
        (Some(old_wal), Some(new_wal)) => {
            if old_wal.name != new_wal.name
                || old_wal.directory_identity != new_wal.directory_identity
                || old_wal.manifest_digest != new_wal.manifest_digest
            {
                return Err(store_error("cleanup WAL changed its sealed tree binding"));
            }
            match (&old_wal.active, &new_wal.active) {
                (None, None) if old_wal.completed == new_wal.completed => Ok(()),
                (None, Some(_)) if old_wal.completed == new_wal.completed => Ok(()),
                (Some(old_active), Some(new_active))
                    if old_active == new_active && old_wal.completed == new_wal.completed =>
                {
                    Ok(())
                }
                (Some(active), None)
                    if new_wal.completed.len() == old_wal.completed.len() + 1
                        && new_wal.completed.starts_with(&old_wal.completed)
                        && new_wal.completed.last() == Some(&active.progress_key) =>
                {
                    Ok(())
                }
                _ => Err(store_error(
                    "cleanup WAL regressed, changed intent, or skipped an exact completion",
                )),
            }
        }
        (Some(_), None)
            if old.state == new.state
                && new.owned_tree_seal.is_none()
                && old.owned_tree_seal.is_some() =>
        {
            Ok(())
        }
        (Some(_), None) => Err(store_error(
            "cleanup WAL cleared before its owned-tree seal at the same durable state",
        )),
    }
}

fn report_update_is_legal(
    old: &Option<TransactionReport>,
    new: &Option<TransactionReport>,
) -> bool {
    match (old, new) {
        (None, _) => true,
        (Some(_), None) => false,
        (Some(old), Some(new)) if old == new => true,
        (Some(old), Some(new)) => {
            old.project_key == new.project_key
                && old.transaction_id == new.transaction_id
                && old.plan_id == new.plan_id
                && old.mode == new.mode
                && old.outcome == new.outcome
                && old.assurance == new.assurance
                && old.before_tree == new.before_tree
                && old.after_tree == new.after_tree
                && old.snapshots == new.snapshots
                && old.verification == new.verification
                && old.planned_mutations == new.planned_mutations
                && old.actual_mutations == new.actual_mutations
                && is_prefix(&old.events, &new.events)
                && matches!(
                    (old.cleanup, new.cleanup),
                    (Cleanup::Pending, Cleanup::Pending | Cleanup::Complete)
                )
        }
    }
}

fn is_prefix<T: PartialEq>(old: &[T], new: &[T]) -> bool {
    new.starts_with(old)
}
