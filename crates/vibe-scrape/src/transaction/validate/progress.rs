fn validate_names(value: &Journal) -> Result<(), TransactionError> {
    let (expected_name, forbidden_name, role) = match value.mode {
        TransactionMode::Export => (
            value.candidate_name.as_deref(),
            value.quarantine_name.as_deref(),
            "export",
        ),
        TransactionMode::InPlace => (
            value.quarantine_name.as_deref(),
            value.candidate_name.as_deref(),
            "quarantine",
        ),
    };
    if forbidden_name.is_some() {
        return corrupt("journal carries the other mode's owned sibling name");
    }
    if expected_name.is_some() != value.owned_tree_token.is_some() {
        return corrupt("owned sibling name/token presence differs");
    }
    if let Some(name) = expected_name {
        let prefix = match value.mode {
            TransactionMode::Export => ".vibe-scrape-candidate-",
            TransactionMode::InPlace => ".vibe-scrape-quarantine-",
        };
        if name != format!("{prefix}{}", value.transaction_id.0) {
            return corrupt("owned sibling name does not derive from transaction id");
        }
        let expected = ownership_token(value, role);
        if value.owned_tree_token.as_deref() != Some(expected.as_str()) {
            return corrupt("owned sibling token does not derive from journal identity");
        }
    }
    Ok(())
}

fn validate_counters(value: &Journal) -> Result<(), TransactionError> {
    let maximum = match &value.execution {
        PreparedMode::Export(plan) => plan.entries.len(),
        PreparedMode::InPlace(plan) => {
            plan.steps.len() + 1 + usize::from(plan.contract_cleanup_step.is_some())
        }
    };
    if value.completed_steps > maximum {
        return corrupt("completed step count exceeds executable plan");
    }
    if let Some(active) = value.active_step
        && (active >= maximum || active != value.completed_steps)
    {
        return corrupt("active step is not the exact next uncheckpointed step");
    }
    Ok(())
}

fn validate_progress(value: &Journal) -> Result<(), TransactionError> {
    let expected = expected_progress(&value.execution);
    if value.mutation_progress.len() != expected.len() {
        return corrupt("mutation progress does not cover the exact planned mutation set");
    }
    for (actual, expected) in value.mutation_progress.iter().zip(expected) {
        if actual.id != expected.id || actual.kind != expected.kind {
            return corrupt("mutation progress order/identity differs from executable plan");
        }
    }
    for actual in &value.actual_mutations {
        if !value
            .mutation_progress
            .iter()
            .any(|planned| planned.id == actual.id && planned.kind == actual.kind)
        {
            return corrupt("actual mutation evidence names an unplanned mutation");
        }
        match (actual.direction, actual.status) {
            (MutationDirection::Apply, MutationStatus::Applied)
            | (MutationDirection::Rollback, MutationStatus::RolledBack) => {}
            _ => return corrupt("actual mutation evidence carries intent/planned status"),
        }
    }
    for progress in &value.mutation_progress {
        let applied = value.actual_mutations.iter().any(|actual| {
            actual.id == progress.id
                && actual.direction == MutationDirection::Apply
                && actual.status == MutationStatus::Applied
        });
        let rolled_back = value.actual_mutations.iter().any(|actual| {
            actual.id == progress.id
                && actual.direction == MutationDirection::Rollback
                && actual.status == MutationStatus::RolledBack
        });
        match progress.status {
            MutationStatus::Planned | MutationStatus::NoMutation | MutationStatus::ApplyIntent
                if rolled_back =>
            {
                return corrupt("planned/apply-intent mutation has rollback evidence");
            }
            MutationStatus::NoMutation if applied => {
                return corrupt("no-mutation step carries applied evidence");
            }
            MutationStatus::Applied if !applied || rolled_back => {
                return corrupt("applied mutation evidence/status is inconsistent");
            }
            MutationStatus::RollbackIntent if !applied || rolled_back => {
                return corrupt("rollback intent lacks exactly one applied direction");
            }
            MutationStatus::RolledBack if !applied || !rolled_back => {
                return corrupt("rolled-back mutation lacks both actual directions");
            }
            _ => {}
        }
    }
    Ok(())
}

