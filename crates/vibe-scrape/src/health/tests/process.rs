#[test]
fn backend_missing_capability_refuses_before_execution_and_tree_drift_refuses() {
    let temp = tempfile::tempdir().unwrap();
    write_cargo_fixture(temp.path());
    let (project, inventory) = observed(temp.path());
    let contract = contract(
        r#"[[healthcheck]]
id = "rust"
kind = "cargo"
root = "."
build = "check"
workspace = false
locked = false
all_targets = false
tests = "skip"
profile = "dev"
features = []
timeout_seconds = 10"#,
        "strict",
        "deny",
    );
    let health = prepare(
        &project,
        &contract,
        &inventory,
        &mut FakeResolver::new(TestPresence::Absent),
    )
    .unwrap();
    let seal = tree::TreeSeal::from_inventory(&inventory);
    let context = PhaseContext {
        phase: HealthPhase::Before,
        root: temp.path().display().to_string(),
        protected_root: temp.path().display().to_string(),
        scratch: "C:/scratch".to_owned(),
        result: "C:/result".to_owned(),
        same_display_path_required: false,
        transactional_tree_reproof: false,
        expected_tree: seal.clone(),
        cancellation: CancellationToken::new(),
    };
    let mut unsupported = FakeBackend::default();
    assert!(run_phase(&mut unsupported, &health, &context).is_err());
    assert_eq!(unsupported.calls, 0);

    let execution = CommandExecution {
        step: CommandStep::Verify,
        actual_argv: vec!["fake".to_owned()],
        exit_code: 0,
        stdout: empty_stream(8),
        stderr: empty_stream(8),
        result: None,
    };
    let mut changed = seal.clone();
    changed.tree_digest = "sha256:changed".to_owned();
    let mut backend = FakeBackend {
        capabilities: full_capabilities(),
        executions: VecDeque::from([execution]),
        observed: Some(changed),
        calls: 0,
    };
    assert!(matches!(
        run_phase(&mut backend, &health, &context),
        Err(HealthError::Tree(_))
    ));
}

#[test]
fn native_platform_stub_advertises_no_unimplemented_capability() {
    let backend = platform::native_backend();
    assert_eq!(backend.capabilities(), BackendCapabilities::default());
}

#[test]
fn tree_seal_reports_extra_missing_and_changed_entries() {
    let expected = tree::TreeSeal {
        tree_digest: "a".to_owned(),
        entries: vec![tree::TreeSealEntry {
            path: "a".to_owned(),
            kind: tree::TreeEntryKind::File,
            sha256: Some("one".to_owned()),
            bytes: Some(1),
            mode: None,
        }],
    };
    let observed = tree::TreeSeal {
        tree_digest: "b".to_owned(),
        entries: vec![tree::TreeSealEntry {
            path: "b".to_owned(),
            kind: tree::TreeEntryKind::File,
            sha256: Some("two".to_owned()),
            bytes: Some(1),
            mode: None,
        }],
    };
    let diff = expected.compare(&observed);
    assert!(
        diff.iter()
            .any(|item| matches!(item, tree::TreeDifference::Missing(path) if path == "a"))
    );
    assert!(
        diff.iter()
            .any(|item| matches!(item, tree::TreeDifference::Extra(path) if path == "b"))
    );
}

#[test]
fn evidence_store_capacity_blocks_oversized_caps_and_accepts_bounded_panel() {
    let inventory = crate::model::Inventory {
        entries: Vec::new(),
        tree_digest: format!("sha256:{}", "0".repeat(64)),
    };
    let mut oversized = local_process_plan(&["--help"], 10);
    oversized.max_stdout_bytes = crate::contract::MAX_HEALTH_STREAM_BYTES;
    oversized.max_stderr_bytes = crate::contract::MAX_HEALTH_STREAM_BYTES;
    assert_eq!(
        super::prepare::persistence_capacity_blocker(&oversized, &inventory)
            .unwrap()
            .unwrap()
            .code,
        "health-evidence-store-capacity"
    );

    let mut bounded = local_process_plan(&["--help"], 10);
    bounded.max_stdout_bytes = 1024 * 1024;
    bounded.max_stderr_bytes = 1024 * 1024;
    assert!(
        super::prepare::persistence_capacity_blocker(&bounded, &inventory)
            .unwrap()
            .is_none()
    );
}

#[test]
fn local_process_primitive_runs_a_sealed_group_in_an_isolated_view() {
    let phase = tempfile::tempdir().unwrap();
    let protected = tempfile::tempdir().unwrap();
    let mut backend = LocalProcessBackend::new();
    let mut plan = local_process_plan(&["--help"], 10);
    plan.checks[0].sandbox.graceful_termination = false;
    let result = run_phase(&mut backend, &plan, &local_context(&phase, &protected)).unwrap();
    assert_eq!(result.checks.len(), 1);
    assert_eq!(result.checks[0].commands[0].exit_code, 0);
}

