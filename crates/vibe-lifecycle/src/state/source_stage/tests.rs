use super::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::LifecycleStateStore;
use crate::handlers::{
    HandlerError, HandlerRuntime, NoBinaryBackend, NoNativeBackend, NoPackageBindingBackend,
};
use crate::process::{ProcessError, ProcessOutput, ProcessRunner, ProcessSpec, StreamMode};
use crate::{
    ExecutionSession, ExtensionWorld, HandlerExecution, HostExtensionSource, HostIdentity,
    HostProvider, RunMetadata, SelectorSubject, collect_extensions,
};
use vibe_core::manifest::{ExtensionDecl, ExtensionHandler, ExtensionsControl};
use vibe_wire::generated::lifecycle::e1::context::{
    Project as EnvelopeProject, RunAgentMode, World as EnvelopeWorld,
};
use vibe_workspace::hooks::InterpreterProbe;

const REQUEST: &[u8] = b"request";
const PLAN: &[u8] = b"plan";

struct Fixture {
    dir: tempfile::TempDir,
    lease: Arc<LifecycleLease>,
    namespace: SubmissionNamespace,
    scope: SubmissionScope,
    provider: PathBuf,
    run_id: String,
}

impl Fixture {
    fn new(ready: bool) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let provider = dir.path().join("provider");
        std::fs::create_dir(&provider).unwrap();
        std::fs::write(provider.join("run.sh"), b"echo one\n").unwrap();
        let lease = Arc::new(LifecycleLease::acquire(dir.path()).unwrap());
        let scope = SubmissionScope {
            context: "context".into(),
            target: "target".into(),
            principal: "principal".into(),
        };
        let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
        let run_id = "a".repeat(32);
        assert!(matches!(
            SubmissionReservations::reserve(
                &lease, &namespace, &scope, "key", REQUEST, PLAN, &run_id
            )
            .unwrap(),
            ReservationDecision::Created { .. }
        ));
        let state = LifecycleStateStore::begin(
            lease.clone(),
            "generate".into(),
            vec!["validate".into(), "install".into(), "generate".into()],
            "2026-09-30T00:00:00Z".into(),
            run_id.clone(),
            ".".into(),
            false,
        )
        .unwrap();
        drop(state);
        if ready {
            SubmissionReservations::mark_journal_ready(&lease, &namespace, &scope, "key", &run_id)
                .unwrap();
        }
        Self {
            dir,
            lease,
            namespace,
            scope,
            provider,
            run_id,
        }
    }

    fn stage(&self) -> Result<ScriptSourceStage, ScriptSourceStageError> {
        publish_script_source_stage(
            &self.lease,
            &self.namespace,
            &self.scope,
            "key",
            &self.run_id,
            REQUEST,
            PLAN,
            "generate",
            "org.example/script",
            "org.example/provider@1.1.0",
            &self.provider,
            "run.sh",
        )
    }

    fn stage_path(&self) -> PathBuf {
        self.dir
            .path()
            .join(".vibe/lifecycle-outcomes")
            .join(stage_name(&self.run_id, "generate", "org.example/script"))
    }
}

#[test]
fn script_stage_publishes_and_reads_back_exact_bound_source() {
    let fixture = Fixture::new(true);
    let stage = fixture.stage().unwrap();
    stage.recheck_before_spawn().unwrap();
    let bytes = std::fs::read(fixture.stage_path()).unwrap();
    let record: StageRecord = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(record.run_id, fixture.run_id);
    assert_eq!(record.phase, "generate");
    assert_eq!(record.script_relative, "run.sh");
    assert_eq!(record.script_size, 9);
    assert_eq!(record.stage, "invoke-may-effect");
    assert_eq!(
        record.pre_effect_plan_sha256,
        format!("sha256:{:x}", Sha256::digest(PLAN))
    );
    assert_eq!(record.reservation_ref.len(), "sha256:".len() + 64);
}

#[test]
fn script_stage_recheck_refuses_changed_bytes() {
    let fixture = Fixture::new(true);
    let stage = fixture.stage().unwrap();
    std::fs::write(fixture.provider.join("run.sh"), b"echo two\n").unwrap();
    assert!(matches!(
        stage.recheck_before_spawn(),
        Err(ScriptSourceStageError::RecoveryRequired(_))
    ));
    assert!(!fixture.dir.path().join(".vibe/lifecycle").exists());
}

