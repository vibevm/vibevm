//! Interrupted and copied-evidence cases for the protected bound deploy bridge.

use super::DeployError;
use super::binding::{AdmittedLocalDeployBinding, TrustedLocalDeployFacts};
use super::bridge::{
    BoundDeployBridgeInspection, canonical_request, submit_host_admitted_bound_vibe_bin,
};
use super::state::{DeployState, DeploymentHome};
use super::support::{Fixture, selection};
use crate::process::propose_run_id;
use crate::{
    JournalAdmission, LifecycleLease, ReservationDecision, SubmissionReservations, SubmissionScope,
};
use vibe_wire::generated::shared::{
    EnvironmentOperationBindingTargetMode as Mode, EnvironmentOperationBindingTargetOs as Os,
    EnvironmentOperationBindingTargetRole as Role,
};

fn local_os() -> Os {
    match std::env::consts::OS {
        "linux" => Os::Linux,
        "macos" => Os::Macos,
        "windows" => Os::Windows,
        other => panic!("unsupported test host {other}"),
    }
}

fn admitted<'a>(
    execution: &super::DeployExecution<'a>,
    instance: &'a str,
) -> AdmittedLocalDeployBinding {
    AdmittedLocalDeployBinding::issue(TrustedLocalDeployFacts {
        host_context_id: "host-test-context",
        instance_ref: instance,
        selected_generation_ref: Some("generation-a"),
        principal_ref: "principal-a",
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

fn target() -> vibe_core::manifest::DeployTarget {
    crate::mechanism::vibebin::support::target(
        "local-helper",
        "helper.exe",
        Some("command = \"local-helper\""),
    )
}

fn scope(instance: &str) -> SubmissionScope {
    SubmissionScope {
        context: "host-test-context".into(),
        target: instance.into(),
        principal: "principal-a".into(),
    }
}

#[test]
fn protected_interruption_after_first_effect_keeps_intent_and_never_reapplies() {
    let fixture = Fixture::new("interrupted-bytes");
    let targets = [target()];
    let selection = selection("local", &["local-helper"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &selection, &state_home);
    let lease = LifecycleLease::acquire(execution.project_root).unwrap();
    let scope = scope("interrupted-instance");
    let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
    let grant = admitted(&execution, "interrupted-instance");
    let binding = grant.binding().clone();
    let mut observations = 0;
    let mut drift_after_effect = || {
        observations += 1;
        Ok(Some(
            if observations >= 6 {
                "generation-b"
            } else {
                "generation-a"
            }
            .to_owned(),
        ))
    };
    let first = submit_host_admitted_bound_vibe_bin(
        &lease,
        &namespace,
        &scope,
        "key-a",
        &execution,
        &binding,
        grant,
        "principal-a",
        &mut drift_after_effect,
    )
    .unwrap();
    let BoundDeployBridgeInspection::Uncertain { run_id, partial } = first else {
        panic!("interrupted run claimed completion");
    };
    assert!(observations >= 6);
    assert_eq!(partial.intent, "present");
    assert_eq!(partial.receipt, "absent");
    assert_eq!(partial.sidecar, "present");
    assert_eq!(
        SubmissionReservations::journal_admission(&lease, &namespace, &scope, "key-a", &run_id)
            .unwrap(),
        JournalAdmission::Ready
    );
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
    assert!(state.read_intent(&home).unwrap().is_some());
    let sidecar = state.read_lock_resources(&home).unwrap().unwrap();
    assert!(sidecar.pending.is_some() && sidecar.committed.is_none());
    assert!(state.read_receipt(&home).unwrap().is_none());
    assert!(
        std::fs::read_dir(execution.settings_root.join("bin"))
            .unwrap()
            .count()
            >= 2
    );
    let second_grant = admitted(&execution, "interrupted-instance");
    let mut no_second_effect = || -> Result<Option<String>, DeployError> {
        panic!("duplicate invoked generation/provider")
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
        &mut no_second_effect,
    )
    .unwrap();
    assert!(
        matches!(second, BoundDeployBridgeInspection::Uncertain { run_id: observed, .. } if observed == run_id)
    );
}

#[test]
fn pending_c1_reservation_without_bridge_never_acquires_destination() {
    let fixture = Fixture::new("pending-bytes");
    let targets = [target()];
    let selection = selection("local", &["local-helper"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &selection, &state_home);
    let lease = LifecycleLease::acquire(execution.project_root).unwrap();
    let scope = scope("pending-instance");
    let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
    let grant = admitted(&execution, "pending-instance");
    let binding = grant.binding().clone();
    let request = canonical_request(&execution, &binding, "principal-a").unwrap();
    let run_id = propose_run_id(lease.root()).unwrap();
    assert!(matches!(
        SubmissionReservations::reserve(
            &lease,
            &namespace,
            &scope,
            "key-a",
            &request,
            b"pending plan",
            &run_id
        )
        .unwrap(),
        ReservationDecision::Created { .. }
    ));
    assert_eq!(
        SubmissionReservations::journal_admission(&lease, &namespace, &scope, "key-a", &run_id)
            .unwrap(),
        JournalAdmission::Pending
    );
    let mut no_effect = || -> Result<Option<String>, DeployError> {
        panic!("pending reservation entered provider")
    };
    let observed = submit_host_admitted_bound_vibe_bin(
        &lease,
        &namespace,
        &scope,
        "key-a",
        &execution,
        &binding,
        grant,
        "principal-a",
        &mut no_effect,
    )
    .unwrap();
    assert!(
        matches!(observed, BoundDeployBridgeInspection::RecoveryRequired { run_id: observed } if observed == run_id)
    );
    assert!(!execution.settings_root.join(".vibe").exists());
    assert!(!execution.settings_root.join("bin").exists());
    assert!(!execution.settings_root.join("store").exists());
}

#[test]
fn copied_receipt_sidecar_and_result_cannot_complete_another_bound_home() {
    let a = Fixture::new("same-bytes");
    let b = Fixture::new("same-bytes");
    let targets = [target()];
    let selection = selection("local", &["local-helper"]);
    let a_state = a.state_home();
    let b_state = b.state_home();
    let a_execution = a.execution(&targets, &selection, &a_state);
    let b_execution = b.execution(&targets, &selection, &b_state);
    let a_lease = LifecycleLease::acquire(a_execution.project_root).unwrap();
    let b_lease = LifecycleLease::acquire(b_execution.project_root).unwrap();
    let a_scope = scope("instance-a");
    let b_scope = scope("instance-b");
    let a_namespace = SubmissionReservations::issue_namespace(&a_lease, &a_scope, 60_000).unwrap();
    let b_namespace = SubmissionReservations::issue_namespace(&b_lease, &b_scope, 60_000).unwrap();
    let a_grant = admitted(&a_execution, "instance-a");
    let b_grant = admitted(&b_execution, "instance-b");
    let a_binding = a_grant.binding().clone();
    let b_binding = b_grant.binding().clone();
    let mut current_a = || Ok(Some("generation-a".to_owned()));
    let mut current_b = || Ok(Some("generation-a".to_owned()));
    let a_result = submit_host_admitted_bound_vibe_bin(
        &a_lease,
        &a_namespace,
        &a_scope,
        "key-a",
        &a_execution,
        &a_binding,
        a_grant,
        "principal-a",
        &mut current_a,
    )
    .unwrap();
    let b_result = submit_host_admitted_bound_vibe_bin(
        &b_lease,
        &b_namespace,
        &b_scope,
        "key-b",
        &b_execution,
        &b_binding,
        b_grant,
        "principal-a",
        &mut current_b,
    )
    .unwrap();
    let BoundDeployBridgeInspection::Completed { run_id: a_run, .. } = a_result else {
        panic!("a incomplete")
    };
    let BoundDeployBridgeInspection::Completed { run_id: b_run, .. } = b_result else {
        panic!("b incomplete")
    };
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
    for name in ["receipt.json", "lock-resources.json"] {
        std::fs::copy(a_home.directory().join(name), b_home.directory().join(name)).unwrap();
    }
    let replay = || admitted(&b_execution, "instance-b");
    let mut no_second_effect =
        || -> Result<Option<String>, DeployError> { panic!("copy caused second provider call") };
    let first_copy = submit_host_admitted_bound_vibe_bin(
        &b_lease,
        &b_namespace,
        &b_scope,
        "key-b",
        &b_execution,
        &b_binding,
        replay(),
        "principal-a",
        &mut no_second_effect,
    )
    .unwrap();
    assert!(matches!(
        first_copy,
        BoundDeployBridgeInspection::Uncertain { .. }
    ));
    std::fs::copy(
        a_execution.project_root.join(format!(
            ".vibe/deploy-operations/{a_run}.bound-vibe-bin-result.json"
        )),
        b_execution.project_root.join(format!(
            ".vibe/deploy-operations/{b_run}.bound-vibe-bin-result.json"
        )),
    )
    .unwrap();
    let all_copied = submit_host_admitted_bound_vibe_bin(
        &b_lease,
        &b_namespace,
        &b_scope,
        "key-b",
        &b_execution,
        &b_binding,
        replay(),
        "principal-a",
        &mut no_second_effect,
    )
    .unwrap();
    assert!(matches!(
        all_copied,
        BoundDeployBridgeInspection::Uncertain { .. }
    ));
}
