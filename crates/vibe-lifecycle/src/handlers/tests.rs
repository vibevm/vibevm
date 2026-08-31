use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use vibe_core::manifest::{ExtensionDecl, ExtensionHandler, ExtensionsControl};
use vibe_wire::generated::lifecycle::e1::context::{
    Artifact, Context, Project, RunAgentMode, World,
};
use vibe_wire::generated::lifecycle::e1::reply::{Reply, ReplyArtifact, ReplyStatus};
use vibe_workspace::hooks::InterpreterProbe;

use super::*;
use crate::process::{ProcessError, ProcessOutput, allocate_run_id};
use crate::{
    ExecutionSession, ExtensionWorld, HostExtensionSource, HostIdentity, HostProvider, RunMetadata,
    SelectorSubject, collect_extensions,
};

struct FakeRunner {
    output: ProcessOutput,
    reply: Option<Vec<u8>>,
    seen: Mutex<Vec<ProcessSpec>>,
}
impl ProcessRunner for FakeRunner {
    fn run(&self, spec: &ProcessSpec) -> Result<ProcessOutput, ProcessError> {
        self.seen.lock().unwrap().push(spec.clone());
        if let Some(bytes) = &self.reply {
            let path = spec
                .env
                .get(&std::ffi::OsString::from("VIBE_REPLY"))
                .unwrap();
            fs::write(PathBuf::from(path), bytes).unwrap();
        }
        Ok(self.output.clone())
    }
}
struct BashProbe;
impl InterpreterProbe for BashProbe {
    fn has(&self, program: &str) -> bool {
        program == "bash"
    }
}
struct FakeBinary(PathBuf);
impl BinaryBackend for FakeBinary {
    fn resolve_or_build(
        &self,
        _: &crate::ExtensionRegistryRow,
        _: &str,
    ) -> Result<PathBuf, String> {
        Ok(self.0.clone())
    }
}

fn row(root: &Path, handler: ExtensionHandler) -> crate::ExtensionRegistryRow {
    row_at(root, handler, "phase:build")
}

fn row_at(root: &Path, handler: ExtensionHandler, point: &str) -> crate::ExtensionRegistryRow {
    let registry = collect_extensions(ExtensionWorld {
        installed: Vec::new(),
        host: HostExtensionSource {
            provider: HostProvider {
                identity: HostIdentity::ungrouped_project("demo"),
                root: root.into(),
                version: "0.1.0".into(),
                kind: None,
                content_hash: None,
            },
            declarations: vec![ExtensionDecl {
                id: "handler".into(),
                point: point.parse().unwrap(),
                handler,
                config: None,
                auto: None,
                inputs: None,
                applies_to: None,
                compiler_internals: None,
                pass: None,
                when: None,
            }],
            controls: ExtensionsControl::default(),
            mechanisms: Vec::new(),
        },
        effective_stack: None,
    })
    .unwrap();
    registry.plan(point.parse().unwrap(), SelectorSubject::unscoped())[0].clone()
}

#[test]
fn slot_execution_uses_target_cwd_and_compatibility_package_environment() {
    let provider = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    fs::create_dir_all(provider.path().join("hooks")).unwrap();
    fs::write(provider.path().join("hooks/run.sh"), "exit 0").unwrap();
    let row = row_at(
        provider.path(),
        ExtensionHandler::Script {
            base: "hooks/run".into(),
        },
        "slot:pre-install",
    );
    let (mut session, _) = prepared(provider.path(), &row);
    let execution = crate::HandlerExecution::from_row(&row).with_slot_target(crate::SlotTarget {
        group: "org.target".into(),
        name: "package".into(),
        version: "2.0.0".into(),
        kind: "tool".into(),
        root: target.path().to_string_lossy().replace('\\', "/"),
    });
    let context = session
        .envelope_for_execution("install", &execution)
        .unwrap();
    let runner = FakeRunner {
        output: ProcessOutput {
            code: Some(0),
            ..Default::default()
        },
        reply: None,
        seen: Mutex::new(Vec::new()),
    };
    let runtime = HandlerRuntime {
        process: &runner,
        binary: &NoBinaryBackend,
        native: &NoNativeBackend,
        package_binding: &super::NoPackageBindingBackend,
        agent: &crate::NoAgentBackend,
        probe: &BashProbe,
        streams: StreamMode::Capture,
    };
    session
        .dispatch_execution(&execution, context, &runtime, None)
        .unwrap();
    let seen = runner.seen.lock().unwrap();
    assert_eq!(seen[0].cwd, target.path());
    assert_eq!(
        seen[0].env.get(&OsString::from("VIBE_PACKAGE_GROUP")),
        Some(&OsString::from("org.target"))
    );
    assert_eq!(
        seen[0].env.get(&OsString::from("VIBE_PACKAGE_DIR")),
        Some(&OsString::from(
            target.path().to_string_lossy().replace('\\', "/")
        ))
    );
    assert_eq!(
        seen[0].env.get(&OsString::from("VIBE_EXTENSION_PROVIDER")),
        Some(&OsString::from(row.provider().to_string()))
    );
}

