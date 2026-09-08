#[test]
fn missing_safefs_directory_primitive_is_a_typed_blocker() {
    assert_eq!(
        RequiredPrimitive::StableProjectIdentityToken.required_api(),
        "vibe_safefs::Project::identity_token()"
    );
    assert!(
        RequiredPrimitive::ExternalNoFollowStoreAndLock
            .required_api()
            .contains("open_and_lock_project")
    );
    let PreparedMode::Export(plan) = export_prepared().mode else {
        unreachable!()
    };
    let error = SafefsCapabilityGap
        .publish_export_noreplace(&plan, "candidate", "owner")
        .unwrap_err();
    assert_eq!(
        error,
        TransactionError::MissingPrimitive(RequiredPrimitive::AtomicNoReplaceDirectoryRename)
    );
    assert!(error.to_string().contains("rename_child_noreplace_to"));
}

#[test]
fn active_export_step_accepts_exact_prefix_before_or_after_only() {
    for target in [
        DurableBoundary::StepIntentPersisted {
            index: 2,
            id: "export/entry/2/docs/spec.md".into(),
        },
        DurableBoundary::MutationCompleted {
            label: "export-entry-2".into(),
        },
    ] {
        let mut store = MemoryStore::default();
        let mut fs = MemoryFs::default();
        let mut verifier = AcceptingVerifier::default();
        let mut fault = OneFault {
            target: Some(target),
            fired: false,
        };
        execute(
            export_prepared(),
            &mut store,
            &mut fs,
            &mut verifier,
            &mut fault,
        )
        .unwrap_err();
        let report = recover(&mut store, &mut fs, &mut verifier).unwrap();
        assert_eq!(report.outcome, Outcome::RolledBack);
        assert!(fs.candidate.is_none() && fs.output.is_none());
    }
}

#[test]
fn exclusive_candidate_creation_distinguishes_all_ownership_outcomes() {
    let mut not_created = MemoryFs {
        candidate_creation: Some(ExclusiveTreeCreation::NotCreated {
            detail: "raced occupant".into(),
        }),
        ..MemoryFs::default()
    };
    let report = execute(
        export_prepared(),
        &mut MemoryStore::default(),
        &mut not_created,
        &mut AcceptingVerifier::default(),
        &mut NoFaults,
    )
    .unwrap();
    assert_eq!(report.outcome, Outcome::Refused);
    assert!(not_created.candidate.is_none());

    let mut store = MemoryStore::default();
    let mut uncertain = MemoryFs {
        candidate_creation: Some(ExclusiveTreeCreation::CreatedNotReopened {
            detail: "created but identity reopen failed".into(),
        }),
        ..MemoryFs::default()
    };
    execute(
        export_prepared(),
        &mut store,
        &mut uncertain,
        &mut AcceptingVerifier::default(),
        &mut NoFaults,
    )
    .unwrap_err();
    assert!(
        store.pending.is_some(),
        "partial ownership remains recoverable"
    );
    let report = recover(
        &mut store,
        &mut uncertain,
        &mut AcceptingVerifier::default(),
    )
    .unwrap();
    assert_eq!(report.outcome, Outcome::RolledBack);
    assert!(uncertain.candidate.is_none());

    let mut store = MemoryStore::default();
    let mut refused = MemoryFs {
        candidate_creation: Some(ExclusiveTreeCreation::NotCreated {
            detail: "occupied".into(),
        }),
        ..MemoryFs::default()
    };
    let mut fault = OneFault {
        target: Some(DurableBoundary::JournalPersisted(
            TransactionState::Complete,
        )),
        fired: false,
    };
    execute(
        export_prepared(),
        &mut store,
        &mut refused,
        &mut AcceptingVerifier::default(),
        &mut fault,
    )
    .unwrap_err();
    let report = recover(&mut store, &mut refused, &mut AcceptingVerifier::default()).unwrap();
    assert_eq!(report.outcome, Outcome::Refused);
}

