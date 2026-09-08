#[derive(Default)]
struct OneFault {
    target: Option<DurableBoundary>,
    fired: bool,
}

#[derive(Default)]
struct TraceFaults(Vec<DurableBoundary>);

impl FaultInjector for TraceFaults {
    fn boundary(&mut self, boundary: DurableBoundary) -> Result<(), TransactionError> {
        self.0.push(boundary);
        Ok(())
    }
}

impl FaultInjector for OneFault {
    fn boundary(&mut self, boundary: DurableBoundary) -> Result<(), TransactionError> {
        if !self.fired && self.target.as_ref() == Some(&boundary) {
            self.fired = true;
            return Err(TransactionError::FaultInjected(boundary));
        }
        Ok(())
    }
}

fn hash(label: &str) -> Digest {
    digest(label.as_bytes())
}

fn file(path: &str, label: &str) -> TreeEntry {
    TreeEntry {
        path: path.into(),
        kind: TreeEntryKind::File,
        sha256: Some(hash(label)),
        bytes: Some(label.len() as u64),
        mode: Some(0o644),
    }
}

fn directory(path: &str) -> TreeEntry {
    TreeEntry {
        path: path.into(),
        kind: TreeEntryKind::Directory,
        sha256: None,
        bytes: None,
        mode: Some(0o755),
    }
}

fn manifest(label: &str, entries: Vec<TreeEntry>) -> TreeManifest {
    TreeManifest {
        digest: hash(label),
        entries,
    }
}

fn partial_manifest(entries: Vec<TreeEntry>) -> TreeManifest {
    logical_tree_manifest(entries)
}

fn canonical_plan(mode: TransactionMode) -> Vec<u8> {
    let value = serde_json::json!({
        "assertions": [],
        "blockers": [],
        "command": "scrape",
        "contract": {
            "action": "delete-last",
            "contained": true,
            "display_path": "C:/source/vibevm/scrape/contract.toml",
            "sha256": hash("contract bytes").0,
        },
        "contract_boundary": {
            "kind": "delete-last",
            "empty_ancestors": ["vibevm/scrape", "vibevm"],
            "path": "vibevm/scrape/contract.toml",
        },
        "health_baseline": "strict",
        "health_limits": {
            "max_result_bytes": "1024",
            "max_stderr_bytes": "1024",
            "max_stdout_bytes": "1024",
            "termination_grace_seconds": 1,
        },
        "health_plan_id": hash("health").0,
        "healthchecks": [],
        "items": [],
        "mode": match mode { TransactionMode::Export => "export", TransactionMode::InPlace => "in-place" },
        "native_lock_changes": [],
        "plan_id": hash("plan").0,
        "project": { "display_root": "C:/source", "tree_digest": hash("tree").0 },
        "relocations": [],
        "rewrites": [],
        "schema": 1,
        "summary": {
            "delete_last": 0,
            "delete_modified": 0,
            "delete_unknown": 0,
            "delete_unmodified": 0,
            "keep": 0,
            "relocate": 0,
            "rewrite": 0,
        }
    });
    let plan: vibe_wire::generated::scrape::e1::plan::Plan = serde_json::from_value(value).unwrap();
    serde_json::to_vec(&plan).unwrap()
}

fn snapshots(mode: TransactionMode) -> Vec<Snapshot> {
    vec![
        Snapshot {
            kind: SnapshotKind::Contract,
            name: "contract".into(),
            bytes: b"contract bytes".to_vec(),
            mode: Some(0o644),
        },
        Snapshot {
            kind: SnapshotKind::CanonicalPlan,
            name: "plan".into(),
            bytes: canonical_plan(mode),
            mode: Some(0o600),
        },
        Snapshot {
            kind: SnapshotKind::CanonicalContract,
            name: "canonical-contract".into(),
            bytes: b"canonical contract value".to_vec(),
            mode: Some(0o600),
        },
        Snapshot {
            kind: SnapshotKind::Verifier,
            name: "verifier".into(),
            bytes: b"verifier bytes".to_vec(),
            mode: Some(0o755),
        },
        Snapshot {
            kind: SnapshotKind::PreparedAfter,
            name: "after/readme".into(),
            bytes: b"native readme\n".to_vec(),
            mode: Some(0o644),
        },
    ]
}

