//! Owned local compiler/observer fixture through the actual lifecycle runner.
use crate::handlers::{HandlerRuntime, NativeBackend, NativeBackendRequest, NoBinaryBackend};
use crate::process::{StreamMode, SystemProcessRunner};
use crate::*;
use std::{
    fs,
    path::Path,
    sync::{Arc, Mutex},
};
use vibe_wire::generated::lifecycle::e1::context::{Project, RunAgentMode, World};
use vibe_wire::generated::native::e1::reply::{Reply, ReplyArtifact, ReplyStatus};
use vibe_workspace::hooks::SystemProbe;

struct LocalOperations {
    calls: Mutex<Vec<String>>,
}
impl NativeBackend for LocalOperations {
    fn invoke(&self, request: NativeBackendRequest<'_>) -> Result<Reply, String> {
        let root = Path::new(&request.context.project.root);
        self.calls.lock().unwrap().push(request.extension_id.into());
        let (id, path) = match request.extension_id {
            "produce" => {
                fs::create_dir_all(root.join("src")).map_err(|e| e.to_string())?;
                fs::write(root.join("src/product.rs"),"pub fn answer() -> u32 { 42 }\n#[test] fn actual_answer() { assert_eq!(answer(),42); }\n").map_err(|e|e.to_string())?;
                ("source", "src/product.rs".into())
            }
            "compile" => {
                fs::create_dir_all(root.join("target")).map_err(|e| e.to_string())?;
                let relative = format!("target/actual-test{}", std::env::consts::EXE_SUFFIX);
                let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
                let result = std::process::Command::new(rustc)
                    .args([
                        "--edition=2024",
                        "--test",
                        "src/product.rs",
                        "-o",
                        &relative,
                    ])
                    .current_dir(root)
                    .output()
                    .map_err(|e| e.to_string())?;
                if !result.status.success() {
                    return Err(String::from_utf8_lossy(&result.stderr).into());
                }
                ("compiled", relative)
            }
            "observe" => {
                let executable = root.join(format!(
                    "target/actual-test{}",
                    std::env::consts::EXE_SUFFIX
                ));
                let result = std::process::Command::new(executable)
                    .current_dir(root)
                    .output()
                    .map_err(|e| e.to_string())?;
                if !result.status.success() {
                    return Err("actual generated test failed".into());
                }
                fs::create_dir_all(root.join("evidence")).map_err(|e| e.to_string())?;
                fs::write(root.join("evidence/test.stdout"), result.stdout)
                    .map_err(|e| e.to_string())?;
                ("evidence", "evidence/test.stdout".into())
            }
            other => return Err(format!("unexpected fixture handler {other}")),
        };
        Ok(Reply {
            envelope: 1,
            status: ReplyStatus::Ok,
            message: None,
            artifacts: vec![ReplyArtifact {
                id: id.into(),
                kind: "file".into(),
                path: root.join(path).to_string_lossy().into(),
            }],
        })
    }
}
fn rows(root: &Path) -> Vec<ExtensionRegistryRow> {
    let manifest = vibe_core::manifest::Manifest::read(root.join("vibe.toml")).unwrap();
    let world = ExtensionWorld {
        installed: vec![],
        effective_stack: None,
        host: HostExtensionSource {
            provider: HostProvider {
                identity: HostIdentity::coordinate(DependencyProviderId::new(
                    "org.demo".parse().unwrap(),
                    "consumer".parse().unwrap(),
                )),
                root: root.into(),
                version: "1.0.0".into(),
                kind: None,
                content_hash: None,
            },
            declarations: manifest.extensions,
            controls: manifest.extension_controls,
            mechanisms: vec![],
        },
    };
    collect_extensions(world).unwrap().rows().to_vec()
}
fn fixture() -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("intent.txt"), "one stable intent\n").unwrap();
    let mut manifest =
        String::from("[project]\ngroup='org.demo'\nname='consumer'\nversion='1.0.0'\n");
    for (id, phase, effect, input) in [
        ("produce", "create", "product-source", "intent.txt"),
        ("compile", "build", "build-artifact", "src/product.rs"),
        ("observe", "verify", "evidence-observer", "src/product.rs"),
    ] {
        manifest.push_str(&format!("\n[[extension]]\nid='{id}'\npoint='phase:{phase}'\neffects=['{effect}']\ninputs=['{input}']\nhandler={{kind='native',crate_dir='local-fixture'}}\n"));
    }
    fs::write(temp.path().join("vibe.toml"), manifest).unwrap();
    temp
}
fn begin(root: &Path, requested: &str, id: &str, force: bool) -> LifecycleRun {
    begin_selected(root, requested, id, force, ".")
}
fn begin_selected(
    root: &Path,
    requested: &str,
    id: &str,
    force: bool,
    selected: &str,
) -> LifecycleRun {
    LifecycleRun::begin(
        Arc::new(LifecycleLease::acquire(root).unwrap()),
        Project {
            kind: "project".into(),
            manifest: root.join("vibe.toml").to_string_lossy().into(),
            name: "consumer".into(),
            root: root.to_string_lossy().into(),
            spec_roots: vec![],
            version: "1.0.0".into(),
        },
        World {
            deps_root: root.join("vibevm/vibedeps").to_string_lossy().into(),
            lockfile: root.join("vibe.lock").to_string_lossy().into(),
            packages: vec![],
        },
        RunMetadata {
            requested: requested.into(),
            chain: vec!["create".into(), "build".into(), "verify".into()],
            offline: true,
            assume_yes: true,
            agent_mode: RunAgentMode::Cli,
            force,
            trace_compile: false,
            run_id: id.into(),
            started: "2026-10-13T08:00:00Z".into(),
            selected: selected.into(),
        },
        vec!["create".into(), "build".into(), "verify".into()],
    )
    .unwrap()
}
#[test]
fn completed_production_builds_then_verify_observes_without_producer_dispatch() {
    let temp = fixture();
    let root = temp.path();
    let rows = rows(root);
    let operations = LocalOperations {
        calls: Mutex::new(vec![]),
    };
    let runtime = HandlerRuntime {
        process: &SystemProcessRunner,
        binary: &NoBinaryBackend,
        native: &operations,
        package_binding: &NoPackageBindingBackend,
        agent: &NoAgentBackend,
        probe: &SystemProbe,
        streams: StreamMode::Capture,
        protected_script: None,
    };
    let row = |name: &str| {
        HandlerExecution::from_row(rows.iter().find(|r| r.declaration().id == name).unwrap())
    };
    {
        let mut run = begin(root, "create", "00112233445566778899aabbccddeeff", false);
        assert_eq!(
            run.execute_one(&row("produce"), "create", ExecutionReuse::Always, &runtime)
                .unwrap()
                .status,
            crate::state::current::ExecutionRecordStatus::Ok
        );
    }
    let source = fs::read(root.join("src/product.rs")).unwrap();
    {
        let mut run = begin(root, "build", "11112233445566778899aabbccddeeff", true);
        assert_eq!(
            run.execute_one(&row("produce"), "create", ExecutionReuse::Always, &runtime)
                .unwrap()
                .status,
            crate::state::current::ExecutionRecordStatus::Fresh
        );
        run.execute_one(&row("compile"), "build", ExecutionReuse::Always, &runtime)
            .unwrap();
    }
    {
        let mut run = begin(root, "verify", "22112233445566778899aabbccddeeff", true);
        assert_eq!(
            run.execute_one(&row("produce"), "create", ExecutionReuse::Always, &runtime)
                .unwrap()
                .status,
            crate::state::current::ExecutionRecordStatus::Fresh
        );
        assert_eq!(
            run.execute_one(&row("compile"), "build", ExecutionReuse::Always, &runtime)
                .unwrap()
                .status,
            crate::state::current::ExecutionRecordStatus::Fresh
        );
        run.execute_one(&row("observe"), "verify", ExecutionReuse::Always, &runtime)
            .unwrap();
    }
    assert_eq!(
        *operations.calls.lock().unwrap(),
        ["produce", "compile", "observe"]
    );
    assert_eq!(source, fs::read(root.join("src/product.rs")).unwrap());
    assert!(
        fs::read_to_string(root.join("evidence/test.stdout"))
            .unwrap()
            .contains("1 passed")
    );
    let state = LifecycleStateStore::peek(root).unwrap().unwrap();
    assert_eq!(state.schema, 2);
    assert_eq!(
        state.execution["org.demo/consumer#produce"]
            .completion_basis
            .as_ref()
            .unwrap()
            .run_id,
        "00112233445566778899aabbccddeeff"
    );
    println!(
        "ACTUAL_INVOCATION {}",
        serde_json::to_string(&state).unwrap()
    );
}

