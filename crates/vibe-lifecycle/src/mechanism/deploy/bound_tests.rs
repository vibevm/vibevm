//! Bound namespace and pre-effect refusal proofs over disposable roots.

use vibe_wire::generated::shared::{
    EnvironmentOperationBinding, EnvironmentOperationBindingRootBinding,
    EnvironmentOperationBindingTargetMode, EnvironmentOperationBindingTargetOs,
    EnvironmentOperationBindingTargetRole,
};

use super::protocol::{DeployPlan, DeployTargetRequest};
use super::state::{DeployState, DeploymentHome};
use super::support::{Fixture, FixtureProvider, selected, selection, target};
use super::transaction::{bound_plan_hash, plan_hash};
use super::{DeployError, apply_selection, execute_bound_deploy_targets};

fn binding(instance: &str) -> EnvironmentOperationBinding {
    EnvironmentOperationBinding {
        binding_epoch: 1,
        admission_epoch: 1,
        binding_id: format!("binding-{instance}"),
        host_context_id: "trusted-context".into(),
        instance_ref: instance.into(),
        selected_generation_ref: Some("generation-a".into()),
        target_role: EnvironmentOperationBindingTargetRole::ProducedToolRuntime,
        target_mode: EnvironmentOperationBindingTargetMode::WorkspaceProjection,
        target_os: EnvironmentOperationBindingTargetOs::Linux,
        target_architecture: "x86_64".into(),
        target_abi: Some("gnu".into()),
        target_libc: Some("glibc".into()),
        principal_ref: "principal-a".into(),
        storage_binding_ref: "storage-a".into(),
        root_bindings: vec![EnvironmentOperationBindingRootBinding {
            root_role: "workspace".into(),
            capability_ref: format!("root-{instance}"),
        }],
    }
}

#[test]
fn bound_homes_are_instance_keyed_and_never_legacy_keys() {
    let fixture = Fixture::new("helper-bytes");
    let state_home = fixture.state_home();
    let legacy = DeploymentHome::new(&state_home, "org.example/demo", None, "local-helper");
    let a = DeploymentHome::new_bound(
        &state_home,
        "org.example/demo",
        None,
        "local-helper",
        &binding("one"),
    )
    .unwrap();
    let b = DeploymentHome::new_bound(
        &state_home,
        "org.example/demo",
        None,
        "local-helper",
        &binding("two"),
    )
    .unwrap();
    assert_ne!(legacy.id(), a.id());
    assert_ne!(legacy.id(), b.id());
    assert_ne!(a.id(), b.id());
    assert_eq!(
        legacy.id(),
        DeploymentHome::new(&state_home, "org.example/demo", None, "local-helper").id()
    );
}

#[test]
fn bound_apply_refuses_before_selection_or_state_creation() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [target("local-helper", "helper.exe", &[])];
    let selection = selection("local", &["local-helper"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &selection, &state_home);
    assert!(!state_home.exists());
    assert!(matches!(
        execute_bound_deploy_targets(&execution, &binding("one")),
        Err(DeployError::BoundRootAdmissionUnavailable { .. })
    ));
    assert!(!state_home.exists());
    assert!(!fixture.destination.path().join("bin/helper").exists());
}

