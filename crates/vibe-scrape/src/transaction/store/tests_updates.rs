    #[cfg(windows)]
    #[test]
    fn journal_revision_rejects_stale_same_revision_and_skip_ahead_writes() {
        let mut fixture = fixture();
        let initial = fixture.journal.clone();

        let mut same_revision_change = initial.clone();
        same_revision_change.events.push("stale change".to_owned());
        assert!(matches!(
            fixture.store.persist_journal(&same_revision_change),
            Err(TransactionError::Store(_))
        ));

        let mut skip_ahead = initial.clone();
        skip_ahead.revision = 2;
        skip_ahead.events.push("skipped revision".to_owned());
        assert!(matches!(
            fixture.store.persist_journal(&skip_ahead),
            Err(TransactionError::Store(_))
        ));

        fixture.journal.snapshot_active = Some(0);
        fixture.journal.revision = 1;
        fixture.store.persist_journal(&fixture.journal).unwrap();
        assert!(matches!(
            fixture.store.persist_journal(&initial),
            Err(TransactionError::Store(_))
        ));
    }

    #[cfg(windows)]
    #[test]
    fn snapshot_prefix_cannot_advance_without_intent_and_exact_data() {
        let mut fixture = fixture();

        let mut skipped_intent = fixture.journal.clone();
        skipped_intent.revision = 1;
        skipped_intent.snapshots_persisted = 1;
        assert!(matches!(
            fixture.store.persist_journal(&skipped_intent),
            Err(TransactionError::Store(_))
        ));

        fixture.journal.revision = 1;
        fixture.journal.snapshot_active = Some(0);
        fixture.store.persist_journal(&fixture.journal).unwrap();
        fixture.journal.revision = 2;
        fixture.journal.snapshot_active = None;
        fixture.journal.snapshots_persisted = 1;
        assert!(matches!(
            fixture.store.persist_journal(&fixture.journal),
            Err(TransactionError::Store(_))
        ));
    }

    #[cfg(windows)]
    #[test]
    fn retirement_refuses_to_adopt_an_unjournaled_transaction_root_entry() {
        let mut fixture = fixture();
        make_terminal(&mut fixture);
        let foreign_relative = format!(
            "{}/{}/{}/foreign.bin",
            TRANSACTIONS_DIRECTORY,
            project_component(&fixture.key).unwrap(),
            fixture.journal.transaction_id.0
        );
        fixture
            .store
            .write_durable(&foreign_relative, b"foreign", "foreign test entry")
            .unwrap();

        assert!(matches!(
            fixture.store.retire_transaction(&fixture.journal),
            Err(TransactionError::Store(_))
        ));
        assert_eq!(
            fs::read(fixture.store.state_root().join(foreign_relative)).unwrap(),
            b"foreign"
        );
        assert!(
            fixture
                .store
                .transaction_directory(&fixture.key, &fixture.journal.transaction_id)
                .unwrap()
                .is_some()
        );
    }

    #[cfg(windows)]
    #[test]
    fn incomplete_transaction_bootstrap_is_left_for_explicit_manual_recovery() {
        let mut fixture = fixture();
        let journal = fixture
            .store
            .state_root()
            .join(journal_relative(&fixture.key, &fixture.journal.transaction_id).unwrap());
        fs::remove_file(journal).unwrap();

        assert!(matches!(
            fixture.store.pending(&fixture.key),
            Err(TransactionError::Store(_))
        ));
        assert!(
            fixture
                .store
                .transaction_directory(&fixture.key, &fixture.journal.transaction_id)
                .unwrap()
                .is_some()
        );
    }

    #[cfg(windows)]
    #[test]
    fn retirement_recovers_after_one_uncheckpointed_removal() {
        let mut fixture = fixture();
        make_terminal(&mut fixture);
        let (owner, directory) = fixture
            .store
            .load_owner(&fixture.key, &fixture.journal.transaction_id)
            .unwrap();
        let manifest = observe_safefs_manifest(&directory).unwrap();
        drop(directory);
        let mut wire = RetirementWire {
            schema: 2,
            project_key: fixture.key.0.clone(),
            transaction_id: fixture.journal.transaction_id.0.clone(),
            stable_report_sha256: sha256_bytes(
                &fixture
                    .store
                    .require_stable_complete_report(&fixture.journal)
                    .unwrap(),
            ),
            ownership_token: owner.ownership_token,
            directory_identity: owner.directory_identity,
            manifest: safefs_manifest_to_wire(&manifest),
            completed: Vec::new(),
            active: None,
            tree_removed: false,
        };
        let project_home = fixture
            .store
            .open_project_home(&fixture.key)
            .unwrap()
            .unwrap();
        let identity = OwnedDirectoryIdentity::from_token(&wire.directory_identity).unwrap();
        let progress = OwnedTreeCleanupProgress::new();
        let CleanupPreparation::Intent(intent) = project_home
            .prepare_owned_child_retirement(
                &fixture.journal.transaction_id.0,
                &wire.ownership_token,
                &identity,
                &manifest,
                &progress,
            )
            .unwrap()
        else {
            panic!("nonempty transaction must have a retirement intent")
        };
        wire.active = Some(cleanup_intent_to_wire(&intent));
        fixture
            .store
            .write_retirement(&fixture.key, &fixture.journal.transaction_id, &wire)
            .unwrap();
        project_home
            .execute_owned_child_retirement(
                &fixture.journal.transaction_id.0,
                &wire.ownership_token,
                &identity,
                &manifest,
                &progress,
                &intent,
            )
            .unwrap();
        let root = fixture.store.state_root().to_path_buf();
        drop(fixture.store);

        let mut restarted = SystemTransactionStore::new(root).unwrap();
        restarted
            .prove_outside_project(&fixture.project_root.display().to_string())
            .unwrap();
        restarted.lock_project(&fixture.key).unwrap();
        assert!(restarted.pending(&fixture.key).unwrap().is_none());
    }

    #[cfg(windows)]
    fn sha256(bytes: &[u8]) -> Digest {
        let mut hash = Sha256::new();
        hash.update(bytes);
        Digest(format!("sha256:{:x}", hash.finalize()))
    }

    #[test]
    fn state_root_must_be_absolute() {
        assert!(matches!(
            SystemTransactionStore::new("relative"),
            Err(TransactionError::Store(_))
        ));
    }

    #[cfg(windows)]
    #[test]
    fn oversized_initial_canonical_or_execution_bytes_create_no_transaction_home() {
        let template = fixture();
        let scope = tempfile::tempdir().unwrap();
        let state_root = scope.path().join("state");
        let mut store = SystemTransactionStore::new(&state_root).unwrap();
        store
            .prove_outside_project(&template.project_root.display().to_string())
            .unwrap();
        store.lock_project(&template.key).unwrap();
        let project_home = state_root
            .join(TRANSACTIONS_DIRECTORY)
            .join(project_component(&template.key).unwrap());

        let mut canonical = template.journal.clone();
        canonical.transaction_id = TransactionId("TXOVERSIZECANONICAL".into());
        canonical.canonical_plan = vec![b'x'; 16 * 1024 * 1024 + 1];
        canonical.verification_workspace =
            Some(store.verification_workspace_intent(&canonical).unwrap());
        assert!(store.create_transaction(&canonical).is_err());
        assert!(!project_home.exists());

        let mut execution = template.journal.clone();
        execution.transaction_id = TransactionId("TXOVERSIZEEXECUTION".into());
        let PreparedMode::Export(plan) = &mut execution.execution else {
            unreachable!()
        };
        // JSON escapes every quote, pushing the exact revision-zero envelope
        // over 64 MiB without allocating a second 64 MiB source string.
        plan.output_display_path = "\"".repeat(MAX_TRANSACTION_JOURNAL_BYTES / 2 + 1);
        execution.verification_workspace =
            Some(store.verification_workspace_intent(&execution).unwrap());
        assert!(store.create_transaction(&execution).is_err());
        assert!(!project_home.exists());
    }

    #[test]
    fn lock_key_must_identify_the_project_used_for_disjointness_proof() {
        let scope = tempfile::tempdir().unwrap();
        let project_root = scope.path().join("project");
        fs::create_dir(&project_root).unwrap();
        let state_root = scope.path().join("state");
        fs::create_dir(&state_root).unwrap();
        let mut store = SystemTransactionStore::new(state_root).unwrap();
        store
            .prove_outside_project(&project_root.display().to_string())
            .unwrap();
        let wrong = ProjectKey(format!("sha256:{}", "f".repeat(64)));
        assert!(matches!(
            store.lock_project(&wrong),
            Err(TransactionError::Store(_))
        ));
        let exact = derive_project_key(
            &Project::open(&project_root)
                .unwrap()
                .identity_token()
                .unwrap(),
        );
        store.lock_project(&exact).unwrap();
    }

    #[test]
    fn strict_enum_payloads_reject_unknown_and_duplicate_members() {
        let unknown = serde_json::from_slice::<ExportPayload>(
            br#"{"prepared-after":{"snapshot_name":"after","unexpected":true}}"#,
        );
        assert!(unknown.is_err());
        let duplicate = serde_json::from_slice::<ExportPayload>(
            br#"{"prepared-after":{"snapshot_name":"first","snapshot_name":"second"}}"#,
        );
        assert!(duplicate.is_err());
    }

    #[test]
    fn retirement_progress_requires_a_durable_parent_sync() {
        assert!(matches!(
            require_namespace_checkpoint(
                DirectoryDurability::Unsupported(std::io::ErrorKind::PermissionDenied),
                "retirement step",
            ),
            Err(TransactionError::Store(_))
        ));
    }

    #[test]
    fn stable_report_updates_are_only_append_only_pending_cleanup_transitions() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../formats/corpora/scrape/e1/valid/report-minimal.json");
        let mut pending: report_wire::Report =
            serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        pending.cleanup = report_wire::ReportCleanup::Pending;

        let mut appended = pending.clone();
        appended.recovery.push(report_wire::RecoveryStep {
            action: "cleanup".to_owned(),
            operation_id: "in-place/quarantine".to_owned(),
            result: report_wire::RecoveryStepResult::Complete,
            sequence: 0,
        });
        assert!(canonical_report_update_is_legal(&pending, &appended));

        let mut complete = appended.clone();
        complete.cleanup = report_wire::ReportCleanup::Complete;
        assert!(canonical_report_update_is_legal(&appended, &complete));

        let mut rewritten = appended.clone();
        rewritten.plan_id = format!("sha256:{}", "f".repeat(64));
        assert!(!canonical_report_update_is_legal(&pending, &rewritten));
        assert!(!canonical_report_update_is_legal(&complete, &pending));
    }
