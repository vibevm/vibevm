//! Interrupted and copied-evidence cases for the protected bound deploy bridge.

use super::DeployError;
use super::binding::{AdmittedLocalDeployBinding, TrustedLocalDeployFacts};
use super::bridge::{
    BoundDeployBridgeInspection, canonical_request, inspect_host_admitted_bound_vibe_bin_cold,
    submit_host_admitted_bound_vibe_bin,
};
use super::state::{DeployState, DeploymentHome};
use super::support::{Fixture, selection};
use crate::process::propose_run_id;
use crate::retained_mechanism::{
    DeployColdHostFacts, DeployInspection, FixedDeployHostAdmission, submit_fixed_bound_vibe_bin,
};
use crate::{
    JournalAdmission, LifecycleLease, ReservationDecision, SubmissionReservations, SubmissionScope,
};
use vibe_core::manifest::{DeployTarget, Manifest};
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
    reissued(
        execution.project_root,
        execution.settings_root,
        execution.user_home,
        instance,
    )
}

fn reissued(
    project_root: &std::path::Path,
    settings_root: &std::path::Path,
    user_home: &std::path::Path,
    instance: &str,
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
        project_root,
        settings_root,
        user_home,
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

fn authored_targets(fixture: &Fixture) -> Vec<DeployTarget> {
    let source = concat!(
        "[project]\nname = 'demo'\nversion = '1.1.0'\n\n",
        "[[artifacts.build]]\nid = 'helper'\nmechanism = 'build:cargo'\n",
        "outputs = [{ id = 'helper.exe', kind = 'executable' }]\n\n",
        "[[deploy.target]]\nid = 'local-helper'\nartifact = 'helper.exe'\n",
        "mechanism = 'deploy:vibe-bin'\ndepends_on = []\n",
        "config = { command = 'local-helper' }\n\n",
        "[deploy.profiles.local]\ntargets = ['local-helper']\n",
    );
    std::fs::write(fixture.project.path().join("vibe.toml"), source).unwrap();
    Manifest::parse_str(source).unwrap().deploy.unwrap().targets
}

fn public_facts<'a>(
    execution: &super::DeployExecution<'a>,
    instance: &'a str,
) -> DeployColdHostFacts<'a> {
    DeployColdHostFacts {
        host_context_id: "host-test-context",
        instance_ref: instance,
        selected_generation_ref: Some("generation-a"),
        principal_ref: "principal-a",
        admission_epoch: 1,
        binding_nonce: instance,
        allow_alias_resolution: false,
        target_abi: None,
        target_libc: None,
        project_root: execution.project_root,
        settings_root: execution.settings_root,
        user_home: execution.user_home,
    }
}

