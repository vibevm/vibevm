//! The first host-admitted builtin effect stays inside disposable worlds.

use super::binding::{AdmittedLocalDeployBinding, TrustedLocalDeployFacts};
use super::state::{DeployState, DeploymentHome};
use super::support::{Fixture, selection};
use super::{DeployError, execute_host_admitted_bound_vibe_bin};
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
