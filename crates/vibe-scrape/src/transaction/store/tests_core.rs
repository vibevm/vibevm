    use std::fs;

    use tempfile::TempDir;

    use super::*;

    #[cfg(windows)]
    struct Fixture {
        _scope: TempDir,
        project_root: PathBuf,
        store: SystemTransactionStore,
        key: ProjectKey,
        journal: Journal,
        snapshot_bytes: Vec<Vec<u8>>,
    }

    #[cfg(windows)]
    fn fixture() -> Fixture {
        let scope = tempfile::tempdir().unwrap();
        let project_root = scope.path().join("project");
        fs::create_dir(&project_root).unwrap();
        let state_root = scope.path().join("external-state");
        let key = derive_project_key(
            &Project::open(&project_root)
                .unwrap()
                .identity_token()
                .unwrap(),
        );
        let plan_id = Digest(format!("sha256:{}", "2".repeat(64)));
        let canonical_plan = canonical_plan_bytes(&project_root, &plan_id);
        let snapshot_bytes = vec![
            b"contract".to_vec(),
            b"canonical-contract".to_vec(),
            canonical_plan.clone(),
        ];
        let snapshots = [
            (SnapshotKind::Contract, "contract"),
            (SnapshotKind::CanonicalContract, "canonical-contract"),
            (SnapshotKind::CanonicalPlan, "canonical-plan"),
        ]
        .into_iter()
        .zip(&snapshot_bytes)
        .map(|((kind, name), bytes)| SnapshotRecord {
            kind,
            name: name.to_owned(),
            sha256: sha256(bytes),
            bytes: bytes.len() as u64,
            mode: None,
        })
        .collect();
        let tree = TreeManifest {
            digest: Digest(format!("sha256:{}", "0".repeat(64))),
            entries: Vec::new(),
        };
        let mut journal = Journal {
            schema: JOURNAL_EPOCH,
            revision: 0,
            project_key: key.clone(),
            transaction_id: TransactionId("TX000001".to_owned()),
            mode: TransactionMode::Export,
            plan_id,
            project_display_root: project_root.display().to_string(),
            canonical_plan,
            verification_workspace: None,
            execution: PreparedMode::Export(Box::new(ExportPlan {
                output_identity: "output-identity".to_owned(),
                output_parent_identity: "output-parent-identity".to_owned(),
                output_display_path: project_root.join("output").display().to_string(),
                output_name: "output".to_owned(),
                before_same_display_path: false,
                after_same_display_path: false,
                entries: Vec::new(),
                source_tree: tree.clone(),
                final_manifest: tree,
            })),
            state: TransactionState::Preparing,
            snapshots,
            snapshots_persisted: 0,
            snapshot_active: None,
            candidate_name: None,
            quarantine_name: None,
            owned_tree_token: None,
            owned_tree_seal: None,
            cleanup_wal: None,
            completed_steps: 0,
            active_step: None,
            mutation_progress: vec![
                MutationProgress {
                    id: "export/candidate".to_owned(),
                    kind: PlannedMutationKind::ExportCandidateCreate,
                    status: MutationStatus::Planned,
                },
                MutationProgress {
                    id: "export/publish".to_owned(),
                    kind: PlannedMutationKind::ExportPublish,
                    status: MutationStatus::Planned,
                },
            ],
            actual_mutations: Vec::new(),
            settlement_intent: None,
            delivered_tree: None,
            verification: Vec::new(),
            events: Vec::new(),
            report: None,
        };
        let mut store = SystemTransactionStore::new(state_root).unwrap();
        store
            .prove_outside_project(&journal.project_display_root)
            .unwrap();
        store.lock_project(&key).unwrap();
        journal.verification_workspace =
            Some(store.verification_workspace_intent(&journal).unwrap());
        store.create_transaction(&journal).unwrap();
        Fixture {
            _scope: scope,
            project_root,
            store,
            key,
            journal,
            snapshot_bytes,
        }
    }

    #[cfg(windows)]
    fn canonical_plan_bytes(project_root: &Path, plan_id: &Digest) -> Vec<u8> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../formats/corpora/scrape/e1/valid/plan-minimal.json");
        let mut plan: ScrapePlanWire = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        plan.plan_id = plan_id.0.clone();
        plan.project.display_root = project_root.display().to_string();
        plan.mode = vibe_wire::generated::scrape::e1::plan::Mode::Export;
        serde_json::to_vec(&plan).unwrap()
    }

    #[cfg(windows)]
    fn persist_all_snapshots(fixture: &mut Fixture) {
        for index in 0..fixture.snapshot_bytes.len() {
            fixture.journal.snapshot_active = Some(index);
            fixture.journal.revision += 1;
            fixture.store.persist_journal(&fixture.journal).unwrap();
            fixture
                .store
                .persist_snapshot(
                    &fixture.journal.transaction_id,
                    &fixture.journal.snapshots[index],
                    &fixture.snapshot_bytes[index],
                )
                .unwrap();
            fixture.journal.snapshots_persisted = index + 1;
            fixture.journal.snapshot_active = None;
            fixture.journal.revision += 1;
            fixture.store.persist_journal(&fixture.journal).unwrap();
        }
        fixture.journal.state = TransactionState::Prepared;
        fixture.journal.revision += 1;
        fixture.store.persist_journal(&fixture.journal).unwrap();
    }

    #[cfg(windows)]
    fn refused_report(journal: &Journal) -> TransactionReport {
        TransactionReport {
            project_key: journal.project_key.clone(),
            transaction_id: journal.transaction_id.clone(),
            plan_id: journal.plan_id.clone(),
            mode: journal.mode,
            outcome: Outcome::Refused,
            assurance: Assurance::Full,
            cleanup: Cleanup::Complete,
            before_tree: Some(match &journal.execution {
                PreparedMode::Export(plan) => plan.source_tree.digest.clone(),
                PreparedMode::InPlace(plan) => plan.before_tree.digest.clone(),
            }),
            after_tree: None,
            snapshots: journal.snapshots.clone(),
            verification: journal.verification.clone(),
            planned_mutations: journal
                .mutation_progress
                .iter()
                .map(|progress| PlannedMutationEvidence {
                    id: progress.id.clone(),
                    kind: progress.kind,
                })
                .collect(),
            actual_mutations: journal.actual_mutations.clone(),
            events: Vec::new(),
        }
    }

    #[cfg(windows)]
    fn make_terminal(fixture: &mut Fixture) {
        persist_all_snapshots(fixture);
        let report = refused_report(&fixture.journal);
        fixture.journal.state = TransactionState::Complete;
        fixture.journal.settlement_intent = Some(Outcome::Refused);
        fixture.journal.report = Some(report);
        fixture.journal.revision += 1;
        fixture.store.persist_journal(&fixture.journal).unwrap();
        let report = fixture.journal.report.clone().unwrap();
        let canonical = fixture
            .store
            .canonical_report_bytes(&fixture.journal, &report)
            .unwrap();
        fixture.store.persist_report(&report, &canonical).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn durable_snapshots_and_journal_survive_store_restart() {
        let mut fixture = fixture();
        persist_all_snapshots(&mut fixture);
        let root = fixture.store.state_root().to_path_buf();
        drop(fixture.store);

        let mut restarted = SystemTransactionStore::new(root).unwrap();
        restarted
            .prove_outside_project(&fixture.project_root.display().to_string())
            .unwrap();
        restarted.lock_project(&fixture.key).unwrap();
        let recovered = restarted.pending(&fixture.key).unwrap().unwrap();
        assert_eq!(recovered, fixture.journal);
        assert_eq!(
            restarted
                .read_snapshot(&recovered, "canonical-plan")
                .unwrap(),
            fixture.snapshot_bytes[2]
        );
        assert_eq!(
            restarted.verify_snapshot_progress(&recovered).unwrap(),
            SnapshotActiveObservation::None
        );
    }

    #[cfg(windows)]
    #[test]
    fn strict_bounded_journal_corruption_fails_closed() {
        let fixture = fixture();
        let relative = journal_relative(&fixture.key, &fixture.journal.transaction_id).unwrap();
        fixture
            .store
            .write_durable(
                &relative,
                br#"{"schema":2,"unexpected":true}"#,
                "corrupt test journal",
            )
            .unwrap();
        let error = fixture
            .store
            .load_journal(&fixture.key, &fixture.journal.transaction_id);
        assert!(matches!(error, Err(TransactionError::Store(_))));
    }

    #[test]
    fn exact_pre_public_epoch_one_shape_refuses_with_restart_recipe() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "../../formats/corpora/scrape-transaction-journal/e2/invalid/legacy-epoch1-shape.json",
        );
        let bytes = fs::read(path).unwrap();
        let error = journal_wire::decode(&bytes).expect_err("epoch 1 has no implicit migration");
        assert!(error.contains("pre-public"), "{error}");
        assert!(error.contains("restart scrape"), "{error}");
        assert!(
            error.contains("automatic migration is not available"),
            "{error}"
        );
    }

    #[cfg(windows)]
    #[test]
    fn owner_seal_rejects_semantically_valid_immutable_journal_drift() {
        let fixture = fixture();
        let mut corrupted = fixture.journal.clone();
        let PreparedMode::Export(plan) = &mut corrupted.execution else {
            unreachable!()
        };
        plan.output_identity = "different-but-valid-output-identity".to_owned();
        let bytes = journal_wire::encode(&corrupted).unwrap();
        let relative = journal_relative(&fixture.key, &fixture.journal.transaction_id).unwrap();
        fixture
            .store
            .write_durable(&relative, &bytes, "drifted test journal")
            .unwrap();

        assert!(matches!(
            fixture
                .store
                .load_journal(&fixture.key, &fixture.journal.transaction_id),
            Err(TransactionError::Store(_))
        ));
    }

    #[cfg(windows)]
    #[test]
    fn embedded_canonical_plan_is_strict_utf8_not_an_unbounded_byte_array() {
        let fixture = fixture();
        let bytes = journal_wire::encode(&fixture.journal).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            value["canonical_plan"].as_str().unwrap().as_bytes(),
            fixture.journal.canonical_plan
        );
        assert_eq!(journal_wire::decode(&bytes).unwrap(), fixture.journal);
    }

    #[cfg(windows)]
    #[test]
    fn stable_report_is_a_retryable_one_shot() {
        let mut fixture = fixture();
        make_terminal(&mut fixture);
        let report = fixture.journal.report.clone().unwrap();
        let canonical = fixture
            .store
            .canonical_report_bytes(&fixture.journal, &report)
            .unwrap();
        fixture.store.persist_report(&report, &canonical).unwrap();
        let mut different = report;
        different.events.push("different".to_owned());
        assert!(matches!(
            fixture.store.persist_report(&different, &canonical),
            Err(TransactionError::Store(_))
        ));
    }

    #[cfg(windows)]
    #[test]
    fn caller_supplied_terminal_state_cannot_retire_a_live_transaction() {
        let mut fixture = fixture();
        let mut forged = fixture.journal.clone();
        let report = refused_report(&forged);
        forged.state = TransactionState::Complete;
        forged.settlement_intent = Some(Outcome::Refused);
        forged.report = Some(report);

        assert!(matches!(
            fixture.store.retire_transaction(&forged),
            Err(TransactionError::Store(_))
        ));
        assert_eq!(
            fixture.store.pending(&fixture.key).unwrap(),
            Some(fixture.journal.clone())
        );
    }