#[test]
fn preparation_journal_is_recoverable_at_creation_each_snapshot_and_store_failure() {
    let snapshot_count = export_prepared().snapshots.len();
    let mut targets = vec![DurableBoundary::TransactionCreated];
    for index in 0..snapshot_count {
        targets.extend([
            DurableBoundary::SnapshotIntentPersisted { index },
            DurableBoundary::SnapshotDataPersisted { index },
            DurableBoundary::SnapshotPersisted { index },
        ]);
    }
    for target in targets {
        let mut store = MemoryStore::default();
        let mut fs = MemoryFs::default();
        let mut fault = OneFault {
            target: Some(target),
            fired: false,
        };
        execute(
            export_prepared(),
            &mut store,
            &mut fs,
            &mut AcceptingVerifier::default(),
            &mut fault,
        )
        .unwrap_err();
        assert_eq!(
            store.pending.as_ref().unwrap().state,
            TransactionState::Preparing
        );
        let report = recover(&mut store, &mut fs, &mut AcceptingVerifier::default()).unwrap();
        assert_eq!(report.outcome, Outcome::Refused);
        assert_eq!(fs.source_mutations, 0);
    }

    let mut store = MemoryStore {
        fail_snapshot_at: Some(2),
        ..MemoryStore::default()
    };
    let mut fs = MemoryFs::default();
    execute(
        export_prepared(),
        &mut store,
        &mut fs,
        &mut AcceptingVerifier::default(),
        &mut NoFaults,
    )
    .unwrap_err();
    assert_eq!(store.pending.as_ref().unwrap().snapshots_persisted, 2);
    store.fail_snapshot_at = None;
    let report = recover(&mut store, &mut fs, &mut AcceptingVerifier::default()).unwrap();
    assert_eq!(report.outcome, Outcome::Refused);

    let mut store = MemoryStore {
        fail_snapshot_after_write_at: Some(2),
        ..MemoryStore::default()
    };
    let mut fs = MemoryFs::default();
    execute(
        export_prepared(),
        &mut store,
        &mut fs,
        &mut AcceptingVerifier::default(),
        &mut NoFaults,
    )
    .unwrap_err();
    assert_eq!(store.pending.as_ref().unwrap().snapshot_active, Some(2));
    assert_eq!(store.snapshots.len(), 3);
    let report = recover(&mut store, &mut fs, &mut AcceptingVerifier::default()).unwrap();
    assert_eq!(report.outcome, Outcome::Refused);
}

#[test]
fn pending_gate_runs_under_lock_before_preparation_closure() {
    let mut store = MemoryStore::default();
    let mut fs = MemoryFs::default();
    let mut fault = OneFault {
        target: Some(DurableBoundary::MutationCompleted {
            label: "export-candidate-create".into(),
        }),
        fired: false,
    };
    execute(
        export_prepared(),
        &mut store,
        &mut fs,
        &mut AcceptingVerifier::default(),
        &mut fault,
    )
    .unwrap_err();
    let prepared_called = std::cell::Cell::new(false);
    let result = Engine::new(
        &mut store,
        &mut fs,
        &mut AcceptingVerifier::default(),
        &mut NoFaults,
    )
    .execute_locked(
        "stable-project-identity",
        "C:/source",
        || -> Result<PreparedTransaction, TransactionError> {
            prepared_called.set(true);
            Ok(export_prepared())
        },
    );
    assert!(result.is_err());
    assert!(!prepared_called.get());
}

