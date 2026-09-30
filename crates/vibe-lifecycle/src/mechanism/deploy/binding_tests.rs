//! Host-issued admission over disposable roots; no provider effect is enabled.

use super::binding::{AdmittedLocalDeployBinding, LocalAdmissionError, TrustedLocalDeployFacts};
use super::model::DeployExecution;
use super::support::{Fixture, selection, target};
use crate::mechanism::vibebin::bound::BoundVibeBinError;
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

fn facts<'a>(
    execution: &DeployExecution<'a>,
    instance: &'a str,
    nonce: &'a str,
    principal: &'a str,
    mode: Mode,
) -> TrustedLocalDeployFacts<'a> {
    TrustedLocalDeployFacts {
        host_context_id: "host-test-context",
        instance_ref: instance,
        selected_generation_ref: Some("generation-a"),
        principal_ref: principal,
        admission_epoch: 1,
        binding_nonce: nonce,
        allow_alias_resolution: false,
        target_mode: mode,
        target_role: Role::ProducedToolRuntime,
        target_os: local_os(),
        target_architecture: std::env::consts::ARCH,
        target_abi: None,
        target_libc: None,
        project_root: execution.project_root,
        settings_root: execution.settings_root,
        user_home: execution.user_home,
    }
}

#[test]
#[cfg(unix)]
fn trusted_alias_needs_explicit_opt_in_and_reports_weaker_observation() {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new("same-package-bytes");
    let targets = [target("local-helper", "helper.exe", &[])];
    let selection = selection("local", &["local-helper"]);
    let state = fixture.state_home();
    let execution = fixture.execution(&targets, &selection, &state);
    let alias_dir = tempfile::tempdir().unwrap();
    let alias = alias_dir.path().join("settings-alias");
    symlink(execution.settings_root, &alias).unwrap();
    let mut selected = facts(
        &execution,
        "instance-a",
        "nonce-a",
        "principal-a",
        Mode::UserProfile,
    );
    selected.settings_root = &alias;
    assert!(matches!(
        AdmittedLocalDeployBinding::issue(selected),
        Err(LocalAdmissionError::RootUnavailable {
            role: "settings",
            ..
        })
    ));
    let mut selected = facts(
        &execution,
        "instance-a",
        "nonce-a",
        "principal-a",
        Mode::UserProfile,
    );
    selected.settings_root = &alias;
    selected.allow_alias_resolution = true;
    let admitted = AdmittedLocalDeployBinding::issue(selected).unwrap();
    assert_eq!(admitted.alias_warnings().len(), 1);
    admitted.reobserve_roots().unwrap();
    let traversal = alias_dir.path().join("nested/../settings-alias");
    let mut malformed = facts(
        &execution,
        "instance-a",
        "nonce-a",
        "principal-a",
        Mode::UserProfile,
    );
    malformed.settings_root = &traversal;
    malformed.allow_alias_resolution = true;
    assert!(matches!(
        AdmittedLocalDeployBinding::issue(malformed),
        Err(LocalAdmissionError::RootUnavailable {
            role: "settings",
            ..
        })
    ));
    let other = tempfile::tempdir().unwrap();
    std::fs::remove_file(&alias).unwrap();
    symlink(other.path(), &alias).unwrap();
    assert!(matches!(
        admitted.reobserve_roots(),
        Err(LocalAdmissionError::RootChanged { role: "settings" })
    ));
}

#[test]
fn disjoint_trusted_local_homes_get_distinct_binding_and_lock() {
    let a = Fixture::new("same-package-bytes");
    let b = Fixture::new("same-package-bytes");
    let targets = [target("local-helper", "helper.exe", &[])];
    let selection = selection("local", &["local-helper"]);
    let a_state = a.state_home();
    let b_state = b.state_home();
    let a_execution = a.execution(&targets, &selection, &a_state);
    let b_execution = b.execution(&targets, &selection, &b_state);
    let admitted_a = AdmittedLocalDeployBinding::issue(facts(
        &a_execution,
        "instance-a",
        "nonce-a",
        "principal-a",
        Mode::UserProfile,
    ))
    .unwrap();
    let admitted_b = AdmittedLocalDeployBinding::issue(facts(
        &b_execution,
        "instance-b",
        "nonce-b",
        "principal-b",
        Mode::UserProfile,
    ))
    .unwrap();
    admitted_a
        .preflight(&a_execution, admitted_a.binding(), "principal-a")
        .unwrap();
    admitted_b
        .preflight(&b_execution, admitted_b.binding(), "principal-b")
        .unwrap();
    assert_ne!(
        admitted_a.binding().binding_id,
        admitted_b.binding().binding_id
    );
    let locked_a = admitted_a.acquire_destination().unwrap();
    let locked_b = admitted_b.acquire_destination().unwrap();
    locked_a.reobserve_before_commit().unwrap();
    locked_b.reobserve_before_commit().unwrap();
    assert_ne!(
        locked_a.settings().root_identity().unwrap(),
        locked_b.settings().root_identity().unwrap()
    );
}

#[test]
fn same_physical_settings_root_refuses_second_participating_writer() {
    let fixture = Fixture::new("same-package-bytes");
    let targets = [target("local-helper", "helper.exe", &[])];
    let selection = selection("local", &["local-helper"]);
    let state = fixture.state_home();
    let execution = fixture.execution(&targets, &selection, &state);
    let first = AdmittedLocalDeployBinding::issue(facts(
        &execution,
        "instance-a",
        "nonce-a",
        "principal-a",
        Mode::UserProfile,
    ))
    .unwrap();
    let second = AdmittedLocalDeployBinding::issue(facts(
        &execution,
        "instance-b",
        "nonce-b",
        "principal-a",
        Mode::UserProfile,
    ))
    .unwrap();
    let held = first.acquire_destination().unwrap();
    assert!(matches!(
        second.acquire_destination(),
        Err(LocalAdmissionError::Destination(
            BoundVibeBinError::LockBusy
        ))
    ));
    held.reobserve_before_commit().unwrap();
}

#[test]
fn changed_body_principal_or_mode_refuses_before_destination_lock() {
    let fixture = Fixture::new("same-package-bytes");
    let targets = [target("local-helper", "helper.exe", &[])];
    let selection = selection("local", &["local-helper"]);
    let state = fixture.state_home();
    let execution = fixture.execution(&targets, &selection, &state);
    assert!(matches!(
        AdmittedLocalDeployBinding::issue(facts(
            &execution,
            "instance-a",
            "nonce-a",
            "principal-a",
            Mode::WorkspaceProjection
        )),
        Err(LocalAdmissionError::UnsupportedMode)
    ));
    let admitted = AdmittedLocalDeployBinding::issue(facts(
        &execution,
        "instance-a",
        "nonce-a",
        "principal-a",
        Mode::UserProfile,
    ))
    .unwrap();
    assert!(matches!(
        admitted.preflight(&execution, admitted.binding(), "other-principal"),
        Err(LocalAdmissionError::ExecutionMismatch { field: "principal" })
    ));
    let other = tempfile::tempdir().unwrap();
    let changed = DeployExecution {
        settings_root: other.path(),
        ..execution
    };
    assert!(matches!(
        admitted.preflight(&changed, admitted.binding(), "principal-a"),
        Err(LocalAdmissionError::ExecutionMismatch {
            field: "settings_root"
        })
    ));
    let mut changed_binding = admitted.binding().clone();
    changed_binding.target_mode = Mode::WorkspaceProjection;
    assert!(matches!(
        admitted.preflight(&execution, &changed_binding, "principal-a"),
        Err(LocalAdmissionError::BindingMismatch)
    ));
}
