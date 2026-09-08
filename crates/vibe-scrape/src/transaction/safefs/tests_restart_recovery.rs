    #[test]
    fn precreated_quarantine_topology_rebinds_capture_removal_and_contract_intents() {
        let scope = tempfile::tempdir().unwrap();
        let project_path = scope.path().join("project");
        fs::create_dir_all(project_path.join("src")).unwrap();
        fs::create_dir_all(project_path.join("vibevm/scrape")).unwrap();
        fs::write(project_path.join("src/lib.rs"), b"source").unwrap();
        fs::write(project_path.join("vibevm/data"), b"data").unwrap();
        fs::write(
            project_path.join("vibevm/scrape/contract.toml"),
            b"contract",
        )
        .unwrap();
        let project = SafefsProject::open(&project_path).unwrap();
        let project_token = project.identity_token().unwrap();
        let parent = SafefsProject::open(scope.path()).unwrap();
        let source = file(b"source");
        let data = file(b"data");
        let contract = file(b"contract");
        let capture = MutationStep {
            id: "capture-source".into(),
            pair_id: Some("rewrite-source".into()),
            kind: MutationKind::CaptureBeforeImage,
            transitions: vec![
                transition(
                    Location::Project,
                    "src/lib.rs",
                    PathState::File(source.clone()),
                    PathState::File(source.clone()),
                ),
                transition(
                    Location::Quarantine,
                    "before/src/lib.rs",
                    PathState::Absent,
                    PathState::File(source),
                ),
            ],
        };
        let removal = MutationStep {
            id: "remove-data".into(),
            pair_id: None,
            kind: MutationKind::QuarantineFile,
            transitions: vec![
                transition(
                    Location::Project,
                    "vibevm/data",
                    PathState::File(data.clone()),
                    PathState::Absent,
                ),
                transition(
                    Location::Quarantine,
                    "payload/vibevm/data",
                    PathState::Absent,
                    PathState::File(data),
                ),
            ],
        };
        let contract_step = MutationStep {
            id: "contract-last".into(),
            pair_id: None,
            kind: MutationKind::ContractDeleteLast,
            transitions: vec![
                transition(
                    Location::Project,
                    "vibevm/scrape/contract.toml",
                    PathState::File(contract.clone()),
                    PathState::Absent,
                ),
                transition(
                    Location::Quarantine,
                    "payload/vibevm/scrape/contract.toml",
                    PathState::Absent,
                    PathState::File(contract),
                ),
                transition(
                    Location::Project,
                    "vibevm/scrape",
                    PathState::EmptyDirectory { mode: None },
                    PathState::Absent,
                ),
                transition(
                    Location::Project,
                    "vibevm",
                    PathState::EmptyDirectory { mode: None },
                    PathState::Absent,
                ),
            ],
        };
        let pre_contract = transaction_manifest(vec![
            TreeEntry {
                path: "src".into(),
                kind: TreeEntryKind::Directory,
                sha256: None,
                bytes: None,
                mode: None,
            },
            entry("src/lib.rs", b"source"),
            TreeEntry {
                path: "vibevm".into(),
                kind: TreeEntryKind::Directory,
                sha256: None,
                bytes: None,
                mode: None,
            },
            TreeEntry {
                path: "vibevm/scrape".into(),
                kind: TreeEntryKind::Directory,
                sha256: None,
                bytes: None,
                mode: None,
            },
            entry("vibevm/scrape/contract.toml", b"contract"),
        ]);
        let plan = InPlacePlan {
            quarantine_parent_identity: parent.identity_token().unwrap(),
            before_same_display_path: false,
            after_same_display_path: false,
            steps: vec![capture.clone(), removal.clone()],
            contract: ContractCommit::DeleteLast {
                path: "vibevm/scrape/contract.toml".into(),
                empty_ancestors: vec!["vibevm/scrape".into(), "vibevm".into()],
            },
            contract_step: contract_step.clone(),
            contract_cleanup_step: None,
            before_tree: model_tree_at(&project, &project.root_dir().unwrap()).unwrap(),
            pre_contract_tree: pre_contract,
            post_contract_tree: transaction_manifest(vec![
                TreeEntry {
                    path: "src".into(),
                    kind: TreeEntryKind::Directory,
                    sha256: None,
                    bytes: None,
                    mode: None,
                },
                entry("src/lib.rs", b"source"),
            ]),
            after_tree: transaction_manifest(vec![
                TreeEntry {
                    path: "src".into(),
                    kind: TreeEntryKind::Directory,
                    sha256: None,
                    bytes: None,
                    mode: None,
                },
                entry("src/lib.rs", b"source"),
            ]),
        };
        let quarantine = ".vibe-scrape-quarantine-TXN007";
        let owner = "owner-topology-restart";
        let mut adapter = SafefsTransactionFilesystem::open(&project_path, &project_token).unwrap();
        adapter.create_quarantine(&plan, quarantine, owner).unwrap();

        for (index, step) in [capture, removal, contract_step].iter().enumerate() {
            let before_seal = adapter.owned_tree_seal(quarantine, owner).unwrap();
            adapter
                .apply_step(&plan, quarantine, owner, step, None)
                .unwrap();
            let mut journal = cleanup_journal(
                PreparedMode::InPlace(Box::new(plan.clone())),
                quarantine,
                owner,
                before_seal,
            );
            journal.state = super::super::TransactionState::Mutating;
            journal.completed_steps = index;
            journal.active_step = Some(index);
            journal.mutation_progress = vec![super::super::MutationProgress {
                id: step.id.clone(),
                kind: super::super::PlannedMutationKind::InPlace(step.kind),
                status: super::super::MutationStatus::ApplyIntent,
            }];
            drop(adapter);
            adapter = SafefsTransactionFilesystem::open(&project_path, &project_token).unwrap();
            adapter.rebind_from_journal(&journal).unwrap();
        }
        assert!(!project_path.join("vibevm").exists());
        assert_eq!(
            fs::read(project_path.join("src/lib.rs")).unwrap(),
            b"source"
        );
    }

    #[test]
    fn capture_and_rewrite_stages_rebind_before_and_after_publication() {
        for kind in [
            MutationKind::CaptureBeforeImage,
            MutationKind::AtomicRewrite,
        ] {
            for staged_before_publish in [true, false] {
                let scope = tempfile::tempdir().unwrap();
                let project_path = scope.path().join("project");
                fs::create_dir_all(project_path.join("src")).unwrap();
                fs::write(project_path.join("src/lib.rs"), b"old").unwrap();
                let project = SafefsProject::open(&project_path).unwrap();
                let project_token = project.identity_token().unwrap();
                let parent = SafefsProject::open(scope.path()).unwrap();
                let old = file(b"old");
                let new = file(b"new");
                let capture = MutationStep {
                    id: "capture-rewrite".into(),
                    pair_id: Some("rewrite".into()),
                    kind: MutationKind::CaptureBeforeImage,
                    transitions: vec![
                        transition(
                            Location::Project,
                            "src/lib.rs",
                            PathState::File(old.clone()),
                            PathState::File(old.clone()),
                        ),
                        transition(
                            Location::Quarantine,
                            "before/src/lib.rs",
                            PathState::Absent,
                            PathState::File(old.clone()),
                        ),
                    ],
                };
                let rewrite = MutationStep {
                    id: "rewrite-source".into(),
                    pair_id: Some("rewrite".into()),
                    kind: MutationKind::AtomicRewrite,
                    transitions: vec![transition(
                        Location::Project,
                        "src/lib.rs",
                        PathState::File(old),
                        PathState::File(new),
                    )],
                };
                let tree = model_tree_at(&project, &project.root_dir().unwrap()).unwrap();
                let plan = InPlacePlan {
                    quarantine_parent_identity: parent.identity_token().unwrap(),
                    before_same_display_path: false,
                    after_same_display_path: false,
                    steps: vec![capture.clone(), rewrite.clone()],
                    contract: ContractCommit::ExternalPreserve,
                    contract_step: MutationStep {
                        id: "external-contract".into(),
                        pair_id: None,
                        kind: MutationKind::ContractExternalPreserve,
                        transitions: Vec::new(),
                    },
                    contract_cleanup_step: None,
                    before_tree: tree.clone(),
                    pre_contract_tree: tree.clone(),
                    post_contract_tree: tree.clone(),
                    after_tree: tree,
                };
                let quarantine = if kind == MutationKind::CaptureBeforeImage {
                    ".vibe-scrape-quarantine-TXN010"
                } else {
                    ".vibe-scrape-quarantine-TXN011"
                };
                let owner = if staged_before_publish {
                    "owner-stage-before"
                } else {
                    "owner-stage-after"
                };
                let mut adapter =
                    SafefsTransactionFilesystem::open(&project_path, &project_token).unwrap();
                adapter.create_quarantine(&plan, quarantine, owner).unwrap();
                if kind == MutationKind::AtomicRewrite {
                    adapter
                        .apply_step(&plan, quarantine, owner, &capture, None)
                        .unwrap();
                }
                let before_seal = adapter.owned_tree_seal(quarantine, owner).unwrap();
                let (step, index, target, bytes) = if kind == MutationKind::CaptureBeforeImage {
                    (&capture, 0, "before/src/lib.rs", b"old".as_slice())
                } else {
                    (&rewrite, 1, "src/lib.rs", b"new".as_slice())
                };
                let stage = transaction_stage_name(owner, &format!("apply:{}", step.id), target);
                let stage_path = if kind == MutationKind::CaptureBeforeImage {
                    scope
                        .path()
                        .join(quarantine)
                        .join("before/src")
                        .join(&stage)
                } else {
                    project_path.join("src").join(&stage)
                };
                if staged_before_publish {
                    fs::write(&stage_path, bytes).unwrap();
                } else {
                    adapter
                        .apply_step(
                            &plan,
                            quarantine,
                            owner,
                            step,
                            (kind == MutationKind::AtomicRewrite).then_some(bytes),
                        )
                        .unwrap();
                }
                let mut journal = cleanup_journal(
                    PreparedMode::InPlace(Box::new(plan.clone())),
                    quarantine,
                    owner,
                    before_seal,
                );
                journal.state = super::super::TransactionState::Mutating;
                journal.completed_steps = index;
                journal.active_step = Some(index);
                journal.mutation_progress = vec![super::super::MutationProgress {
                    id: step.id.clone(),
                    kind: super::super::PlannedMutationKind::InPlace(step.kind),
                    status: super::super::MutationStatus::ApplyIntent,
                }];
                drop(adapter);
                let mut restarted =
                    SafefsTransactionFilesystem::open(&project_path, &project_token).unwrap();
                restarted.rebind_from_journal(&journal).unwrap();
                if staged_before_publish {
                    if kind == MutationKind::AtomicRewrite {
                        restarted
                            .cleanup_unpublished_step_stage(&plan, quarantine, owner, step)
                            .unwrap();
                    } else {
                        restarted
                            .apply_step(&plan, quarantine, owner, step, None)
                            .unwrap();
                    }
                }
                assert!(!stage_path.exists());
                if kind == MutationKind::CaptureBeforeImage {
                    assert_eq!(
                        fs::read(scope.path().join(quarantine).join("before/src/lib.rs")).unwrap(),
                        b"old"
                    );
                } else if staged_before_publish {
                    assert_eq!(fs::read(project_path.join("src/lib.rs")).unwrap(), b"old");
                } else {
                    assert_eq!(
                        restarted
                            .observe_step(&plan, quarantine, owner, &rewrite)
                            .unwrap(),
                        SealedObservation::After
                    );
                    restarted
                        .rollback_step(&plan, quarantine, owner, &rewrite)
                        .unwrap();
                    assert_eq!(fs::read(project_path.join("src/lib.rs")).unwrap(), b"old");
                }
            }
        }
    }
