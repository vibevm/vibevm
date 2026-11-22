//! The binary handler protocol: stdin carries the exact context, stdout is
//! the whole reply, and a nonzero exit, a contaminated or empty stdout or a
//! malformed reply is refused.

use super::*;

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