#[test]
fn script_stage_same_run_and_key_requires_recovery_without_overwrite() {
    let fixture = Fixture::new(true);
    fixture.stage().unwrap();
    let original = std::fs::read(fixture.stage_path()).unwrap();
    assert!(matches!(
        fixture.stage(),
        Err(ScriptSourceStageError::RecoveryRequired(_))
    ));
    assert_eq!(std::fs::read(fixture.stage_path()).unwrap(), original);
}

#[test]
fn script_stage_foreign_scope_and_not_ready_leave_no_stage() {
    let pending = Fixture::new(false);
    assert!(matches!(
        pending.stage(),
        Err(ScriptSourceStageError::RecoveryRequired(_))
    ));
    assert!(!pending.stage_path().exists());

    let ready = Fixture::new(true);
    let mut foreign = ready.scope.clone();
    foreign.principal = "someone-else".into();
    assert!(matches!(
        publish_script_source_stage(
            &ready.lease,
            &ready.namespace,
            &foreign,
            "key",
            &ready.run_id,
            REQUEST,
            PLAN,
            "generate",
            "org.example/script",
            "org.example/provider@1.1.0",
            &ready.provider,
            "run.sh",
        ),
        Err(ScriptSourceStageError::RecoveryRequired(_))
    ));
    assert!(!ready.stage_path().exists());
}

#[test]
fn script_stage_refuses_mismatched_canonical_plan() {
    let fixture = Fixture::new(true);
    assert!(matches!(
        publish_script_source_stage(
            &fixture.lease,
            &fixture.namespace,
            &fixture.scope,
            "key",
            &fixture.run_id,
            REQUEST,
            b"different-plan",
            "generate",
            "org.example/script",
            "org.example/provider@1.1.0",
            &fixture.provider,
            "run.sh",
        ),
        Err(ScriptSourceStageError::RecoveryRequired(_))
    ));
    assert!(!fixture.stage_path().exists());
}

#[test]
fn script_stage_refuses_phase_outside_reserved_lifecycle_chain() {
    let fixture = Fixture::new(true);
    assert!(matches!(
        publish_script_source_stage(
            &fixture.lease,
            &fixture.namespace,
            &fixture.scope,
            "key",
            &fixture.run_id,
            REQUEST,
            PLAN,
            "deploy",
            "org.example/script",
            "org.example/provider@1.1.0",
            &fixture.provider,
            "run.sh",
        ),
        Err(ScriptSourceStageError::RecoveryRequired(_))
    ));
    assert!(!fixture.stage_path().exists());
}

#[test]
fn script_stage_refuses_oversize_source_without_publication() {
    let fixture = Fixture::new(true);
    std::fs::write(fixture.provider.join("run.sh"), vec![b'x'; SCRIPT_CAP + 1]).unwrap();
    assert!(matches!(
        fixture.stage(),
        Err(ScriptSourceStageError::SourceRefused(_))
    ));
    assert!(!fixture.stage_path().exists());
}

#[cfg(unix)]
#[test]
fn script_stage_refuses_symlink_source_without_publication() {
    let fixture = Fixture::new(true);
    std::fs::rename(
        fixture.provider.join("run.sh"),
        fixture.provider.join("actual.sh"),
    )
    .unwrap();
    std::os::unix::fs::symlink("actual.sh", fixture.provider.join("run.sh")).unwrap();
    assert!(matches!(
        fixture.stage(),
        Err(ScriptSourceStageError::SourceRefused(_))
    ));
    assert!(!fixture.stage_path().exists());
}

struct CountedProcess(AtomicUsize);
impl ProcessRunner for CountedProcess {
    fn run(&self, _: &ProcessSpec) -> Result<ProcessOutput, ProcessError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(ProcessOutput {
            code: Some(0),
            ..ProcessOutput::default()
        })
    }
}
struct BashProbe;
impl InterpreterProbe for BashProbe {
    fn has(&self, program: &str) -> bool {
        program == "bash"
    }
}
struct NoProbe;
impl InterpreterProbe for NoProbe {
    fn has(&self, _: &str) -> bool {
        false
    }
}

