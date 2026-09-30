//! Focused validate-only machine submission proofs.

use super::*;

#[path = "tests/fixture.rs"]
mod fixture;

fn project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("vibe.toml"),
        "[project]\nname = 'demo'\nversion = '1.1.0'\n",
    )
    .unwrap();
    dir
}

fn scope() -> SubmissionScope {
    SubmissionScope {
        context: "context".into(),
        target: "root".into(),
        principal: "owner".into(),
    }
}

fn issue(root: &Path, scope: &SubmissionScope) -> SubmissionNamespace {
    let lease = LifecycleLease::acquire(root).unwrap();
    SubmissionReservations::issue_namespace(&lease, scope, 60_000).unwrap()
}

fn request() -> DefaultLifecycleRequest {
    DefaultLifecycleRequest {
        requested: Phase::Validate,
        force: false,
        agent_mode: RunAgentMode::Cli,
        assume_yes: true,
        trace_flag: false,
        install_inputs: Default::default(),
        policy: Default::default(),
    }
}

fn call(
    root: &Path,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
) -> Result<SubmissionOutcome<RetainedLifecycleResult>> {
    submit_validate_lifecycle(
        root,
        namespace,
        scope,
        key,
        request(),
        TargetOs::Linux,
        "2026-09-30T00:00:05Z".parse().unwrap(),
    )
}

fn install_request() -> DefaultLifecycleRequest {
    let mut asked = request();
    asked.requested = Phase::Install;
    asked
}

fn call_install(
    root: &Path,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
) -> Result<SubmissionOutcome<RetainedLifecycleResult>> {
    submit_empty_world_install(
        root,
        namespace,
        scope,
        key,
        install_request(),
        TargetOs::Linux,
        "2026-09-30T00:00:05Z".parse().unwrap(),
    )
}

#[test]
fn empty_world_install_writes_boot_after_stage_and_replays_exact_result() {
    let dir = project();
    let scope = scope();
    let namespace = issue(dir.path(), &scope);
    let manifest_before = std::fs::read(dir.path().join("vibe.toml")).unwrap();
    assert!(!dir.path().join("vibe.lock").exists());
    let first = call_install(dir.path(), &namespace, &scope, "install-one").unwrap();
    let SubmissionOutcome::Invoked { run_id, outcome } = first else {
        panic!("first empty-world install must invoke");
    };
    assert_eq!(outcome.marker.outcome, MarkerOutcome::Completed);
    assert_eq!(outcome.marker.effect_state, EffectState::Uncertain);
    assert!(
        dir.path()
            .join(".vibe/lifecycle-outcomes")
            .join(format!("{run_id}.install-entered.json"))
            .is_file()
    );
    let boot = dir.path().join("vibevm/vibespecs/boot/INDEX.md");
    assert!(boot.is_file(), "empty-world install must regenerate boot");
    assert_eq!(
        std::fs::read(dir.path().join("vibe.toml")).unwrap(),
        manifest_before
    );
    assert!(!dir.path().join("vibe.lock").exists());
    std::fs::write(&boot, b"sentinel after completed install").unwrap();
    let second = call_install(dir.path(), &namespace, &scope, "install-one").unwrap();
    assert_eq!(second, SubmissionOutcome::Replayed { run_id, outcome });
    assert_eq!(
        std::fs::read(&boot).unwrap(),
        b"sentinel after completed install"
    );
}

