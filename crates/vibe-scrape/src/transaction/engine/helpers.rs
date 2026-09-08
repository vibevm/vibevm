use super::*;

pub(super) enum CleanupPlan<'a> {
    Export(&'a ExportPlan),
    InPlace(&'a InPlacePlan),
}

pub(super) fn prepared_after_for_export<'a>(
    entry: &ExportEntry,
    snapshots: &'a [Snapshot],
) -> Result<Option<&'a [u8]>, TransactionError> {
    match &entry.payload {
        Some(ExportPayload::PreparedAfter { snapshot_name }) => snapshots
            .iter()
            .find(|snapshot| snapshot.name == *snapshot_name)
            .map(|snapshot| Some(snapshot.bytes.as_slice()))
            .ok_or_else(|| {
                TransactionError::InvalidPrepared(format!(
                    "missing prepared-after snapshot `{snapshot_name}`"
                ))
            }),
        _ => Ok(None),
    }
}

pub(super) fn prepared_after_for_step<'a>(
    step: &MutationStep,
    snapshots: &'a [Snapshot],
) -> Result<Option<&'a [u8]>, TransactionError> {
    if step.kind != MutationKind::AtomicRewrite {
        return Ok(None);
    }
    let name = format!("after/{}", step.id);
    snapshots
        .iter()
        .find(|snapshot| snapshot.name == name)
        .map(|snapshot| Some(snapshot.bytes.as_slice()))
        .ok_or_else(|| TransactionError::InvalidPrepared(format!("missing `{name}`")))
}

pub(super) fn expected_export_manifests(journal: &Journal, plan: &ExportPlan) -> Vec<TreeManifest> {
    let before = partial_export_manifest(plan, journal.completed_steps);
    let mut expected = vec![before];
    if let Some(active) = journal.active_step {
        let after = partial_export_manifest(plan, active.saturating_add(1));
        if !expected.contains(&after) {
            expected.push(after);
        }
    }
    expected
}

pub(super) fn partial_export_manifest(plan: &ExportPlan, count: usize) -> TreeManifest {
    let count = count.min(plan.final_manifest.entries.len());
    if count == plan.final_manifest.entries.len() {
        return plan.final_manifest.clone();
    }
    logical_tree_manifest(plan.final_manifest.entries[..count].to_vec())
}

pub(super) fn require_exact_tree(
    observed: OwnedTreeObservation,
    expected: &TreeManifest,
    label: &str,
) -> Result<(), TransactionError> {
    match observed {
        OwnedTreeObservation::Exact(actual) if actual == *expected => Ok(()),
        OwnedTreeObservation::Third { detail } => {
            Err(TransactionError::ThirdState(format!("{label}: {detail}")))
        }
        other => Err(TransactionError::ThirdState(format!(
            "{label}: expected exact manifest, observed {other:?}"
        ))),
    }
}

pub(super) fn validate_transaction_id(value: &TransactionId) -> Result<(), TransactionError> {
    if !(6..=64).contains(&value.0.len())
        || !value.0.bytes().all(|byte| byte.is_ascii_alphanumeric())
    {
        return Err(TransactionError::Store(
            "store minted a transaction id outside 6..64 ASCII alphanumerics".to_owned(),
        ));
    }
    Ok(())
}

pub(super) fn ownership_token(journal: &Journal, role: &str) -> String {
    let material = format!(
        "vibe-scrape-owner-e1\0{}\0{}\0{role}",
        journal.project_key.0, journal.transaction_id.0
    );
    digest(material.as_bytes()).0
}

pub(super) fn validate_transition(
    mode: TransactionMode,
    from: &TransactionState,
    to: &TransactionState,
) -> Result<(), TransactionError> {
    use TransactionState as S;
    let valid = match mode {
        TransactionMode::Export => matches!(
            (from, to),
            (S::Preparing, S::Prepared)
                | (S::Prepared, S::Candidate)
                | (S::Candidate, S::PublishedPendingVerify)
                | (S::PublishedPendingVerify, S::Verified)
                | (
                    S::Prepared | S::Candidate | S::PublishedPendingVerify,
                    S::RollingBack
                )
                | (S::RollingBack, S::RollingBack)
                | (S::RollingBack, S::RolledBack | S::RollbackFailed)
                | (S::Verified, S::CleanupPending)
                | (S::CleanupPending, S::CleanupPending | S::Complete)
        ),
        TransactionMode::InPlace => matches!(
            (from, to),
            (S::Preparing, S::Prepared)
                | (S::Prepared, S::BeforePassed)
                | (S::BeforePassed, S::Mutating)
                | (S::Mutating, S::ContractBoundary(_))
                | (S::ContractBoundary(_), S::Verified)
                | (
                    S::Prepared | S::BeforePassed | S::Mutating | S::ContractBoundary(_),
                    S::RollingBack
                )
                | (S::RollingBack, S::RollingBack)
                | (S::RollingBack, S::RolledBack | S::RollbackFailed)
                | (S::Verified, S::CleanupPending)
                | (S::CleanupPending, S::CleanupPending | S::Complete)
        ),
    };
    if valid {
        Ok(())
    } else {
        Err(TransactionError::Store(format!(
            "invalid {mode:?} journal transition {from:?} -> {to:?}"
        )))
    }
}

