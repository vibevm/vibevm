//! Focused validate-only machine submission proofs.

use super::*;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use vibe_lifecycle::process::{ProcessError, ProcessOutput, ProcessRunner, ProcessSpec};

struct CountedScriptProcess {
    count: AtomicUsize,
    root: PathBuf,
}
impl CountedScriptProcess {
    fn new(root: &Path) -> Self {
        Self {
            count: AtomicUsize::new(0),
            root: root.to_path_buf(),
        }
    }
    fn count(&self) -> usize {
        self.count.load(Ordering::SeqCst)
    }
}
impl ProcessRunner for CountedScriptProcess {
    fn run(&self, _: &ProcessSpec) -> Result<ProcessOutput, ProcessError> {
        let staged = std::fs::read_dir(self.root.join(".vibe/lifecycle-outcomes"))
            .unwrap()
            .any(|entry| {
                entry
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .ends_with(".script-source.json")
            });
        assert!(
            staged,
            "script source stage must be durable before process.run"
        );
        self.count.fetch_add(1, Ordering::SeqCst);
        Ok(ProcessOutput {
            code: Some(0),
            ..ProcessOutput::default()
        })
    }
}
struct ScriptProbe;
impl vibe_workspace::hooks::InterpreterProbe for ScriptProbe {
    fn has(&self, program: &str) -> bool {
        matches!(program, "bash" | "powershell")
    }
}

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

fn script_project() -> tempfile::TempDir {
    let dir = project();
    std::fs::write(dir.path().join("vibe.toml"), ONE_SCRIPT_VALIDATE_MANIFEST).unwrap();
    std::fs::write(dir.path().join("run.sh"), b"echo one\n").unwrap();
    if cfg!(windows) {
        std::fs::remove_file(dir.path().join("run.sh")).unwrap();
        std::fs::write(dir.path().join("run.ps1"), b"Write-Output one\n").unwrap();
    }
    dir
}

fn agent_project() -> tempfile::TempDir {
    let dir = project();
    std::fs::write(dir.path().join("vibe.toml"), ONE_AGENT_VALIDATE_MANIFEST).unwrap();
    let prompt = dir.path().join(ONE_AGENT_PROMPT_PATH);
    std::fs::create_dir_all(prompt.parent().unwrap()).unwrap();
    std::fs::write(prompt, b"# Prompt {#root}\n\nWrite the guide.\n").unwrap();
    dir
}

fn agent_request() -> DefaultLifecycleRequest {
    let mut asked = request();
    asked.agent_mode = RunAgentMode::Agent;
    asked
}

#[test]
fn one_agent_preflight_binds_exact_local_prompt_without_effect() {
    let dir = agent_project();
    let admitted =
        admit_one_agent_validate_fixture(dir.path(), &agent_request(), TargetOs::Linux).unwrap();
    assert_eq!(admitted.source.prompt_relative, ONE_AGENT_PROMPT_PATH);
    assert_eq!(
        admitted.source.source.bytes,
        b"# Prompt {#root}\n\nWrite the guide.\n"
    );
    assert_eq!(admitted.address, ONE_AGENT_PROMPT_ADDRESS);
    assert!(admitted.execution_key.contains("park"));
    assert!(!admitted.provider.is_empty());
    let resolver = vibe_lifecycle::agent::SelectedWorldPromptResolver::new(dir.path());
    let resolved = resolver
        .resolve(&vibe_lifecycle::agent::PromptRequest {
            address: admitted.address.clone(),
            provider_root: admitted.source.provider_root.clone(),
            provider_group: "org.demo".into(),
            provider_name: "demo".into(),
            selected_world: Default::default(),
        })
        .unwrap();
    assert!(resolved.text.contains("Write the guide."));
    assert!(resolved.unsupported.is_empty());
    let request_value: serde_json::Value =
        serde_json::from_slice(&admitted.canonical_request).unwrap();
    let plan_value: serde_json::Value = serde_json::from_slice(&admitted.canonical_plan).unwrap();
    assert_eq!(plan_value["format"], "vibevm.lifecycle.pre-effect-plan/5");
    assert_eq!(plan_value["agent_mode"], "agent");
    assert_eq!(request_value["prompt_source"], plan_value["prompt_source"]);
    assert!(!dir.path().join(".vibe/machine-submissions.json").exists());
    assert!(!dir.path().join(".vibe/lifecycle.toml").exists());
    assert!(!dir.path().join(".vibe/agentic").exists());
}

