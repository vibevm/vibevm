#[test]
fn phase_view_must_equal_expected_seal_before_child_execution() {
    let mut before_verifier = AcceptingVerifier {
        drift_view: Some(VerificationPhase::Before),
        ..AcceptingVerifier::default()
    };
    let mut store = MemoryStore::default();
    let mut fs = MemoryFs::default();
    let report = execute(
        export_prepared(),
        &mut store,
        &mut fs,
        &mut before_verifier,
        &mut NoFaults,
    )
    .unwrap();
    assert_eq!(report.outcome, Outcome::Refused);
    assert_eq!(before_verifier.execute_calls, 0);
    assert_eq!(fs.source_mutations, 0);

    let mut after_verifier = AcceptingVerifier {
        drift_view: Some(VerificationPhase::AfterHealth),
        ..AcceptingVerifier::default()
    };
    let mut store = MemoryStore::default();
    let mut fs = MemoryFs::default();
    let report = execute(
        complex_in_place_prepared(),
        &mut store,
        &mut fs,
        &mut after_verifier,
        &mut NoFaults,
    )
    .unwrap();
    assert_eq!(report.outcome, Outcome::RolledBack);
    assert!(fs.steps.values().all(|state| *state == StepWorld::Before));

    let mut source_drift = AcceptingVerifier {
        drift_real: Some(VerificationPhase::SourceUnchanged),
        ..AcceptingVerifier::default()
    };
    let mut fs = MemoryFs::default();
    let error = execute(
        export_prepared(),
        &mut MemoryStore::default(),
        &mut fs,
        &mut source_drift,
        &mut NoFaults,
    )
    .unwrap_err();
    assert!(matches!(error, TransactionError::ThirdState(_)));
    assert!(fs.output.is_none() && fs.candidate.is_none());
}

#[test]
fn report_and_retire_store_failures_recover_only_in_the_durable_direction() {
    for mut store in [
        MemoryStore {
            fail_report_once: true,
            ..MemoryStore::default()
        },
        MemoryStore {
            fail_retire_once: true,
            ..MemoryStore::default()
        },
    ] {
        let mut fs = MemoryFs::default();
        execute(
            export_prepared(),
            &mut store,
            &mut fs,
            &mut AcceptingVerifier::default(),
            &mut NoFaults,
        )
        .unwrap_err();
        assert!(matches!(
            store.pending.as_ref().unwrap().state,
            TransactionState::Verified
                | TransactionState::CleanupPending
                | TransactionState::Complete
        ));
        let report = recover(&mut store, &mut fs, &mut AcceptingVerifier::default()).unwrap();
        assert_eq!(report.outcome, Outcome::Verified);
        assert_eq!(report.cleanup, Cleanup::Complete);
        assert!(fs.output.is_some());
    }
}

#[test]
fn terminal_journal_with_embedded_report_precedes_stable_copy_and_retire() {
    let mut store = MemoryStore::default();
    let report = execute(
        export_prepared(),
        &mut store,
        &mut MemoryFs::default(),
        &mut AcceptingVerifier::default(),
        &mut NoFaults,
    )
    .unwrap();
    let terminal = store
        .events
        .iter()
        .rposition(|event| event == "journal/Complete")
        .unwrap();
    let stable = store
        .events
        .iter()
        .rposition(|event| event == "report")
        .unwrap();
    let retire = store
        .events
        .iter()
        .rposition(|event| event == "retire")
        .unwrap();
    assert!(terminal < stable && stable < retire);
    assert_eq!(store.reports.last(), Some(&report));
}

