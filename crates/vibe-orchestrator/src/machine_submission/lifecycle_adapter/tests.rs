//! Focused validate-only machine submission proofs.

use super::*;

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
fn pending_reservation_precedes_header_and_never_allocates_scratch() {
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
        VALIDATE_REQUEST,
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
    assert!(dir.path().join(".vibe/lifecycle.toml").exists());
    assert!(!dir.path().join(".vibe/lifecycle").exists());
}

#[test]
fn changed_witness_and_foreign_header_do_not_start_another_run() {
    let dir = project();
    let scope = scope();
    let namespace = issue(dir.path(), &scope);
    let first = call(dir.path(), &namespace, &scope, "one").unwrap();
    let SubmissionOutcome::Invoked { run_id, .. } = first else {
        panic!("first run");
    };
    let state_before = std::fs::read(dir.path().join(".vibe/lifecycle.toml")).unwrap();
    std::fs::write(
        dir.path().join("vibe.toml"),
        "[project]\nname = 'changed'\nversion = '1.1.0'\n",
    )
    .unwrap();
    let same_key = call(dir.path(), &namespace, &scope, "one")
        .unwrap_err()
        .to_string();
    assert!(same_key.contains("different request or plan"), "{same_key}");
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
    let error = ValidateJournal::header(&lease, &run_id, Some("2026-09-30T00:00:00Z"))
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
