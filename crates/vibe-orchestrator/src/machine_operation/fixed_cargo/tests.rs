use super::*;
use serde_json::json;
use std::{fs, path::Path, sync::Arc};
use vibe_core::manifest::{ExtensionsControl, MechanismRoutes};
use vibe_extension_registry::collect_mechanisms;
use vibe_lifecycle::{ExtensionWorld, HostExtensionSource, HostIdentity, HostProvider};
use vibe_wire::generated::machine::e1::operation_envelope as e;

fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("Cargo.toml"), b"[package]\nname = \"vibe-r8-fixture\"\nversion = \"1.1.0\"\nedition = \"2021\"\n\n[[bin]]\nname = \"r8probe\"\npath = \"src/main.rs\"\n\n[workspace]\n").unwrap();
    fs::create_dir_all(root.path().join("src")).unwrap();
    fs::write(
        root.path().join("src/main.rs"),
        b"fn main() {\n    println!(\"r8\");\n}\n",
    )
    .unwrap();
    root
}
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
fn scope() -> SubmissionScope {
    SubmissionScope {
        context: "host-context".into(),
        target: "fixture-tool".into(),
        principal: "owner".into(),
    }
}
fn host(
    root: &Path,
) -> (
    Arc<Project>,
    SubmissionNamespace,
    SubmissionScope,
    FixedCargoHost,
) {
    let project = Arc::new(Project::open(root).unwrap());
    let trusted = scope();
    let lease = LifecycleLease::acquire(root).unwrap();
    let namespace = SubmissionReservations::issue_namespace(&lease, &trusted, 60_000).unwrap();
    drop(lease);
    let host = FixedCargoHost::from_trusted_host(
        project.clone(),
        namespace.clone(),
        trusted.clone(),
        registry(),
        Arc::new(MechanismRoutes::default()),
    )
    .unwrap();
    (project, namespace, trusted, host)
}
fn action(kind: &str, key: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({"schema":1,"action":{"kind":kind,"key":key}})).unwrap()
}
fn envelope(kind: &str, key: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({"schema":1,"operation_id":OPERATION_ID,
        "domain_owner":DOMAIN_OWNER,"source_id":null,"evidence_id":null,
        "body":{"stage":"input","format_id":"machine-fixed-cargo-action-request",
            "payload_json":String::from_utf8(action(kind,key)).unwrap()}}))
    .unwrap()
}

#[test]
fn no_grant_foreign_scope_root_and_client_authority_refuse_before_cargo() {
    let root = fixture();
    let (project, namespace, trusted, host) = host(root.path());
    let absent =
        FixedCargoRoute::dispatch(None, &action("submit", "once"), root.path()).unwrap_err();
    assert!(!absent.effects_may_have_begun);
    assert!(absent.error.requested_scope.is_none());
    let foreign = SubmissionScope {
        principal: "foreign".into(),
        ..trusted.clone()
    };
    assert!(matches!(
        FixedCargoHost::from_trusted_host(
            project,
            namespace,
            foreign,
            registry(),
            Arc::new(MechanismRoutes::default())
        ),
        Err(FixedCargoHostError::Namespace(
            SubmissionError::ForeignScope
        ))
    ));
    let unrelated = SubmissionScope {
        target: "launcher".into(),
        ..scope()
    };
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let unrelated_namespace =
        SubmissionReservations::issue_namespace(&lease, &unrelated, 60_000).unwrap();
    drop(lease);
    assert!(matches!(
        FixedCargoHost::from_trusted_host(
            Arc::new(Project::open(root.path()).unwrap()),
            unrelated_namespace,
            unrelated,
            registry(),
            Arc::new(MechanismRoutes::default())
        ),
        Err(FixedCargoHostError::ForeignTarget)
    ));
    let route = FixedCargoRoute::new(Arc::new(host));
    let other = tempfile::tempdir().unwrap();
    let mismatch = FixedCargoRoute::dispatch(Some(&route), &action("submit", "once"), other.path())
        .unwrap_err();
    assert!(!mismatch.effects_may_have_begun);
    for body in [
        json!({"schema":1,"action":{"kind":"submit","key":"once","namespace":"caller"}}),
        json!({"schema":1,"action":{"kind":"submit","key":"once","argv":["cargo"]}}),
        json!({"schema":1,"action":{"kind":"submit","key":""}}),
    ] {
        assert!(
            FixedCargoRoute::dispatch(
                Some(&route),
                &serde_json::to_vec(&body).unwrap(),
                root.path()
            )
            .is_err()
        );
    }
    assert!(!root.path().join("target").exists());
    assert!(!root.path().join("Cargo.lock").exists());
}

