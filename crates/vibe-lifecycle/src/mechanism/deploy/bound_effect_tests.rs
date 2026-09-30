//! The first host-admitted builtin effect stays inside disposable worlds.

use super::binding::{AdmittedLocalDeployBinding, TrustedLocalDeployFacts};
use super::bridge::BoundDeployBridgeError;
use super::bridge::{BoundDeployBridgeInspection, submit_host_admitted_bound_vibe_bin};
use super::state::{DeployState, DeploymentHome};
use super::support::{Fixture, selection};
use super::view::DeployStateView;
use super::{DeployError, execute_host_admitted_bound_vibe_bin};
use crate::{LifecycleLease, SubmissionReservations, SubmissionScope};
use vibe_wire::generated::shared::{
    EnvironmentOperationBindingTargetMode as Mode, EnvironmentOperationBindingTargetOs as Os,
    EnvironmentOperationBindingTargetRole as Role,
};

fn local_os() -> Os {
    match std::env::consts::OS {
        "windows" => Os::Windows,
        "linux" => Os::Linux,
        "macos" => Os::Macos,
        other => panic!("unsupported test host {other}"),
    }
}

fn admitted<'a>(
    execution: &super::DeployExecution<'a>,
    instance: &'a str,
    principal: &'a str,
) -> AdmittedLocalDeployBinding {
    AdmittedLocalDeployBinding::issue(TrustedLocalDeployFacts {
        host_context_id: "host-test-context",
        instance_ref: instance,
        selected_generation_ref: Some("generation-a"),
        principal_ref: principal,
        admission_epoch: 1,
        binding_nonce: instance,
        allow_alias_resolution: false,
        target_mode: Mode::UserProfile,
        target_role: Role::ProducedToolRuntime,
        target_os: local_os(),
        target_architecture: std::env::consts::ARCH,
        target_abi: None,
        target_libc: None,
        project_root: execution.project_root,
        settings_root: execution.settings_root,
        user_home: execution.user_home,
    })
    .unwrap()
}

fn builtin_target() -> vibe_core::manifest::DeployTarget {
    crate::mechanism::vibebin::support::target(
        "local-helper",
        "helper.exe",
        Some("command = \"local-helper\""),
    )
}

#[test]
fn protected_bound_deploy_replays_verified_receipt_without_second_effect() {
    let fixture = Fixture::new("protected-bytes");
    let targets = [builtin_target()];
    let selection = selection("local", &["local-helper"]);
    let state = fixture.state_home();
    let execution = fixture.execution(&targets, &selection, &state);
    let lease = LifecycleLease::acquire(execution.project_root).unwrap();
    let scope = SubmissionScope {
        context: "host-test-context".into(),
        target: "instance-protected".into(),
        principal: "principal-a".into(),
    };
    let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
    let grant = admitted(&execution, "instance-protected", "principal-a");
    let binding = grant.binding().clone();
    let mut current = || Ok(Some("generation-a".to_owned()));
    let first = submit_host_admitted_bound_vibe_bin(
        &lease,
        &namespace,
        &scope,
        "key-a",
        &execution,
        &binding,
        grant,
        "principal-a",
        &mut current,
    )
    .unwrap();
    let (run_id, receipt) = match first {
        BoundDeployBridgeInspection::Completed {
            run_id,
            receipt,
            replayed: false,
        } => (run_id, receipt),
        other => panic!("first protected deploy did not complete: {other:?}"),
    };
    assert_eq!(receipt.generation, 0);
    assert_eq!(
        SubmissionReservations::journal_admission(&lease, &namespace, &scope, "key-a", &run_id)
            .unwrap(),
        crate::JournalAdmission::Ready
    );
    let before = std::fs::read_dir(execution.settings_root.join("store"))
        .unwrap()
        .count();
    let second_grant = admitted(&execution, "instance-protected", "principal-a");
    let mut never_call = || -> Result<Option<String>, DeployError> {
        panic!("replay reached generation or provider")
    };
    let second = submit_host_admitted_bound_vibe_bin(
        &lease,
        &namespace,
        &scope,
        "key-a",
        &execution,
        &binding,
        second_grant,
        "principal-a",
        &mut never_call,
    )
    .unwrap();
    assert!(
        matches!(second, BoundDeployBridgeInspection::Completed { run_id: ref observed,
        replayed: true, .. } if observed == &run_id)
    );
    assert_eq!(
        std::fs::read_dir(execution.settings_root.join("store"))
            .unwrap()
            .count(),
        before
    );
}

