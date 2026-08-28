use specmark::verifies;
use vibe_core::manifest::{
    ExtensionConfig, ExtensionDecl, ExtensionHandler, ExtensionUse, ExtensionsControl,
};
use vibe_core::{ContentHash, Group, PackageKind, PackageName};
use vibe_wire::generated::lifecycle::e1::context::{Project, RunAgentMode, World, WorldPackage};
use vibe_wire::generated::lifecycle::e1::reply::ReplyStatus;

use super::*;
use crate::{
    DependencyExtensionSource, DependencyProvider, DependencyProviderId, ExtensionWorld,
    HostExtensionSource, HostIdentity, HostProvider, SelectorSubject, collect_extensions,
};

fn config(source: &str) -> ExtensionConfig {
    ExtensionConfig::from_table(toml::from_str(source).unwrap())
}

fn builtin(id: &str, name: &str, message: Option<&str>) -> ExtensionDecl {
    ExtensionDecl {
        id: id.to_string(),
        point: "phase:build".parse().unwrap(),
        handler: ExtensionHandler::Builtin {
            name: name.to_string(),
        },
        config: message.map(|message| config(&format!("message = {message:?}"))),
        auto: None,
        inputs: None,
        applies_to: None,
        compiler_internals: None,
        pass: None,
        when: None,
    }
}

fn host_registry(declarations: Vec<ExtensionDecl>) -> crate::ExtensionRegistry {
    collect_extensions(ExtensionWorld {
        installed: Vec::new(),
        host: HostExtensionSource {
            provider: HostProvider {
                identity: HostIdentity::ungrouped_project("demo"),
                root: ".".into(),
                version: "0.2.0".to_string(),
                kind: None,
                content_hash: None,
            },
            declarations,
            controls: ExtensionsControl::default(),
        },
        effective_stack: None,
    })
    .unwrap()
}

fn dependency_registry(override_message: &str) -> crate::ExtensionRegistry {
    let id = DependencyProviderId::new(
        Group::parse("org.demo").unwrap(),
        PackageName::parse("provider").unwrap(),
    );
    let key = vibe_core::manifest::ExtensionKey::for_package(id.group(), id.name(), "announce");
    collect_extensions(ExtensionWorld {
        installed: vec![DependencyExtensionSource {
            provider: DependencyProvider {
                id,
                root: "vibevm/vibedeps/org.demo.provider/1.0.0".into(),
                version: "1.0.0".to_string(),
                kind: PackageKind::Tool,
                content_hash: ContentHash::parse("sha256:aa").unwrap(),
            },
            declarations: vec![builtin("announce", "log", Some("authored"))],
        }],
        host: HostExtensionSource {
            provider: HostProvider {
                identity: HostIdentity::ungrouped_project("demo"),
                root: ".".into(),
                version: "0.2.0".to_string(),
                kind: None,
                content_hash: None,
            },
            declarations: Vec::new(),
            controls: ExtensionsControl {
                disable: Vec::new(),
                uses: vec![ExtensionUse {
                    reference: key,
                    config: Some(config(&format!("message = {override_message:?}"))),
                }],
            },
        },
        effective_stack: None,
    })
    .unwrap()
}

fn session() -> ExecutionSession {
    ExecutionSession::new(
        Project {
            kind: "project".to_string(),
            manifest: "C:/work/demo/vibe.toml".to_string(),
            name: "demo".to_string(),
            root: "C:/work/demo".to_string(),
            spec_roots: vec!["C:/work/demo/vibevm/vibespecs".to_string()],
            version: "0.2.0".to_string(),
        },
        World {
            deps_root: "C:/work/demo/vibevm/vibedeps".to_string(),
            lockfile: "C:/work/demo/vibe.lock".to_string(),
            packages: vec![WorldPackage {
                group: "org.demo".to_string(),
                kind: "tool".to_string(),
                name: "provider".to_string(),
                slot: "C:/work/demo/vibevm/vibedeps/org.demo.provider/1.0.0".to_string(),
                version: "1.0.0".to_string(),
            }],
        },
        RunMetadata {
            requested: "test".to_string(),
            chain: vec!["validate", "install", "generate", "build", "test"]
                .into_iter()
                .map(str::to_string)
                .collect(),
            offline: true,
            assume_yes: true,
            agent_mode: RunAgentMode::Cli,
            force: false,
            trace_compile: false,
            run_id: "00000000000000000000000000000000".to_string(),
            started: "2026-08-25T12:00:00Z".to_string(),
            selected: ".".to_string(),
        },
    )
}