fn export_prepared() -> PreparedTransaction {
    let entries = vec![
        file("README.md", "native readme\n"),
        directory("docs"),
        file("docs/spec.md", "spec"),
        directory("src"),
        file("src/main.rs", "main"),
    ];
    PreparedTransaction {
        project_identity_token: "stable-project-identity".into(),
        project_display_root: "C:/source".into(),
        plan_id: hash("plan"),
        canonical_plan: canonical_plan(TransactionMode::Export),
        snapshots: snapshots(TransactionMode::Export),
        mode: PreparedMode::Export(Box::new(ExportPlan {
            output_identity: "absent-output-slot".into(),
            output_parent_identity: "output-parent-volume".into(),
            output_display_path: "C:/delivery".into(),
            output_name: "delivery".into(),
            before_same_display_path: false,
            after_same_display_path: false,
            entries: vec![
                ExportEntry {
                    target_path: "README.md".into(),
                    kind: TreeEntryKind::File,
                    mode: Some(0o644),
                    payload: Some(ExportPayload::PreparedAfter {
                        snapshot_name: "after/readme".into(),
                    }),
                },
                ExportEntry {
                    target_path: "docs".into(),
                    kind: TreeEntryKind::Directory,
                    mode: Some(0o755),
                    payload: None,
                },
                ExportEntry {
                    target_path: "docs/spec.md".into(),
                    kind: TreeEntryKind::File,
                    mode: Some(0o644),
                    payload: Some(ExportPayload::Source {
                        source_path: "vibevm/spec.md".into(),
                        before: FileState {
                            sha256: hash("spec"),
                            bytes: 4,
                            mode: Some(0o644),
                        },
                    }),
                },
                ExportEntry {
                    target_path: "src".into(),
                    kind: TreeEntryKind::Directory,
                    mode: Some(0o755),
                    payload: None,
                },
                ExportEntry {
                    target_path: "src/main.rs".into(),
                    kind: TreeEntryKind::File,
                    mode: Some(0o644),
                    payload: Some(ExportPayload::Source {
                        source_path: "src/main.rs".into(),
                        before: FileState {
                            sha256: hash("main"),
                            bytes: 4,
                            mode: Some(0o644),
                        },
                    }),
                },
            ],
            source_tree: manifest(
                "source",
                vec![
                    file("README.md", "old readme"),
                    directory("src"),
                    file("src/main.rs", "main"),
                    directory("vibevm"),
                    file("vibevm/spec.md", "spec"),
                ],
            ),
            final_manifest: manifest("final", entries),
        })),
    }
}

fn transition(path: &str, before: PathState, after: PathState) -> PathTransition {
    PathTransition {
        location: Location::Project,
        path: path.into(),
        before,
        after,
    }
}

fn present(label: &str) -> PathState {
    PathState::File(FileState {
        sha256: hash(label),
        bytes: label.len() as u64,
        mode: Some(0o644),
    })
}

