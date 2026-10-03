use super::*;
use vibe_core::manifest::{ExtensionsControl, MechanismRoutes};
use vibe_extension_registry::collect_mechanisms;
use vibe_lifecycle::{ExtensionWorld, HostExtensionSource, HostIdentity, HostProvider};

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
fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("Cargo.toml"), b"[package]\nname = \"vibe-r8-fixture\"\nversion = \"1.1.0\"\nedition = \"2021\"\n\n[[bin]]\nname = \"r8probe\"\npath = \"src/main.rs\"\n\n[workspace]\n").unwrap();
    std::fs::create_dir_all(root.path().join("src")).unwrap();
    std::fs::write(
        root.path().join("src/main.rs"),
        b"fn main() {\n    println!(\"r8\");\n}\n",
    )
    .unwrap();
    root
}
fn service(root: &std::path::Path) -> CargoGrantService {
    CargoGrantService::from_trusted_host(
        Arc::new(Project::open(root).unwrap()),
        "vibe-cli-project-local/e1".into(),
        TargetOs::current().unwrap(),
        registry(),
        Arc::new(MechanismRoutes::default()),
    )
    .unwrap()
}
fn namespace_count(root: &std::path::Path) -> usize {
    let index: serde_json::Value = serde_json::from_slice(
        &std::fs::read(root.join(".vibe/machine-submissions.json")).unwrap(),
    )
    .unwrap();
    index["namespaces"].as_object().unwrap().len()
}
fn no_effect(root: &std::path::Path) {
    assert!(!root.join("target").exists());
    assert!(!root.join("Cargo.lock").exists());
    assert!(!root.join(".vibe/build-operations").exists());
}

#[test]
fn trusted_builtin_constructor_has_no_routes_or_issuance_before_issue() {
    let root = fixture();
    let grant = CargoGrantService::from_trusted_builtin_host(
        Arc::new(Project::open(root.path()).unwrap()),
        "shared-principal".into(),
        TargetOs::current().unwrap(),
    )
    .unwrap();
    assert!(grant.routes.is_empty());
    verify_cargo_plane(&grant.registry, &grant.routes).unwrap();
    assert!(!root.path().join(&grant.path).exists());
    assert!(!root.path().join(".vibe/machine-submissions.json").exists());
    no_effect(root.path());
}

#[test]
fn first_issue_and_reattach_keep_one_cargo_namespace_and_current_policy() {
    let root = fixture();
    let grant = service(root.path());
    assert!(matches!(
        grant.reattach(),
        Err(CargoGrantError::MissingPointer)
    ));
    let host = grant.issue(60_000).unwrap();
    assert!(matches!(
        host.inspect("unknown").unwrap(),
        vibe_lifecycle::retained_mechanism::BuildInspection::Absent
    ));
    let pointer = std::fs::read(root.path().join(&grant.path)).unwrap();
    assert!(matches!(
        grant.issue(60_000),
        Err(CargoGrantError::AlreadyIssued)
    ));
    drop(host);
    let cold = service(root.path()).reattach().unwrap();
    assert!(matches!(
        cold.inspect("unknown").unwrap(),
        vibe_lifecycle::retained_mechanism::BuildInspection::Absent
    ));
    assert_eq!(
        std::fs::read(root.path().join(&grant.path)).unwrap(),
        pointer
    );
    assert_eq!(namespace_count(root.path()), 1);
    no_effect(root.path());
}

#[test]
fn missing_corrupt_and_foreign_pointers_never_reissue_or_build() {
    let missing = fixture();
    let grant = service(missing.path());
    grant.issue(60_000).unwrap();
    std::fs::remove_file(missing.path().join(&grant.path)).unwrap();
    assert!(matches!(
        grant.reattach(),
        Err(CargoGrantError::MissingPointer)
    ));
    assert!(matches!(
        grant.issue(60_000),
        Err(CargoGrantError::MissingPointer)
    ));
    assert_eq!(namespace_count(missing.path()), 1);
    no_effect(missing.path());

    let corrupt = fixture();
    let grant = service(corrupt.path());
    grant.issue(60_000).unwrap();
    std::fs::write(corrupt.path().join(&grant.path), b"{broken").unwrap();
    assert!(matches!(
        grant.reattach(),
        Err(CargoGrantError::InvalidPointer(_))
    ));
    assert!(matches!(
        grant.issue(60_000),
        Err(CargoGrantError::InvalidPointer(_))
    ));
    assert_eq!(namespace_count(corrupt.path()), 1);
    no_effect(corrupt.path());

    let first = fixture();
    let second = fixture();
    let one = service(first.path());
    let two = service(second.path());
    one.issue(60_000).unwrap();
    two.issue(60_000).unwrap();
    std::fs::write(
        second.path().join(&two.path),
        std::fs::read(first.path().join(&one.path)).unwrap(),
    )
    .unwrap();
    assert!(matches!(
        two.reattach(),
        Err(CargoGrantError::ForeignBinding)
    ));
    assert!(matches!(
        two.issue(60_000),
        Err(CargoGrantError::ForeignBinding)
    ));
    assert_eq!(namespace_count(second.path()), 1);
    no_effect(second.path());
}

