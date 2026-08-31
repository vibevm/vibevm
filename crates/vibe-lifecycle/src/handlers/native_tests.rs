use super::*;

type SeenNative = (
    String,
    ExtensionPoint,
    Option<u32>,
    vibe_wire::generated::native::e1::context::Context,
);

struct FakeNative {
    seen: Mutex<Vec<SeenNative>>,
}

impl NativeBackend for FakeNative {
    fn invoke(
        &self,
        request: NativeBackendRequest<'_>,
    ) -> Result<vibe_wire::generated::native::e1::reply::Reply, String> {
        self.seen
            .lock()
            .map_err(|_| "fake native observation lock was poisoned".to_owned())?
            .push((
                request.extension_id.to_owned(),
                request.point,
                request.ir_schema,
                request.context.clone(),
            ));
        Ok(vibe_wire::generated::native::e1::reply::Reply {
            artifacts: vec![ReplyArtifact {
                id: "native-output".into(),
                kind: "file".into(),
                path: "native-output.txt".into(),
            }],
            envelope: 1,
            status: ReplyStatus::Skip,
            message: Some("native message".into()),
        })
    }
}

#[test]
fn native_dispatch_converts_exact_context_and_reply_without_tasks() {
    let dir = tempfile::tempdir().unwrap();
    let row = row(
        dir.path(),
        ExtensionHandler::Native {
            crate_dir: Some("native".into()),
            prebuilt: None,
        },
    );
    let (_, context) = prepared(dir.path(), &row);
    let backend = FakeNative {
        seen: Mutex::new(Vec::new()),
    };
    let runner = FakeRunner {
        output: ProcessOutput::default(),
        reply: None,
        seen: Mutex::new(Vec::new()),
    };
    let runtime = HandlerRuntime {
        process: &runner,
        binary: &NoBinaryBackend,
        native: &backend,
        package_binding: &super::NoPackageBindingBackend,
        agent: &crate::NoAgentBackend,
        probe: &BashProbe,
        streams: StreamMode::Capture,
    };

    let (reply, streams) = runtime.dispatch(&row, &context).unwrap();
    assert_eq!(reply.status, ReplyStatus::Skip);
    assert_eq!(reply.message.as_deref(), Some("native message"));
    assert_eq!(reply.artifacts[0].id, "native-output");
    assert!(reply.tasks.is_empty(), "native replies cannot retain tasks");
    assert_eq!(streams, HandlerStreams::default());

    let seen = backend.seen.lock().unwrap();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].0, "handler");
    assert_eq!(seen[0].1, "phase:build".parse().unwrap());
    assert_eq!(seen[0].2, None);
    assert_eq!(seen[0].3.artifacts, context.artifacts);
    assert_eq!(seen[0].3.execution, context.execution);
    assert_eq!(seen[0].3.io, context.io);
    assert_eq!(seen[0].3.project, context.project);
    assert_eq!(seen[0].3.run, context.run);
    assert_eq!(seen[0].3.slot_target, context.slot_target);
    assert_eq!(seen[0].3.world, context.world);
    assert_eq!(seen[0].3.point, context.point);
}

#[test]
fn compiler_native_is_refused_before_the_lifecycle_backend() {
    let dir = tempfile::tempdir().unwrap();
    let declaration = vibe_core::manifest::ExtensionDecl {
        id: "compile-native".into(),
        point: "compile:source".parse().unwrap(),
        handler: ExtensionHandler::Native {
            crate_dir: Some("native".into()),
            prebuilt: None,
        },
        config: None,
        auto: None,
        inputs: None,
        applies_to: None,
        compiler_internals: None,
        pass: None,
        when: None,
    };
    let world = crate::ExtensionWorld {
        installed: Vec::new(),
        host: crate::HostExtensionSource {
            provider: crate::HostProvider {
                identity: crate::HostIdentity::ungrouped_project("native-fixture"),
                root: dir.path().to_path_buf(),
                version: "0.1.0".into(),
                kind: None,
                content_hash: None,
            },
            declarations: vec![declaration],
            controls: vibe_core::manifest::ExtensionsControl::default(),
            mechanisms: Vec::new(),
        },
        effective_stack: None,
    };
    let registry = crate::collect_extensions(world).unwrap();
    let row = &registry.rows()[0];
    let (_, context) = prepared(dir.path(), row);
    let backend = FakeNative {
        seen: Mutex::new(Vec::new()),
    };
    let runner = FakeRunner {
        output: ProcessOutput::default(),
        reply: None,
        seen: Mutex::new(Vec::new()),
    };
    let runtime = HandlerRuntime {
        process: &runner,
        binary: &NoBinaryBackend,
        native: &backend,
        package_binding: &super::NoPackageBindingBackend,
        agent: &crate::NoAgentBackend,
        probe: &BashProbe,
        streams: StreamMode::Capture,
    };

    let error = runtime
        .dispatch(row, &context)
        .expect_err("compile-native cannot enter lifecycle dispatch");
    assert!(
        matches!(error, HandlerError::Native { reason, .. } if reason.contains("compiler-native"))
    );
    assert!(backend.seen.lock().unwrap().is_empty());
}