fn in_place_prepared() -> PreparedTransaction {
    let remove = MutationStep {
        id: "remove-metadata".into(),
        pair_id: None,
        kind: MutationKind::QuarantineFile,
        transitions: vec![
            transition("vibevm/data", present("data"), PathState::Absent),
            PathTransition {
                location: Location::Quarantine,
                path: "payload/vibevm/data".into(),
                before: PathState::Absent,
                after: present("data"),
            },
        ],
    };
    let contract = MutationStep {
        id: "contract-last".into(),
        pair_id: None,
        kind: MutationKind::ContractDeleteLast,
        transitions: vec![
            transition(
                "vibevm/scrape/contract.toml",
                present("contract bytes"),
                PathState::Absent,
            ),
            PathTransition {
                location: Location::Quarantine,
                path: "payload/vibevm/scrape/contract.toml".into(),
                before: PathState::Absent,
                after: present("contract bytes"),
            },
        ],
    };
    let contract_cleanup = MutationStep {
        id: "contract-ancestor-tree-park".into(),
        pair_id: None,
        kind: MutationKind::ContractAncestorTreePark,
        transitions: vec![
            transition(
                "vibevm",
                PathState::Tree(SubtreeState {
                    digest: hash("contract-ancestor-tree"),
                    root_mode: Some(0o755),
                    descendants: vec![SubtreeEntry {
                        relative_path: "scrape".into(),
                        kind: TreeEntryKind::Directory,
                        sha256: None,
                        bytes: None,
                        mode: Some(0o755),
                    }],
                }),
                PathState::Absent,
            ),
            PathTransition {
                location: Location::Quarantine,
                path: "directories/contract-ancestors".into(),
                before: PathState::Absent,
                after: PathState::Tree(SubtreeState {
                    digest: hash("contract-ancestor-tree"),
                    root_mode: Some(0o755),
                    descendants: vec![SubtreeEntry {
                        relative_path: "scrape".into(),
                        kind: TreeEntryKind::Directory,
                        sha256: None,
                        bytes: None,
                        mode: Some(0o755),
                    }],
                }),
            },
        ],
    };
    PreparedTransaction {
        project_identity_token: "stable-project-identity".into(),
        project_display_root: "C:/source".into(),
        plan_id: hash("plan"),
        canonical_plan: canonical_plan(TransactionMode::InPlace),
        snapshots: snapshots(TransactionMode::InPlace),
        mode: PreparedMode::InPlace(Box::new(InPlacePlan {
            quarantine_parent_identity: "same-volume-parent".into(),
            before_same_display_path: false,
            after_same_display_path: false,
            steps: vec![remove],
            contract: ContractCommit::DeleteLast {
                path: "vibevm/scrape/contract.toml".into(),
                empty_ancestors: vec!["vibevm/scrape".into(), "vibevm".into()],
            },
            contract_step: contract,
            contract_cleanup_step: Some(contract_cleanup),
            before_tree: manifest(
                "before",
                vec![
                    directory("vibevm"),
                    file("vibevm/data", "data"),
                    directory("vibevm/scrape"),
                    file("vibevm/scrape/contract.toml", "contract bytes"),
                ],
            ),
            pre_contract_tree: manifest(
                "pre-contract",
                vec![
                    directory("vibevm"),
                    directory("vibevm/scrape"),
                    file("vibevm/scrape/contract.toml", "contract bytes"),
                ],
            ),
            post_contract_tree: manifest(
                "post-contract",
                vec![directory("vibevm"), directory("vibevm/scrape")],
            ),
            after_tree: manifest("after", Vec::new()),
        })),
    }
}