#[test]
fn explicit_full_pass_produces_before_actual_compile_and_test() {
    let temp = fixture();
    let root = temp.path();
    let rows = rows(root);
    let operations = LocalOperations {
        calls: Mutex::new(vec![]),
    };
    let runtime = HandlerRuntime {
        process: &SystemProcessRunner,
        binary: &NoBinaryBackend,
        native: &operations,
        package_binding: &NoPackageBindingBackend,
        agent: &NoAgentBackend,
        probe: &SystemProbe,
        streams: StreamMode::Capture,
        protected_script: None,
    };
    let mut run = begin(root, "full-pass", "33112233445566778899aabbccddeeff", true);
    for (name, phase) in [
        ("produce", "create"),
        ("compile", "build"),
        ("observe", "verify"),
    ] {
        let row = rows
            .iter()
            .find(|row| row.declaration().id == name)
            .unwrap();
        run.execute_one(
            &HandlerExecution::from_row(row),
            phase,
            ExecutionReuse::Always,
            &runtime,
        )
        .unwrap();
    }
    assert_eq!(
        *operations.calls.lock().unwrap(),
        ["produce", "compile", "observe"]
    );
    assert_eq!(
        invocation_chain(Phase::Verify, InvocationIntent::FullPass),
        [
            Phase::Validate,
            Phase::Install,
            Phase::Generate,
            Phase::Create,
            Phase::Build,
            Phase::Test,
            Phase::Verify
        ]
    );
    assert!(
        fs::read_to_string(root.join("evidence/test.stdout"))
            .unwrap()
            .contains("1 passed")
    );
}