#[test]
fn empty_install_attach_after_lock_change_reads_original_marker_only() {
    let dir = project();
    let scope = scope();
    let namespace = issue(dir.path(), &scope);
    let first = call_install(dir.path(), &namespace, &scope, "install-one").unwrap();
    let SubmissionOutcome::Invoked { run_id, outcome } = first else {
        panic!("first empty install must invoke")
    };
    let boot = dir.path().join("vibevm/vibespecs/boot/INDEX.md");
    std::fs::write(&boot, b"first-run sentinel").unwrap();
    let state_before = std::fs::read(dir.path().join(".vibe/lifecycle.toml")).unwrap();
    let index_before = std::fs::read(dir.path().join(".vibe/machine-submissions.json")).unwrap();
    let mut lock = vibe_core::manifest::Lockfile::empty("vibe 1.1.0", "2026-09-30T00:00:00Z");
    lock.meta
        .root_dependencies
        .push(vibe_core::PackageRef::parse("org.demo/changed@=1.1.0").unwrap());
    lock.write(dir.path().join("vibe.lock")).unwrap();
    assert_eq!(
        call_install(dir.path(), &namespace, &scope, "install-one").unwrap(),
        SubmissionOutcome::Replayed { run_id, outcome }
    );
    assert_eq!(std::fs::read(&boot).unwrap(), b"first-run sentinel");
    assert_eq!(
        std::fs::read(dir.path().join(".vibe/lifecycle.toml")).unwrap(),
        state_before
    );
    assert_eq!(
        std::fs::read(dir.path().join(".vibe/machine-submissions.json")).unwrap(),
        index_before
    );
    let fresh = call_install(dir.path(), &namespace, &scope, "install-two")
        .unwrap_err()
        .to_string();
    assert!(fresh.contains("preexisting lock dependencies"), "{fresh}");
}

#[test]
fn early_attach_rejects_changed_request_and_foreign_scope_without_writes() {
    let dir = project();
    let scope = scope();
    let namespace = issue(dir.path(), &scope);
    assert!(matches!(
        call(dir.path(), &namespace, &scope, "one").unwrap(),
        SubmissionOutcome::Invoked { .. }
    ));
    let index_before = std::fs::read(dir.path().join(".vibe/machine-submissions.json")).unwrap();
    let lease = LifecycleLease::acquire(dir.path()).unwrap();
    let mismatch = attach_existing_lifecycle(&lease, &namespace, &scope, "one", b"changed")
        .unwrap_err()
        .to_string();
    assert!(mismatch.contains("different request or plan"), "{mismatch}");
    let foreign = SubmissionScope {
        principal: "other".into(),
        ..scope.clone()
    };
    let refused = attach_existing_lifecycle(&lease, &namespace, &foreign, "one", VALIDATE_LINUX)
        .unwrap_err()
        .to_string();
    assert!(refused.contains("another context"), "{refused}");
    assert_eq!(
        std::fs::read(dir.path().join(".vibe/machine-submissions.json")).unwrap(),
        index_before
    );
}

#[test]
fn different_target_conflicts_but_new_observation_time_replays_original() {
    let dir = project();
    let scope = scope();
    let namespace = issue(dir.path(), &scope);
    let first = call(dir.path(), &namespace, &scope, "one").unwrap();
    let SubmissionOutcome::Invoked { run_id, outcome } = first else {
        panic!("first Validate must invoke")
    };
    let index_before = std::fs::read(dir.path().join(".vibe/machine-submissions.json")).unwrap();
    let wrong_target = submit_validate_lifecycle(
        dir.path(),
        &namespace,
        &scope,
        "one",
        request(),
        TargetOs::Macos,
        "2026-10-01T00:00:05Z".parse().unwrap(),
    )
    .unwrap_err()
    .to_string();
    assert!(
        wrong_target.contains("different request or plan"),
        "{wrong_target}"
    );
    assert_eq!(
        submit_validate_lifecycle(
            dir.path(),
            &namespace,
            &scope,
            "one",
            request(),
            TargetOs::Linux,
            "2026-10-01T00:00:05Z".parse().unwrap(),
        )
        .unwrap(),
        SubmissionOutcome::Replayed { run_id, outcome }
    );
    assert_eq!(
        std::fs::read(dir.path().join(".vibe/machine-submissions.json")).unwrap(),
        index_before
    );
}

#[test]
fn variable_policy_cannot_alias_a_fixed_existing_request() {
    let dir = project();
    let scope = scope();
    let namespace = issue(dir.path(), &scope);
    assert!(matches!(
        call(dir.path(), &namespace, &scope, "one").unwrap(),
        SubmissionOutcome::Invoked { .. }
    ));
    let mut altered = request();
    altered.policy.offline = true;
    let refused = submit_validate_lifecycle(
        dir.path(),
        &namespace,
        &scope,
        "one",
        altered,
        TargetOs::Linux,
        "2026-10-01T00:00:05Z".parse().unwrap(),
    )
    .unwrap_err()
    .to_string();
    assert!(refused.contains("fixed request refuses"), "{refused}");
}