#[test]
fn one_agent_changed_prompt_conflicts_with_reserved_request() {
    let dir = agent_project();
    let first =
        admit_one_agent_validate_fixture(dir.path(), &agent_request(), TargetOs::Linux).unwrap();
    std::fs::write(
        dir.path().join(ONE_AGENT_PROMPT_PATH),
        b"# Prompt {#root}\n\nWrite the manual.\n",
    )
    .unwrap();
    assert!(vibe_lifecycle::process::recheck_prompt_source_witness(&first.source).is_err());
    let second =
        admit_one_agent_validate_fixture(dir.path(), &agent_request(), TargetOs::Linux).unwrap();
    assert_ne!(first.canonical_request, second.canonical_request);
    assert_ne!(first.canonical_plan, second.canonical_plan);
    let scope = scope();
    let namespace = issue(dir.path(), &scope);
    let lease = Arc::new(LifecycleLease::acquire(dir.path()).unwrap());
    let run_id = vibe_lifecycle::process::propose_run_id(dir.path()).unwrap();
    SubmissionReservations::reserve(
        &lease,
        &namespace,
        &scope,
        "agent",
        &first.canonical_request,
        &first.canonical_plan,
        &run_id,
    )
    .unwrap();
    let before = std::fs::read(dir.path().join(".vibe/machine-submissions.json")).unwrap();
    let conflict = SubmissionReservations::inspect_existing_request(
        &lease,
        &namespace,
        &scope,
        "agent",
        &second.canonical_request,
    )
    .unwrap_err();
    assert!(matches!(
        conflict,
        vibe_lifecycle::SubmissionError::KeyConflict
    ));
    assert_eq!(
        std::fs::read(dir.path().join(".vibe/machine-submissions.json")).unwrap(),
        before
    );
}

