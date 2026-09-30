//! Bound namespace and pre-effect refusal proofs over disposable roots.

use vibe_wire::generated::shared::{
    EnvironmentOperationBinding, EnvironmentOperationBindingRootBinding,
    EnvironmentOperationBindingTargetMode, EnvironmentOperationBindingTargetOs,
    EnvironmentOperationBindingTargetRole,
};

use super::protocol::{DeployPlan, DeployTargetRequest};
use super::sidecar::{LOCK_RESOURCES_EPOCH, LockBinding, LockResources};
use super::state::{CHECKPOINT_EPOCH, CheckpointRecord, InverseRecord};
use super::state::{DeployState, DeploymentHome};
use super::support::{Fixture, FixtureProvider, selected, selection, target};
use super::transaction::{bound_plan_hash, plan_hash};
use super::view::DeployStateView;
use super::{DeployError, apply_selection, execute_bound_deploy_targets};
use vibe_safefs::Project;

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
fn bound_sidecars_checkpoints_and_inverse_refuse_copied_or_legacy_identity() {
    let fixture = Fixture::new("helper-bytes");
    let root = fixture.state_home();
    let state = DeployState::open(&root).unwrap();
    let a_binding = binding("sidecar-a");
    let b_binding = binding("sidecar-b");
    let a =
        DeploymentHome::new_bound(&root, "org.example/demo", None, "helper", &a_binding).unwrap();
    let b =
        DeploymentHome::new_bound(&root, "org.example/demo", None, "helper", &b_binding).unwrap();
    let plan_hash = "a".repeat(64);
    let sidecar = LockResources {
        schema: LOCK_RESOURCES_EPOCH,
        committed: None,
        pending: Some(LockBinding {
            operation_binding: Some(a_binding.clone()),
            generation: 0,
            plan_hash: plan_hash.clone(),
            resources: vec!["bin/helper".into()],
            native: None,
        }),
    };
    state.write_lock_resources(&a, &sidecar).unwrap();
    assert!(matches!(
        state.write_lock_resources(&b, &sidecar),
        Err(DeployError::BindingMismatch { .. })
    ));
    let checkpoint = CheckpointRecord {
        schema: CHECKPOINT_EPOCH,
        operation_binding: Some(a_binding.clone()),
        plan_hash: plan_hash.clone(),
        completed: vec!["store/payload".into()],
    };
    state.write_checkpoints(&a, &checkpoint).unwrap();
    assert!(matches!(
        state.write_checkpoints(&b, &checkpoint),
        Err(DeployError::BindingMismatch { .. })
    ));
    assert!(matches!(
        state.read_checkpoints(&a, &"b".repeat(64)),
        Err(DeployError::BindingMismatch { .. })
    ));
    let mut inverse = InverseRecord::new(
        0,
        "org.vibevm/vibe#vibe-bin",
        "bin/helper",
        &plan_hash,
        "pointer:a",
    );
    assert!(matches!(
        state.write_inverse(&a, &inverse),
        Err(DeployError::BindingMismatch { .. })
    ));
    inverse.operation_binding = Some(a_binding);
    state.write_inverse(&a, &inverse).unwrap();
    assert!(matches!(
        state.write_inverse(&b, &inverse),
        Err(DeployError::BindingMismatch { .. })
    ));
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
    assert!(matches!(
        super::sidecar::inverse_locks(
            None,
            &a_receipt,
            super::sidecar::Ownership {
                target: "local-helper",
                pin: "org.example/deployers#fixture",
                reference: false,
            }
        ),
        Err(DeployError::LockSidecarMissing { .. })
    ));
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

#[test]
fn bound_state_home_from_two_held_settings_handles_shares_records() {
    use vibe_wire::generated::deploy_intent::{DeployIntent, DeployTargetIdentity};
    let settings_dir = tempfile::tempdir().unwrap();
    let first = Project::open(settings_dir.path()).unwrap();
    let alias = Project::open(settings_dir.path()).unwrap();
    let state = DeployState::open_bound(&first).unwrap();
    state.verify_bound_home(&alias).unwrap();
    let state_root = settings_dir.path().join("state/deployments");
    let owner = binding("one");
    let home = DeploymentHome::new_bound(
        &state_root,
        "org.example/demo",
        None,
        "local-helper",
        &owner,
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
        operation_binding: Some(owner),
    };
    state.write_intent(&home, &intent).unwrap();
    let view = DeployStateView::open_bound(&alias).unwrap();
    assert_eq!(view.read_intent(&home).unwrap(), Some(intent));
}

#[test]
fn bound_read_view_absent_is_no_create_and_malformed_refuses() {
    let settings_dir = tempfile::tempdir().unwrap();
    let settings = Project::open(settings_dir.path()).unwrap();
    let state_root = settings_dir.path().join("state/deployments");
    let view = DeployStateView::open_bound(&settings).unwrap();
    let home = DeploymentHome::new(&state_root, "org.example/demo", None, "local-helper");
    assert!(view.read_intent(&home).unwrap().is_none());
    assert!(!settings_dir.path().join("state").exists());
    std::fs::write(settings_dir.path().join("state"), b"foreign file").unwrap();
    assert!(DeployStateView::open_bound(&settings).is_err());
    assert!(DeployState::open_bound(&settings).is_err());
}

#[test]
fn retargeted_bound_state_name_refuses_and_held_write_stays_on_original() {
    use vibe_wire::generated::deploy_intent::{DeployIntent, DeployTargetIdentity};
    let settings_dir = tempfile::tempdir().unwrap();
    let settings = Project::open(settings_dir.path()).unwrap();
    let state = DeployState::open_bound(&settings).unwrap();
    let original = settings_dir.path().join("state/deployments");
    let moved = settings_dir.path().join("state/old-deployments");
    if std::fs::rename(&original, &moved).is_err() {
        // A host that prevents renaming an open directory prevents this
        // retargeting path while the held capability exists.
        state.verify_bound_home(&settings).unwrap();
        return;
    }
    std::fs::create_dir(&original).unwrap();
    assert!(matches!(
        state.verify_bound_home(&settings),
        Err(DeployError::BoundStateHomeChanged { .. })
    ));
    let owner = binding("one");
    let home =
        DeploymentHome::new_bound(&original, "org.example/demo", None, "local-helper", &owner)
            .unwrap();
    let intent = DeployIntent {
        schema: 1,
        plan_hash: "b".repeat(64),
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
        operation_binding: Some(owner),
    };
    // Production must stop at verify_bound_home; this write proves the held
    // capability itself cannot be redirected by changing its display path.
    state.write_intent(&home, &intent).unwrap();
    assert!(moved.join(home.id()).join("intent.json").exists());
    assert!(!original.join(home.id()).join("intent.json").exists());
}