#[test]
fn journal_store_failure_after_candidate_staging_recovers_from_prior_durable_state() {
    let mut store = MemoryStore {
        fail_journal_state: Some(TransactionState::Candidate),
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
    assert_eq!(
        store.pending.as_ref().unwrap().state,
        TransactionState::Prepared
    );
    assert!(fs.candidate.is_some());
    let report = recover(&mut store, &mut fs, &mut AcceptingVerifier::default()).unwrap();
    assert_eq!(report.outcome, Outcome::RolledBack);
    assert!(fs.candidate.is_none() && fs.output.is_none());
}

#[test]
fn every_recovery_boundary_restarts_to_the_same_exact_rollback_report() {
    fn interrupted() -> (MemoryStore, MemoryFs) {
        let mut store = MemoryStore::default();
        let mut fs = MemoryFs::default();
        let mut crash = OneFault {
            target: Some(DurableBoundary::MutationCompleted {
                label: "in-place-step-3".into(),
            }),
            fired: false,
        };
        execute(
            complex_in_place_prepared(),
            &mut store,
            &mut fs,
            &mut AcceptingVerifier::default(),
            &mut crash,
        )
        .unwrap_err();
        (store, fs)
    }

    let (mut store, mut fs) = interrupted();
    let mut trace = TraceFaults::default();
    Engine::new(
        &mut store,
        &mut fs,
        &mut AcceptingVerifier::default(),
        &mut trace,
    )
    .recover("stable-project-identity", "C:/source")
    .unwrap();
    let mut boundaries = trace.0;
    boundaries.sort_by_key(|boundary| format!("{boundary:?}"));
    boundaries.dedup();
    assert!(boundaries.iter().any(|boundary| matches!(
        boundary,
        DurableBoundary::StepRollbackIntentPersisted { .. }
    )));

    for target in boundaries {
        let (mut store, mut fs) = interrupted();
        let mut fault = OneFault {
            target: Some(target.clone()),
            fired: false,
        };
        let first = Engine::new(
            &mut store,
            &mut fs,
            &mut AcceptingVerifier::default(),
            &mut fault,
        )
        .recover("stable-project-identity", "C:/source");
        assert!(matches!(first, Err(TransactionError::FaultInjected(_))));
        assert!(fault.fired, "unreached recovery boundary {target:?}");
        let report = recover(&mut store, &mut fs, &mut AcceptingVerifier::default()).unwrap();
        assert_eq!(report.outcome, Outcome::RolledBack, "{target:?}");
        assert_eq!(report.cleanup, Cleanup::Complete, "{target:?}");
        assert!(fs.steps.values().all(|state| *state == StepWorld::Before));
        assert!(!fs.quarantine);
        assert!(store.pending.is_none());
    }
}

#[cfg(windows)]
#[test]
fn production_store_and_safefs_restart_recover_a_real_applied_export_step() {
    use std::fs;

    let scope = tempfile::tempdir().unwrap();
    let source = scope.path().join("source");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("kept.txt"), b"kept").unwrap();
    let output = scope.path().join("output");
    let state_root = scope.path().join("external-state");

    let source_project = vibe_safefs::Project::open(&source).unwrap();
    let project_identity = source_project.identity_token().unwrap();
    let key = project_key(&project_identity);
    let source_tree = observe_real_tree(&source).unwrap();
    let file = source_tree
        .entries
        .iter()
        .find(|entry| entry.path == "kept.txt")
        .unwrap()
        .clone();
    let output_parent = vibe_safefs::Project::open(scope.path()).unwrap();
    let output_slot = vibe_safefs::Project::pin_absent_path(&output).unwrap();
    let mut wire_plan: vibe_wire::generated::scrape::e1::plan::Plan =
        serde_json::from_slice(&canonical_plan(TransactionMode::Export)).unwrap();
    wire_plan.project.display_root = source.display().to_string();
    wire_plan.project.tree_digest = source_tree.digest.0.clone();
    let canonical = serde_json::to_vec(&wire_plan).unwrap();
    let snapshots = vec![
        Snapshot {
            kind: SnapshotKind::Contract,
            name: "contract".into(),
            bytes: b"contract".to_vec(),
            mode: None,
        },
        Snapshot {
            kind: SnapshotKind::CanonicalContract,
            name: "canonical-contract".into(),
            bytes: b"canonical-contract".to_vec(),
            mode: None,
        },
        Snapshot {
            kind: SnapshotKind::CanonicalPlan,
            name: "plan".into(),
            bytes: canonical.clone(),
            mode: None,
        },
        Snapshot {
            kind: SnapshotKind::Verifier,
            name: "verifier".into(),
            bytes: b"verifier".to_vec(),
            mode: None,
        },
    ];
    let prepared = PreparedTransaction {
        project_identity_token: project_identity.clone(),
        project_display_root: source.display().to_string(),
        plan_id: hash("plan"),
        canonical_plan: canonical,
        snapshots,
        mode: PreparedMode::Export(Box::new(ExportPlan {
            output_identity: output_slot.identity_token(),
            output_parent_identity: output_parent.identity_token().unwrap(),
            output_display_path: output.display().to_string(),
            output_name: "output".into(),
            before_same_display_path: false,
            after_same_display_path: false,
            entries: vec![ExportEntry {
                target_path: "kept.txt".into(),
                kind: TreeEntryKind::File,
                mode: file.mode,
                payload: Some(ExportPayload::Source {
                    source_path: "kept.txt".into(),
                    before: FileState {
                        sha256: file.sha256.clone().unwrap(),
                        bytes: file.bytes.unwrap(),
                        mode: file.mode,
                    },
                }),
            }],
            source_tree: source_tree.clone(),
            final_manifest: source_tree.clone(),
        })),
    };

    let mut store = SystemTransactionStore::new(&state_root).unwrap();
    let mut filesystem = SafefsTransactionFilesystem::for_prepared(&prepared).unwrap();
    let mut verifier = RealTreeVerifier;
    let mut fault = OneFault {
        target: Some(DurableBoundary::OwnedTreeMutationBeforeReseal {
            label: "export-entry-0".into(),
        }),
        fired: false,
    };
    let error = Engine::new(&mut store, &mut filesystem, &mut verifier, &mut fault)
        .execute_locked(&project_identity, &source.display().to_string(), || {
            Ok(prepared)
        })
        .unwrap_err();
    assert!(matches!(error, TransactionError::FaultInjected(_)));
    let transaction = store.pending(&key).unwrap().unwrap().transaction_id;
    drop(filesystem);
    drop(store);

    let mut restarted_store = SystemTransactionStore::new(&state_root).unwrap();
    let mut restarted_filesystem =
        SafefsTransactionFilesystem::open(&source, &project_identity).unwrap();
    let report = Engine::new(
        &mut restarted_store,
        &mut restarted_filesystem,
        &mut RealTreeVerifier,
        &mut NoFaults,
    )
    .recover(&project_identity, &source.display().to_string())
    .unwrap();
    assert_eq!(report.outcome, Outcome::RolledBack);
    assert_eq!(fs::read(source.join("kept.txt")).unwrap(), b"kept");
    assert!(!output.exists());
    assert!(
        !scope
            .path()
            .join(format!(".vibe-scrape-candidate-{}", transaction.0))
            .exists()
    );
    let stable_report = state_root
        .join("reports")
        .join(format!("{}.json", transaction.0));
    let wire: serde_json::Value =
        serde_json::from_slice(&fs::read(stable_report).unwrap()).unwrap();
    assert_eq!(wire["outcome"], "rolled-back");
    assert!(
        !state_root
            .join("t")
            .join(&key.0)
            .join(&transaction.0)
            .exists()
    );
}