fn prepared(root: &Path, row: &crate::ExtensionRegistryRow) -> (ExecutionSession, Context) {
    fs::write(
        root.join("vibe.toml"),
        "[project]\nname='demo'\nversion='0.1.0'\n",
    )
    .unwrap();
    let run_id = allocate_run_id(root).unwrap();
    let text = root.to_string_lossy().replace('\\', "/");
    let session = ExecutionSession::new(
        Project {
            kind: "project".into(),
            manifest: format!("{text}/vibe.toml"),
            name: "demo".into(),
            root: text.clone(),
            spec_roots: Vec::new(),
            version: "0.1.0".into(),
        },
        World {
            deps_root: format!("{text}/vibedeps"),
            lockfile: format!("{text}/vibe.lock"),
            packages: Vec::new(),
        },
        RunMetadata {
            requested: "build".into(),
            chain: vec![
                "validate".into(),
                "install".into(),
                "generate".into(),
                "build".into(),
            ],
            offline: false,
            assume_yes: true,
            agent_mode: RunAgentMode::Cli,
            force: false,
            trace_compile: false,
            run_id,
            started: "2026-08-25T00:00:00Z".into(),
            selected: ".".into(),
        },
    );
    let context = session.envelope_for("build", row).unwrap();
    (session, context)
}
fn ok_reply() -> Vec<u8> {
    br#"{"artifacts":[],"envelope":1,"status":"ok","tasks":[]}"#.to_vec()
}

#[test]
fn script_exit_zero_without_reply_defaults_ok_and_carries_exact_wire_env() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("hooks")).unwrap();
    fs::write(dir.path().join("hooks/run.sh"), "exit 0").unwrap();
    let row = row(
        dir.path(),
        ExtensionHandler::Script {
            base: "hooks/run".into(),
        },
    );
    let (mut session, context) = prepared(dir.path(), &row);
    let runner = FakeRunner {
        output: ProcessOutput {
            code: Some(0),
            ..Default::default()
        },
        reply: None,
        seen: Mutex::new(Vec::new()),
    };
    let runtime = HandlerRuntime {
        process: &runner,
        binary: &NoBinaryBackend,
        native: &NoNativeBackend,
        package_binding: &super::NoPackageBindingBackend,
        agent: &crate::NoAgentBackend,
        probe: &BashProbe,
        streams: StreamMode::Capture,
    };
    let outcome = session
        .dispatch_prepared_with(&row, context, &runtime)
        .unwrap();
    assert_eq!(outcome.reply.status, ReplyStatus::Ok);
    let seen = runner.seen.lock().unwrap();
    assert_eq!(seen[0].program, PathBuf::from("bash"));
    for key in [
        "VIBE_CONTEXT",
        "VIBE_REPLY",
        "VIBE_PROJECT_ROOT",
        "VIBE_EXTENSION_PROVIDER",
    ] {
        assert!(seen[0].env.contains_key(&std::ffi::OsString::from(key)));
    }
    assert!(
        !seen[0]
            .env
            .keys()
            .any(|key| key.to_string_lossy().contains("TOKEN"))
    );
}

#[test]
fn script_nonzero_wins_over_valid_reply() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("hooks")).unwrap();
    fs::write(dir.path().join("hooks/run.sh"), "exit 9").unwrap();
    let row = row(
        dir.path(),
        ExtensionHandler::Script {
            base: "hooks/run".into(),
        },
    );
    let (mut session, context) = prepared(dir.path(), &row);
    let runner = FakeRunner {
        output: ProcessOutput {
            code: Some(9),
            ..Default::default()
        },
        reply: Some(ok_reply()),
        seen: Mutex::new(Vec::new()),
    };
    let runtime = HandlerRuntime {
        process: &runner,
        binary: &NoBinaryBackend,
        native: &NoNativeBackend,
        package_binding: &super::NoPackageBindingBackend,
        agent: &crate::NoAgentBackend,
        probe: &BashProbe,
        streams: StreamMode::Capture,
    };
    let error = session
        .dispatch_prepared_with(&row, context, &runtime)
        .unwrap_err()
        .to_string();
    assert!(error.contains("exited nonzero"), "{error}");
}

