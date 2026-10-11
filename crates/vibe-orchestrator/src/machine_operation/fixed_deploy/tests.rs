//! Route composition tests only. Real destination effects are covered by the
//! lifecycle bridge's authored temp-root public_ cases.
use super::*;
use serde_json::json;
use std::{
    fs,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use vibe_core::manifest::{ExtensionsControl, MechanismRoutes};
use vibe_extension_registry::{
    ExtensionWorld, HostExtensionSource, HostIdentity, HostProvider, collect_mechanisms,
};
use vibe_lifecycle::ClientExecutable;
use vibe_lifecycle::retained_mechanism::{DeployCompletion, DeployPartial};
use vibe_wire::generated::deploy_receipt::DeployReceipt;

fn registry() -> Arc<MechanismRegistry> {
    let world = ExtensionWorld {
        installed: Vec::new(),
        host: HostExtensionSource {
            provider: HostProvider {
                identity: HostIdentity::ungrouped_project("demo"),
                root: ".".into(),
                version: "1.1.0".into(),
                kind: None,
                content_hash: None,
            },
            declarations: Vec::new(),
            controls: ExtensionsControl::default(),
            mechanisms: Vec::new(),
        },
        effective_stack: None,
    };
    Arc::new(collect_mechanisms(&world).unwrap())
}
fn clients() -> ClientExecutables {
    ClientExecutables {
        claude: ClientExecutable::Missing {
            command: "claude".into(),
        },
        codex: ClientExecutable::Missing {
            command: "codex".into(),
        },
        opencode: ClientExecutable::Missing {
            command: "opencode".into(),
        },
    }
}
struct Fixture {
    project: tempfile::TempDir,
    settings: tempfile::TempDir,
    home: tempfile::TempDir,
    namespace: SubmissionNamespace,
    scope: SubmissionScope,
}
impl Fixture {
    fn new() -> Self {
        let project = tempfile::tempdir().unwrap();
        let settings = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let scope = SubmissionScope {
            context: "host-context".into(),
            target: "instance-a".into(),
            principal: "owner".into(),
        };
        let lease = LifecycleLease::acquire(project.path()).unwrap();
        let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
        Self {
            project,
            settings,
            home,
            namespace,
            scope,
        }
    }
    fn host(&self) -> Arc<FixedDeployHost> {
        let target = DeployTarget {
            id: "local-helper".into(),
            artifact: "helper.exe".into(),
            mechanism: "deploy:vibe-bin".parse().unwrap(),
            provider: None,
            when: None,
            depends_on: Some(Vec::new()),
            config: None,
        };
        Arc::new(
            FixedDeployHost::from_trusted_host(
                Arc::new(Project::open(self.project.path()).unwrap()),
                Arc::new(Project::open(self.settings.path()).unwrap()),
                Arc::new(Project::open(self.home.path()).unwrap()),
                self.namespace.clone(),
                self.scope.clone(),
                "local".into(),
                target,
                registry(),
                Arc::new(MechanismRoutes::default()),
                clients(),
                "org.example/demo".into(),
                Some("generation-a".into()),
                "nonce-a".into(),
                1,
                Arc::new(|| Ok(Some("generation-a".into()))),
            )
            .unwrap(),
        )
    }
    fn no_c1_key(&self, key: &str) {
        let lease = LifecycleLease::acquire(self.project.path()).unwrap();
        assert!(
            SubmissionReservations::inspect_existing_key(&lease, &self.namespace, &self.scope, key)
                .unwrap()
                .is_none()
        );
    }
}
fn action(kind: &str, key: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({"schema":1,"action":{"kind":kind,"key":key}})).unwrap()
}
fn receipt() -> DeployReceipt {
    serde_json::from_value(json!({"schema":1,"identity":{"project":"org.example/demo"},
        "profile":"local","target":"local-helper","generation":0,
        "artifact_digest":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "provider":{"key":"org.vibevm/vibe#vibe-bin"},
        "desired_config_digest":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "scope":"user","resources":[{"resource":"bin/local-helper","post_digest":"cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"}],
        "reversible":true,"applied_at":"2026-10-03T12:00:00Z","status":"verified",
        "finalized_at":"2026-10-03T12:00:01Z"})).unwrap()
}
fn completed() -> DeployInspection {
    DeployInspection::Completed {
        run_id: "0123456789abcdef0123456789abcdef".into(),
        result: DeployCompletion { receipt: receipt() },
    }
}

#[test]
fn explicit_profile_matches_trusted_selection_before_c1() {
    let fixture = Fixture::new();
    let host = fixture.host();
    assert!(host.check_requested_profile("local").is_ok());
    for requested in ["", "other", "LOCAL", " local "] {
        assert!(matches!(
            host.submit_with_requested_profile("profile-key", requested),
            Err(FixedDeployHostError::RequestedProfileMismatch)
        ));
        fixture.no_c1_key("profile-key");
    }
}

#[test]
fn strict_wire_and_host_root_refuse_before_owner_call() {
    let fixture = Fixture::new();
    let route = FixedDeployRoute::new(fixture.host());
    let calls = AtomicUsize::new(0);
    let missing =
        FixedDeployRoute::dispatch(None, &action("submit", "once"), fixture.project.path())
            .unwrap_err();
    assert!(!missing.effects_may_have_begun);
    assert!(missing.error.requested_scope.is_none());
    let foreign = tempfile::tempdir().unwrap();
    let mismatch = FixedDeployRoute::dispatch_owned_with_owner_call(
        Some(&route),
        &action("submit", "once"),
        foreign.path(),
        PUBLICATION_CAP,
        |_, _| {
            calls.fetch_add(1, Ordering::SeqCst);
            Ok(completed())
        },
    )
    .unwrap_err();
    assert!(!mismatch.effects_may_have_begun);
    for input in [
        json!({"schema":1,"action":{"kind":"submit","key":"once","profile":"foreign"}}),
        json!({"schema":1,"action":{"kind":"submit","key":"once","namespace":"caller"}}),
        json!({"schema":1,"action":{"kind":"submit","key":"once","settings_root":"/tmp"}}),
        json!({"schema":2,"action":{"kind":"submit","key":"once"}}),
    ] {
        let bytes = serde_json::to_vec(&input).unwrap();
        assert!(
            FixedDeployRoute::dispatch_owned_with_owner_call(
                Some(&route),
                &bytes,
                fixture.project.path(),
                PUBLICATION_CAP,
                |_, _| {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Ok(completed())
                },
            )
            .is_err()
        );
    }
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    fixture.no_c1_key("once");
}