#[test]
fn nonzero_health_preserves_exact_argv_and_bounded_stream_evidence() {
    let phase = tempfile::tempdir().unwrap();
    let protected = tempfile::tempdir().unwrap();
    let mut backend = LocalProcessBackend::new();
    let mut plan = local_process_plan(&["--definitely-invalid-vibe-health-option"], 10);
    plan.checks[0].sandbox.graceful_termination = false;
    let error = run_phase(&mut backend, &plan, &local_context(&phase, &protected)).unwrap_err();
    let HealthError::CommandFailed { execution, .. } = error else {
        panic!("expected retained command failure, got {error}")
    };
    assert!(
        execution
            .actual_argv
            .iter()
            .any(|arg| arg == "--definitely-invalid-vibe-health-option")
    );
    assert!(execution.stderr.total_bytes != 0 || execution.stdout.total_bytes != 0);
}

#[test]
fn later_command_failure_preserves_every_prior_execution() {
    let phase = tempfile::tempdir().unwrap();
    let protected = tempfile::tempdir().unwrap();
    let mut backend = LocalProcessBackend::new();
    let mut plan = local_process_plan(&["--help"], 10);
    plan.checks[0].sandbox.graceful_termination = false;
    let mut failing = plan.checks[0].commands[0].clone();
    failing.argv = vec![PreparedArg::Literal(
        "--definitely-invalid-vibe-health-option".to_owned(),
    )];
    plan.checks[0].commands.push(failing);
    let error = run_phase(&mut backend, &plan, &local_context(&phase, &protected)).unwrap_err();
    let HealthError::CommandFailed {
        prior_executions,
        execution,
        ..
    } = error
    else {
        panic!("expected retained later command failure, got {error}")
    };
    assert_eq!(prior_executions.len(), 1);
    assert_eq!(prior_executions[0].exit_code, 0);
    assert!(
        prior_executions[0]
            .actual_argv
            .iter()
            .any(|arg| arg == "--help")
    );
    assert!(
        execution
            .actual_argv
            .iter()
            .any(|arg| arg == "--definitely-invalid-vibe-health-option")
    );
}

#[test]
fn later_check_failure_preserves_every_completed_check() {
    let phase = tempfile::tempdir().unwrap();
    let protected = tempfile::tempdir().unwrap();
    let mut backend = LocalProcessBackend::new();
    let mut plan = local_process_plan(&["--help"], 10);
    plan.checks[0].sandbox.graceful_termination = false;
    plan.checks[0].id = "first".to_owned();
    let mut second = plan.checks[0].clone();
    second.id = "second".to_owned();
    second.commands[0].argv = vec![PreparedArg::Literal(
        "--definitely-invalid-vibe-health-option".to_owned(),
    )];
    plan.checks.push(second);
    let error = run_phase(&mut backend, &plan, &local_context(&phase, &protected)).unwrap_err();
    let HealthError::CommandFailed {
        prior_checks,
        prior_executions,
        ..
    } = error
    else {
        panic!("expected retained later check failure, got {error}")
    };
    assert_eq!(prior_checks.len(), 1);
    assert_eq!(prior_checks[0].id, "first");
    assert_eq!(prior_checks[0].commands.len(), 1);
    assert!(prior_executions.is_empty());
}

#[test]
fn exact_copy_after_is_accepted_only_with_reduced_assurance() {
    let phase = tempfile::tempdir().unwrap();
    let protected = tempfile::tempdir().unwrap();
    let mut backend = LocalProcessBackend::new();
    let mut plan = local_process_plan(&["--help"], 10);
    plan.checks[0].sandbox.graceful_termination = false;
    plan.checks[0].sandbox.termination_mode = if cfg!(windows) {
        TerminationMode::ForcedTree
    } else {
        TerminationMode::GracefulThenForced
    };
    let before = run_phase(&mut backend, &plan, &local_context(&phase, &protected)).unwrap();
    let mut after_context = local_context(&phase, &protected);
    after_context.phase = HealthPhase::After;
    let mut after = run_phase(&mut backend, &plan, &after_context).unwrap();
    // The transaction verifier marks a different-path exact copy reduced;
    // run_phase itself has no authority to classify how the view was created.
    after.assurance_reduced = true;
    assert!(after.assurance_reduced);
    assert_eq!(
        judge(BaselinePolicy::Strict, &before, &after),
        BaselineDecision::AcceptReduced
    );
}