#[test]
fn successful_script_reply_is_canonicalized_and_pending_file_is_consumed() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("hooks")).unwrap();
    fs::write(dir.path().join("hooks/run.sh"), "exit 0").unwrap();
    let row = row(
        dir.path(),
        ExtensionHandler::Script {
            base: "hooks/run".into(),
        },
    );
    let (mut session, context) = prepared(dir.path(), &row);
    let runner = FakeRunner {
        output: ProcessOutput {
            code: Some(0),
            ..Default::default()
        },
        reply: Some(ok_reply()),
        seen: Mutex::new(Vec::new()),
    };
    let runtime = HandlerRuntime {
        process: &runner,
        binary: &NoBinaryBackend,
        native: &NoNativeBackend,
        package_binding: &super::NoPackageBindingBackend,
        agent: &crate::NoAgentBackend,
        probe: &BashProbe,
        streams: StreamMode::Capture,
    };
    session
        .dispatch_prepared_with(&row, context, &runtime)
        .unwrap();
    let pending = PathBuf::from(
        runner.seen.lock().unwrap()[0]
            .env
            .get(&OsString::from("VIBE_REPLY"))
            .unwrap(),
    );
    assert!(!pending.exists());
    assert!(pending.parent().unwrap().read_dir().unwrap().any(|entry| {
        entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("reply.json-")
    }));
}

#[test]
fn binary_stdin_is_exact_context_and_contaminated_stdout_fails() {
    let dir = tempfile::tempdir().unwrap();
    let artifact = dir
        .path()
        .join(if cfg!(windows) { "tool.exe" } else { "tool" });
    fs::write(&artifact, "stub").unwrap();
    let row = row(
        dir.path(),
        ExtensionHandler::Binary {
            name: "tool".into(),
        },
    );
    let (mut session, context) = prepared(dir.path(), &row);
    let mut stdout = ok_reply();
    stdout.extend_from_slice(b" log");
    let runner = FakeRunner {
        output: ProcessOutput {
            code: Some(0),
            stdout,
            ..Default::default()
        },
        reply: None,
        seen: Mutex::new(Vec::new()),
    };
    let binary = FakeBinary(artifact);
    let runtime = HandlerRuntime {
        process: &runner,
        binary: &binary,
        native: &NoNativeBackend,
        package_binding: &super::NoPackageBindingBackend,
        agent: &crate::NoAgentBackend,
        probe: &BashProbe,
        streams: StreamMode::Capture,
    };
    let error = session
        .dispatch_prepared_with(&row, context.clone(), &runtime)
        .unwrap_err()
        .to_string();
    assert!(error.contains("invalid reply"), "{error}");
    assert_eq!(
        runner.seen.lock().unwrap()[0].stdin.as_deref(),
        Some(serde_json::to_vec(&context).unwrap().as_slice())
    );
}

#[test]
fn binary_nonzero_wins_over_a_valid_reply_and_empty_stdout_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let artifact = dir
        .path()
        .join(if cfg!(windows) { "tool.exe" } else { "tool" });
    fs::write(&artifact, "stub").unwrap();
    let row = row(
        dir.path(),
        ExtensionHandler::Binary {
            name: "tool".into(),
        },
    );
    let (mut session, context) = prepared(dir.path(), &row);
    let binary = FakeBinary(artifact);
    for (code, stdout, expected) in [
        (Some(7), ok_reply(), "exited nonzero"),
        (Some(0), Vec::new(), "empty or >1 MiB reply"),
    ] {
        let runner = FakeRunner {
            output: ProcessOutput {
                code,
                stdout,
                ..Default::default()
            },
            reply: None,
            seen: Mutex::new(Vec::new()),
        };
        let runtime = HandlerRuntime {
            process: &runner,
            binary: &binary,
            native: &NoNativeBackend,
            package_binding: &super::NoPackageBindingBackend,
            agent: &crate::NoAgentBackend,
            probe: &BashProbe,
            streams: StreamMode::Capture,
        };
        let error = session
            .dispatch_prepared_with(&row, context.clone(), &runtime)
            .unwrap_err()
            .to_string();
        assert!(error.contains(expected), "{error}");
    }
}