#[test]
fn empty_install_marker_loss_requires_recovery_and_never_regenerates_boot() {
    let dir = project();
    let scope = scope();
    let namespace = issue(dir.path(), &scope);
    let first = call_install(dir.path(), &namespace, &scope, "install-one").unwrap();
    let SubmissionOutcome::Invoked { run_id, .. } = first else {
        panic!("first install must invoke");
    };
    let boot = dir.path().join("vibevm/vibespecs/boot/INDEX.md");
    std::fs::write(&boot, b"sentinel after interrupted report").unwrap();
    std::fs::remove_file(
        dir.path()
            .join(".vibe/lifecycle-outcomes")
            .join(format!("{run_id}.marker.json")),
    )
    .unwrap();
    assert_eq!(
        call_install(dir.path(), &namespace, &scope, "install-one").unwrap(),
        SubmissionOutcome::RecoveryRequired { run_id }
    );
    assert_eq!(
        std::fs::read(&boot).unwrap(),
        b"sentinel after interrupted report"
    );
}

#[test]
fn empty_install_same_key_attaches_original_after_manifest_change_without_boot_write() {
    let dir = project();
    let scope = scope();
    let namespace = issue(dir.path(), &scope);
    let first = call_install(dir.path(), &namespace, &scope, "install-one").unwrap();
    let SubmissionOutcome::Invoked { run_id, outcome } = first else {
        panic!("first empty install must invoke")
    };
    let boot = dir.path().join("vibevm/vibespecs/boot/INDEX.md");
    std::fs::write(&boot, b"sentinel after first install").unwrap();
    let state_before = std::fs::read(dir.path().join(".vibe/lifecycle.toml")).unwrap();
    std::fs::write(
        dir.path().join("vibe.toml"),
        "[project]\nname = 'changed'\nversion = '1.1.0'\n",
    )
    .unwrap();
    let index_before = std::fs::read(dir.path().join(".vibe/machine-submissions.json")).unwrap();
    assert_eq!(
        call_install(dir.path(), &namespace, &scope, "install-one").unwrap(),
        SubmissionOutcome::Replayed { run_id, outcome }
    );
    assert_eq!(
        std::fs::read(dir.path().join(".vibe/machine-submissions.json")).unwrap(),
        index_before
    );
    let new_key = call_install(dir.path(), &namespace, &scope, "install-two")
        .unwrap_err()
        .to_string();
    assert!(new_key.contains("uncertain"), "{new_key}");
    assert_eq!(
        std::fs::read(&boot).unwrap(),
        b"sentinel after first install"
    );
    assert_eq!(
        std::fs::read(dir.path().join(".vibe/lifecycle.toml")).unwrap(),
        state_before
    );
}

#[test]
fn empty_install_refuses_declared_package_before_stage_or_boot() {
    let dir = project();
    std::fs::write(
        dir.path().join("vibe.toml"),
        "[project]\nname = 'demo'\nversion = '1.1.0'\n[requires.packages]\n'org.demo/missing' = '=1.1.0'\n",
    )
    .unwrap();
    let scope = scope();
    let namespace = issue(dir.path(), &scope);
    let error = call_install(dir.path(), &namespace, &scope, "install-one")
        .unwrap_err()
        .to_string();
    assert!(error.contains("refuses requirements"), "{error}");
    assert!(!dir.path().join(".vibe/lifecycle-outcomes").exists());
    assert!(!dir.path().join("vibevm/vibespecs/boot/INDEX.md").exists());
}