#[test]
fn public_authored_fixed_deploy_submit_and_replay_keep_one_owner_run() {
    let fixture = Fixture::new("public-bound-bytes");
    let targets = authored_targets(&fixture);
    let selected = selection("local", &["local-helper"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &selected, &state_home);
    let lease = LifecycleLease::acquire(execution.project_root).unwrap();
    let trusted = scope("public-instance");
    let namespace = SubmissionReservations::issue_namespace(&lease, &trusted, 60_000).unwrap();
    let mut generation = || Ok(Some("generation-a".to_owned()));
    let permit = FixedDeployHostAdmission::issue_from_trusted_host(
        &lease,
        &namespace,
        &trusted,
        "same-key",
        &execution,
        public_facts(&execution, "public-instance"),
        &mut generation,
    )
    .unwrap();
    let first = submit_fixed_bound_vibe_bin(permit, "same-key", &mut generation).unwrap();
    let DeployInspection::Completed { run_id, result } = &first else {
        panic!("fixed authored deployment did not complete: {first:?}")
    };
    assert_eq!(result.receipt.profile, "local");
    let record = execution.project_root.join(format!(
        ".vibe/deploy-operations/{run_id}.bound-vibe-bin-result.json"
    ));
    let retained = std::fs::read(&record).unwrap();
    let c1 = SubmissionReservations::inspect_existing_key(&lease, &namespace, &trusted, "same-key")
        .unwrap()
        .unwrap();
    assert_eq!(&c1.domain_run_id, run_id);
    assert_eq!(c1.journal_admission, JournalAdmission::Ready);
    let mut generation = || Ok(Some("generation-a".to_owned()));
    let replay = FixedDeployHostAdmission::issue_from_trusted_host(
        &lease,
        &namespace,
        &trusted,
        "same-key",
        &execution,
        public_facts(&execution, "public-instance"),
        &mut generation,
    )
    .unwrap();
    let mut no_second_effect = || -> Result<Option<String>, DeployError> {
        panic!("same-key replay invoked generation or provider")
    };
    assert_eq!(
        submit_fixed_bound_vibe_bin(replay, "same-key", &mut no_second_effect).unwrap(),
        first
    );
    assert_eq!(std::fs::read(&record).unwrap(), retained);
    assert_eq!(
        SubmissionReservations::inspect_existing_key(&lease, &namespace, &trusted, "same-key")
            .unwrap()
            .unwrap()
            .domain_run_id,
        *run_id
    );
    let index: serde_json::Value = serde_json::from_slice(
        &std::fs::read(
            execution
                .project_root
                .join(".vibe/machine-submissions.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(index["namespaces"].as_object().unwrap().len(), 1);
    SubmissionReservations::retire_namespace(&lease, &namespace, &trusted).unwrap();
    let unreserved = FixedDeployHostAdmission::issue_from_trusted_host(
        &lease,
        &namespace,
        &trusted,
        "unreserved",
        &execution,
        public_facts(&execution, "public-instance"),
        &mut generation,
    )
    .unwrap();
    assert_eq!(
        submit_fixed_bound_vibe_bin(unreserved, "unreserved", &mut no_second_effect).unwrap(),
        DeployInspection::Retired { run_id: None },
    );
    std::fs::write(
        execution.project_root.join("vibe.toml"),
        b"new authored bytes",
    )
    .unwrap();
    drop(lease);
    let cold_lease = LifecycleLease::acquire(execution.project_root).unwrap();
    let mut no_new_generation = || -> Result<Option<String>, DeployError> {
        panic!("reserved old key reread current generation")
    };
    let third = FixedDeployHostAdmission::issue_from_trusted_host(
        &cold_lease,
        &namespace,
        &trusted,
        "same-key",
        &execution,
        public_facts(&execution, "public-instance"),
        &mut no_new_generation,
    )
    .unwrap();
    assert_eq!(
        submit_fixed_bound_vibe_bin(third, "same-key", &mut no_second_effect).unwrap(),
        first,
        "old key must attach to retained capsule after current manifest drift"
    );
    assert_eq!(std::fs::read(&record).unwrap(), retained);
}

#[test]
fn public_authored_deploy_refuses_profile_scope_generation_and_source_drift_before_effect() {
    let fixture = Fixture::new("public-refusal-bytes");
    let targets = authored_targets(&fixture);
    let selected = selection("local", &["local-helper"]);
    let foreign_profile = selection("foreign", &["local-helper"]);
    let foreign_target = selection("local", &["other-target"]);
    let mut altered_targets = targets.clone();
    altered_targets[0].config = None;
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &selected, &state_home);
    let bad_execution = fixture.execution(&targets, &foreign_profile, &state_home);
    let bad_target_execution = fixture.execution(&targets, &foreign_target, &state_home);
    let bad_config_execution = fixture.execution(&altered_targets, &selected, &state_home);
    let lease = LifecycleLease::acquire(execution.project_root).unwrap();
    let trusted = scope("refusal-instance");
    let namespace = SubmissionReservations::issue_namespace(&lease, &trusted, 60_000).unwrap();
    let mut generation = || Ok(Some("generation-a".to_owned()));
    assert!(
        FixedDeployHostAdmission::issue_from_trusted_host(
            &lease,
            &namespace,
            &trusted,
            "new",
            &bad_execution,
            public_facts(&bad_execution, "refusal-instance"),
            &mut generation,
        )
        .is_err()
    );
    assert!(
        FixedDeployHostAdmission::issue_from_trusted_host(
            &lease,
            &namespace,
            &trusted,
            "new",
            &bad_target_execution,
            public_facts(&bad_target_execution, "refusal-instance"),
            &mut generation,
        )
        .is_err()
    );
    assert!(
        FixedDeployHostAdmission::issue_from_trusted_host(
            &lease,
            &namespace,
            &trusted,
            "new",
            &bad_config_execution,
            public_facts(&bad_config_execution, "refusal-instance"),
            &mut generation,
        )
        .is_err()
    );
    let foreign = scope("other-instance");
    assert!(matches!(
        FixedDeployHostAdmission::issue_from_trusted_host(
            &lease,
            &namespace,
            &foreign,
            "new",
            &execution,
            public_facts(&execution, "other-instance"),
            &mut generation,
        ),
        Err(
            crate::retained_mechanism::RetainedMechanismError::Submission(
                crate::SubmissionError::ForeignScope
            )
        )
    ));
    let mut stale_generation = || Ok(Some("generation-b".to_owned()));
    assert!(
        FixedDeployHostAdmission::issue_from_trusted_host(
            &lease,
            &namespace,
            &trusted,
            "new",
            &execution,
            public_facts(&execution, "refusal-instance"),
            &mut stale_generation,
        )
        .is_err()
    );
    let mut wrong_root = public_facts(&execution, "refusal-instance");
    wrong_root.settings_root = fixture.destination.path();
    assert!(
        FixedDeployHostAdmission::issue_from_trusted_host(
            &lease,
            &namespace,
            &trusted,
            "new",
            &execution,
            wrong_root,
            &mut generation,
        )
        .is_err()
    );
    let permit = FixedDeployHostAdmission::issue_from_trusted_host(
        &lease,
        &namespace,
        &trusted,
        "new",
        &execution,
        public_facts(&execution, "refusal-instance"),
        &mut generation,
    )
    .unwrap();
    std::fs::write(execution.project_root.join("vibe.toml"), b"changed source").unwrap();
    let refusal = submit_fixed_bound_vibe_bin(permit, "new", &mut generation).unwrap_err();
    assert!(!refusal.effects_may_have_begun);
    assert!(refusal.known_run_id.is_none() && refusal.owner_observation.is_none());
    assert!(
        SubmissionReservations::inspect_existing_key(&lease, &namespace, &trusted, "new")
            .unwrap()
            .is_none()
    );
    assert!(!execution.settings_root.join("bin").exists());
    assert!(
        !execution
            .project_root
            .join(".vibe/deploy-operations")
            .exists()
    );
}

#[test]
fn public_damaged_result_attaches_as_uncertain_without_redeploy() {
    let fixture = Fixture::new("public-damaged-bytes");
    let targets = authored_targets(&fixture);
    let selected = selection("local", &["local-helper"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &selected, &state_home);
    let lease = LifecycleLease::acquire(execution.project_root).unwrap();
    let trusted = scope("damage-instance");
    let namespace = SubmissionReservations::issue_namespace(&lease, &trusted, 60_000).unwrap();
    let mut generation = || Ok(Some("generation-a".to_owned()));
    let permit = FixedDeployHostAdmission::issue_from_trusted_host(
        &lease,
        &namespace,
        &trusted,
        "damaged",
        &execution,
        public_facts(&execution, "damage-instance"),
        &mut generation,
    )
    .unwrap();
    let first = submit_fixed_bound_vibe_bin(permit, "damaged", &mut generation).unwrap();
    let DeployInspection::Completed { run_id, .. } = first else {
        panic!("first deploy completed")
    };
    std::fs::remove_file(execution.project_root.join(format!(
        ".vibe/deploy-operations/{run_id}.bound-vibe-bin-result.json"
    )))
    .unwrap();
    let permit = FixedDeployHostAdmission::issue_from_trusted_host(
        &lease,
        &namespace,
        &trusted,
        "damaged",
        &execution,
        public_facts(&execution, "damage-instance"),
        &mut generation,
    )
    .unwrap();
    let mut no_effect =
        || -> Result<Option<String>, DeployError> { panic!("damaged key redeployed") };
    assert!(
        matches!(submit_fixed_bound_vibe_bin(permit, "damaged", &mut no_effect).unwrap(),
        DeployInspection::Uncertain { run_id: seen, .. } if seen == run_id)
    );
}

#[test]
fn public_post_ready_generation_drift_keeps_run_and_partial_effect_truth() {
    let fixture = Fixture::new("public-post-ready-bytes");
    let targets = authored_targets(&fixture);
    let selected = selection("local", &["local-helper"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &selected, &state_home);
    let lease = LifecycleLease::acquire(execution.project_root).unwrap();
    let trusted = scope("post-ready-instance");
    let namespace = SubmissionReservations::issue_namespace(&lease, &trusted, 60_000).unwrap();
    let mut observations = 0;
    let mut drift_after_effect = || {
        observations += 1;
        Ok(Some(
            if observations >= 7 {
                "generation-b"
            } else {
                "generation-a"
            }
            .to_owned(),
        ))
    };
    let permit = FixedDeployHostAdmission::issue_from_trusted_host(
        &lease,
        &namespace,
        &trusted,
        "interrupted",
        &execution,
        public_facts(&execution, "post-ready-instance"),
        &mut drift_after_effect,
    )
    .unwrap();
    let first =
        submit_fixed_bound_vibe_bin(permit, "interrupted", &mut drift_after_effect).unwrap();
    let DeployInspection::Uncertain { run_id, partial } = first else {
        panic!("post-Ready generation drift claimed completion")
    };
    assert!(observations >= 7);
    assert_eq!(partial.intent, "present");
    assert_eq!(partial.sidecar, "present");
    assert_eq!(
        SubmissionReservations::journal_admission(
            &lease,
            &namespace,
            &trusted,
            "interrupted",
            &run_id,
        )
        .unwrap(),
        JournalAdmission::Ready
    );
    let mut current = || Ok(Some("generation-a".to_owned()));
    let replay = FixedDeployHostAdmission::issue_from_trusted_host(
        &lease,
        &namespace,
        &trusted,
        "interrupted",
        &execution,
        public_facts(&execution, "post-ready-instance"),
        &mut current,
    )
    .unwrap();
    let mut no_second_effect = || -> Result<Option<String>, DeployError> {
        panic!("post-Ready replay invoked generation/provider")
    };
    assert!(
        matches!(submit_fixed_bound_vibe_bin(replay, "interrupted", &mut no_second_effect).unwrap(),
        DeployInspection::Uncertain { run_id: seen, .. } if seen == run_id)
    );
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

#[test]
fn cold_deploy_attach_recovers_completed_with_reissued_held_roots() {
    let fixture = Fixture::new("cold-completed-bytes");
    let trusted = scope("cold-instance");
    let (token, original, project, settings, user_home) = {
        let targets = [target()];
        let selection = selection("local", &["local-helper"]);
        let state_home = fixture.state_home();
        let execution = fixture.execution(&targets, &selection, &state_home);
        let lease = LifecycleLease::acquire(execution.project_root).unwrap();
        let namespace = SubmissionReservations::issue_namespace(&lease, &trusted, 60_000).unwrap();
        let grant = admitted(&execution, "cold-instance");
        let binding = grant.binding().clone();
        let mut current = || Ok(Some("generation-a".to_owned()));
        let result = submit_host_admitted_bound_vibe_bin(
            &lease,
            &namespace,
            &trusted,
            "key-a",
            &execution,
            &binding,
            grant,
            "principal-a",
            &mut current,
        )
        .unwrap();
        assert!(matches!(
            result,
            BoundDeployBridgeInspection::Completed { .. }
        ));
        (
            namespace.id().to_owned(),
            result,
            execution.project_root.to_path_buf(),
            execution.settings_root.to_path_buf(),
            execution.user_home.to_path_buf(),
        )
    };
    let lease = LifecycleLease::acquire(&project).unwrap();
    let grant = reissued(&project, &settings, &user_home, "cold-instance");
    let cold = inspect_host_admitted_bound_vibe_bin_cold(&lease, &token, &trusted, "key-a", &grant)
        .unwrap()
        .unwrap();
    let BoundDeployBridgeInspection::Completed {
        run_id,
        receipt,
        replayed,
    } = cold
    else {
        panic!("cold attach must retain verified completion")
    };
    assert!(replayed);
    let BoundDeployBridgeInspection::Completed {
        run_id: original_run,
        receipt: original_receipt,
        ..
    } = original
    else {
        unreachable!()
    };
    assert_eq!(run_id, original_run);
    assert_eq!(receipt, original_receipt);
    let foreign = scope("another-instance");
    assert!(
        inspect_host_admitted_bound_vibe_bin_cold(&lease, &token, &foreign, "key-a", &grant,)
            .is_err()
    );
    let capsule = project
        .join(".vibe/deploy-operations")
        .join(format!("{run_id}.bound-vibe-bin-input.json"));
    let original_capsule = std::fs::read(&capsule).unwrap();
    std::fs::write(&capsule, b"{\"tampered\":true}").unwrap();
    assert!(matches!(
        inspect_host_admitted_bound_vibe_bin_cold(&lease, &token, &trusted, "key-a", &grant)
            .unwrap()
            .unwrap(),
        BoundDeployBridgeInspection::Uncertain { run_id: seen, .. } if seen == run_id
    ));
    let mut changed: serde_json::Value = serde_json::from_slice(&original_capsule).unwrap();
    changed["request"] = serde_json::Value::String("{}".into());
    std::fs::write(&capsule, serde_json::to_vec(&changed).unwrap()).unwrap();
    assert!(matches!(
        inspect_host_admitted_bound_vibe_bin_cold(&lease, &token, &trusted, "key-a", &grant)
            .unwrap()
            .unwrap(),
        BoundDeployBridgeInspection::Uncertain { run_id: seen, .. } if seen == run_id
    ));
    let bin_entries = std::fs::read_dir(settings.join("bin")).unwrap().count();
    std::fs::remove_file(&capsule).unwrap();
    let missing =
        inspect_host_admitted_bound_vibe_bin_cold(&lease, &token, &trusted, "key-a", &grant)
            .unwrap()
            .unwrap();
    assert!(
        matches!(missing, BoundDeployBridgeInspection::Uncertain { run_id: seen, partial }
        if seen == run_id && partial.receipt == "present" && partial.sidecar == "present")
    );
    assert_eq!(
        std::fs::read_dir(settings.join("bin")).unwrap().count(),
        bin_entries
    );
}

#[test]
fn cold_deploy_attach_retains_interrupted_partial_without_provider() {
    let fixture = Fixture::new("cold-partial-bytes");
    let trusted = scope("cold-partial");
    let (token, original_run, project, settings, user_home) = {
        let targets = [target()];
        let selection = selection("local", &["local-helper"]);
        let state_home = fixture.state_home();
        let execution = fixture.execution(&targets, &selection, &state_home);
        let lease = LifecycleLease::acquire(execution.project_root).unwrap();
        let namespace = SubmissionReservations::issue_namespace(&lease, &trusted, 60_000).unwrap();
        let grant = admitted(&execution, "cold-partial");
        let binding = grant.binding().clone();
        let mut observations = 0;
        let mut drift = || {
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
            &trusted,
            "key-a",
            &execution,
            &binding,
            grant,
            "principal-a",
            &mut drift,
        )
        .unwrap();
        let BoundDeployBridgeInspection::Uncertain { run_id, partial } = first else {
            panic!("interrupted first run must be partial")
        };
        assert_eq!(partial.intent, "present");
        (
            namespace.id().to_owned(),
            run_id,
            execution.project_root.to_path_buf(),
            execution.settings_root.to_path_buf(),
            execution.user_home.to_path_buf(),
        )
    };
    let lease = LifecycleLease::acquire(&project).unwrap();
    let grant = reissued(&project, &settings, &user_home, "cold-partial");
    let cold = inspect_host_admitted_bound_vibe_bin_cold(&lease, &token, &trusted, "key-a", &grant)
        .unwrap()
        .unwrap();
    let BoundDeployBridgeInspection::Uncertain { run_id, partial } = cold else {
        panic!("cold attach must not replay interrupted deploy")
    };
    assert_eq!(run_id, original_run);
    assert_eq!(partial.intent, "present");
    assert_eq!(partial.receipt, "absent");
}

#[test]
fn cold_deploy_reissued_same_path_after_settings_replacement_refuses() {
    let fixture = Fixture::new("cold-root-bytes");
    let trusted = scope("cold-root");
    let (token, project, settings, user_home) = {
        let targets = [target()];
        let selection = selection("local", &["local-helper"]);
        let state_home = fixture.state_home();
        let execution = fixture.execution(&targets, &selection, &state_home);
        let lease = LifecycleLease::acquire(execution.project_root).unwrap();
        let namespace = SubmissionReservations::issue_namespace(&lease, &trusted, 60_000).unwrap();
        let grant = admitted(&execution, "cold-root");
        let binding = grant.binding().clone();
        let mut current = || Ok(Some("generation-a".to_owned()));
        assert!(matches!(
            submit_host_admitted_bound_vibe_bin(
                &lease,
                &namespace,
                &trusted,
                "key-a",
                &execution,
                &binding,
                grant,
                "principal-a",
                &mut current,
            )
            .unwrap(),
            BoundDeployBridgeInspection::Completed { .. }
        ));
        (
            namespace.id().to_owned(),
            execution.project_root.to_path_buf(),
            execution.settings_root.to_path_buf(),
            execution.user_home.to_path_buf(),
        )
    };
    let moved = settings.with_file_name(format!(
        "{}-cold-old",
        settings.file_name().unwrap().to_string_lossy()
    ));
    std::fs::rename(&settings, &moved).unwrap();
    std::fs::create_dir(&settings).unwrap();
    {
        let lease = LifecycleLease::acquire(&project).unwrap();
        let grant = reissued(&project, &settings, &user_home, "cold-root");
        assert!(
            inspect_host_admitted_bound_vibe_bin_cold(&lease, &token, &trusted, "key-a", &grant,)
                .is_err()
        );
    }
    std::fs::remove_dir(&settings).unwrap();
    std::fs::rename(moved, settings).unwrap();
}