#[test]
fn binary_nonzero_never_exposes_protocol_stdout_as_a_report_stream() {
    let dir = tempfile::tempdir().unwrap();
    let artifact = dir
        .path()
        .join(if cfg!(windows) { "tool.exe" } else { "tool" });
    fs::write(&artifact, "stub").unwrap();
    let row = row(
        dir.path(),
        ExtensionHandler::Binary {
            name: "tool".into(),
        },
    );
    let (mut session, context) = prepared(dir.path(), &row);
    let runner = FakeRunner {
        output: ProcessOutput {
            code: Some(23),
            stdout: b"protocol-contamination".to_vec(),
            stderr: b"diagnostic".to_vec(),
            stdout_truncated: true,
            ..Default::default()
        },
        reply: None,
        seen: Mutex::new(Vec::new()),
    };
    let binary = FakeBinary(artifact);
    let runtime = HandlerRuntime {
        process: &runner,
        binary: &binary,
        native: &NoNativeBackend,
        package_binding: &super::NoPackageBindingBackend,
        agent: &crate::NoAgentBackend,
        probe: &BashProbe,
        streams: StreamMode::Capture,
    };
    let error = session
        .dispatch_prepared_with(&row, context, &runtime)
        .unwrap_err();
    let crate::DispatchError::Handler { error, .. } = error else {
        panic!("unexpected binary failure variant");
    };
    let HandlerError::NonZero { streams, .. } = error.as_ref() else {
        panic!("unexpected typed handler failure");
    };
    assert!(streams.stdout.is_empty());
    assert!(streams.stdout_truncated);
    assert_eq!(streams.stderr, "diagnostic");
}

#[test]
fn process_reply_refuses_tasks_unknown_fields_and_artifact_id_collisions() {
    let dir = tempfile::tempdir().unwrap();
    let artifact = dir
        .path()
        .join(if cfg!(windows) { "tool.exe" } else { "tool" });
    fs::write(&artifact, "stub").unwrap();
    let row = row(
        dir.path(),
        ExtensionHandler::Binary {
            name: "tool".into(),
        },
    );
    let (mut session, mut context) = prepared(dir.path(), &row);
    context.artifacts.push(Artifact {
        id: "same".into(),
        kind: "file".into(),
        path: context.project.manifest.clone(),
        phase: "install".into(),
    });
    let binary = FakeBinary(artifact);
    let cases = [
        serde_json::json!({
            "artifacts": [], "envelope": 1, "status": "ok", "tasks": ["forbidden"]
        }),
        serde_json::json!({
            "artifacts": [], "envelope": 1, "status": "ok", "tasks": [], "unknown": true
        }),
        serde_json::json!({
            "artifacts": [{"id":"same","kind":"file","path":context.project.manifest}],
            "envelope": 1, "status": "ok", "tasks": []
        }),
        serde_json::json!({
            "artifacts": [
                {"id":"one","kind":"file","path":context.project.manifest},
                {"id":"two","kind":"file","path":context.project.manifest}
            ],
            "envelope": 1, "status": "ok", "tasks": []
        }),
    ];
    for reply in cases {
        let runner = FakeRunner {
            output: ProcessOutput {
                code: Some(0),
                stdout: serde_json::to_vec(&reply).unwrap(),
                ..Default::default()
            },
            reply: None,
            seen: Mutex::new(Vec::new()),
        };
        let runtime = HandlerRuntime {
            process: &runner,
            binary: &binary,
            native: &NoNativeBackend,
            package_binding: &super::NoPackageBindingBackend,
            agent: &crate::NoAgentBackend,
            probe: &BashProbe,
            streams: StreamMode::Capture,
        };
        let error = session
            .dispatch_prepared_with(&row, context.clone(), &runtime)
            .unwrap_err()
            .to_string();
        assert!(error.contains("invalid reply"), "{error}");
    }
}

#[test]
fn artifact_path_cannot_exit_and_reenter_with_parent_components() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("sub")).unwrap();
    fs::write(dir.path().join("artifact.txt"), "x").unwrap();
    let row = row(dir.path(), ExtensionHandler::Builtin { name: "log".into() });
    let (_, context) = prepared(dir.path(), &row);
    let root_name = dir.path().file_name().unwrap().to_string_lossy();
    let authored = dir
        .path()
        .join("sub")
        .join("..")
        .join("..")
        .join(root_name.as_ref())
        .join("artifact.txt")
        .to_string_lossy()
        .replace('\\', "/");
    let reply = Reply {
        artifacts: vec![ReplyArtifact {
            id: "escape-reenter".into(),
            kind: "file".into(),
            path: authored,
        }],
        envelope: 1,
        message: None,
        status: ReplyStatus::Ok,
        tasks: Vec::new(),
    };
    let error = validate_reply(&reply, &context, "qualified@slot(target)").unwrap_err();
    // The shared generic row law now catches this lexically, before any
    // filesystem walk — the same refusal, one step earlier and one owner.
    assert!(
        error.to_string().contains("non-normal path component"),
        "{error}"
    );
}

#[path = "native_tests.rs"]
mod native;