#[test]
fn empty_install_refuses_lock_dependencies_before_stage_or_boot() {
    let dir = project();
    let mut lock = vibe_core::manifest::Lockfile::empty("vibe 1.1.0", "2026-09-30T00:00:00Z");
    lock.meta
        .root_dependencies
        .push(vibe_core::PackageRef::parse("org.demo/missing@=1.1.0").unwrap());
    lock.write(dir.path().join("vibe.lock")).unwrap();
    let scope = scope();
    let namespace = issue(dir.path(), &scope);
    let error = call_install(dir.path(), &namespace, &scope, "install-one")
        .unwrap_err()
        .to_string();
    assert!(error.contains("preexisting lock dependencies"), "{error}");
    assert!(!dir.path().join(".vibe/lifecycle-outcomes").exists());
    assert!(!dir.path().join("vibevm/vibespecs/boot/INDEX.md").exists());
}

#[test]
fn validate_result_is_retained_and_lost_response_replays_exact_bytes() {
    let dir = project();
    let scope = scope();
    let namespace = issue(dir.path(), &scope);
    let first = call(dir.path(), &namespace, &scope, "one").unwrap();
    let SubmissionOutcome::Invoked { run_id, outcome } = first else {
        panic!("first call must invoke a real validate run");
    };
    assert_eq!(outcome.marker.run_id, run_id);
    assert_eq!(outcome.marker.outcome, MarkerOutcome::Completed);
    assert!(outcome.report_bytes.is_some());
    let report: vibe_wire::generated::lifecycle_report::LifecycleReport =
        serde_json::from_slice(outcome.report_bytes.as_deref().unwrap()).unwrap();
    assert!(report.ok);
    assert_eq!(report.steps.len(), 1);
    assert_eq!(report.steps[0].phase, "validate");
    assert!(
        !dir.path().join(".vibe/lifecycle").exists(),
        "validate has no scratch"
    );
    let state_before = std::fs::read(dir.path().join(".vibe/lifecycle.toml")).unwrap();
    let second = call(dir.path(), &namespace, &scope, "one").unwrap();
    assert_eq!(second, SubmissionOutcome::Replayed { run_id, outcome });
    assert_eq!(
        std::fs::read(dir.path().join(".vibe/lifecycle.toml")).unwrap(),
        state_before
    );
}

#[test]
fn pending_reservation_attaches_read_only_without_header_or_scratch() {
    let dir = project();
    let scope = scope();
    let lease = Arc::new(LifecycleLease::acquire(dir.path()).unwrap());
    let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
    let (snapshot, witness) = SelectedManifest::read_witnessed(dir.path()).unwrap();
    let selection = snapshot.prepare();
    let id = select_run_identity_for_reservation(
        &lease,
        dir.path(),
        "validate",
        &["validate".into()],
        ".",
        RunAgentMode::Cli,
        false,
        false,
        "2026-09-30T00:00:00Z".into(),
    )
    .unwrap();
    let plan = canonical_lifecycle_pre_effect_plan(&LifecyclePreEffectPlanInput {
        lease_root: lease.root(),
        selected_root: dir.path(),
        selected_node: ".",
        witness: &witness,
        workspace_witness: &witness,
        package_source: None,
        requested: Phase::Validate,
        agent_mode: &RunAgentMode::Cli,
        trace_flag: false,
        effective_trace: false,
        force: false,
        assume_yes: true,
        policy: &request().policy,
        install_inputs: &request().install_inputs,
    })
    .unwrap();
    assert!(selection.loaded_root().is_some());
    let reserved = SubmissionReservations::reserve(
        &lease,
        &namespace,
        &scope,
        "one",
        VALIDATE_LINUX,
        &plan,
        &id.run_id,
    )
    .unwrap();
    assert!(matches!(
        reserved,
        vibe_lifecycle::ReservationDecision::Created { .. }
    ));
    assert!(!dir.path().join(".vibe/lifecycle.toml").exists());
    assert!(!dir.path().join(".vibe/lifecycle").exists());
    drop(lease);
    let replay = call(dir.path(), &namespace, &scope, "one").unwrap();
    assert_eq!(
        replay,
        SubmissionOutcome::RecoveryRequired { run_id: id.run_id }
    );
    assert!(!dir.path().join(".vibe/lifecycle.toml").exists());
    assert!(!dir.path().join(".vibe/lifecycle").exists());
}