#[test]
fn changed_input_effect_output_or_original_selected_cannot_dispatch_producer() {
    for perturb in ["input", "output", "effects", "selected"] {
        let temp = fixture();
        let root = temp.path();
        let operations = LocalOperations {
            calls: Mutex::new(vec![]),
        };
        let runtime = HandlerRuntime {
            process: &SystemProcessRunner,
            binary: &NoBinaryBackend,
            native: &operations,
            package_binding: &NoPackageBindingBackend,
            agent: &NoAgentBackend,
            probe: &SystemProbe,
            streams: StreamMode::Capture,
            protected_script: None,
        };
        let before_rows = rows(root);
        let produce = HandlerExecution::from_row(
            before_rows
                .iter()
                .find(|r| r.declaration().id == "produce")
                .unwrap(),
        );
        {
            let mut run = begin(root, "create", "00112233445566778899aabbccddeeff", false);
            run.execute_one(&produce, "create", ExecutionReuse::Always, &runtime)
                .unwrap();
        }
        match perturb {
            "input" => fs::write(root.join("intent.txt"), "changed intent\n").unwrap(),
            "output" => {
                fs::write(root.join("src/product.rs"), "externally changed source\n").unwrap()
            }
            "effects" => {
                let text = fs::read_to_string(root.join("vibe.toml")).unwrap();
                fs::write(
                    root.join("vibe.toml"),
                    text.replacen(
                        "effects=['product-source']",
                        "effects=['product-source','read-only']",
                        1,
                    ),
                )
                .unwrap();
            }
            "selected" => {}
            _ => unreachable!(),
        }
        let source = fs::read(root.join("src/product.rs")).unwrap();
        let current = rows(root);
        let row = HandlerExecution::from_row(
            current
                .iter()
                .find(|r| r.declaration().id == "produce")
                .unwrap(),
        );
        let mut run = begin_selected(
            root,
            "verify",
            "44112233445566778899aabbccddeeff",
            true,
            if perturb == "selected" { "other" } else { "." },
        );
        let error = run
            .execute_one(&row, "create", ExecutionReuse::Always, &runtime)
            .unwrap_err();
        assert!(
            matches!(
                error,
                LifecycleRunError::Invocation(super::InvocationRefusal::CreationRequired { .. })
            ),
            "{error}"
        );
        assert_eq!(*operations.calls.lock().unwrap(), ["produce"]);
        assert_eq!(source, fs::read(root.join("src/product.rs")).unwrap());
    }
}