#[test]
fn full_receipt_and_cold_replay_projection_leave_c1_to_owner() {
    let fixture = Fixture::new();
    let route = FixedDeployRoute::new(fixture.host());
    let calls = AtomicUsize::new(0);
    let first = FixedDeployRoute::dispatch_owned_with_owner_call(
        Some(&route),
        &action("submit", "same"),
        fixture.project.path(),
        PUBLICATION_CAP,
        |_, _| {
            calls.fetch_add(1, Ordering::SeqCst);
            Ok(completed())
        },
    )
    .unwrap()
    .0;
    let first_json = serde_json::to_value(&first).unwrap();
    assert_eq!(first_json["reply"]["kind"], "completed");
    assert_eq!(first_json["reply"]["receipt_format_id"], "deploy-receipt");
    let nested: DeployReceipt =
        serde_json::from_str(first_json["reply"]["receipt_json"].as_str().unwrap()).unwrap();
    assert_eq!(nested, receipt());
    vibe_wire::behaviour::deploy_records::validate_receipt(&nested).unwrap();
    fs::write(
        fixture.project.path().join("vibe.toml"),
        "changed authored source",
    )
    .unwrap();
    let replay = FixedDeployRoute::dispatch_owned_with_owner_call(
        Some(&route),
        &action("inspect", "same"),
        fixture.project.path(),
        PUBLICATION_CAP,
        |_, _| {
            calls.fetch_add(1, Ordering::SeqCst);
            Ok(completed())
        },
    )
    .unwrap()
    .0;
    assert_eq!(replay, first);
    assert_eq!(
        calls.load(Ordering::SeqCst),
        2,
        "route forwards two observations without adding a C1 fence"
    );
    fixture.no_c1_key("same");
}

#[test]
fn partial_and_post_effect_error_keep_run_scope_and_uncertainty() {
    let fixture = Fixture::new();
    let route = FixedDeployRoute::new(fixture.host());
    let partial = DeployInspection::Uncertain {
        run_id: "0123456789abcdef0123456789abcdef".into(),
        partial: DeployPartial {
            intent: "present".into(),
            receipt: "absent".into(),
            sidecar: "record unreadable".into(),
        },
    };
    let reply = FixedDeployRoute::dispatch_owned_with_owner_call(
        Some(&route),
        &action("inspect", "once"),
        fixture.project.path(),
        PUBLICATION_CAP,
        |_, _| Ok(partial.clone()),
    )
    .unwrap()
    .0;
    let value = serde_json::to_value(reply).unwrap();
    assert_eq!(value["reply"]["partial"]["sidecar"]["kind"], "unavailable");
    assert_eq!(
        value["reply"]["partial"]["sidecar"]["reason"],
        "record unreadable"
    );
    let failure = FixedDeploySubmitError {
        source: vibe_lifecycle::retained_mechanism::RetainedMechanismError::Refused(
            "after Ready".into(),
        ),
        known_run_id: Some("0123456789abcdef0123456789abcdef".into()),
        owner_observation: Some(partial),
        effects_may_have_begun: true,
    };
    let refused = FixedDeployRoute::dispatch_owned_with_owner_call(
        Some(&route),
        &action("submit", "once"),
        fixture.project.path(),
        PUBLICATION_CAP,
        |_, _| Err(FixedDeployHostError::Submit(failure)),
    )
    .unwrap_err();
    assert!(refused.effects_may_have_begun);
    assert_eq!(
        refused.error.known_run_id.as_deref(),
        Some("0123456789abcdef0123456789abcdef")
    );
    assert_eq!(
        refused.error.requested_scope.as_ref().unwrap().target,
        "instance-a"
    );
    assert_eq!(refused.error.requested_phase.as_deref(), Some("deploy"));
    assert_eq!(
        refused.error.effect_observation,
        contract::err::FixedDeployActionErrorEffectObservation::Uncertain
    );
    let envelope = envelope::error(&refused).unwrap();
    let vibe_wire::generated::machine::e1::operation_envelope::Body::Error(body) = envelope.body
    else {
        panic!("typed error")
    };
    assert_eq!(
        body.effect_state,
        vibe_wire::generated::machine::e1::operation_envelope::BodyErrorEffectState::Uncertain
    );
    let unavailable = FixedDeployRoute::dispatch_owned_with_owner_call(
        Some(&route),
        &action("submit", "once"),
        fixture.project.path(),
        PUBLICATION_CAP,
        |_, _| {
            Err(FixedDeployHostError::Admission(
                vibe_lifecycle::retained_mechanism::RetainedMechanismError::Refused(
                    "owner admission unavailable".into(),
                ),
            ))
        },
    )
    .unwrap_err();
    assert!(unavailable.effects_may_have_begun);
    assert_eq!(
        unavailable.error.effect_observation,
        contract::err::FixedDeployActionErrorEffectObservation::Unavailable
    );
    fixture.no_c1_key("once");
}