#[test]
fn protected_bound_deploy_missing_result_is_uncertain_and_never_reapplies() {
    let fixture = Fixture::new("protected-bytes");
    let targets = [builtin_target()];
    let selection = selection("local", &["local-helper"]);
    let state = fixture.state_home();
    let execution = fixture.execution(&targets, &selection, &state);
    let lease = LifecycleLease::acquire(execution.project_root).unwrap();
    let scope = SubmissionScope {
        context: "host-test-context".into(),
        target: "instance-partial".into(),
        principal: "principal-a".into(),
    };
    let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
    let grant = admitted(&execution, "instance-partial", "principal-a");
    let binding = grant.binding().clone();
    let mut current = || Ok(Some("generation-a".to_owned()));
    let first = submit_host_admitted_bound_vibe_bin(
        &lease,
        &namespace,
        &scope,
        "key-a",
        &execution,
        &binding,
        grant,
        "principal-a",
        &mut current,
    )
    .unwrap();
    let BoundDeployBridgeInspection::Completed { run_id, .. } = first else {
        panic!("first did not finish")
    };
    std::fs::remove_file(execution.project_root.join(format!(
        ".vibe/deploy-operations/{run_id}.bound-vibe-bin-result.json"
    )))
    .unwrap();
    let second_grant = admitted(&execution, "instance-partial", "principal-a");
    let mut never_call =
        || -> Result<Option<String>, DeployError> { panic!("duplicate reran provider") };
    let second = submit_host_admitted_bound_vibe_bin(
        &lease,
        &namespace,
        &scope,
        "key-a",
        &execution,
        &binding,
        second_grant,
        "principal-a",
        &mut never_call,
    )
    .unwrap();
    assert!(
        matches!(second, BoundDeployBridgeInspection::Uncertain { partial, .. }
        if partial.receipt == "present" && partial.sidecar == "present")
    );
    std::fs::remove_file(execution.project_root.join(format!(
        ".vibe/deploy-operations/{run_id}.bound-vibe-bin-bridge.json"
    )))
    .unwrap();
    let third_grant = admitted(&execution, "instance-partial", "principal-a");
    let third = submit_host_admitted_bound_vibe_bin(
        &lease,
        &namespace,
        &scope,
        "key-a",
        &execution,
        &binding,
        third_grant,
        "principal-a",
        &mut never_call,
    )
    .unwrap();
    assert!(matches!(
        third,
        BoundDeployBridgeInspection::RecoveryRequired { .. }
    ));
    let third_grant = admitted(&execution, "instance-partial", "principal-a");
    let conflict = submit_host_admitted_bound_vibe_bin(
        &lease,
        &namespace,
        &scope,
        "key-a",
        &execution,
        &binding,
        third_grant,
        "different-principal",
        &mut never_call,
    );
    assert!(conflict.is_err());
}

#[test]
fn protected_bound_deploy_changed_request_conflicts_before_provider() {
    let fixture = Fixture::new("protected-bytes");
    let targets = [builtin_target()];
    let selection = selection("local", &["local-helper"]);
    let state = fixture.state_home();
    let execution = fixture.execution(&targets, &selection, &state);
    let lease = LifecycleLease::acquire(execution.project_root).unwrap();
    let scope = SubmissionScope {
        context: "host-test-context".into(),
        target: "instance-conflict".into(),
        principal: "principal-a".into(),
    };
    let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
    let grant = admitted(&execution, "instance-conflict", "principal-a");
    let binding = grant.binding().clone();
    let mut current = || Ok(Some("generation-a".to_owned()));
    assert!(matches!(
        submit_host_admitted_bound_vibe_bin(
            &lease,
            &namespace,
            &scope,
            "key-a",
            &execution,
            &binding,
            grant,
            "principal-a",
            &mut current
        )
        .unwrap(),
        BoundDeployBridgeInspection::Completed { .. }
    ));
    let mut changed_targets = targets.clone();
    changed_targets[0].artifact = "other-artifact".into();
    let changed = super::DeployExecution {
        targets: &changed_targets,
        ..execution
    };
    let changed_grant = admitted(&changed, "instance-conflict", "principal-a");
    let mut never_call =
        || -> Result<Option<String>, DeployError> { panic!("conflict reached provider") };
    assert!(matches!(
        submit_host_admitted_bound_vibe_bin(
            &lease,
            &namespace,
            &scope,
            "key-a",
            &changed,
            &binding,
            changed_grant,
            "principal-a",
            &mut never_call
        ),
        Err(BoundDeployBridgeError::Submission(
            crate::SubmissionError::KeyConflict
        ))
    ));
}