#[test]
fn effect_and_completion_frames_match_independent_longhand_and_keep_invocation_separate() {
    use sha2::{Digest, Sha256};
    fn frame(h: &mut Sha256, label: &str, bytes: &[u8]) {
        h.update((label.len() as u64).to_be_bytes());
        h.update(label.as_bytes());
        h.update((bytes.len() as u64).to_be_bytes());
        h.update(bytes);
    }
    fn json<T: serde::Serialize>(value: &T) -> Vec<u8> {
        serde_json::to_vec(&serde_json::to_value(value).unwrap()).unwrap()
    }
    let temp = fixture();
    let root = temp.path();
    let rows = rows(root);
    let row = rows
        .iter()
        .find(|r| r.declaration().id == "produce")
        .unwrap();
    let execution = HandlerExecution::from_row(row);
    let run = begin(root, "create", "00112233445566778899aabbccddeeff", false);
    let context = run
        .session
        .as_ref()
        .unwrap()
        .envelope_for_execution("create", &execution)
        .unwrap();
    let actual = crate::state::prepare_handler_execution_with(&execution, &context, None).unwrap();
    let mut material: Vec<(&str, Vec<u8>)> = vec![
        ("effects.count", b"1".to_vec()),
        ("effects.item", b"product-source".to_vec()),
        ("key", b"org.demo/consumer#produce".to_vec()),
        ("phase", b"create".to_vec()),
        ("point", b"phase:create".to_vec()),
        ("slot-target", b"null".to_vec()),
        ("handler-kind", b"native".to_vec()),
        ("handler-crate", b"local-fixture".to_vec()),
        ("effective-config", json(&context.execution.config)),
        ("provider-id", b"org.demo/consumer".to_vec()),
        ("provider-version", b"1.0.0".to_vec()),
        ("provider-content", vec![]),
        ("requested", b"create".to_vec()),
        ("chain", json(&context.run.chain)),
        ("offline", vec![1]),
        ("agent-mode", json(&context.run.agent_mode)),
        ("project", json(&context.project)),
        ("world", json(&context.world)),
        ("artifacts", json(&context.artifacts)),
        ("manifest-presence", b"present".to_vec()),
        ("manifest", fs::read(root.join("vibe.toml")).unwrap()),
        ("lockfile-presence", b"missing".to_vec()),
        ("input-pattern", b"intent.txt".to_vec()),
        ("input-path", b"intent.txt".to_vec()),
        ("input-bytes", b"one stable intent\n".to_vec()),
    ];
    let mut exec = Sha256::new();
    exec.update(b"vibe-lifecycle-fingerprint-effects\0epoch=2\0");
    let mut completion = Sha256::new();
    completion.update(b"vibe-completion-basis\0epoch=1\0");
    frame(
        &mut completion,
        "completion.execution",
        b"org.demo/consumer#produce",
    );
    frame(
        &mut completion,
        "completion.selected_root",
        root.to_str().unwrap().as_bytes(),
    );
    for (label, bytes) in &material {
        frame(&mut exec, label, bytes);
        if !matches!(*label, "requested" | "chain") {
            frame(&mut completion, label, bytes);
        }
    }
    assert_eq!(actual.fingerprint, format!("sha256:{:x}", exec.finalize()));
    assert_eq!(
        actual.completion_digest,
        Some(format!("sha256:{:x}", completion.finalize()))
    );
    let mut declaration = Sha256::new();
    declaration.update(b"vibe-execution-declaration-effects-v2\0epoch=2\0");
    for (label, bytes) in [
        ("effects.count", "1"),
        ("effects.item", "product-source"),
        ("execution", "org.demo/consumer#produce"),
        ("phase", "create"),
        ("point", "phase:create"),
        ("slot_present", "0"),
        ("handler_kind", "native"),
        ("handler_crate_present", "1"),
        ("handler_crate", "local-fixture"),
        ("handler_prebuilt_present", "0"),
    ] {
        frame(&mut declaration, label, bytes.as_bytes());
    }
    frame(
        &mut declaration,
        "effective_config",
        &json(&context.execution.config),
    );
    for (label, bytes) in [
        ("provider_kind", "host"),
        ("provider_id", "org.demo/consumer"),
        ("provider_version", "1.0.0"),
        ("provider_content_present", "0"),
        ("inputs_present", "1"),
        ("pattern_count", "1"),
        ("pattern", "intent.txt"),
        ("pass_present", "0"),
        ("compiler_internals_present", "0"),
        ("prompt_present", "0"),
    ] {
        frame(&mut declaration, label, bytes.as_bytes());
    }
    assert_eq!(
        actual.declaration_fingerprint,
        format!("sha256:{:x}", declaration.finalize())
    );
    let mut verify = context.clone();
    verify.run.requested = "verify".into();
    verify.run.chain = vec!["create".into(), "build".into(), "verify".into()];
    let changed = crate::state::prepare_handler_execution_with(&execution, &verify, None).unwrap();
    assert_ne!(actual.fingerprint, changed.fingerprint);
    assert_eq!(actual.completion_digest, changed.completion_digest);
    assert_eq!(
        actual.declaration_fingerprint,
        changed.declaration_fingerprint
    );
    material.clear(); // no producer/observer or additional filesystem walk invoked by the oracle.
    println!(
        "{}",
        serde_json::json!({"recipe":"A1-independent-frames","execution":actual.fingerprint,"declaration":actual.declaration_fingerprint,"completion":actual.completion_digest})
    );
}