#[cfg(windows)]
#[test]
fn production_engine_recovers_an_export_file_staged_before_publication() {
    use std::fs;

    let scope = tempfile::tempdir().unwrap();
    let source = scope.path().join("source-stage");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("kept.txt"), b"kept").unwrap();
    let output = scope.path().join("output-stage");
    let state_root = scope.path().join("external-stage-state");

    let source_project = vibe_safefs::Project::open(&source).unwrap();
    let project_identity = source_project.identity_token().unwrap();
    let key = project_key(&project_identity);
    let source_tree = observe_real_tree(&source).unwrap();
    let file = source_tree
        .entries
        .iter()
        .find(|entry| entry.path == "kept.txt")
        .unwrap()
        .clone();
    let output_parent = vibe_safefs::Project::open(scope.path()).unwrap();
    let output_slot = vibe_safefs::Project::pin_absent_path(&output).unwrap();
    let mut wire_plan: vibe_wire::generated::scrape::e1::plan::Plan =
        serde_json::from_slice(&canonical_plan(TransactionMode::Export)).unwrap();
    wire_plan.project.display_root = source.display().to_string();
    wire_plan.project.tree_digest = source_tree.digest.0.clone();
    let canonical = serde_json::to_vec(&wire_plan).unwrap();
    let prepared = PreparedTransaction {
        project_identity_token: project_identity.clone(),
        project_display_root: source.display().to_string(),
        plan_id: hash("plan"),
        canonical_plan: canonical.clone(),
        snapshots: vec![
            Snapshot {
                kind: SnapshotKind::Contract,
                name: "contract".into(),
                bytes: b"contract".to_vec(),
                mode: None,
            },
            Snapshot {
                kind: SnapshotKind::CanonicalContract,
                name: "canonical-contract".into(),
                bytes: b"canonical-contract".to_vec(),
                mode: None,
            },
            Snapshot {
                kind: SnapshotKind::CanonicalPlan,
                name: "plan".into(),
                bytes: canonical,
                mode: None,
            },
            Snapshot {
                kind: SnapshotKind::Verifier,
                name: "verifier".into(),
                bytes: b"verifier".to_vec(),
                mode: None,
            },
        ],
        mode: PreparedMode::Export(Box::new(ExportPlan {
            output_identity: output_slot.identity_token(),
            output_parent_identity: output_parent.identity_token().unwrap(),
            output_display_path: output.display().to_string(),
            output_name: "output-stage".into(),
            before_same_display_path: false,
            after_same_display_path: false,
            entries: vec![ExportEntry {
                target_path: "kept.txt".into(),
                kind: TreeEntryKind::File,
                mode: file.mode,
                payload: Some(ExportPayload::Source {
                    source_path: "kept.txt".into(),
                    before: FileState {
                        sha256: file.sha256.clone().unwrap(),
                        bytes: file.bytes.unwrap(),
                        mode: file.mode,
                    },
                }),
            }],
            source_tree: source_tree.clone(),
            final_manifest: source_tree,
        })),
    };

    let mut store = SystemTransactionStore::new(&state_root).unwrap();
    let mut filesystem = SafefsTransactionFilesystem::for_prepared(&prepared).unwrap();
    let mut fault = OneFault {
        target: Some(DurableBoundary::StepIntentPersisted {
            index: 0,
            id: "export/entry/0/kept.txt".into(),
        }),
        fired: false,
    };
    let error = Engine::new(
        &mut store,
        &mut filesystem,
        &mut RealTreeVerifier,
        &mut fault,
    )
    .execute_locked(&project_identity, &source.display().to_string(), || {
        Ok(prepared)
    })
    .unwrap_err();
    assert!(matches!(error, TransactionError::FaultInjected(_)));
    assert!(fault.fired);
    let journal = store.pending(&key).unwrap().unwrap();
    let transaction = journal.transaction_id.clone();
    let candidate = journal.candidate_name.clone().unwrap();
    let owner = journal.owned_tree_token.clone().unwrap();
    assert_eq!(journal.active_step, Some(0));
    drop(filesystem);

    let stage_name = super::safefs::transaction_stage_name(&owner, "export:kept.txt", "kept.txt");
    fs::write(scope.path().join(&candidate).join(&stage_name), b"kept").unwrap();
    drop(store);

    let mut restarted_store = SystemTransactionStore::new(&state_root).unwrap();
    let mut restarted_filesystem =
        SafefsTransactionFilesystem::open(&source, &project_identity).unwrap();
    let report = Engine::new(
        &mut restarted_store,
        &mut restarted_filesystem,
        &mut RealTreeVerifier,
        &mut NoFaults,
    )
    .recover(&project_identity, &source.display().to_string())
    .unwrap();
    assert_eq!(report.outcome, Outcome::RolledBack);
    assert_eq!(report.cleanup, Cleanup::Complete);
    assert_eq!(fs::read(source.join("kept.txt")).unwrap(), b"kept");
    assert!(!output.exists());
    assert!(!scope.path().join(candidate).exists());
    assert!(
        !state_root
            .join("t")
            .join(&key.0)
            .join(&transaction.0)
            .exists()
    );
    let stable_report = state_root
        .join("reports")
        .join(format!("{}.json", transaction.0));
    let wire: serde_json::Value =
        serde_json::from_slice(&fs::read(stable_report).unwrap()).unwrap();
    assert_eq!(wire["outcome"], "rolled-back");
}
