#[test]
fn in_place_third_state_never_overwrites_user_bytes() {
    let mut store = MemoryStore::default();
    let mut fs = MemoryFs::default();
    let mut verifier = AcceptingVerifier::default();
    let mut faults = OneFault {
        target: Some(DurableBoundary::MutationCompleted {
            label: "in-place-step-0".into(),
        }),
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
    fs.steps.insert("remove-metadata".into(), StepWorld::Third);
    let error = Engine::new(&mut store, &mut fs, &mut verifier, &mut NoFaults)
        .recover("stable-project-identity", "C:/source")
        .unwrap_err();
    assert!(matches!(error, TransactionError::ThirdState(_)));
    assert_eq!(fs.steps["remove-metadata"], StepWorld::Third);
}

#[test]
fn final_health_receives_final_path_and_exact_tree_seal() {
    let mut store = MemoryStore::default();
    let mut fs = MemoryFs::default();
    let mut verifier = AcceptingVerifier::default();
    let report = execute(
        export_prepared(),
        &mut store,
        &mut fs,
        &mut verifier,
        &mut OneFault::default(),
    )
    .unwrap();
    assert_eq!(report.outcome, Outcome::Verified);
    let final_calls = verifier
        .calls
        .iter()
        .filter(|call| call.0 != VerificationPhase::Before)
        .collect::<Vec<_>>();
    assert!(final_calls.iter().all(|call| {
        call.1 == VerificationRootKind::ExportFinal
            && call.2 == "C:/delivery"
            && call.3 == hash("final")
            && !call.4
    }));
    assert_eq!(
        fs.prepared_payloads.get("README.md").map(Vec::as_slice),
        Some(b"native readme\n".as_slice())
    );
    let output = fs.output.as_ref().unwrap();
    assert!(output.iter().any(|entry| entry.path == "src/main.rs"));
    assert!(output.iter().any(|entry| entry.path == "docs/spec.md"));
    assert!(!output.iter().any(|entry| entry.path == ".git"));
}

#[test]
fn every_reachable_durable_boundary_accepts_deterministic_fault_injection() {
    #[derive(Clone, Copy)]
    enum Scenario {
        ExportSuccess,
        ExportRollback,
        ExportRace,
        InPlaceSuccess,
        InPlaceRollback,
        InPlaceExternalSuccess,
    }
    impl Scenario {
        fn prepared(self) -> PreparedTransaction {
            match self {
                Self::ExportSuccess | Self::ExportRollback | Self::ExportRace => export_prepared(),
                Self::InPlaceSuccess | Self::InPlaceRollback => complex_in_place_prepared(),
                Self::InPlaceExternalSuccess => external_preserve_prepared(),
            }
        }
        fn verifier(self) -> AcceptingVerifier {
            AcceptingVerifier {
                fail: match self {
                    Self::ExportRollback | Self::InPlaceRollback => {
                        Some(VerificationPhase::AfterHealth)
                    }
                    _ => None,
                },
                ..AcceptingVerifier::default()
            }
        }
        fn filesystem(self) -> MemoryFs {
            if matches!(self, Self::ExportRace) {
                MemoryFs {
                    output_occupied: true,
                    ..MemoryFs::default()
                }
            } else {
                MemoryFs::default()
            }
        }
    }

    let scenarios = [
        Scenario::ExportSuccess,
        Scenario::ExportRollback,
        Scenario::ExportRace,
        Scenario::InPlaceSuccess,
        Scenario::InPlaceRollback,
        Scenario::InPlaceExternalSuccess,
    ];
    let mut traces = Vec::new();
    for scenario in scenarios {
        let mut trace = TraceFaults::default();
        execute(
            scenario.prepared(),
            &mut MemoryStore::default(),
            &mut scenario.filesystem(),
            &mut scenario.verifier(),
            &mut trace,
        )
        .unwrap();
        traces.push((scenario, trace.0));
    }
    let unique = traces
        .iter()
        .flat_map(|(_, trace)| trace.iter().cloned())
        .collect::<Vec<_>>();
    assert!(
        unique
            .iter()
            .any(|b| matches!(b, DurableBoundary::SnapshotPersisted { .. }))
    );
    assert!(
        unique
            .iter()
            .any(|b| matches!(b, DurableBoundary::StepIntentPersisted { .. }))
    );
    assert!(
        unique
            .iter()
            .any(|b| matches!(b, DurableBoundary::StepCompletionPersisted { .. }))
    );
    assert!(
        unique
            .iter()
            .any(|b| matches!(b, DurableBoundary::VerificationCompleted(_)))
    );
    assert!(
        unique
            .iter()
            .any(|b| matches!(b, DurableBoundary::CleanupCompleted))
    );
    assert!(
        unique
            .iter()
            .any(|b| matches!(b, DurableBoundary::StepRollbackIntentPersisted { .. }))
    );
    assert!(
        unique
            .iter()
            .any(|b| matches!(b, DurableBoundary::StepRollbackCompletionPersisted { .. }))
    );
    assert!(unique.iter().any(|b| matches!(
        b,
        DurableBoundary::JournalPersisted(TransactionState::ContractBoundary(
            ContractBoundaryAction::ExternalPreserved
        ))
    )));

    for (scenario, boundaries) in traces {
        let mut boundaries = boundaries;
        boundaries.sort_by_key(|boundary| format!("{boundary:?}"));
        boundaries.dedup();
        for target in boundaries {
            let mut store = MemoryStore::default();
            let mut fs = scenario.filesystem();
            let mut verifier = scenario.verifier();
            let mut fault = OneFault {
                target: Some(target.clone()),
                fired: false,
            };
            let result = execute(
                scenario.prepared(),
                &mut store,
                &mut fs,
                &mut verifier,
                &mut fault,
            );
            assert!(fault.fired, "boundary was not reachable: {target:?}");
            assert!(matches!(result, Err(TransactionError::FaultInjected(_))));
            let Some(pending) = store.pending.as_ref() else {
                assert!(matches!(
                    target,
                    DurableBoundary::StoreProvedExternal | DurableBoundary::ProjectLockAcquired
                ));
                assert_eq!(fs.source_mutations, 0);
                continue;
            };
            let state_at_crash = pending.state.clone();
            let outcome_at_crash = pending.report.as_ref().map(|report| report.outcome);
            let settlement_at_crash = pending.settlement_intent;
            let recovered = recover(&mut store, &mut fs, &mut AcceptingVerifier::default())
                .unwrap_or_else(|error| {
                    panic!("recovering {target:?} from {state_at_crash:?}: {error}")
                });
            let expected = match state_at_crash {
                TransactionState::Preparing => Outcome::Refused,
                TransactionState::Verified | TransactionState::CleanupPending => Outcome::Verified,
                TransactionState::Complete => outcome_at_crash.expect("complete has report"),
                _ if settlement_at_crash == Some(Outcome::Refused) => Outcome::Refused,
                _ => Outcome::RolledBack,
            };
            assert_eq!(recovered.outcome, expected, "target {target:?}");
            assert_eq!(recovered.cleanup, Cleanup::Complete, "target {target:?}");
            assert!(!recovered.planned_mutations.is_empty());
            assert!(recovered.actual_mutations.iter().all(|actual| matches!(
                (actual.direction, actual.status),
                (MutationDirection::Apply, MutationStatus::Applied)
                    | (MutationDirection::Rollback, MutationStatus::RolledBack)
            )));
            assert!(store.pending.is_none(), "target {target:?}");
            match (scenario, expected) {
                (
                    Scenario::ExportSuccess | Scenario::ExportRollback | Scenario::ExportRace,
                    Outcome::Verified,
                ) => {
                    assert!(fs.output.is_some() && fs.candidate.is_none())
                }
                (Scenario::ExportSuccess | Scenario::ExportRollback | Scenario::ExportRace, _) => {
                    assert!(fs.output.is_none() && fs.candidate.is_none())
                }
                (
                    Scenario::InPlaceSuccess
                    | Scenario::InPlaceRollback
                    | Scenario::InPlaceExternalSuccess,
                    Outcome::Verified,
                ) => {
                    assert!(fs.steps.values().all(|state| *state == StepWorld::After));
                    assert!(!fs.quarantine);
                }
                (
                    Scenario::InPlaceSuccess
                    | Scenario::InPlaceRollback
                    | Scenario::InPlaceExternalSuccess,
                    _,
                ) => {
                    assert!(fs.steps.values().all(|state| *state == StepWorld::Before));
                    assert!(!fs.quarantine);
                }
            }
        }
    }
}

#[test]
fn cleanup_pending_survives_and_recovery_only_rolls_forward() {
    let mut store = MemoryStore::default();
    let mut fs = MemoryFs {
        cleanup_failures: 1,
        ..MemoryFs::default()
    };
    let mut verifier = AcceptingVerifier::default();
    let report = execute(
        in_place_prepared(),
        &mut store,
        &mut fs,
        &mut verifier,
        &mut OneFault::default(),
    )
    .unwrap();
    assert_eq!(report.outcome, Outcome::Verified);
    assert_eq!(report.cleanup, Cleanup::Pending);
    assert_eq!(
        store.pending.as_ref().unwrap().state,
        TransactionState::CleanupPending
    );
    assert_eq!(fs.steps["remove-metadata"], StepWorld::After);
    assert_eq!(fs.steps["contract-last"], StepWorld::After);
    let pending_events = report.events.clone();

    let recovered = Engine::new(&mut store, &mut fs, &mut verifier, &mut NoFaults)
        .recover("stable-project-identity", "C:/source")
        .unwrap();
    assert_eq!(recovered.outcome, Outcome::Verified);
    assert_eq!(recovered.cleanup, Cleanup::Complete);
    assert_eq!(
        &recovered.events[..pending_events.len()],
        pending_events.as_slice(),
        "cleanup-pending evidence must remain an append-only prefix"
    );
    assert_eq!(fs.steps["contract-last"], StepWorld::After);
    assert!(!fs.quarantine);
}

#[test]
fn cleanup_syscall_before_completion_checkpoint_is_replayed_from_exact_intent() {
    let mut store = MemoryStore::default();
    let mut fs = MemoryFs::default();
    let mut fault = OneFault {
        target: Some(DurableBoundary::CleanupMutationCompleted {
            progress_key: "root".into(),
        }),
        fired: false,
    };
    let error = execute(
        in_place_prepared(),
        &mut store,
        &mut fs,
        &mut AcceptingVerifier::default(),
        &mut fault,
    )
    .unwrap_err();
    assert!(matches!(error, TransactionError::FaultInjected(_)));
    assert!(!fs.quarantine, "the root syscall occurred before the crash");
    let pending = store.pending.as_ref().unwrap();
    assert_eq!(
        pending
            .cleanup_wal
            .as_ref()
            .and_then(|wal| wal.active.as_ref())
            .map(|intent| intent.progress_key.as_str()),
        Some("root")
    );

    let report = recover(&mut store, &mut fs, &mut AcceptingVerifier::default()).unwrap();
    assert_eq!(report.outcome, Outcome::Verified);
    assert_eq!(report.cleanup, Cleanup::Complete);
    assert!(
        report
            .events
            .iter()
            .any(|event| event.contains("recovered completed syscall for `root`"))
    );
}

#[test]
fn recovery_uses_journal_without_any_contract_source_input() {
    let mut store = MemoryStore::default();
    let mut fs = MemoryFs::default();
    let mut verifier = AcceptingVerifier::default();
    let mut faults = OneFault {
        target: Some(DurableBoundary::MutationCompleted {
            label: "in-place-step-0".into(),
        }),
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
    let contract_record = store
        .pending
        .as_ref()
        .unwrap()
        .snapshots
        .iter()
        .find(|record| record.kind == SnapshotKind::Contract)
        .unwrap();
    assert_eq!(contract_record.sha256, digest(b"contract bytes"));
    Engine::new(&mut store, &mut fs, &mut verifier, &mut NoFaults)
        .recover("stable-project-identity", "C:/source")
        .unwrap();
}