#[test]
fn local_backend_materializes_sealed_exit_code_custom_bundle() {
    let phase = tempfile::tempdir().unwrap();
    let protected = tempfile::tempdir().unwrap();
    let mut backend = LocalProcessBackend::new();
    let mut plan = local_process_plan(&[], 10);
    let check = &mut plan.checks[0];
    check.kind = HealthcheckKind::Custom;
    check.protocol = ResultProtocol::ExitCode;
    check.effects = EffectPlan {
        reads: vec!["**".to_owned()],
        writes: Vec::new(),
        spawn: true,
    };
    check.sandbox = SandboxRequirement::for_check(NetworkMode::Inherit, true, true);
    check.sandbox.read_policy_enforcement = false;
    check.commands[0].argv = vec![
        PreparedArg::BundlePath("health.txt".to_owned()),
        PreparedArg::Literal("--list".to_owned()),
    ];
    let member_sha = format!("sha256:{:x}", Sha256::digest(b"ok"));
    check.custom_bundle = Some(CustomBundle {
        sha256: format!("sha256:{}", "1".repeat(64)),
        source: "health.txt".to_owned(),
        entries: vec![
            BundleEntry {
                path: "health.txt".to_owned(),
                kind: BundleEntryKind::File,
                sha256: Some(member_sha),
                bytes: Some(2),
                mode: None,
                content: Some(b"ok".to_vec()),
            },
            BundleEntry {
                path: "support".to_owned(),
                kind: BundleEntryKind::Directory,
                sha256: None,
                bytes: None,
                mode: None,
                content: None,
            },
            BundleEntry {
                path: "support/extra.txt".to_owned(),
                kind: BundleEntryKind::File,
                sha256: Some(format!("sha256:{:x}", Sha256::digest(b"extra"))),
                bytes: Some(5),
                mode: None,
                content: Some(b"extra".to_vec()),
            },
        ],
    });
    let context = local_context(&phase, &protected);
    let result = run_phase(&mut backend, &plan, &context).unwrap();
    assert_eq!(result.checks[0].commands[0].exit_code, 0);
    assert!(
        std::path::Path::new(&context.scratch)
            .join("local/verifier-bundle/support/extra.txt")
            .is_file()
    );
}

#[test]
fn local_backend_child_sleeps_only_when_invoked_as_the_timeout_fixture() {
    if std::env::var_os("VIBE_HEALTH_TIMEOUT_FIXTURE").is_some() {
        std::thread::sleep(std::time::Duration::from_secs(5));
    }
}

#[test]
fn local_backend_child_mutates_only_when_invoked_as_the_tree_drift_fixture() {
    if let Some(target) = std::env::var_os("VIBE_HEALTH_TREE_DRIFT_FIXTURE") {
        std::fs::write(target, b"changed").unwrap();
    }
}

#[test]
#[allow(clippy::zombie_processes)] // Deliberate orphan: the outer command-group must own/reap it.
fn local_backend_pipe_descendant_fixture() {
    match std::env::var("VIBE_HEALTH_PIPE_FIXTURE").as_deref() {
        Ok("leader") => {
            std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "health::tests::local_backend_pipe_descendant_fixture",
                    "--nocapture",
                ])
                .env("VIBE_HEALTH_PIPE_FIXTURE", "descendant")
                .spawn()
                .unwrap();
        }
        Ok("descendant") => std::thread::sleep(std::time::Duration::from_secs(10)),
        _ => {}
    }
}

#[test]
fn local_process_primitive_times_out_and_terminates_its_group() {
    let phase = tempfile::tempdir().unwrap();
    let protected = tempfile::tempdir().unwrap();
    let mut plan = local_process_plan(
        &[
            "--exact",
            "health::tests::local_backend_child_sleeps_only_when_invoked_as_the_timeout_fixture",
            "--nocapture",
        ],
        1,
    );
    plan.checks[0].sandbox.graceful_termination = false;
    plan.checks[0].commands[0].environment.insert(
        "VIBE_HEALTH_TIMEOUT_FIXTURE".to_owned(),
        EnvironmentValue::Literal("1".to_owned()),
    );
    let error = run_phase(
        &mut LocalProcessBackend::new(),
        &plan,
        &local_context(&phase, &protected),
    )
    .unwrap_err();
    let HealthError::TimedOut {
        execution,
        phase: HealthPhase::Before,
        ..
    } = error
    else {
        panic!("expected retained timeout evidence, got {error}")
    };
    assert!(execution.actual_argv.iter().any(|arg| arg == "--nocapture"));
}