fn execution_for_handler(
    fixture: &Fixture,
    handler: ExtensionHandler,
) -> (
    HandlerExecution,
    vibe_wire::generated::lifecycle::e1::context::Context,
) {
    let registry = collect_extensions(ExtensionWorld {
        installed: Vec::new(),
        host: HostExtensionSource {
            provider: HostProvider {
                identity: HostIdentity::ungrouped_project("provider"),
                root: fixture.provider.clone(),
                version: "1.1.0".into(),
                kind: None,
                content_hash: None,
            },
            declarations: vec![ExtensionDecl {
                id: "script".into(),
                point: "phase:generate".parse().unwrap(),
                handler,
                config: None,
                auto: None,
                inputs: None,
                applies_to: None,
                compiler_internals: None,
                pass: None,
                when: None,
                machine_operation_schema: None,
            }],
            controls: ExtensionsControl::default(),
            mechanisms: Vec::new(),
        },
        effective_stack: None,
    })
    .unwrap();
    let row = registry.plan(
        "phase:generate".parse().unwrap(),
        SelectorSubject::unscoped(),
    )[0]
    .clone();
    let execution = HandlerExecution::from_row(&row);
    let root = fixture.dir.path().to_string_lossy().replace('\\', "/");
    let session = ExecutionSession::new(
        EnvelopeProject {
            kind: "project".into(),
            manifest: format!("{root}/vibe.toml"),
            name: "demo".into(),
            root: root.clone(),
            spec_roots: Vec::new(),
            version: "1.1.0".into(),
        },
        EnvelopeWorld {
            deps_root: format!("{root}/vibedeps"),
            lockfile: format!("{root}/vibe.lock"),
            packages: Vec::new(),
        },
        RunMetadata {
            requested: "generate".into(),
            chain: vec!["validate".into(), "install".into(), "generate".into()],
            offline: false,
            assume_yes: true,
            agent_mode: RunAgentMode::Cli,
            force: false,
            trace_compile: false,
            run_id: fixture.run_id.clone(),
            started: "2026-09-30T00:00:00Z".into(),
            selected: ".".into(),
        },
    );
    let context = session
        .envelope_for_execution("generate", &execution)
        .unwrap();
    (execution, context)
}

fn script_execution(
    fixture: &Fixture,
) -> (
    HandlerExecution,
    vibe_wire::generated::lifecycle::e1::context::Context,
) {
    execution_for_handler(fixture, ExtensionHandler::Script { base: "run".into() })
}

fn stage_for_execution(fixture: &Fixture, execution: &HandlerExecution) -> ScriptSourceStage {
    publish_script_source_stage(
        &fixture.lease,
        &fixture.namespace,
        &fixture.scope,
        "key",
        &fixture.run_id,
        REQUEST,
        PLAN,
        "generate",
        &execution.key(),
        &execution.row().provider().to_string(),
        &fixture.provider,
        "run.sh",
    )
    .unwrap()
}

#[test]
fn protected_script_drift_refuses_before_scratch_and_process() {
    let fixture = Fixture::new(true);
    let (execution, context) = script_execution(&fixture);
    let stage = stage_for_execution(&fixture, &execution);
    std::fs::write(fixture.provider.join("run.sh"), b"echo two\n").unwrap();
    let process = CountedProcess(AtomicUsize::new(0));
    let runtime = HandlerRuntime {
        process: &process,
        binary: &NoBinaryBackend,
        native: &NoNativeBackend,
        package_binding: &NoPackageBindingBackend,
        agent: &crate::NoAgentBackend,
        probe: &BashProbe,
        streams: StreamMode::Capture,
        protected_script: Some(&stage),
    };
    let error = runtime
        .dispatch_execution(&execution, &context, None)
        .unwrap_err();
    assert!(matches!(error, HandlerError::SourceStage { .. }), "{error}");
    assert_eq!(process.0.load(Ordering::SeqCst), 0);
    assert!(!fixture.dir.path().join(".vibe/lifecycle").exists());
}

#[test]
fn protected_script_stage_cannot_dispatch_twice() {
    let fixture = Fixture::new(true);
    let (execution, context) = script_execution(&fixture);
    let stage = stage_for_execution(&fixture, &execution);
    let process = CountedProcess(AtomicUsize::new(0));
    let runtime = HandlerRuntime {
        process: &process,
        binary: &NoBinaryBackend,
        native: &NoNativeBackend,
        package_binding: &NoPackageBindingBackend,
        agent: &crate::NoAgentBackend,
        probe: &BashProbe,
        streams: StreamMode::Capture,
        protected_script: Some(&stage),
    };
    runtime
        .dispatch_execution(&execution, &context, None)
        .unwrap();
    assert_eq!(process.0.load(Ordering::SeqCst), 1);
    let error = runtime
        .dispatch_execution(&execution, &context, None)
        .unwrap_err();
    assert!(matches!(error, HandlerError::SourceStage { .. }), "{error}");
    assert_eq!(process.0.load(Ordering::SeqCst), 1);
}