fn complex_in_place_prepared() -> PreparedTransaction {
    let old = present("old cargo");
    let new = present("new cargo");
    let spec = SubtreeState {
        digest: hash("spec-tree"),
        root_mode: Some(0o755),
        descendants: vec![SubtreeEntry {
            relative_path: "guide.md".into(),
            kind: TreeEntryKind::File,
            sha256: Some(hash("spec")),
            bytes: Some(4),
            mode: Some(0o644),
        }],
    };
    let data = present("data");
    let capture = MutationStep {
        id: "capture-cargo".into(),
        pair_id: Some("cargo-pair".into()),
        kind: MutationKind::CaptureBeforeImage,
        transitions: vec![
            transition("Cargo.toml", old.clone(), old.clone()),
            PathTransition {
                location: Location::Quarantine,
                path: "before/Cargo.toml".into(),
                before: PathState::Absent,
                after: old.clone(),
            },
        ],
    };
    let rewrite = MutationStep {
        id: "rewrite-cargo".into(),
        pair_id: Some("cargo-pair".into()),
        kind: MutationKind::AtomicRewrite,
        transitions: vec![transition("Cargo.toml", old, new)],
    };
    let create_docs = MutationStep {
        id: "create-docs".into(),
        pair_id: None,
        kind: MutationKind::CreateRelocationParent,
        transitions: vec![transition(
            "docs",
            PathState::Absent,
            PathState::EmptyDirectory { mode: Some(0o755) },
        )],
    };
    let relocate = MutationStep {
        id: "relocate-spec".into(),
        pair_id: None,
        kind: MutationKind::Relocate,
        transitions: vec![
            transition(
                "vibevm/specs",
                PathState::Tree(spec.clone()),
                PathState::Absent,
            ),
            transition("docs/specs", PathState::Absent, PathState::Tree(spec)),
        ],
    };
    let remove = MutationStep {
        id: "remove-data".into(),
        pair_id: None,
        kind: MutationKind::QuarantineFile,
        transitions: vec![
            transition("vibevm/data", data.clone(), PathState::Absent),
            PathTransition {
                location: Location::Quarantine,
                path: "payload/vibevm/data".into(),
                before: PathState::Absent,
                after: data,
            },
        ],
    };
    let prune = MutationStep {
        id: "prune-empty".into(),
        pair_id: None,
        kind: MutationKind::PruneEmptyDirectory,
        transitions: vec![transition(
            "vibevm/empty",
            PathState::EmptyDirectory { mode: Some(0o755) },
            PathState::Absent,
        )],
    };
    let mut prepared = in_place_prepared();
    prepared.snapshots.push(Snapshot {
        kind: SnapshotKind::PreparedAfter,
        name: "after/rewrite-cargo".into(),
        bytes: b"new cargo".to_vec(),
        mode: Some(0o644),
    });
    let PreparedMode::InPlace(plan) = &mut prepared.mode else {
        unreachable!()
    };
    plan.steps = vec![capture, rewrite, create_docs, relocate, remove, prune];
    plan.before_tree = manifest(
        "complex-before",
        vec![
            file("Cargo.toml", "old cargo"),
            directory("vibevm"),
            file("vibevm/data", "data"),
            directory("vibevm/empty"),
            directory("vibevm/scrape"),
            file("vibevm/scrape/contract.toml", "contract bytes"),
            directory("vibevm/specs"),
            file("vibevm/specs/guide.md", "spec"),
        ],
    );
    plan.pre_contract_tree = manifest(
        "complex-pre-contract",
        vec![
            file("Cargo.toml", "new cargo"),
            directory("docs"),
            directory("docs/specs"),
            file("docs/specs/guide.md", "spec"),
            directory("vibevm"),
            directory("vibevm/scrape"),
            file("vibevm/scrape/contract.toml", "contract bytes"),
        ],
    );
    plan.post_contract_tree = manifest(
        "complex-post-contract",
        vec![
            file("Cargo.toml", "new cargo"),
            directory("docs"),
            directory("docs/specs"),
            file("docs/specs/guide.md", "spec"),
            directory("vibevm"),
            directory("vibevm/scrape"),
        ],
    );
    plan.after_tree = manifest(
        "complex-after",
        vec![
            file("Cargo.toml", "new cargo"),
            directory("docs"),
            directory("docs/specs"),
            file("docs/specs/guide.md", "spec"),
        ],
    );
    prepared
}

fn external_preserve_prepared() -> PreparedTransaction {
    let mut prepared = in_place_prepared();
    let PreparedMode::InPlace(plan) = &mut prepared.mode else {
        unreachable!()
    };
    plan.steps.clear();
    plan.contract = ContractCommit::ExternalPreserve;
    plan.contract_step = MutationStep {
        id: "external-preserve".into(),
        pair_id: None,
        kind: MutationKind::ContractExternalPreserve,
        transitions: Vec::new(),
    };
    plan.contract_cleanup_step = None;
    plan.before_tree = manifest("external", Vec::new());
    plan.pre_contract_tree = plan.before_tree.clone();
    plan.post_contract_tree = plan.before_tree.clone();
    plan.after_tree = plan.before_tree.clone();
    prepared
}

fn execute<I: FaultInjector>(
    prepared: PreparedTransaction,
    store: &mut MemoryStore,
    fs: &mut MemoryFs,
    verifier: &mut AcceptingVerifier,
    faults: &mut I,
) -> Result<TransactionReport, TransactionError> {
    let identity = prepared.project_identity_token.clone();
    let root = prepared.project_display_root.clone();
    Engine::new(store, fs, verifier, faults).execute_locked(&identity, &root, || Ok(prepared))
}

fn recover(
    store: &mut MemoryStore,
    fs: &mut MemoryFs,
    verifier: &mut AcceptingVerifier,
) -> Result<TransactionReport, TransactionError> {
    Engine::new(store, fs, verifier, &mut NoFaults).recover("stable-project-identity", "C:/source")
}