#[test]
fn current_policy_drift_refuses_live_submit_and_cold_attach() {
    let root = fixture();
    let grant = service(root.path());
    let host = grant.issue(60_000).unwrap();
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let owner = PolicyOwner::attach_readonly(grant.project.clone(), grant.scope.clone()).unwrap();
    let current = owner.current_under_lease(&lease).unwrap();
    owner
        .update_under_lease(
            &lease,
            current.generation(),
            TrustedPolicySource::from_host_identifier("different-trusted-source".into()).unwrap(),
            cargo_policy_facts(),
        )
        .unwrap();
    drop(lease);
    assert!(matches!(
        host.submit("new"),
        Err(FixedCargoHostError::Policy(
            PolicyError::StartupDisagreement
        ))
    ));
    assert!(matches!(
        host.inspect("old"),
        Err(FixedCargoHostError::Policy(
            PolicyError::StartupDisagreement
        ))
    ));
    assert!(matches!(
        grant.reattach(),
        Err(CargoGrantError::Policy(PolicyError::StartupDisagreement))
    ));
    assert_eq!(namespace_count(root.path()), 1);
    no_effect(root.path());
}

#[test]
fn retired_grant_reattaches_reserved_key_but_never_starts_fresh_cargo() {
    let root = fixture();
    let grant = service(root.path());
    grant.issue(60_000).unwrap();
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let pointer = grant.read(&lease).unwrap().unwrap();
    let namespace = grant.validate_namespace(&lease, &pointer).unwrap();
    let run_id = vibe_lifecycle::process::propose_run_id(root.path()).unwrap();
    assert!(matches!(
        SubmissionReservations::reserve(
            &lease,
            &namespace,
            &grant.scope,
            "old",
            b"request",
            b"plan",
            &run_id,
        )
        .unwrap(),
        vibe_lifecycle::ReservationDecision::Created { .. }
    ));
    SubmissionReservations::retire_namespace(&lease, &namespace, &grant.scope).unwrap();
    drop(lease);
    let cold = grant.reattach().unwrap();
    assert!(matches!(cold.inspect("old").unwrap(),
        vibe_lifecycle::retained_mechanism::BuildInspection::RecoveryRequired { run_id: seen } if seen == run_id));
    assert!(matches!(
        cold.submit("fresh").unwrap(),
        vibe_lifecycle::retained_mechanism::BuildInspection::Retired { run_id: None }
    ));
    assert_eq!(namespace_count(root.path()), 1);
    no_effect(root.path());
}

#[test]
fn nonbuiltin_route_and_reattach_plane_drift_refuse_before_c1_or_cargo() {
    let root = fixture();
    let mut routed = MechanismRoutes::default();
    routed.insert(
        "build:cargo".parse().unwrap(),
        "org.vibevm/vibe#vibe-bin".parse().unwrap(),
    );
    let wrong = CargoGrantService::from_trusted_host(
        Arc::new(Project::open(root.path()).unwrap()),
        "vibe-cli-project-local/e1".into(),
        TargetOs::current().unwrap(),
        registry(),
        Arc::new(routed.clone()),
    );
    assert!(matches!(
        wrong,
        Err(CargoGrantError::Host(FixedCargoHostError::MechanismPlane(
            _
        )))
    ));
    assert!(!root.path().join(".vibe/machine-submissions.json").exists());
    no_effect(root.path());

    let original = service(root.path());
    original.issue(60_000).unwrap();
    let pointer = std::fs::read(root.path().join(&original.path)).unwrap();
    let drift = CargoGrantService::from_trusted_host(
        Arc::new(Project::open(root.path()).unwrap()),
        "vibe-cli-project-local/e1".into(),
        TargetOs::current().unwrap(),
        registry(),
        Arc::new(routed),
    );
    assert!(matches!(
        drift,
        Err(CargoGrantError::Host(FixedCargoHostError::MechanismPlane(
            _
        )))
    ));
    assert_eq!(
        std::fs::read(root.path().join(&original.path)).unwrap(),
        pointer
    );
    assert_eq!(namespace_count(root.path()), 1);
    no_effect(root.path());
}