#[test]
fn post_owner_root_drift_retains_completed_observation_and_may_effect_truth() {
    let root = fixture();
    let (_project, _namespace, _scope, host) = host(root.path());
    let old = root.path().with_extension("original-cargo-root");
    let error = host
        .submit_with_after_owner("drift", |_| {
            fs::rename(root.path(), &old).unwrap();
            fs::create_dir(root.path()).unwrap();
        })
        .unwrap_err();
    fs::remove_dir(root.path()).unwrap();
    fs::rename(&old, root.path()).unwrap();
    let (may_effect, observation) = dispatch::owner_error_parts(&error);
    assert!(may_effect);
    let Some(BuildInspection::Completed { run_id, result }) = observation else {
        panic!("post-owner error lost completed Cargo truth: {error:?}")
    };
    assert!(result.warning.contains("rustup"));
    assert!(
        root.path()
            .join(format!(".vibe/build-operations/{run_id}.cargo-result.json"))
            .is_file()
    );
    let route = FixedCargoRoute::new(Arc::new(host));
    let refusal = dispatch::failure(
        Some(&route),
        contract::err::FixedCargoActionErrorCode::SubmissionRefused,
        "physical root changed after owner completion",
        Some("submit"),
        may_effect,
        Some(BuildInspection::Completed {
            run_id: run_id.clone(),
            result,
        }),
        Some(error),
        Some("drift"),
    );
    let published = envelope::error(&refusal).unwrap();
    let e::Body::Error(body) = published.body else {
        panic!("typed owner error")
    };
    assert_eq!(body.effect_state, e::BodyErrorEffectState::Completed);
    assert_eq!(body.provenance_id.as_deref(), Some(run_id.as_str()));
    let value: serde_json::Value =
        serde_json::from_str(body.payload_json.as_ref().unwrap()).unwrap();
    assert_eq!(value["known_run_id"], run_id);
    assert_eq!(value["requested_scope"]["target"], "fixture-tool");
    assert_eq!(value["effect_observation"], "landed");
}

#[test]
fn fixed_owner_same_key_attaches_once_and_damaged_result_stays_uncertain() {
    let root = fixture();
    let (_project, namespace, trusted, host) = host(root.path());
    let route = FixedCargoRoute::new(Arc::new(host));
    let first =
        FixedCargoRoute::exchange_local(Some(&route), &envelope("submit", "once"), root.path())
            .unwrap();
    let e::Body::Result(first_body) = first.body else {
        panic!("first result")
    };
    assert_eq!(first_body.effect_state, e::BodyResultEffectState::Completed);
    let first_reply: serde_json::Value =
        serde_json::from_str(first_body.payload_json.as_ref().unwrap()).unwrap();
    assert_eq!(first_reply["reply"]["kind"], "completed");
    assert!(
        first_reply["reply"]["warning"]
            .as_str()
            .unwrap()
            .contains("rustup")
    );
    let run_id = first_reply["reply"]["run_id"].as_str().unwrap();
    let result = root
        .path()
        .join(format!(".vibe/build-operations/{run_id}.cargo-result.json"));
    assert!(result.is_file());
    let retained = fs::read(&result).unwrap();
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    SubmissionReservations::retire_namespace(&lease, &namespace, &trusted).unwrap();
    drop(lease);
    let replay =
        FixedCargoRoute::dispatch(Some(&route), &action("submit", "once"), root.path()).unwrap();
    assert_eq!(serde_json::to_value(replay).unwrap(), first_reply);
    assert_eq!(fs::read(&result).unwrap(), retained);
    assert_eq!(
        fs::read_dir(root.path().join(".vibe/build-operations"))
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e
                .file_name()
                .to_string_lossy()
                .ends_with(".cargo-result.json"))
            .count(),
        1
    );
    fs::remove_file(&result).unwrap();
    let uncertain =
        FixedCargoRoute::exchange_local(Some(&route), &envelope("inspect", "once"), root.path())
            .unwrap();
    let e::Body::Result(body) = uncertain.body else {
        panic!("uncertain result")
    };
    assert_eq!(body.effect_state, e::BodyResultEffectState::Uncertain);
    let view: serde_json::Value =
        serde_json::from_str(body.payload_json.as_ref().unwrap()).unwrap();
    assert_eq!(view["reply"]["kind"], "uncertain");
    let retry =
        FixedCargoRoute::dispatch(Some(&route), &action("submit", "once"), root.path()).unwrap();
    assert_eq!(serde_json::to_value(retry).unwrap(), view);
    assert!(!result.exists());
}