#[test]
fn after_copy_tree_drift_retains_evidence_and_leaves_delivered_tree_unchanged() {
    let phase = tempfile::tempdir().unwrap();
    let protected = tempfile::tempdir().unwrap();
    let mut plan = local_process_plan(
        &[
            "--exact",
            "health::tests::local_backend_child_mutates_only_when_invoked_as_the_tree_drift_fixture",
            "--nocapture",
        ],
        10,
    );
    plan.checks[0].sandbox.graceful_termination = false;
    plan.checks[0].commands[0].environment.insert(
        "VIBE_HEALTH_TREE_DRIFT_FIXTURE".to_owned(),
        EnvironmentValue::Literal("health-mutation.txt".to_owned()),
    );
    let mut context = local_context(&phase, &protected);
    context.phase = HealthPhase::After;
    let error = run_phase(&mut LocalProcessBackend::new(), &plan, &context).unwrap_err();
    let HealthError::CommandChangedTree { execution, .. } = error else {
        panic!("expected retained tree-drift evidence, got {error}")
    };
    assert!(execution.actual_argv.iter().any(|arg| arg == "--nocapture"));
    assert!(phase.path().join("health-mutation.txt").is_file());
    assert_eq!(std::fs::read_dir(protected.path()).unwrap().count(), 0);
}

#[test]
fn timeout_covers_descendant_held_pipes_without_hanging() {
    let phase = tempfile::tempdir().unwrap();
    let protected = tempfile::tempdir().unwrap();
    let mut plan = local_process_plan(
        &[
            "--exact",
            "health::tests::local_backend_pipe_descendant_fixture",
            "--nocapture",
        ],
        1,
    );
    plan.checks[0].sandbox.graceful_termination = false;
    plan.checks[0].commands[0].environment.insert(
        "VIBE_HEALTH_PIPE_FIXTURE".to_owned(),
        EnvironmentValue::Literal("leader".to_owned()),
    );
    let started = std::time::Instant::now();
    let error = run_phase(
        &mut LocalProcessBackend::new(),
        &plan,
        &local_context(&phase, &protected),
    )
    .unwrap_err();
    assert!(error.to_string().contains("timed out"));
    assert!(started.elapsed() < std::time::Duration::from_secs(5));
}

#[test]
fn cancellation_terminates_live_descendant_group_and_is_phase_typed() {
    let phase = tempfile::tempdir().unwrap();
    let protected = tempfile::tempdir().unwrap();
    let mut plan = local_process_plan(
        &[
            "--exact",
            "health::tests::local_backend_pipe_descendant_fixture",
            "--nocapture",
        ],
        30,
    );
    plan.checks[0].sandbox.graceful_termination = false;
    plan.checks[0].commands[0].environment.insert(
        "VIBE_HEALTH_PIPE_FIXTURE".to_owned(),
        EnvironmentValue::Literal("leader".to_owned()),
    );
    let mut context = local_context(&phase, &protected);
    let cancellation = context.cancellation.clone();
    let trigger = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(100));
        cancellation.cancel();
    });
    let started = std::time::Instant::now();
    let error = run_phase(&mut LocalProcessBackend::new(), &plan, &context).unwrap_err();
    trigger.join().unwrap();
    let HealthError::Cancelled {
        phase: HealthPhase::Before,
        disposition: CancellationDisposition::RefuseBefore,
        execution,
        ..
    } = error
    else {
        panic!("expected retained before-cancellation evidence, got {error}")
    };
    assert!(execution.actual_argv.iter().any(|arg| arg == "--nocapture"));
    assert!(started.elapsed() < std::time::Duration::from_secs(5));

    context.phase = HealthPhase::After;
    context.cancellation = CancellationToken::new();
    context.cancellation.cancel();
    let error = run_phase(&mut LocalProcessBackend::new(), &plan, &context).unwrap_err();
    assert!(matches!(
        error,
        HealthError::Cancelled {
            phase: HealthPhase::After,
            disposition: CancellationDisposition::RollbackAfter,
            ..
        }
    ));
}

#[cfg(windows)]
#[test]
fn windows_sealed_asset_handle_blocks_replacement_and_write() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("tool.exe");
    let bytes = std::fs::read(std::env::current_exe().unwrap()).unwrap();
    std::fs::write(&path, &bytes).unwrap();
    let asset = AssetIdentity {
        id: "tool".to_owned(),
        role: AssetRole::CustomNative,
        display_path: path.display().to_string(),
        sha256: format!("sha256:{:x}", Sha256::digest(&bytes)),
        bytes: bytes.len() as u64,
        mode: None,
        platform_identity: "test".to_owned(),
        version: "test".to_owned(),
        version_kind: VersionKind::Content,
        source: AssetSource::Resolved,
        live_identity: Some(opaque_identity(&path)),
    };
    let anchor = tempfile::tempdir().unwrap();
    let identity_project = Project::open(anchor.path()).unwrap();
    let held = super::local::verify_asset(&asset, &identity_project).unwrap();
    assert!(std::fs::rename(&path, temp.path().join("replacement.exe")).is_err());
    assert!(std::fs::OpenOptions::new().write(true).open(&path).is_err());
    drop(held);
    assert!(std::fs::rename(&path, temp.path().join("replacement.exe")).is_ok());
}