#[test]
fn changed_witness_same_key_attaches_original_and_fresh_key_refuses() {
    let dir = project();
    let scope = scope();
    let namespace = issue(dir.path(), &scope);
    let first = call(dir.path(), &namespace, &scope, "one").unwrap();
    let SubmissionOutcome::Invoked { run_id, outcome } = first else {
        panic!("first run");
    };
    let state_before = std::fs::read(dir.path().join(".vibe/lifecycle.toml")).unwrap();
    std::fs::write(
        dir.path().join("vibe.toml"),
        "[project]\nname = 'changed'\nversion = '1.1.0'\n",
    )
    .unwrap();
    assert_eq!(
        call(dir.path(), &namespace, &scope, "one").unwrap(),
        SubmissionOutcome::Replayed {
            run_id: run_id.clone(),
            outcome
        }
    );
    let new_key = call(dir.path(), &namespace, &scope, "two")
        .unwrap_err()
        .to_string();
    assert!(new_key.contains("uncertain"), "{new_key}");
    assert_eq!(
        std::fs::read(dir.path().join(".vibe/lifecycle.toml")).unwrap(),
        state_before
    );
    assert!(!dir.path().join(".vibe/lifecycle").exists());
    assert!(vibe_lifecycle::process::is_valid_run_id(&run_id));
}

#[test]
fn missing_marker_after_run_is_recovery_required_without_a_second_run() {
    let dir = project();
    let scope = scope();
    let namespace = issue(dir.path(), &scope);
    let first = call(dir.path(), &namespace, &scope, "one").unwrap();
    let SubmissionOutcome::Invoked { run_id, .. } = first else {
        panic!("first run");
    };
    let state_before = std::fs::read(dir.path().join(".vibe/lifecycle.toml")).unwrap();
    std::fs::remove_file(
        dir.path()
            .join(".vibe/lifecycle-outcomes")
            .join(format!("{run_id}.marker.json")),
    )
    .unwrap();
    assert_eq!(
        call(dir.path(), &namespace, &scope, "one").unwrap(),
        SubmissionOutcome::RecoveryRequired { run_id }
    );
    assert_eq!(
        std::fs::read(dir.path().join(".vibe/lifecycle.toml")).unwrap(),
        state_before
    );
}

#[test]
fn ready_header_check_refuses_a_different_start_for_the_proposed_run() {
    let dir = project();
    let lease = Arc::new(LifecycleLease::acquire(dir.path()).unwrap());
    let run_id = "a".repeat(32);
    let store = LifecycleStateStore::begin(
        lease.clone(),
        "validate".into(),
        vec!["validate".into()],
        "2026-09-29T00:00:00Z".into(),
        run_id.clone(),
        ".".into(),
        false,
    )
    .unwrap();
    drop(store);
    let error = ProtectedJournal::header(
        &lease,
        &run_id,
        Some("2026-09-30T00:00:00Z"),
        Phase::Validate,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("differs"), "{error}");
    assert!(!dir.path().join(".vibe/lifecycle").exists());
}

#[test]
fn workspace_member_refuses_before_reservation_or_run() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("vibe.toml"),
        "[project]\nname = 'root'\nversion = '1.1.0'\n[workspace]\nmembers = ['member']\n",
    )
    .unwrap();
    let member = dir.path().join("member");
    std::fs::create_dir(&member).unwrap();
    std::fs::write(
        member.join("vibe.toml"),
        "[package]\ngroup = 'org.demo'\nname = 'member'\nkind = 'flow'\nversion = '1.1.0'\n",
    )
    .unwrap();
    let scope = scope();
    let namespace = issue(dir.path(), &scope);
    let error = call(&member, &namespace, &scope, "one")
        .unwrap_err()
        .to_string();
    assert!(error.contains("exactly one workspace node"), "{error}");
    assert!(!dir.path().join(".vibe/lifecycle.toml").exists());
    assert!(!member.join(".vibe/lifecycle").exists());
}
