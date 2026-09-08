#[test]
fn project_key_is_deterministic_and_domain_separated() {
    assert_eq!(project_key("same"), project_key("same"));
    assert_ne!(project_key("same"), project_key("different"));
    assert_eq!(
        hash("abc").0,
        "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn snapshots_and_prepared_journal_are_durable_before_export_mutation() {
    let mut store = MemoryStore::default();
    let mut fs = MemoryFs::default();
    let mut verifier = AcceptingVerifier::default();
    let mut faults = OneFault {
        target: Some(DurableBoundary::MutationCompleted {
            label: "export-candidate-create".into(),
        }),
        fired: false,
    };
    let error = execute(
        export_prepared(),
        &mut store,
        &mut fs,
        &mut verifier,
        &mut faults,
    )
    .unwrap_err();
    assert!(matches!(error, TransactionError::FaultInjected(_)));
    let first_journal = store
        .events
        .iter()
        .position(|event| event == "journal/Prepared")
        .unwrap();
    assert_eq!(
        store.events[..first_journal]
            .iter()
            .filter(|event| event.starts_with("snapshot/"))
            .count(),
        5
    );
    assert_eq!(fs.source_mutations, 0, "export never mutates source");
}

#[test]
fn export_restart_after_publication_rolls_back_exact_output() {
    let mut store = MemoryStore::default();
    let mut fs = MemoryFs::default();
    let mut verifier = AcceptingVerifier::default();
    let mut faults = OneFault {
        target: Some(DurableBoundary::MutationCompleted {
            label: "export-publish".into(),
        }),
        fired: false,
    };
    execute(
        export_prepared(),
        &mut store,
        &mut fs,
        &mut verifier,
        &mut faults,
    )
    .unwrap_err();
    assert!(fs.output.is_some() && fs.candidate.is_none());
    assert_eq!(
        store.pending.as_ref().unwrap().state,
        TransactionState::Candidate
    );

    let report = Engine::new(&mut store, &mut fs, &mut verifier, &mut NoFaults)
        .recover("stable-project-identity", "C:/source")
        .unwrap();
    assert_eq!(report.outcome, Outcome::RolledBack);
    assert!(fs.output.is_none() && fs.candidate.is_none());
    assert_eq!(fs.source_mutations, 0);
}

#[test]
fn ordinary_before_health_errors_terminal_refuse_without_pending_mutation() {
    for prepared in [export_prepared(), complex_in_place_prepared()] {
        let mut store = MemoryStore::default();
        let mut fs = MemoryFs::default();
        let mut verifier = AcceptingVerifier {
            error: Some(VerificationPhase::Before),
            ..AcceptingVerifier::default()
        };
        let report = execute(prepared, &mut store, &mut fs, &mut verifier, &mut NoFaults).unwrap();
        assert_eq!(report.outcome, Outcome::Refused);
        assert_eq!(report.cleanup, Cleanup::Complete);
        assert!(store.pending.is_none());
        assert_eq!(fs.source_mutations, 0);
        assert!(!fs.quarantine && fs.candidate.is_none() && fs.output.is_none());
    }
}

#[test]
fn export_third_state_refuses_without_overwriting_concurrent_data() {
    let mut store = MemoryStore::default();
    let mut fs = MemoryFs::default();
    let mut verifier = AcceptingVerifier::default();
    let mut faults = OneFault {
        target: Some(DurableBoundary::MutationCompleted {
            label: "export-publish".into(),
        }),
        fired: false,
    };
    execute(
        export_prepared(),
        &mut store,
        &mut fs,
        &mut verifier,
        &mut faults,
    )
    .unwrap_err();
    fs.output_third = true;
    let error = Engine::new(&mut store, &mut fs, &mut verifier, &mut NoFaults)
        .recover("stable-project-identity", "C:/source")
        .unwrap_err();
    assert!(matches!(error, TransactionError::ThirdState(_)));
    assert!(fs.output.is_some(), "third-state output remains untouched");
    assert_eq!(
        store.pending.as_ref().unwrap().state,
        TransactionState::RollbackFailed
    );
}

#[test]
fn published_output_disappearance_is_third_state_not_successful_rollback() {
    let mut store = MemoryStore::default();
    let mut fs = MemoryFs::default();
    let prepared = export_prepared();
    let publish_index = match &prepared.mode {
        PreparedMode::Export(plan) => plan.entries.len() + 1,
        _ => unreachable!(),
    };
    let mut fault = OneFault {
        target: Some(DurableBoundary::StepCompletionPersisted {
            index: publish_index,
            id: "export/publish".into(),
        }),
        fired: false,
    };
    execute(
        prepared,
        &mut store,
        &mut fs,
        &mut AcceptingVerifier::default(),
        &mut fault,
    )
    .unwrap_err();
    fs.output = None;
    let error = recover(&mut store, &mut fs, &mut AcceptingVerifier::default()).unwrap_err();
    assert!(matches!(error, TransactionError::ThirdState(_)));
}

#[test]
fn rollback_failed_recovery_republishes_embedded_report_and_retries_store_failure() {
    let mut store = MemoryStore::default();
    let mut fs = MemoryFs::default();
    let mut crash = OneFault {
        target: Some(DurableBoundary::MutationCompleted {
            label: "export-publish".into(),
        }),
        fired: false,
    };
    execute(
        export_prepared(),
        &mut store,
        &mut fs,
        &mut AcceptingVerifier::default(),
        &mut crash,
    )
    .unwrap_err();
    fs.output_third = true;
    let mut rollback_failed_crash = OneFault {
        target: Some(DurableBoundary::JournalPersisted(
            TransactionState::RollbackFailed,
        )),
        fired: false,
    };
    let first = Engine::new(
        &mut store,
        &mut fs,
        &mut AcceptingVerifier::default(),
        &mut rollback_failed_crash,
    )
    .recover("stable-project-identity", "C:/source");
    assert!(matches!(first, Err(TransactionError::FaultInjected(_))));
    assert_eq!(
        store
            .pending
            .as_ref()
            .unwrap()
            .report
            .as_ref()
            .unwrap()
            .outcome,
        Outcome::RollbackFailed
    );
    store.fail_report_once = true;
    assert!(matches!(
        recover(&mut store, &mut fs, &mut AcceptingVerifier::default()),
        Err(TransactionError::Store(_))
    ));
    assert!(matches!(
        recover(&mut store, &mut fs, &mut AcceptingVerifier::default()),
        Err(TransactionError::ThirdState(_))
    ));
    assert_eq!(
        store.reports.last().unwrap().outcome,
        Outcome::RollbackFailed
    );
}

#[test]
fn complete_partial_preparation_retires_without_ephemeral_snapshot_files() {
    let mut store = MemoryStore::default();
    let mut fs = MemoryFs::default();
    let mut data_crash = OneFault {
        target: Some(DurableBoundary::SnapshotDataPersisted { index: 0 }),
        fired: false,
    };
    execute(
        export_prepared(),
        &mut store,
        &mut fs,
        &mut AcceptingVerifier::default(),
        &mut data_crash,
    )
    .unwrap_err();
    let mut terminal_crash = OneFault {
        target: Some(DurableBoundary::JournalPersisted(
            TransactionState::Complete,
        )),
        fired: false,
    };
    let first = Engine::new(
        &mut store,
        &mut fs,
        &mut AcceptingVerifier::default(),
        &mut terminal_crash,
    )
    .recover("stable-project-identity", "C:/source");
    assert!(matches!(first, Err(TransactionError::FaultInjected(_))));
    store.snapshots.clear();
    let report = recover(&mut store, &mut fs, &mut AcceptingVerifier::default()).unwrap();
    assert_eq!(report.outcome, Outcome::Refused);
    assert!(store.pending.is_none());
}

#[test]
fn destination_race_is_refused_and_raced_occupant_survives() {
    let mut store = MemoryStore::default();
    let mut fs = MemoryFs {
        output_occupied: true,
        ..MemoryFs::default()
    };
    let report = execute(
        export_prepared(),
        &mut store,
        &mut fs,
        &mut AcceptingVerifier::default(),
        &mut OneFault::default(),
    )
    .unwrap();
    assert_eq!(report.outcome, Outcome::Refused);
    assert!(fs.output_occupied);
    assert!(fs.candidate.is_none());
    assert_eq!(fs.source_mutations, 0);
}

#[test]
fn unavailable_atomic_noreplace_publication_refuses_and_cleans_candidate() {
    let mut fs = MemoryFs {
        publish_unsupported: true,
        ..MemoryFs::default()
    };
    let report = execute(
        export_prepared(),
        &mut MemoryStore::default(),
        &mut fs,
        &mut AcceptingVerifier::default(),
        &mut NoFaults,
    )
    .unwrap();
    assert_eq!(report.outcome, Outcome::Refused);
    assert!(fs.candidate.is_none() && fs.output.is_none());
}

#[test]
fn contract_boundary_is_rollback_capable_and_contract_is_restored() {
    let mut store = MemoryStore::default();
    let mut fs = MemoryFs::default();
    let mut verifier = AcceptingVerifier::default();
    let mut faults = OneFault {
        target: Some(DurableBoundary::JournalPersisted(
            TransactionState::ContractBoundary(ContractBoundaryAction::DeleteLastMoved),
        )),
        fired: false,
    };
    execute(
        in_place_prepared(),
        &mut store,
        &mut fs,
        &mut verifier,
        &mut faults,
    )
    .unwrap_err();
    assert_eq!(
        store.pending.as_ref().unwrap().state,
        TransactionState::ContractBoundary(ContractBoundaryAction::DeleteLastMoved)
    );
    assert_eq!(fs.steps.get("contract-last"), Some(&StepWorld::After));
    let report = Engine::new(&mut store, &mut fs, &mut verifier, &mut NoFaults)
        .recover("stable-project-identity", "C:/source")
        .unwrap();
    assert_eq!(report.outcome, Outcome::RolledBack);
    assert_eq!(fs.steps.get("contract-last"), Some(&StepWorld::Before));
    assert_eq!(fs.steps.get("remove-metadata"), Some(&StepWorld::Before));
}

#[test]
fn quarantine_creation_crash_is_inferred_and_reported_in_both_directions() {
    let mut store = MemoryStore::default();
    let mut fs = MemoryFs::default();
    let mut fault = OneFault {
        target: Some(DurableBoundary::MutationCompleted {
            label: "quarantine-create".into(),
        }),
        fired: false,
    };
    execute(
        in_place_prepared(),
        &mut store,
        &mut fs,
        &mut AcceptingVerifier::default(),
        &mut fault,
    )
    .unwrap_err();
    assert!(fs.quarantine);
    let report = recover(&mut store, &mut fs, &mut AcceptingVerifier::default()).unwrap();
    assert_eq!(report.outcome, Outcome::RolledBack);
    assert!(!fs.quarantine);
    assert!(report.actual_mutations.iter().any(|actual| {
        actual.id == "in-place/quarantine"
            && actual.direction == MutationDirection::Apply
            && actual.status == MutationStatus::Applied
    }));
    assert!(report.actual_mutations.iter().any(|actual| {
        actual.id == "in-place/quarantine"
            && actual.direction == MutationDirection::Rollback
            && actual.status == MutationStatus::RolledBack
    }));
}