fn validate_cleanup(value: &Journal) -> Result<(), TransactionError> {
    let Some(wal) = &value.cleanup_wal else {
        return Ok(());
    };
    let refusal_cleanup = value.mode == TransactionMode::Export
        && value.state == TransactionState::Candidate
        && value.settlement_intent == Some(Outcome::Refused);
    if !matches!(
        value.state,
        TransactionState::RollingBack
            | TransactionState::CleanupPending
            | TransactionState::RolledBack
            | TransactionState::RollbackFailed
            | TransactionState::Complete
    ) && !refusal_cleanup
    {
        return corrupt("owned-tree cleanup WAL exists outside cleanup-capable state");
    }
    let seal = value
        .owned_tree_seal
        .as_ref()
        .ok_or_else(|| invalid_error("cleanup WAL has no owned-tree seal"))?;
    if value.owned_tree_token.is_none()
        || wal.directory_identity != seal.directory_identity
        || wal.manifest_digest != seal.manifest_digest
    {
        return corrupt("cleanup WAL binding differs from owned-tree evidence");
    }
    let expected_name = match value.mode {
        TransactionMode::Export => value.candidate_name.as_deref(),
        TransactionMode::InPlace => value.quarantine_name.as_deref(),
    };
    if expected_name != Some(wal.name.as_str()) {
        return corrupt("cleanup WAL name differs from the journaled owned sibling");
    }
    let order = cleanup_order(seal);
    if wal.completed.len() > order.len()
        || wal
            .completed
            .iter()
            .zip(&order)
            .any(|(actual, expected)| actual != expected)
    {
        return corrupt("cleanup completion list is not the canonical prefix");
    }
    if let Some(active) = &wal.active {
        if wal.completed.len() >= order.len()
            || active.progress_key != order[wal.completed.len()]
            || active.expected.path != active.path
            || active.intent_token.is_empty()
        {
            return corrupt("active cleanup intent is not the exact next canonical entry");
        }
        if active.root {
            if active.progress_key != "root"
                || active.path != wal.name
                || active.expected.kind != TreeEntryKind::Directory
            {
                return corrupt("root cleanup intent has an invalid shape");
            }
        } else {
            let expected = seal
                .entries
                .iter()
                .find(|entry| cleanup_key(entry) == active.progress_key)
                .ok_or_else(|| invalid_error("cleanup intent entry is absent from its seal"))?;
            if &active.expected != expected {
                return corrupt("cleanup intent expected state differs from its sealed entry");
            }
        }
    }
    Ok(())
}

fn cleanup_order(seal: &OwnedTreeSeal) -> Vec<String> {
    let mut files = seal
        .entries
        .iter()
        .filter(|entry| entry.kind == TreeEntryKind::File)
        .map(cleanup_key)
        .collect::<Vec<_>>();
    files.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
    let mut directories = seal
        .entries
        .iter()
        .filter(|entry| entry.kind == TreeEntryKind::Directory)
        .collect::<Vec<_>>();
    directories.sort_by(|left, right| {
        path_depth(&right.path)
            .cmp(&path_depth(&left.path))
            .then_with(|| right.path.as_bytes().cmp(left.path.as_bytes()))
    });
    files.extend(directories.into_iter().map(cleanup_key));
    files.push("root".to_owned());
    files
}

fn cleanup_key(entry: &OwnedEntrySeal) -> String {
    format!(
        "{}:{}",
        match entry.kind {
            TreeEntryKind::File => "file",
            TreeEntryKind::Directory => "directory",
        },
        entry.path
    )
}