#[test]
fn strict_journal_validation_rejects_corruption_before_recovery_mutation() {
    let mut store = MemoryStore::default();
    let mut fs = MemoryFs::default();
    let mut fault = OneFault {
        target: Some(DurableBoundary::MutationCompleted {
            label: "export-candidate-create".into(),
        }),
        fired: false,
    };
    execute(
        export_prepared(),
        &mut store,
        &mut fs,
        &mut AcceptingVerifier::default(),
        &mut fault,
    )
    .unwrap_err();
    let original = store.pending.clone().unwrap();
    let key = project_key("stable-project-identity");
    let mut corruptions = Vec::new();
    let mut state = original.clone();
    state.state = TransactionState::BeforePassed;
    corruptions.push(state);
    let mut counter = original.clone();
    counter.completed_steps = usize::MAX;
    corruptions.push(counter);
    let mut active = original.clone();
    active.active_step = Some(2);
    corruptions.push(active);
    let mut name = original.clone();
    name.candidate_name = Some("foreign".into());
    corruptions.push(name);
    let mut progress = original;
    progress.mutation_progress.pop();
    corruptions.push(progress);
    for journal in corruptions {
        assert!(super::validate::journal(&journal, &key, "C:/source").is_err());
    }

    let mut store = MemoryStore::default();
    let mut verified_fault = OneFault {
        target: Some(DurableBoundary::JournalPersisted(
            TransactionState::Verified,
        )),
        fired: false,
    };
    execute(
        export_prepared(),
        &mut store,
        &mut MemoryFs::default(),
        &mut AcceptingVerifier::default(),
        &mut verified_fault,
    )
    .unwrap_err();
    let verified = store.pending.unwrap();
    assert_eq!(
        verified.verification.last().map(|record| record.phase),
        Some(VerificationPhase::SourceUnchanged)
    );
    let mut missing = verified.clone();
    missing.verification.pop();
    assert!(super::validate::journal(&missing, &key, "C:/source").is_err());
    let mut false_gate = verified.clone();
    false_gate
        .verification
        .last_mut()
        .unwrap()
        .evidence
        .accepted = false;
    assert!(super::validate::journal(&false_gate, &key, "C:/source").is_err());
    let mut duplicate = verified;
    duplicate
        .verification
        .push(duplicate.verification[0].clone());
    assert!(super::validate::journal(&duplicate, &key, "C:/source").is_err());
}

#[test]
fn closed_mutation_grammar_executes_rewrite_relocate_remove_prune_and_external_preserve() {
    let mut store = MemoryStore::default();
    let mut fs = MemoryFs::default();
    let report = execute(
        complex_in_place_prepared(),
        &mut store,
        &mut fs,
        &mut AcceptingVerifier::default(),
        &mut NoFaults,
    )
    .unwrap();
    assert_eq!(report.outcome, Outcome::Verified);
    assert!(
        report
            .planned_mutations
            .iter()
            .any(|item| { item.kind == PlannedMutationKind::InPlace(MutationKind::AtomicRewrite) })
    );
    assert!(
        report
            .planned_mutations
            .iter()
            .any(|item| { item.kind == PlannedMutationKind::InPlace(MutationKind::Relocate) })
    );
    assert!(
        report
            .actual_mutations
            .iter()
            .any(|item| { item.id == "prune-empty" && item.direction == MutationDirection::Apply })
    );

    let mut invalid = complex_in_place_prepared();
    let PreparedMode::InPlace(plan) = &mut invalid.mode else {
        unreachable!()
    };
    plan.steps.swap(1, 4);
    let mut untouched = MemoryFs::default();
    assert!(
        execute(
            invalid,
            &mut MemoryStore::default(),
            &mut untouched,
            &mut AcceptingVerifier::default(),
            &mut NoFaults,
        )
        .is_err()
    );
    assert_eq!(untouched.source_mutations, 0);

    let mut external_fs = MemoryFs::default();
    let external = execute(
        external_preserve_prepared(),
        &mut MemoryStore::default(),
        &mut external_fs,
        &mut AcceptingVerifier::default(),
        &mut NoFaults,
    )
    .unwrap();
    assert_eq!(external.outcome, Outcome::Verified);
    assert!(
        !external
            .actual_mutations
            .iter()
            .any(|item| item.id == "external-preserve")
    );
    assert_eq!(external_fs.source_mutations, 0);

    let mut collision = in_place_prepared();
    let PreparedMode::InPlace(plan) = &mut collision.mode else {
        unreachable!()
    };
    plan.contract_step.id = "in-place/quarantine".into();
    assert!(
        execute(
            collision,
            &mut MemoryStore::default(),
            &mut MemoryFs::default(),
            &mut AcceptingVerifier::default(),
            &mut NoFaults,
        )
        .is_err()
    );

    for malicious in ["other/file.toml", "vibevm/data"] {
        let mut prepared = in_place_prepared();
        let PreparedMode::InPlace(plan) = &mut prepared.mode else {
            unreachable!()
        };
        plan.contract = ContractCommit::DeleteLast {
            path: malicious.into(),
            empty_ancestors: vec!["other".into()],
        };
        assert!(
            execute(
                prepared,
                &mut MemoryStore::default(),
                &mut MemoryFs::default(),
                &mut AcceptingVerifier::default(),
                &mut NoFaults,
            )
            .is_err()
        );
    }
}