#[test]
fn bound_sidecar_copied_to_another_binding_is_refused_by_no_create_view() {
    let a = Fixture::new("same-package-bytes");
    let b = Fixture::new("same-package-bytes");
    let targets = [builtin_target()];
    let selection = selection("local", &["local-helper"]);
    let a_state = a.state_home();
    let b_state = b.state_home();
    let a_execution = a.execution(&targets, &selection, &a_state);
    let b_execution = b.execution(&targets, &selection, &b_state);
    let a_grant = admitted(&a_execution, "instance-a", "principal-a");
    let a_binding = a_grant.binding().clone();
    let b_binding = admitted(&b_execution, "instance-b", "principal-a")
        .binding()
        .clone();
    let mut current = || Ok(Some("generation-a".to_owned()));
    execute_host_admitted_bound_vibe_bin(
        &a_execution,
        &a_binding,
        a_grant,
        "principal-a",
        &mut current,
    )
    .unwrap();
    let a_home = DeploymentHome::new_bound(
        a_execution.state_home,
        a_execution.project,
        a_execution.package,
        "local-helper",
        &a_binding,
    )
    .unwrap();
    let b_home = DeploymentHome::new_bound(
        b_execution.state_home,
        b_execution.project,
        b_execution.package,
        "local-helper",
        &b_binding,
    )
    .unwrap();
    std::fs::create_dir_all(b_home.directory()).unwrap();
    std::fs::copy(
        a_home.directory().join("lock-resources.json"),
        b_home.directory().join("lock-resources.json"),
    )
    .unwrap();
    let settings = vibe_safefs::Project::open(b_execution.settings_root).unwrap();
    let view = DeployStateView::open_bound(&settings).unwrap();
    assert!(matches!(
        view.read_lock_resources(&b_home),
        Err(DeployError::BindingMismatch { .. })
    ));
}

#[test]
fn two_admitted_homes_publish_physical_builtin_and_distinct_bound_receipts() {
    let a = Fixture::new("same-package-bytes");
    let b = Fixture::new("same-package-bytes");
    let targets = [builtin_target()];
    let selection = selection("local", &["local-helper"]);
    let a_state = a.state_home();
    let b_state = b.state_home();
    let a_execution = a.execution(&targets, &selection, &a_state);
    let b_execution = b.execution(&targets, &selection, &b_state);
    let a_admission = admitted(&a_execution, "instance-a", "principal-a");
    let b_admission = admitted(&b_execution, "instance-b", "principal-b");
    let a_binding = a_admission.binding().clone();
    let b_binding = b_admission.binding().clone();
    assert_ne!(a_binding.binding_id, b_binding.binding_id);
    let mut current_a = || Ok(Some("generation-a".to_owned()));
    let mut current_b = || Ok(Some("generation-a".to_owned()));
    let first = execute_host_admitted_bound_vibe_bin(
        &a_execution,
        &a_binding,
        a_admission,
        "principal-a",
        &mut current_a,
    )
    .unwrap();
    let second = execute_host_admitted_bound_vibe_bin(
        &b_execution,
        &b_binding,
        b_admission,
        "principal-b",
        &mut current_b,
    )
    .unwrap();
    assert_eq!(first.len(), 1);
    assert_eq!(second.len(), 1);
    assert_eq!(first[0].generation, 0);
    assert_eq!(second[0].generation, 0);
    assert!(!first[0].resources.is_empty());
    assert_eq!(first[0].resources, second[0].resources);
    for (execution, binding) in [(&a_execution, &a_binding), (&b_execution, &b_binding)] {
        let state =
            DeployState::open_bound(&vibe_safefs::Project::open(execution.settings_root).unwrap())
                .unwrap();
        let home = DeploymentHome::new_bound(
            execution.state_home,
            execution.project,
            execution.package,
            "local-helper",
            binding,
        )
        .unwrap();
        let receipt = state.read_receipt(&home).unwrap().unwrap();
        assert_eq!(receipt.operation_binding.as_ref(), Some(binding));
        assert_eq!(receipt.generation, 0);
        assert_eq!(receipt.resources.len(), 2);
        assert!(state.read_intent(&home).unwrap().is_none());
        assert!(execution.settings_root.join("bin").is_dir());
        assert!(execution.settings_root.join("store").is_dir());
        assert!(
            std::fs::read_dir(execution.settings_root.join("bin"))
                .unwrap()
                .count()
                >= 2
        );
        assert!(
            std::fs::read_dir(execution.settings_root.join("store"))
                .unwrap()
                .count()
                >= 1
        );
    }
}