#[test]
fn protected_script_guard_rechecks_after_initial_admission() {
    let fixture = Fixture::new(true);
    let (execution, _) = script_execution(&fixture);
    let stage = stage_for_execution(&fixture, &execution);
    let script = fixture.provider.join("run.sh").canonicalize().unwrap();
    let gate = stage
        .begin(
            &fixture.run_id,
            "generate",
            &execution.key(),
            &execution.row().provider().to_string(),
            &fixture.provider,
            &script,
        )
        .unwrap();
    std::fs::write(&script, b"echo changed after admission\n").unwrap();
    assert!(matches!(
        gate.before_spawn(),
        Err(ScriptSourceStageError::RecoveryRequired(_))
    ));
    assert!(!fixture.dir.path().join(".vibe/lifecycle").exists());
}

#[test]
fn protected_script_stage_refuses_binary_before_resolution_or_scratch() {
    let fixture = Fixture::new(true);
    let (script, _) = script_execution(&fixture);
    let stage = stage_for_execution(&fixture, &script);
    let (binary, context) =
        execution_for_handler(&fixture, ExtensionHandler::Binary { name: "bin".into() });
    let process = CountedProcess(AtomicUsize::new(0));
    let runtime = HandlerRuntime {
        process: &process,
        binary: &NoBinaryBackend,
        native: &NoNativeBackend,
        package_binding: &NoPackageBindingBackend,
        agent: &crate::NoAgentBackend,
        probe: &BashProbe,
        streams: StreamMode::Capture,
        protected_script: Some(&stage),
    };
    let error = runtime
        .dispatch_execution(&binary, &context, None)
        .unwrap_err();
    assert!(matches!(error, HandlerError::SourceStage { .. }), "{error}");
    assert_eq!(process.0.load(Ordering::SeqCst), 0);
    assert!(!fixture.dir.path().join(".vibe/lifecycle").exists());
    let (script, script_context) = script_execution(&fixture);
    let second = runtime
        .dispatch_execution(&script, &script_context, None)
        .unwrap_err();
    assert!(
        matches!(second, HandlerError::SourceStage { .. }),
        "{second}"
    );
    assert_eq!(process.0.load(Ordering::SeqCst), 0);
    assert!(!fixture.dir.path().join(".vibe/lifecycle").exists());
}

#[test]
fn protected_script_missing_interpreter_spends_stage_before_retry() {
    let fixture = Fixture::new(true);
    let (execution, context) = script_execution(&fixture);
    let stage = stage_for_execution(&fixture, &execution);
    let process = CountedProcess(AtomicUsize::new(0));
    let unavailable = HandlerRuntime {
        process: &process,
        binary: &NoBinaryBackend,
        native: &NoNativeBackend,
        package_binding: &NoPackageBindingBackend,
        agent: &crate::NoAgentBackend,
        probe: &NoProbe,
        streams: StreamMode::Capture,
        protected_script: Some(&stage),
    };
    let first = unavailable
        .dispatch_execution(&execution, &context, None)
        .unwrap_err();
    assert!(
        matches!(first, HandlerError::NoInterpreter { .. }),
        "{first}"
    );
    let available = HandlerRuntime {
        probe: &BashProbe,
        ..unavailable
    };
    let second = available
        .dispatch_execution(&execution, &context, None)
        .unwrap_err();
    assert!(
        matches!(second, HandlerError::SourceStage { .. }),
        "{second}"
    );
    assert_eq!(process.0.load(Ordering::SeqCst), 0);
    assert!(!fixture.dir.path().join(".vibe/lifecycle").exists());
}

#[test]
fn protected_script_malformed_context_spends_stage_before_retry() {
    let fixture = Fixture::new(true);
    let (execution, context) = script_execution(&fixture);
    let stage = stage_for_execution(&fixture, &execution);
    let process = CountedProcess(AtomicUsize::new(0));
    let runtime = HandlerRuntime {
        process: &process,
        binary: &NoBinaryBackend,
        native: &NoNativeBackend,
        package_binding: &NoPackageBindingBackend,
        agent: &crate::NoAgentBackend,
        probe: &BashProbe,
        streams: StreamMode::Capture,
        protected_script: Some(&stage),
    };
    let mut malformed = context.clone();
    malformed.io.scratch = "invalid".into();
    assert!(matches!(
        runtime.dispatch_execution(&execution, &malformed, None),
        Err(HandlerError::SourceStage { .. })
    ));
    assert!(matches!(
        runtime.dispatch_execution(&execution, &context, None),
        Err(HandlerError::SourceStage { .. })
    ));
    assert_eq!(process.0.load(Ordering::SeqCst), 0);
    assert!(!fixture.dir.path().join(".vibe/lifecycle").exists());
}