#[test]
fn bound_receipts_stay_distinct_and_copied_legacy_receipt_refuses() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [target("local-helper", "helper.exe", &[])];
    let selection = selection("local", &["local-helper"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &selection, &state_home);
    let resolved = vec![selected(
        &targets[0],
        Box::new(FixtureProvider::new(
            fixture.destination.path(),
            &["bin/helper"],
        )),
    )];
    apply_selection(&execution, &resolved).unwrap();
    let state = DeployState::open(&state_home).unwrap();
    let legacy = DeploymentHome::new(&state_home, "org.example/demo", None, "local-helper");
    let prior = state.read_receipt(&legacy).unwrap().unwrap();
    let a_binding = binding("one");
    let b_binding = binding("two");
    let a = DeploymentHome::new_bound(
        &state_home,
        "org.example/demo",
        None,
        "local-helper",
        &a_binding,
    )
    .unwrap();
    let b = DeploymentHome::new_bound(
        &state_home,
        "org.example/demo",
        None,
        "local-helper",
        &b_binding,
    )
    .unwrap();
    assert!(matches!(
        state.write_receipt(&a, &prior),
        Err(DeployError::BindingMismatch { .. })
    ));
    let mut a_receipt = prior.clone();
    a_receipt.operation_binding = Some(a_binding.clone());
    state.write_receipt(&a, &a_receipt).unwrap();
    let mut b_receipt = prior.clone();
    b_receipt.operation_binding = Some(b_binding.clone());
    state.write_receipt(&b, &b_receipt).unwrap();
    assert_eq!(
        state.read_receipt(&a).unwrap().unwrap().operation_binding,
        Some(a_binding.clone())
    );
    assert_eq!(
        state.read_receipt(&b).unwrap().unwrap().operation_binding,
        Some(b_binding.clone())
    );
    let rows = state.receipts().unwrap();
    assert_eq!(rows.len(), 3);
    assert_eq!(
        rows.iter()
            .find(|(id, _)| id == a.id())
            .unwrap()
            .1
            .operation_binding
            .clone(),
        Some(a_binding.clone())
    );
    let inspected = super::model::row(
        a.id().into(),
        &rows.iter().find(|(id, _)| id == a.id()).unwrap().1,
    );
    assert_eq!(inspected.operation_binding, Some(a_binding));
    assert_ne!(a.id(), b.id());
    // The same package bytes under another bound instance cannot borrow
    // its receipt, either through apply's reader or the no-create planner.
    let copied_bound = std::fs::read(a.directory().join("receipt.json")).unwrap();
    std::fs::write(b.directory().join("receipt.json"), copied_bound).unwrap();
    assert!(matches!(
        state.read_receipt(&b),
        Err(DeployError::BindingMismatch { .. })
    ));
    let view = super::view::DeployStateView::open(&state_home).unwrap();
    assert!(matches!(
        view.read_receipt(&b),
        Err(DeployError::BindingMismatch { .. })
    ));
    state.write_receipt(&b, &b_receipt).unwrap();
    // A valid legacy receipt copied under the bound key remains a refusal.
    let copied = std::fs::read(legacy.directory().join("receipt.json")).unwrap();
    std::fs::write(b.directory().join("receipt.json"), copied).unwrap();
    assert!(matches!(
        state.read_receipt(&b),
        Err(DeployError::BindingMismatch { .. })
    ));
    assert!(matches!(
        state.receipts(),
        Err(DeployError::BindingMismatch { .. })
    ));
}

#[test]
fn bound_plan_hash_changes_with_principal_instance_and_mode() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [target("local-helper", "helper.exe", &[])];
    let selection = selection("local", &["local-helper"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &selection, &state_home);
    let request = DeployTargetRequest {
        target: &targets[0],
        profile: &selection.profile,
        project_root: execution.project_root,
        settings_root: execution.settings_root,
        user_home: execution.user_home,
        clients: execution.clients,
        prior_receipt: None,
        recovery_intent: None,
        artifact: None,
        staging: None,
    };
    let plan = DeployPlan {
        resources: vec![],
        lock_resources: vec![],
        config_digest: "a".repeat(64),
        reversible: true,
        summary: "fixture".into(),
    };
    let legacy = plan_hash(&request, "org.example/deployers#fixture", &plan);
    let one = binding("one");
    let two = binding("two");
    let first = bound_plan_hash(&request, "org.example/deployers#fixture", &plan, &one);
    assert_ne!(first, legacy);
    assert_ne!(
        first,
        bound_plan_hash(&request, "org.example/deployers#fixture", &plan, &two)
    );
    let mut principal = one.clone();
    principal.principal_ref = "different-principal".into();
    assert_ne!(
        first,
        bound_plan_hash(&request, "org.example/deployers#fixture", &plan, &principal)
    );
    let mut mode = one;
    mode.target_mode = EnvironmentOperationBindingTargetMode::UserProfile;
    assert_ne!(
        first,
        bound_plan_hash(&request, "org.example/deployers#fixture", &plan, &mode)
    );
    assert_eq!(
        legacy,
        plan_hash(&request, "org.example/deployers#fixture", &plan)
    );
}

#[test]
fn bound_home_refuses_unbound_intent_before_it_is_written() {
    use vibe_wire::generated::deploy_intent::{DeployIntent, DeployTargetIdentity};
    let fixture = Fixture::new("helper-bytes");
    let state_home = fixture.state_home();
    let state = DeployState::open(&state_home).unwrap();
    let home = DeploymentHome::new_bound(
        &state_home,
        "org.example/demo",
        None,
        "local-helper",
        &binding("one"),
    )
    .unwrap();
    let intent = DeployIntent {
        schema: 1,
        plan_hash: "a".repeat(64),
        resources: vec![],
        started_at: "2026-09-30T00:00:00Z".parse().unwrap(),
        target: DeployTargetIdentity {
            generation: 0,
            profile: "local".into(),
            project: "org.example/demo".into(),
            target: "local-helper".into(),
            package: None,
        },
        prior_generation: None,
        operation_binding: None,
    };
    assert!(matches!(
        state.write_intent(&home, &intent),
        Err(DeployError::BindingMismatch { .. })
    ));
    assert!(!home.directory().join("intent.json").exists());
}