#[test]
fn changed_body_or_stale_host_generation_refuses_before_effect() {
    let fixture = Fixture::new("same-package-bytes");
    let alternate = Fixture::new("same-package-bytes");
    let targets = [builtin_target()];
    let selection = selection("local", &["local-helper"]);
    let state = fixture.state_home();
    let execution = fixture.execution(&targets, &selection, &state);
    let alternate_settings = alternate.settings.path();
    let changed = super::DeployExecution {
        settings_root: alternate_settings,
        ..execution
    };
    let grant = admitted(&execution, "instance-a", "principal-a");
    let binding = grant.binding().clone();
    let mut current = || Ok(Some("generation-a".to_owned()));
    assert!(matches!(
        execute_host_admitted_bound_vibe_bin(
            &changed,
            &binding,
            grant,
            "principal-a",
            &mut current,
        ),
        Err(DeployError::BoundGrantChanged { .. })
    ));
    assert!(!execution.settings_root.join("bin").exists());
    assert!(!alternate_settings.join("bin").exists());
    assert!(!execution.settings_root.join(".vibe").exists());

    let grant = admitted(&execution, "instance-a", "principal-a");
    let mut stale = || Ok(Some("generation-b".to_owned()));
    assert!(matches!(
        execute_host_admitted_bound_vibe_bin(
            &execution,
            &binding,
            grant,
            "principal-a",
            &mut stale,
        ),
        Err(DeployError::BoundGrantChanged { .. })
    ));
    assert!(!execution.settings_root.join(".vibe").exists());
}

#[test]
fn generation_drift_after_publication_retains_bound_intent_and_reports_uncertain() {
    let fixture = Fixture::new("same-package-bytes");
    let targets = [builtin_target()];
    let selection = selection("local", &["local-helper"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &selection, &state_home);
    let grant = admitted(&execution, "instance-a", "principal-a");
    let binding = grant.binding().clone();
    let mut observations = 0;
    let mut current = || {
        observations += 1;
        Ok(Some(if observations >= 6 {
            "generation-b".to_owned()
        } else {
            "generation-a".to_owned()
        }))
    };
    assert!(matches!(
        execute_host_admitted_bound_vibe_bin(
            &execution,
            &binding,
            grant,
            "principal-a",
            &mut current,
        ),
        Err(DeployError::BoundPublicationUncertain { .. })
    ));
    assert!(observations >= 6);
    let settings = vibe_safefs::Project::open(execution.settings_root).unwrap();
    let state = DeployState::open_bound(&settings).unwrap();
    let home = DeploymentHome::new_bound(
        execution.state_home,
        execution.project,
        execution.package,
        "local-helper",
        &binding,
    )
    .unwrap();
    let intent = state.read_intent(&home).unwrap().unwrap();
    assert_eq!(intent.operation_binding.as_ref(), Some(&binding));
    assert!(state.read_receipt(&home).unwrap().is_none());
    assert!(
        std::fs::read_dir(execution.settings_root.join("bin"))
            .unwrap()
            .count()
            >= 2
    );
    assert!(
        std::fs::read_dir(execution.settings_root.join("store"))
            .unwrap()
            .count()
            >= 1
    );
}