fn build_rows(registry: &crate::ExtensionRegistry) -> Vec<crate::ExtensionRegistryRow> {
    registry
        .plan("phase:build".parse().unwrap(), SelectorSubject::unscoped())
        .into_iter()
        .cloned()
        .collect()
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ENVELOPE-LAW")]
fn a_traced_run_metadata_leaves_the_handler_envelope_byte_identical() {
    // The sticky trace bit is HOST observation, not handler input: the
    // generated `run` member carries neither it nor any shadow of it,
    // and a traced invocation's envelope is member-for-member the
    // untraced one.
    let registry = host_registry(vec![builtin("announce", "log", Some("traced"))]);
    let rows = build_rows(&registry);
    let mut traced = session();
    traced.run.trace_compile = true;

    let traced_envelope = traced.envelope_for("build", &rows[0]).unwrap();
    let plain_envelope = session().envelope_for("build", &rows[0]).unwrap();
    assert_eq!(
        serde_json::to_value(&traced_envelope).unwrap(),
        serde_json::to_value(&plain_envelope).unwrap(),
        "the domain trace bit never changes the serialized envelope"
    );
    let run = serde_json::to_value(&traced_envelope).unwrap()["run"].clone();
    let members: std::collections::BTreeSet<&str> = run
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        members,
        [
            "agent_mode",
            "assume_yes",
            "chain",
            "force",
            "offline",
            "phase",
            "requested"
        ]
        .into_iter()
        .collect(),
        "the handler run member keeps its exact epoch-1 field set"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ENVELOPE-LAW")]
fn generated_envelope_carries_the_complete_epoch_one_context_and_override() {
    let registry = dependency_registry("{phase}:{project}:{package}:{future}");
    let rows = build_rows(&registry);
    let envelope = session().envelope_for("build", &rows[0]).unwrap();

    assert_eq!(envelope.envelope, 1);
    assert_eq!(envelope.point, "phase:build");
    assert_eq!(envelope.execution.id, "announce");
    assert_eq!(envelope.execution.package, "org.demo/provider");
    assert_eq!(
        envelope.execution.config["message"]
            .as_ref()
            .and_then(serde_json::Value::as_str),
        Some("{phase}:{project}:{package}:{future}"),
    );
    assert_eq!(envelope.project.root, "C:/work/demo");
    assert_eq!(
        envelope.project.spec_roots,
        ["C:/work/demo/vibevm/vibespecs"]
    );
    assert_eq!(envelope.world.packages.len(), 1);
    assert_eq!(
        envelope.world.packages[0].slot,
        "C:/work/demo/vibevm/vibedeps/org.demo.provider/1.0.0"
    );
    assert_eq!(envelope.run.requested, "test");
    assert_eq!(envelope.run.phase, "build");
    assert!(envelope.run.offline);
    assert!(envelope.run.assume_yes);
    assert!(!envelope.run.force);
    assert!(envelope.artifacts.is_empty());
    let scratch = std::path::Path::new(&envelope.io.scratch);
    assert_eq!(
        scratch.parent().unwrap().file_name().unwrap(),
        "00000000000000000000000000000000"
    );
    assert_eq!(scratch.file_name().unwrap().to_string_lossy().len(), 64);
}

#[test]
fn prepared_dispatch_returns_the_exact_envelope_that_was_observed() {
    let registry = host_registry(vec![builtin("prepared", "log", Some("message"))]);
    let row = registry.plan("phase:build".parse().unwrap(), SelectorSubject::unscoped())[0].clone();
    let mut execution = session();
    let prepared = execution.envelope_for("build", &row).unwrap();

    let outcome = execution.dispatch_prepared(&row, prepared.clone()).unwrap();

    assert_eq!(outcome.envelope, prepared);
}

#[test]
fn placeholder_values_are_never_recursively_expanded() {
    let registry = host_registry(vec![builtin(
        "literal",
        "log",
        Some("{project}|{phase}|{package}"),
    )]);
    let rows = build_rows(&registry);
    let mut execution = session();
    execution.project.name = "demo-{phase}-{package}".to_string();
    let batch = execution.dispatch_phase("build", &rows);
    assert_eq!(
        batch.outcomes[0].reply.message.as_deref(),
        Some("demo-{phase}-{package}|build|__host__/demo"),
    );
}

#[test]
fn opaque_keys_receive_collision_free_utf8_hex_scratch_components() {
    let registry = host_registry(vec![
        builtin("a/b", "log", Some("one")),
        builtin("a#b", "log", Some("two")),
    ]);
    let rows = build_rows(&registry);
    let execution = session();
    let first = execution.envelope_for("build", &rows[0]).unwrap();
    let second = execution.envelope_for("build", &rows[1]).unwrap();
    assert_ne!(first.io.scratch, second.io.scratch);
    for scratch in [first.io.scratch, second.io.scratch] {
        let component = scratch.trim_end_matches('/').rsplit('/').next().unwrap();
        assert!(component.bytes().all(|byte| byte.is_ascii_hexdigit()));
        assert_eq!(component.len() % 2, 0);
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TEST-LOG-PLUGIN")]
fn log_renders_known_placeholders_and_preserves_unknown_braces() {
    let registry = dependency_registry("{phase}:{project}:{package}:{future}");
    let rows = build_rows(&registry);
    let batch = session().dispatch_phase("build", &rows);

    assert!(batch.failure.is_none());
    assert_eq!(batch.outcomes.len(), 1);
    assert_eq!(batch.outcomes[0].reply.status, ReplyStatus::Ok);
    assert_eq!(
        batch.outcomes[0].reply.message.as_deref(),
        Some("build:demo:org.demo/provider:{future}"),
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#H-BUILTIN")]
fn unknown_builtin_and_wrong_or_missing_message_fail_with_the_exact_key() {
    for declaration in [
        builtin("unknown", "missing", Some("hello")),
        builtin("missing-message", "log", None),
        {
            let mut row = builtin("wrong-message", "log", None);
            row.config = Some(config("message = 7"));
            row
        },
    ] {
        let registry = host_registry(vec![declaration]);
        let rows = build_rows(&registry);
        let batch = session().dispatch_phase("build", &rows);
        let error = batch.failure.unwrap().to_string();
        assert!(error.contains(rows[0].key().as_str()), "{error}");
        assert!(batch.outcomes.is_empty());
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#FAILURE-BY-PHASE")]
fn first_failure_stops_before_a_later_valid_builtin() {
    let registry = host_registry(vec![
        builtin("first", "log", Some("first")),
        builtin("stop", "unknown", Some("stop")),
        builtin("never", "log", Some("never")),
    ]);
    let rows = build_rows(&registry);
    let batch = session().dispatch_phase("build", &rows);

    assert_eq!(batch.outcomes.len(), 1);
    assert_eq!(batch.outcomes[0].reply.message.as_deref(), Some("first"));
    let failure = batch.failure.unwrap().to_string();
    assert!(failure.contains("#stop"), "{failure}");
    assert!(!failure.contains("#never"), "{failure}");
}

#[test]
fn non_builtin_handler_fails_loudly_instead_of_remaining_planned() {
    let mut declaration = builtin("script", "log", Some("unused"));
    declaration.handler = ExtensionHandler::Binary {
        name: "tool".to_string(),
    };
    let registry = host_registry(vec![declaration]);
    let rows = build_rows(&registry);
    let batch = session().dispatch_phase("build", &rows);
    let error = batch.failure.unwrap().to_string();
    assert!(error.contains("handler kind `binary`"), "{error}");
    assert!(error.contains("#script"), "{error}");
}

#[test]
fn owned_plan_survives_registry_drop_without_recollection_or_resort() {
    let registry = host_registry(vec![
        builtin("first", "log", Some("first")),
        builtin("second", "log", Some("second")),
    ]);
    let plan = crate::ExecutablePlan::from_points(
        &registry,
        [("build".to_string(), "phase:build".parse().unwrap())],
        SelectorSubject::unscoped(),
    );
    drop(registry);
    let rows = plan.iter().map(|item| item.row.clone()).collect::<Vec<_>>();
    let batch = session().dispatch_phase("build", &rows);
    let messages = batch
        .outcomes
        .iter()
        .map(|outcome| outcome.reply.message.as_deref().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(messages, ["first", "second"]);
}

#[test]
fn durable_post_softness_is_typed_and_display_independent() {
    let streams = HandlerStreams {
        stdout: "out".into(),
        stderr: "diagnostic wording changed".into(),
        stdout_truncated: false,
        stderr_truncated: true,
    };
    let nonzero = || DispatchError::Handler {
        key: "typed-nonzero".into(),
        error: Box::new(crate::handlers::HandlerError::NonZero {
            key: "typed-nonzero".into(),
            code: Some(17),
            stderr: "no legacy phrase here".into(),
            streams: Box::new(streams.clone()),
        }),
        streams: Box::new(streams.clone()),
    };
    let misleading = DispatchError::Handler {
        key: "spawn".into(),
        error: Box::new(crate::handlers::HandlerError::Process {
            key: "spawn".into(),
            reason: "unrelated transport text says exited nonzero".into(),
        }),
        streams: Box::new(HandlerStreams::default()),
    };
    let fail_reply = DispatchError::FailedReply {
        key: "reply".into(),
        status: "fail".into(),
        message: None,
        streams: Box::new(streams.clone()),
    };
    assert!(nonzero().is_durable_soft_post());
    assert!(!misleading.is_durable_soft_post());
    assert!(fail_reply.is_durable_soft_post());

    let registry = host_registry(vec![builtin("envelope", "log", Some("x"))]);
    let row = build_rows(&registry).remove(0);
    let envelope = session().envelope_for("build", &row).unwrap();
    let transition = crate::FailedExecutionTransition {
        envelope,
        message: "typed".into(),
        streams: streams.clone(),
    };
    let soft = crate::LifecycleRunError::FailedTransition {
        transition: Box::new(transition.clone()),
        source: Box::new(nonzero()),
    };
    let checkpoint = crate::LifecycleRunError::Checkpoint {
        key: "typed-nonzero".into(),
        primary: "handler failed".into(),
        checkpoint: Box::new(crate::LifecycleStateError::Publication {
            path: "lifecycle.toml".into(),
            stage: vibe_safefs::PublishStage::BeforePublication,
            failure: "checkpoint failed".into(),
        }),
        transition: Some(Box::new(transition)),
        dispatch: Some(Box::new(nonzero())),
    };
    assert!(soft.is_durable_soft_post());
    assert!(!checkpoint.is_durable_soft_post());
    assert!(checkpoint.failed_transition().is_some());
}