fn validate_state_progress(value: &Journal) -> Result<(), TransactionError> {
    let status = |id: &str| {
        value
            .mutation_progress
            .iter()
            .find(|progress| progress.id == id)
            .map(|progress| progress.status)
    };
    if value.state == TransactionState::Preparing
        && value
            .mutation_progress
            .iter()
            .any(|progress| progress.status != MutationStatus::Planned)
    {
        return corrupt("preparation journal carries mutation progress");
    }
    match (&value.execution, &value.state) {
        (PreparedMode::Export(plan), TransactionState::Candidate) => {
            if value.completed_steps != plan.entries.len() || value.active_step.is_some() {
                return corrupt("candidate state does not have a complete export entry prefix");
            }
        }
        (
            PreparedMode::Export(plan),
            TransactionState::PublishedPendingVerify
            | TransactionState::Verified
            | TransactionState::CleanupPending,
        ) => {
            if value.completed_steps != plan.entries.len()
                || value.active_step.is_some()
                || status("export/publish") != Some(MutationStatus::Applied)
            {
                return corrupt("published/verified export lacks applied publication evidence");
            }
        }
        (PreparedMode::InPlace(_), TransactionState::BeforePassed) => {
            if value.completed_steps != 0 || value.active_step.is_some() {
                return corrupt("before-passed state carries project mutation progress");
            }
        }
        (PreparedMode::InPlace(plan), TransactionState::ContractBoundary(_)) => {
            if value.completed_steps < plan.steps.len() + 1 {
                return corrupt("contract boundary precedes the contract file checkpoint");
            }
        }
        (
            PreparedMode::InPlace(plan),
            TransactionState::Verified | TransactionState::CleanupPending,
        ) => {
            let expected = plan.steps.len() + 1 + usize::from(plan.contract_cleanup_step.is_some());
            if value.completed_steps != expected || value.active_step.is_some() {
                return corrupt(
                    "verified in-place state lacks complete contract-boundary progress",
                );
            }
        }
        _ => {}
    }
    let owned_removed = value.mutation_progress.iter().any(|progress| {
        matches!(
            progress.id.as_str(),
            "export/candidate" | "in-place/quarantine"
        ) && matches!(
            progress.status,
            MutationStatus::RollbackIntent | MutationStatus::RolledBack
        )
    });
    if matches!(
        value.state,
        TransactionState::Candidate
            | TransactionState::PublishedPendingVerify
            | TransactionState::Verified
            | TransactionState::CleanupPending
    ) && (value.owned_tree_token.is_none()
        || (value.owned_tree_seal.is_none() && !owned_removed))
    {
        return corrupt("owned mutation state has no sibling ownership token");
    }
    if value.mode == TransactionMode::InPlace
        && matches!(
            value.state,
            TransactionState::Mutating
                | TransactionState::ContractBoundary(_)
                | TransactionState::Verified
                | TransactionState::CleanupPending
        )
        && (value.owned_tree_token.is_none() || (value.owned_tree_seal.is_none() && !owned_removed))
    {
        return corrupt("in-place mutation state has no quarantine ownership token");
    }
    if value.owned_tree_seal.is_some() && value.owned_tree_token.is_none() {
        return corrupt("owned tree seal has no ownership seed");
    }
    if let Some(seal) = &value.owned_tree_seal {
        if seal.directory_identity.is_empty() || seal.manifest_digest.is_empty() {
            return corrupt("owned tree seal has incomplete identities");
        }
        let mut prior = None;
        for entry in &seal.entries {
            path(&entry.path)?;
            if entry.identity.is_empty()
                || prior.is_some_and(|path: &str| path.as_bytes() >= entry.path.as_bytes())
            {
                return corrupt("owned tree seal entries are incomplete or non-canonical");
            }
            prior = Some(entry.path.as_str());
        }
    }
    Ok(())
}

fn expected_progress(mode: &PreparedMode) -> Vec<PlannedMutationEvidence> {
    match mode {
        PreparedMode::Export(plan) => {
            let mut values = vec![PlannedMutationEvidence {
                id: "export/candidate".to_owned(),
                kind: PlannedMutationKind::ExportCandidateCreate,
            }];
            values.extend(plan.entries.iter().enumerate().map(|(index, entry)| {
                PlannedMutationEvidence {
                    id: format!("export/entry/{index}/{}", entry.target_path),
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
    }
}
