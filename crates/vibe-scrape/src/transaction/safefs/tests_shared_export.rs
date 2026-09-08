    use std::fs;

    use super::*;
    use crate::transaction::{ContractCommit, PathTransition};

    fn file(bytes: &[u8]) -> FileState {
        FileState {
            sha256: digest_bytes(bytes),
            bytes: bytes.len() as u64,
            mode: None,
        }
    }

    fn entry(path: &str, bytes: &[u8]) -> TreeEntry {
        TreeEntry {
            path: path.to_owned(),
            kind: TreeEntryKind::File,
            sha256: Some(digest_bytes(bytes)),
            bytes: Some(bytes.len() as u64),
            mode: None,
        }
    }

    fn cleanup_journal(
        execution: PreparedMode,
        name: &str,
        owner: &str,
        seal: OwnedTreeSeal,
    ) -> Journal {
        Journal {
            schema: 1,
            revision: 0,
            project_key: super::super::ProjectKey("project".to_owned()),
            transaction_id: super::super::TransactionId("TXN001".to_owned()),
            mode: match &execution {
                PreparedMode::Export(_) => super::super::TransactionMode::Export,
                PreparedMode::InPlace(_) => super::super::TransactionMode::InPlace,
            },
            plan_id: Digest(format!("sha256:{}", "0".repeat(64))),
            canonical_plan: Vec::new(),
            verification_workspace: None,
            project_display_root: "test".to_owned(),
            execution,
            state: super::super::TransactionState::RollingBack,
            snapshots: Vec::new(),
            snapshots_persisted: 0,
            snapshot_active: None,
            candidate_name: name
                .starts_with(".vibe-scrape-candidate-")
                .then(|| name.to_owned()),
            quarantine_name: name
                .starts_with(".vibe-scrape-quarantine-")
                .then(|| name.to_owned()),
            owned_tree_token: Some(owner.to_owned()),
            owned_tree_seal: Some(seal),
            cleanup_wal: None,
            completed_steps: 0,
            active_step: None,
            mutation_progress: Vec::new(),
            actual_mutations: Vec::new(),
            settlement_intent: None,
            delivered_tree: None,
            verification: Vec::new(),
            events: Vec::new(),
            report: None,
        }
    }

    fn cleanup_tree(
        adapter: &mut SafefsTransactionFilesystem,
        execution: PreparedMode,
        name: &str,
        owner: &str,
    ) {
        let seal = adapter.owned_tree_seal(name, owner).unwrap();
        let journal = cleanup_journal(execution, name, owner, seal.clone());
        let mut completed = Vec::new();
        loop {
            let OwnedTreeCleanupPreparation::Intent(intent) = adapter
                .prepare_owned_tree_cleanup(&journal, name, owner, &seal, &completed)
                .unwrap()
            else {
                break;
            };
            let completion = adapter
                .execute_owned_tree_cleanup(&journal, name, owner, &seal, &completed, &intent)
                .unwrap();
            completed.push(completion.progress_key);
        }
    }

    fn export_fixture() -> (
        tempfile::TempDir,
        PathBuf,
        ExportPlan,
        SafefsTransactionFilesystem,
    ) {
        let scope = tempfile::tempdir().unwrap();
        let source = scope.path().join("source");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("kept.txt"), b"kept").unwrap();
        let output = scope.path().join("scraped");
        let source_project = SafefsProject::open(&source).unwrap();
        let output_parent = SafefsProject::open(scope.path()).unwrap();
        let output_slot = SafefsProject::pin_absent_path(&output).unwrap();
        let final_manifest = transaction_manifest(vec![entry("kept.txt", b"kept")]);
        let plan = ExportPlan {
            output_identity: output_slot.identity_token(),
            output_parent_identity: output_parent.identity_token().unwrap(),
            output_display_path: output.display().to_string(),
            output_name: "scraped".to_owned(),
            before_same_display_path: false,
            after_same_display_path: false,
            entries: vec![ExportEntry {
                target_path: "kept.txt".to_owned(),
                kind: TreeEntryKind::File,
                mode: None,
                payload: Some(ExportPayload::Source {
                    source_path: "kept.txt".to_owned(),
                    before: file(b"kept"),
                }),
            }],
            source_tree: transaction_manifest(vec![entry("kept.txt", b"kept")]),
            final_manifest,
        };
        let adapter =
            SafefsTransactionFilesystem::open(&source, &source_project.identity_token().unwrap())
                .unwrap();
        (scope, output, plan, adapter)
    }

    #[test]
    fn export_publish_and_exact_rollback() {
        let (_scope, output, plan, mut adapter) = export_fixture();
        let candidate = ".vibe-scrape-candidate-TXN001";
        let owner = "owner-export";
        assert_eq!(
            adapter
                .create_export_candidate(&plan, candidate, owner)
                .unwrap(),
            ExclusiveTreeCreation::Owned
        );
        adapter
            .apply_export_entry(&plan, candidate, owner, &plan.entries[0], None)
            .unwrap();
        assert_eq!(
            adapter
                .observe_export_tree(&plan, ExportTreeSlot::Candidate, candidate, owner)
                .unwrap(),
            OwnedTreeObservation::Exact(plan.final_manifest.clone())
        );
        adapter
            .publish_export_noreplace(&plan, candidate, owner)
            .unwrap();
        assert_eq!(fs::read(output.join("kept.txt")).unwrap(), b"kept");
        assert_eq!(
            adapter
                .observe_export_tree(&plan, ExportTreeSlot::Output, candidate, owner)
                .unwrap(),
            OwnedTreeObservation::Exact(plan.final_manifest.clone())
        );
        adapter.unpublish_export(&plan, candidate, owner).unwrap();
        cleanup_tree(
            &mut adapter,
            PreparedMode::Export(Box::new(plan.clone())),
            candidate,
            owner,
        );
        assert!(!output.exists());
        assert!(!output.parent().unwrap().join(candidate).exists());
    }

    #[test]
    fn restart_never_adopts_a_created_root_before_its_first_identity_seal() {
        let (scope, _output, plan, mut adapter) = export_fixture();
        let candidate = ".vibe-scrape-candidate-TXN004";
        let owner = "owner-unsealed";
        assert_eq!(
            adapter
                .create_export_candidate(&plan, candidate, owner)
                .unwrap(),
            ExclusiveTreeCreation::Owned
        );
        let seal = adapter.owned_tree_seal(candidate, owner).unwrap();
        let mut journal =
            cleanup_journal(PreparedMode::Export(Box::new(plan)), candidate, owner, seal);
        journal.state = super::super::TransactionState::Prepared;
        journal.owned_tree_seal = None;
        drop(adapter);

        let source = scope.path().join("source");
        let project = SafefsProject::open(&source).unwrap();
        let mut restarted =
            SafefsTransactionFilesystem::open(&source, &project.identity_token().unwrap()).unwrap();
        assert!(matches!(
            restarted.rebind_from_journal(&journal),
            Err(TransactionError::ThirdState(message))
                if message.contains("automatic adoption is forbidden")
        ));
        assert!(scope.path().join(candidate).is_dir());
    }

    #[test]
    fn restart_rebinds_a_manifest_reduced_by_durable_cleanup_progress() {
        let (scope, _output, plan, mut adapter) = export_fixture();
        let candidate = ".vibe-scrape-candidate-TXN005";
        let owner = "owner-cleanup-restart";
        adapter
            .create_export_candidate(&plan, candidate, owner)
            .unwrap();
        adapter
            .apply_export_entry(&plan, candidate, owner, &plan.entries[0], None)
            .unwrap();
        let seal = adapter.owned_tree_seal(candidate, owner).unwrap();
        let mut journal = cleanup_journal(
            PreparedMode::Export(Box::new(plan.clone())),
            candidate,
            owner,
            seal.clone(),
        );
        journal.cleanup_wal = Some(super::super::OwnedTreeCleanupWal {
            name: candidate.to_owned(),
            directory_identity: seal.directory_identity.clone(),
            manifest_digest: seal.manifest_digest.clone(),
            completed: Vec::new(),
            active: None,
        });
        let OwnedTreeCleanupPreparation::Intent(first) = adapter
            .prepare_owned_tree_cleanup(&journal, candidate, owner, &seal, &[])
            .unwrap()
        else {
            panic!("non-empty candidate must have a cleanup entry")
        };
        journal.cleanup_wal.as_mut().unwrap().active = Some(first.clone());
        let completion = adapter
            .execute_owned_tree_cleanup(&journal, candidate, owner, &seal, &[], &first)
            .unwrap();
        let wal = journal.cleanup_wal.as_mut().unwrap();
        wal.completed.push(completion.progress_key);
        wal.active = None;
        drop(adapter);

        let source = scope.path().join("source");
        let project = SafefsProject::open(&source).unwrap();
        let mut restarted =
            SafefsTransactionFilesystem::open(&source, &project.identity_token().unwrap()).unwrap();
        restarted.rebind_from_journal(&journal).unwrap();
        let mut completed = journal.cleanup_wal.as_ref().unwrap().completed.clone();
        loop {
            let OwnedTreeCleanupPreparation::Intent(intent) = restarted
                .prepare_owned_tree_cleanup(&journal, candidate, owner, &seal, &completed)
                .unwrap()
            else {
                break;
            };
            let completion = restarted
                .execute_owned_tree_cleanup(&journal, candidate, owner, &seal, &completed, &intent)
                .unwrap();
            completed.push(completion.progress_key);
        }
        assert!(!scope.path().join(candidate).exists());
    }

    #[test]
    fn export_output_race_preserves_occupant_and_cleans_candidate() {
        let (_scope, output, plan, mut adapter) = export_fixture();
        let candidate = ".vibe-scrape-candidate-TXN002";
        let owner = "owner-race";
        assert_eq!(
            adapter
                .create_export_candidate(&plan, candidate, owner)
                .unwrap(),
            ExclusiveTreeCreation::Owned
        );
        adapter
            .apply_export_entry(&plan, candidate, owner, &plan.entries[0], None)
            .unwrap();
        fs::create_dir(&output).unwrap();
        fs::write(output.join("foreign.txt"), b"foreign").unwrap();
        assert!(matches!(
            adapter.publish_export_noreplace(&plan, candidate, owner),
            Err(TransactionError::OutputRace(_))
        ));
        cleanup_tree(
            &mut adapter,
            PreparedMode::Export(Box::new(plan.clone())),
            candidate,
            owner,
        );
        assert_eq!(fs::read(output.join("foreign.txt")).unwrap(), b"foreign");
    }

    #[test]
    fn export_file_stage_rebinds_before_and_after_publication() {
        for staged_before_publish in [true, false] {
            let (scope, _output, plan, mut adapter) = export_fixture();
            let candidate = if staged_before_publish {
                ".vibe-scrape-candidate-TXN008"
            } else {
                ".vibe-scrape-candidate-TXN009"
            };
            let owner = "owner-export-stage";
            adapter
                .create_export_candidate(&plan, candidate, owner)
                .unwrap();
            let before_seal = adapter.owned_tree_seal(candidate, owner).unwrap();
            let stage = transaction_stage_name(
                owner,
                &format!("export:{}", plan.entries[0].target_path),
                &plan.entries[0].target_path,
            );
            if staged_before_publish {
                fs::write(scope.path().join(candidate).join(&stage), b"kept").unwrap();
            } else {
                adapter
                    .apply_export_entry(&plan, candidate, owner, &plan.entries[0], None)
                    .unwrap();
            }
            let mut journal = cleanup_journal(
                PreparedMode::Export(Box::new(plan.clone())),
                candidate,
                owner,
                before_seal,
            );
            journal.state = super::super::TransactionState::Prepared;
            journal.active_step = Some(0);
            journal.mutation_progress = vec![super::super::MutationProgress {
                id: "export/entry/0/kept.txt".into(),
                kind: super::super::PlannedMutationKind::ExportEntry,
                status: super::super::MutationStatus::ApplyIntent,
            }];
            drop(adapter);
            let source = scope.path().join("source");
            let project = SafefsProject::open(&source).unwrap();
            let mut restarted =
                SafefsTransactionFilesystem::open(&source, &project.identity_token().unwrap())
                    .unwrap();
            restarted.rebind_from_journal(&journal).unwrap();
            if staged_before_publish {
                assert_eq!(
                    restarted
                        .observe_export_tree(&plan, ExportTreeSlot::Candidate, candidate, owner,)
                        .unwrap(),
                    OwnedTreeObservation::Exact(transaction_manifest(Vec::new()))
                );
                restarted
                    .apply_export_entry(&plan, candidate, owner, &plan.entries[0], None)
                    .unwrap();
            }
            assert_eq!(
                fs::read(scope.path().join(candidate).join("kept.txt")).unwrap(),
                b"kept"
            );
            assert!(!scope.path().join(candidate).join(stage).exists());
        }
    }

    fn transition(
        location: Location,
        path: &str,
        before: PathState,
        after: PathState,
    ) -> PathTransition {
        PathTransition {
            location,
            path: path.to_owned(),
            before,
            after,
        }
    }