#[test]
fn one_agent_preflight_refuses_foreign_rows_prompt_and_posture() {
    let dir = agent_project();
    let mut wrong = agent_request();
    wrong.agent_mode = RunAgentMode::Cli;
    assert!(admit_one_agent_validate_fixture(dir.path(), &wrong, TargetOs::Linux).is_err());
    let mut wrong = agent_request();
    wrong.policy.offline = true;
    assert!(admit_one_agent_validate_fixture(dir.path(), &wrong, TargetOs::Linux).is_err());
    std::fs::write(
        dir.path().join(ONE_AGENT_PROMPT_PATH),
        b"# Prompt {#root}\n\n#embed spec://x/y/common/Z#x\n",
    )
    .unwrap();
    assert!(
        admit_one_agent_validate_fixture(dir.path(), &agent_request(), TargetOs::Linux).is_err()
    );
    std::fs::write(
        dir.path().join(ONE_AGENT_PROMPT_PATH),
        b"# Prompt {#root}\n\nWrite.\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("vibe.toml"), format!("{ONE_AGENT_VALIDATE_MANIFEST}[[extension]]\nid='extra'\npoint='phase:validate'\nhandler={{kind='agent',prompt='spec://org.demo/demo/common/PROMPT-001#root'}}\n")).unwrap();
    assert!(
        admit_one_agent_validate_fixture(dir.path(), &agent_request(), TargetOs::Linux).is_err()
    );
}

#[test]
fn one_agent_fresh_preflight_refuses_prior_header() {
    let dir = agent_project();
    let lease = Arc::new(LifecycleLease::acquire(dir.path()).unwrap());
    let store = LifecycleStateStore::begin(
        lease.clone(),
        "validate".into(),
        vec!["validate".into()],
        "2026-09-30T00:00:00Z".into(),
        "a".repeat(32),
        ".".into(),
        false,
    )
    .unwrap();
    drop(store);
    drop(lease);
    assert!(
        admit_one_agent_validate_fixture(dir.path(), &agent_request(), TargetOs::Linux).is_err()
    );
}

#[cfg(unix)]
#[test]
fn one_agent_preflight_refuses_symlinked_prompt_before_reservation() {
    use std::os::unix::fs::symlink;
    let dir = agent_project();
    let path = dir.path().join(ONE_AGENT_PROMPT_PATH);
    let actual = path.with_file_name("actual.md");
    std::fs::rename(&path, &actual).unwrap();
    symlink("actual.md", &path).unwrap();
    assert!(
        admit_one_agent_validate_fixture(dir.path(), &agent_request(), TargetOs::Linux).is_err()
    );
    assert!(!dir.path().join(".vibe/machine-submissions.json").exists());
    assert!(!dir.path().join(".vibe/agentic").exists());
}

fn script_call(
    root: &Path,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    process: &CountedScriptProcess,
    after_stage: Option<&dyn Fn()>,
    after_effect: Option<&dyn Fn() -> Result<()>>,
) -> Result<SubmissionOutcome<RetainedLifecycleResult>> {
    submit_one_script_validate_fixture(
        root,
        namespace,
        scope,
        key,
        request(),
        TargetOs::current().expect("supported fixture platform"),
        "2026-09-30T00:00:05Z".parse().unwrap(),
        process,
        &ScriptProbe,
        after_stage,
        after_effect,
    )
}

#[test]
fn one_script_validate_runs_once_after_stage_and_replays_retained_report() {
    let dir = script_project();
    let scope = scope();
    let namespace = issue(dir.path(), &scope);
    let process = CountedScriptProcess::new(dir.path());
    let first = script_call(
        dir.path(),
        &namespace,
        &scope,
        "script",
        &process,
        None,
        None,
    )
    .unwrap();
    let SubmissionOutcome::Invoked { run_id, outcome } = first else {
        panic!("first script must invoke")
    };
    assert_eq!(process.count(), 1);
    assert_eq!(outcome.marker.outcome, MarkerOutcome::Completed);
    let report: vibe_wire::generated::lifecycle_report::LifecycleReport =
        serde_json::from_slice(outcome.report_bytes.as_ref().unwrap()).unwrap();
    assert!(report.ok);
    assert_eq!(report.contributions.len(), 1);
    assert!(
        dir.path()
            .join(".vibe/lifecycle-outcomes")
            .read_dir()
            .unwrap()
            .any(|entry| entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains("script-source"))
    );
    std::fs::write(
        dir.path().join(".vibe/lifecycle.toml"),
        b"later unrelated header",
    )
    .unwrap();
    let second = script_call(
        dir.path(),
        &namespace,
        &scope,
        "script",
        &process,
        None,
        None,
    )
    .unwrap();
    assert_eq!(second, SubmissionOutcome::Replayed { run_id, outcome });
    assert_eq!(process.count(), 1);
}

#[test]
fn one_script_changed_after_stage_refuses_before_process() {
    let dir = script_project();
    let scope = scope();
    let namespace = issue(dir.path(), &scope);
    let process = CountedScriptProcess::new(dir.path());
    let path = dir
        .path()
        .join(if cfg!(windows) { "run.ps1" } else { "run.sh" });
    let change = || std::fs::write(&path, b"changed\n").unwrap();
    let first = script_call(
        dir.path(),
        &namespace,
        &scope,
        "script",
        &process,
        Some(&change),
        None,
    )
    .unwrap();
    let SubmissionOutcome::Invoked { outcome, .. } = first else {
        panic!("source drift must retain an outcome")
    };
    assert_eq!(process.count(), 0);
    assert_eq!(outcome.marker.outcome, MarkerOutcome::Failed);
    assert_eq!(outcome.marker.effect_state, EffectState::Uncertain);
    let report: vibe_wire::generated::lifecycle_report::LifecycleReport =
        serde_json::from_slice(outcome.report_bytes.as_ref().unwrap()).unwrap();
    assert!(!report.ok);
    assert_eq!(report.steps.last().unwrap().status, "fail");
    assert_eq!(report.contributions.len(), 1);
    assert!(dir.path().join(".vibe/lifecycle-outcomes").exists());
}

#[test]
fn one_script_foreign_header_after_stage_refuses_process() {
    let dir = script_project();
    let scope = scope();
    let namespace = issue(dir.path(), &scope);
    let process = CountedScriptProcess::new(dir.path());
    let header = dir.path().join(".vibe/lifecycle.toml");
    let replace = || std::fs::write(&header, b"foreign header").unwrap();
    let result = script_call(
        dir.path(),
        &namespace,
        &scope,
        "script",
        &process,
        Some(&replace),
        None,
    );
    assert!(
        !matches!(result, Ok(SubmissionOutcome::Invoked { .. })),
        "{result:?}"
    );
    assert_eq!(process.count(), 0);
    assert!(dir.path().join(".vibe/lifecycle-outcomes").exists());
}

#[test]
fn one_script_missing_marker_after_counted_effect_never_retries() {
    let dir = script_project();
    let scope = scope();
    let namespace = issue(dir.path(), &scope);
    let process = CountedScriptProcess::new(dir.path());
    let fail = || anyhow::bail!("injected lost result before marker");
    let first = script_call(
        dir.path(),
        &namespace,
        &scope,
        "script",
        &process,
        None,
        Some(&fail),
    );
    assert!(
        matches!(first, Ok(SubmissionOutcome::RecoveryRequired { .. })),
        "{first:?}"
    );
    assert_eq!(process.count(), 1);
    let second = script_call(
        dir.path(),
        &namespace,
        &scope,
        "script",
        &process,
        None,
        None,
    )
    .unwrap();
    assert!(matches!(second, SubmissionOutcome::RecoveryRequired { .. }));
    assert_eq!(process.count(), 1);
}

#[test]
fn one_script_preflight_binds_selected_handler_and_exact_source_without_effect() {
    let dir = script_project();
    let admitted =
        admit_one_script_validate_fixture(dir.path(), &request(), TargetOs::Linux).unwrap();
    assert_eq!(admitted.source.script_relative, "run.sh");
    assert_eq!(admitted.source.source.bytes, b"echo one\n");
    assert!(admitted.execution_key.contains("once"));
    assert!(!admitted.provider.is_empty());
    let request_value: serde_json::Value =
        serde_json::from_slice(&admitted.canonical_request).unwrap();
    let plan_value: serde_json::Value = serde_json::from_slice(&admitted.canonical_plan).unwrap();
    assert_eq!(plan_value["format"], "vibevm.lifecycle.pre-effect-plan/4");
    assert_eq!(request_value["script_source"], plan_value["script_source"]);
    assert!(!dir.path().join(".vibe/lifecycle.toml").exists());
    assert!(!dir.path().join(".vibe/machine-submissions.json").exists());
    assert!(!dir.path().join(".vibe/lifecycle-outcomes").exists());
}

#[test]
fn one_script_preflight_refuses_changed_source_or_second_provider() {
    let dir = script_project();
    let first = admit_one_script_validate_fixture(dir.path(), &request(), TargetOs::Linux).unwrap();
    std::fs::write(dir.path().join("run.sh"), b"echo two\n").unwrap();
    assert!(vibe_lifecycle::process::recheck_script_source_witness(&first.source).is_err());
    let second =
        admit_one_script_validate_fixture(dir.path(), &request(), TargetOs::Linux).unwrap();
    assert_ne!(first.canonical_request, second.canonical_request);
    assert_ne!(first.canonical_plan, second.canonical_plan);
    let scope = scope();
    let namespace = issue(dir.path(), &scope);
    let lease = LifecycleLease::acquire(dir.path()).unwrap();
    let run_id = vibe_lifecycle::process::propose_run_id(dir.path()).unwrap();
    SubmissionReservations::reserve(
        &lease,
        &namespace,
        &scope,
        "script",
        &first.canonical_request,
        &first.canonical_plan,
        &run_id,
    )
    .unwrap();
    let before = std::fs::read(dir.path().join(".vibe/machine-submissions.json")).unwrap();
    let conflict = SubmissionReservations::inspect_existing_request(
        &lease,
        &namespace,
        &scope,
        "script",
        &second.canonical_request,
    )
    .unwrap_err();
    assert!(matches!(
        conflict,
        vibe_lifecycle::SubmissionError::KeyConflict
    ));
    assert_eq!(
        std::fs::read(dir.path().join(".vibe/machine-submissions.json")).unwrap(),
        before
    );
    std::fs::write(dir.path().join("vibe.toml"), format!("{ONE_SCRIPT_VALIDATE_MANIFEST}[[extension]]\nid='extra'\npoint='phase:validate'\nhandler={{kind='script',base='other'}}\n")).unwrap();
    assert!(admit_one_script_validate_fixture(dir.path(), &request(), TargetOs::Linux).is_err());
}

#[cfg(unix)]
#[test]
fn one_script_preflight_refuses_symlinked_script_before_reservation() {
    use std::os::unix::fs::symlink;

    let dir = script_project();
    std::fs::rename(dir.path().join("run.sh"), dir.path().join("actual.sh")).unwrap();
    symlink("actual.sh", dir.path().join("run.sh")).unwrap();
    let error = admit_one_script_validate_fixture(dir.path(), &request(), TargetOs::Linux)
        .err()
        .expect("symlinked script must refuse")
        .to_string();
    assert!(error.contains("script"), "{error}");
    assert!(!dir.path().join(".vibe/machine-submissions.json").exists());
    assert!(!dir.path().join(".vibe/lifecycle.toml").exists());
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
fn validate_with_phase_handler_refuses_before_reservation_or_process() {
    let dir = project();
    std::fs::write(
        dir.path().join("vibe.toml"),
        "[project]\nname='demo'\nversion='1.1.0'\n[[extension]]\nid='once'\npoint='phase:validate'\nhandler={kind='script',base='run'}\n",
    )
    .unwrap();
    let scope = scope();
    let namespace = issue(dir.path(), &scope);
    let index = dir.path().join(".vibe/machine-submissions.json");
    let before = std::fs::read(&index).unwrap();
    let error = call(dir.path(), &namespace, &scope, "handler-key")
        .unwrap_err()
        .to_string();
    assert!(error.contains("handler-free"), "{error}");
    assert_eq!(std::fs::read(index).unwrap(), before);
    assert!(!dir.path().join(".vibe/lifecycle.toml").exists());
    assert!(!dir.path().join(".vibe/lifecycle-outcomes").exists());
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
        script_source: None,
        prompt_source: None,
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