pub(super) fn planned_mutations(mode: &PreparedMode) -> Vec<MutationProgress> {
    let planned = match mode {
        PreparedMode::Export(plan) => {
            let mut values = vec![PlannedMutationEvidence {
                id: "export/candidate".to_owned(),
                kind: PlannedMutationKind::ExportCandidateCreate,
            }];
            values.extend(plan.entries.iter().enumerate().map(|(index, entry)| {
                PlannedMutationEvidence {
                    id: export_entry_id(index, entry),
                    kind: PlannedMutationKind::ExportEntry,
                }
            }));
            values.push(PlannedMutationEvidence {
                id: "export/publish".to_owned(),
                kind: PlannedMutationKind::ExportPublish,
            });
            values
        }
        PreparedMode::InPlace(plan) => {
            let mut values = vec![PlannedMutationEvidence {
                id: "in-place/quarantine".to_owned(),
                kind: PlannedMutationKind::InPlaceQuarantineCreate,
            }];
            values.extend(
                plan.steps
                    .iter()
                    .chain(std::iter::once(&plan.contract_step))
                    .chain(plan.contract_cleanup_step.iter())
                    .map(|step| PlannedMutationEvidence {
                        id: step.id.clone(),
                        kind: PlannedMutationKind::InPlace(step.kind),
                    }),
            );
            values
        }
    };
    planned
        .into_iter()
        .map(|value| MutationProgress {
            id: value.id,
            kind: value.kind,
            status: MutationStatus::Planned,
        })
        .collect()
}

pub(super) fn export_entry_id(index: usize, entry: &ExportEntry) -> String {
    format!("export/entry/{index}/{}", entry.target_path)
}

pub(super) fn set_progress_status(
    journal: &mut Journal,
    id: &str,
    status: MutationStatus,
) -> Result<(), TransactionError> {
    let progress = journal
        .mutation_progress
        .iter_mut()
        .find(|progress| progress.id == id)
        .ok_or_else(|| TransactionError::Store(format!("journal has no mutation `{id}`")))?;
    progress.status = status;
    Ok(())
}

pub(super) fn progress_status(journal: &Journal, id: &str) -> Option<MutationStatus> {
    journal
        .mutation_progress
        .iter()
        .find(|progress| progress.id == id)
        .map(|progress| progress.status)
}

pub(super) fn both_absent_is_explained(journal: &Journal) -> bool {
    let candidate = progress_status(journal, "export/candidate");
    let publish = progress_status(journal, "export/publish");
    let publish_applied_without_rollback = journal.actual_mutations.iter().any(|actual| {
        actual.id == "export/publish" && actual.direction == MutationDirection::Apply
    }) && !journal.actual_mutations.iter().any(|actual| {
        actual.id == "export/publish" && actual.direction == MutationDirection::Rollback
    });
    if publish == Some(MutationStatus::Applied) || publish_applied_without_rollback {
        return false;
    }
    let pre_create = matches!(
        candidate,
        Some(MutationStatus::Planned | MutationStatus::ApplyIntent)
    ) && journal.completed_steps == 0
        && journal.active_step.is_none()
        && !journal.actual_mutations.iter().any(|actual| {
            actual.id == "export/candidate" && actual.direction == MutationDirection::Apply
        });
    let rolled_back = matches!(
        candidate,
        Some(MutationStatus::RollbackIntent | MutationStatus::RolledBack)
    );
    pre_create || rolled_back
}

pub(super) fn record_actual(
    journal: &mut Journal,
    id: &str,
    direction: MutationDirection,
    status: MutationStatus,
    origin: MutationOrigin,
) -> Result<(), TransactionError> {
    let kind = journal
        .mutation_progress
        .iter()
        .find(|progress| progress.id == id)
        .map(|progress| progress.kind)
        .ok_or_else(|| TransactionError::Store(format!("journal has no mutation `{id}`")))?;
    let evidence = ActualMutationEvidence {
        id: id.to_owned(),
        kind,
        direction,
        origin,
        status,
    };
    if journal
        .actual_mutations
        .iter()
        .any(|prior| prior.id == evidence.id && prior.direction == evidence.direction)
    {
        return Ok(());
    }
    journal.actual_mutations.push(evidence);
    Ok(())
}

pub(super) fn infer_actual(
    journal: &mut Journal,
    id: &str,
    direction: MutationDirection,
    origin: MutationOrigin,
) -> Result<(), TransactionError> {
    let status = match direction {
        MutationDirection::Apply => MutationStatus::Applied,
        MutationDirection::Rollback => MutationStatus::RolledBack,
    };
    set_progress_status(journal, id, status)?;
    record_actual(journal, id, direction, status, origin)
}

pub(super) fn infer_export_apply_progress(
    journal: &mut Journal,
    plan: &ExportPlan,
    actual: &TreeManifest,
    origin: MutationOrigin,
) -> Result<(), TransactionError> {
    infer_actual(
        journal,
        "export/candidate",
        MutationDirection::Apply,
        origin,
    )?;
    for (index, entry) in plan.entries.iter().enumerate().take(actual.entries.len()) {
        infer_actual(
            journal,
            &export_entry_id(index, entry),
            MutationDirection::Apply,
            origin,
        )?;
    }
    Ok(())
}

pub(super) fn is_fault(error: &TransactionError) -> bool {
    matches!(error, TransactionError::FaultInjected(_))
}
