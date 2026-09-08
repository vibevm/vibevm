    #[test]
    fn in_place_quarantine_contract_last_and_restore() {
        let scope = tempfile::tempdir().unwrap();
        let project_path = scope.path().join("project");
        fs::create_dir(&project_path).unwrap();
        fs::write(project_path.join("junk.txt"), b"junk").unwrap();
        fs::write(project_path.join("contract.toml"), b"contract").unwrap();
        let project = SafefsProject::open(&project_path).unwrap();
        let parent = SafefsProject::open(scope.path()).unwrap();
        let junk = file(b"junk");
        let contract = file(b"contract");
        let junk_step = MutationStep {
            id: "remove-junk".to_owned(),
            pair_id: None,
            kind: MutationKind::QuarantineFile,
            transitions: vec![
                transition(
                    Location::Project,
                    "junk.txt",
                    PathState::File(junk.clone()),
                    PathState::Absent,
                ),
                transition(
                    Location::Quarantine,
                    "payload/junk.txt",
                    PathState::Absent,
                    PathState::File(junk),
                ),
            ],
        };
        let contract_step = MutationStep {
            id: "contract-delete-last".to_owned(),
            pair_id: None,
            kind: MutationKind::ContractDeleteLast,
            transitions: vec![
                transition(
                    Location::Project,
                    "contract.toml",
                    PathState::File(contract.clone()),
                    PathState::Absent,
                ),
                transition(
                    Location::Quarantine,
                    "payload/contract.toml",
                    PathState::Absent,
                    PathState::File(contract),
                ),
            ],
        };
        let before = transaction_manifest(vec![
            entry("contract.toml", b"contract"),
            entry("junk.txt", b"junk"),
        ]);
        let plan = InPlacePlan {
            quarantine_parent_identity: parent.identity_token().unwrap(),
            before_same_display_path: false,
            after_same_display_path: false,
            steps: vec![junk_step.clone()],
            contract: ContractCommit::DeleteLast {
                path: "contract.toml".to_owned(),
                empty_ancestors: Vec::new(),
            },
            contract_step: contract_step.clone(),
            contract_cleanup_step: None,
            before_tree: before.clone(),
            pre_contract_tree: transaction_manifest(vec![entry("contract.toml", b"contract")]),
            post_contract_tree: transaction_manifest(Vec::new()),
            after_tree: transaction_manifest(Vec::new()),
        };
        let mut adapter =
            SafefsTransactionFilesystem::open(&project_path, &project.identity_token().unwrap())
                .unwrap();
        let quarantine = ".vibe-scrape-quarantine-TXN003";
        let owner = "owner-quarantine";
        assert_eq!(
            adapter.create_quarantine(&plan, quarantine, owner).unwrap(),
            ExclusiveTreeCreation::Owned
        );
        adapter
            .apply_step(&plan, quarantine, owner, &junk_step, None)
            .unwrap();
        assert!(project_path.join("contract.toml").exists());
        adapter
            .apply_step(&plan, quarantine, owner, &contract_step, None)
            .unwrap();
        assert!(!project_path.join("contract.toml").exists());
        adapter
            .rollback_step(&plan, quarantine, owner, &contract_step)
            .unwrap();
        adapter
            .rollback_step(&plan, quarantine, owner, &junk_step)
            .unwrap();
        assert_eq!(fs::read(project_path.join("junk.txt")).unwrap(), b"junk");
        assert_eq!(
            fs::read(project_path.join("contract.toml")).unwrap(),
            b"contract"
        );
        cleanup_tree(
            &mut adapter,
            PreparedMode::InPlace(Box::new(plan.clone())),
            quarantine,
            owner,
        );
        assert!(!scope.path().join(quarantine).exists());
    }

    #[test]
    fn nested_contract_ancestors_restore_before_reverse_rename_after_restart() {
        let scope = tempfile::tempdir().unwrap();
        let project_path = scope.path().join("project");
        fs::create_dir_all(project_path.join("vibevm/scrape")).unwrap();
        fs::write(
            project_path.join("vibevm/scrape/contract.toml"),
            b"contract",
        )
        .unwrap();
        let project = SafefsProject::open(&project_path).unwrap();
        let parent = SafefsProject::open(scope.path()).unwrap();
        let contract = file(b"contract");
        let contract_step = MutationStep {
            id: "contract-delete-last".to_owned(),
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
            ],
        };
        let ancestor_tree = PathState::Tree(super::super::SubtreeState {
            digest: digest_bytes(b"contract-ancestor-tree"),
            root_mode: None,
            descendants: vec![super::super::SubtreeEntry {
                relative_path: "scrape".into(),
                kind: TreeEntryKind::Directory,
                sha256: None,
                bytes: None,
                mode: None,
            }],
        });
        let cleanup_step = MutationStep {
            id: "contract-ancestor-tree-park".into(),
            pair_id: None,
            kind: MutationKind::ContractAncestorTreePark,
            transitions: vec![
                transition(
                    Location::Project,
                    "vibevm",
                    ancestor_tree.clone(),
                    PathState::Absent,
                ),
                transition(
                    Location::Quarantine,
                    "directories/contract-ancestors",
                    PathState::Absent,
                    ancestor_tree,
                ),
            ],
        };
        let before = transaction_manifest(vec![
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
            steps: Vec::new(),
            contract: ContractCommit::DeleteLast {
                path: "vibevm/scrape/contract.toml".into(),
                empty_ancestors: vec!["vibevm/scrape".into(), "vibevm".into()],
            },
            contract_step: contract_step.clone(),
            contract_cleanup_step: Some(cleanup_step.clone()),
            before_tree: before.clone(),
            pre_contract_tree: before,
            post_contract_tree: transaction_manifest(vec![
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
            ]),
            after_tree: transaction_manifest(Vec::new()),
        };
        let quarantine = ".vibe-scrape-quarantine-TXN006";
        let owner = "owner-nested-contract";
        let mut adapter =
            SafefsTransactionFilesystem::open(&project_path, &project.identity_token().unwrap())
                .unwrap();
        adapter.create_quarantine(&plan, quarantine, owner).unwrap();
        adapter
            .apply_step(&plan, quarantine, owner, &contract_step, None)
            .unwrap();
        assert!(project_path.join("vibevm/scrape").is_dir());
        let contract_seal = adapter.owned_tree_seal(quarantine, owner).unwrap();
        let contract_journal = cleanup_journal(
            PreparedMode::InPlace(Box::new(plan.clone())),
            quarantine,
            owner,
            contract_seal,
        );
        drop(adapter);
        let mut adapter =
            SafefsTransactionFilesystem::open(&project_path, &project.identity_token().unwrap())
                .unwrap();
        adapter.rebind_from_journal(&contract_journal).unwrap();
        adapter
            .apply_step(&plan, quarantine, owner, &cleanup_step, None)
            .unwrap();
        assert!(!project_path.join("vibevm").exists());
        let seal = adapter.owned_tree_seal(quarantine, owner).unwrap();
        let journal = cleanup_journal(
            PreparedMode::InPlace(Box::new(plan.clone())),
            quarantine,
            owner,
            seal,
        );
        drop(adapter);

        let mut restarted =
            SafefsTransactionFilesystem::open(&project_path, &project.identity_token().unwrap())
                .unwrap();
        restarted.rebind_from_journal(&journal).unwrap();
        restarted
            .rollback_step(&plan, quarantine, owner, &cleanup_step)
            .unwrap();
        restarted
            .rollback_step(&plan, quarantine, owner, &contract_step)
            .unwrap();
        assert_eq!(
            fs::read(project_path.join("vibevm/scrape/contract.toml")).unwrap(),
            b"contract"
        );
        assert!(project_path.join("vibevm/scrape").is_dir());
    }
