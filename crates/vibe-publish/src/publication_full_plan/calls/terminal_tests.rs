use super::*;
use crate::publication_full_plan::calls::terminal::{self, TerminalInspection};
use crate::publication_full_plan::{Inspection, cold};

fn complete_checkpoints() -> World {
    complete_checkpoints_with_replace(false)
}

fn complete_checkpoints_with_replace(replace: bool) -> World {
    let world = World::new_with_prior_draft(replace);
    let SessionOpen::Ready(session) = world.open() else {
        panic!("sealed ReadyRun absent")
    };
    let mut reader = Reader {
        snapshot: world.snapshot.clone(),
        reads: 0,
        fail_at: None,
        drift_at: None,
        latest_drift_from: None,
    };
    assert_eq!(
        drive_one_read_only(&session, &mut reader).unwrap(),
        NoOpInspection::Next { step: 1 }
    );
    assert_eq!(
        drive_one_read_only(&session, &mut reader).unwrap(),
        NoOpInspection::CheckpointsLanded
    );
    drop(session);
    world
}

fn terminal_reader(world: &World) -> Reader {
    Reader {
        snapshot: world.snapshot.clone(),
        reads: 0,
        fail_at: None,
        drift_at: None,
        latest_drift_from: None,
    }
}

#[test]
fn complete_call_chain_is_distinct_from_fake_partition_result_and_cold_attaches() {
    let world = complete_checkpoints();
    let SessionOpen::Ready(session) = world.open() else {
        panic!()
    };
    assert_eq!(
        terminal::inspect_proved(&session).unwrap(),
        TerminalInspection::Ready { calls: 42 }
    );
    assert!(matches!(
        cold::inspect_full_plan_cold_with_home(
            &world.lease,
            world.namespace.id(),
            &world.scope,
            "noop-key",
            &world.home
        )
        .unwrap(),
        Inspection::CallChainReady { calls: 42, .. }
    ));
    let mut reader = terminal_reader(&world);
    assert_eq!(
        terminal::seal(&session, &mut reader).unwrap(),
        TerminalInspection::Completed { calls: 42 }
    );
    drop(session);
    // Dropping the caller's success response does not require another remote call.
    assert!(matches!(world.open(), SessionOpen::Completed { .. }));
    assert!(matches!(
        cold::inspect_full_plan_cold_with_home(
            &world.lease,
            world.namespace.id(),
            &world.scope,
            "noop-key",
            &world.home
        )
        .unwrap(),
        Inspection::CallChainCompleted { calls: 42, .. }
    ));
    assert!(
        journal::read_bytes(&world.lease, &world.run_id, "result")
            .unwrap()
            .is_none()
    );
    let mut foreign_plan = world.tiny.plan.clone();
    foreign_plan.remote_generation_ref.push_str("-foreign");
    assert!(matches!(
        ReadyRun::open(
            &world.lease,
            &world.namespace,
            &world.scope,
            "noop-key",
            &world.tiny.subject,
            &foreign_plan,
            &world.expected,
            &world.home
        )
        .unwrap(),
        SessionOpen::RecoveryRequired { .. }
    ));
}

#[test]
fn missing_or_tampered_terminal_marker_is_uncertain_after_cold_attach() {
    for mode in 0..3 {
        let world = complete_checkpoints();
        let SessionOpen::Ready(session) = world.open() else {
            panic!()
        };
        assert_eq!(
            terminal::seal(&session, &mut terminal_reader(&world)).unwrap(),
            TerminalInspection::Completed { calls: 42 }
        );
        drop(session);
        let phase = if mode == 2 { "pending" } else { "result" };
        let path = world._root.path().join(format!(
            ".vibe/publication-operations/{}.full-plan-call-terminal-{phase}.json",
            world.run_id
        ));
        if mode == 1 {
            std::fs::write(&path, b"{\"tampered\":true}").unwrap();
        } else {
            std::fs::remove_file(&path).unwrap();
        }
        assert!(matches!(world.open(), SessionOpen::Uncertain { .. }));
        assert!(matches!(
            cold::inspect_full_plan_cold_with_home(
                &world.lease,
                world.namespace.id(),
                &world.scope,
                "noop-key",
                &world.home
            )
            .unwrap(),
            Inspection::CallChainUncertain { .. }
        ));
    }
}

#[test]
fn eof_or_latest_drift_after_terminal_pending_never_seals_or_replays() {
    for latest_drift in [false, true] {
        let world = complete_checkpoints();
        let SessionOpen::Ready(session) = world.open() else {
            panic!()
        };
        let mut reader = terminal_reader(&world);
        if latest_drift {
            reader.latest_drift_from = Some(3);
        } else {
            reader.fail_at = Some(3);
        }
        assert_eq!(
            terminal::seal(&session, &mut reader).unwrap(),
            TerminalInspection::Uncertain
        );
        assert!(
            journal::read_bytes(&world.lease, &world.run_id, "call-terminal-pending")
                .unwrap()
                .is_some()
        );
        assert!(
            journal::read_bytes(&world.lease, &world.run_id, "call-terminal-result")
                .unwrap()
                .is_none()
        );
        let reads = reader.reads;
        assert_eq!(
            terminal::seal(&session, &mut reader).unwrap(),
            TerminalInspection::Uncertain
        );
        assert_eq!(reader.reads, reads);
        drop(session);
        assert!(matches!(world.open(), SessionOpen::Uncertain { .. }));
    }
}

#[test]
fn changed_call_record_is_uncertain_instead_of_escaping_as_decode_error() {
    let world = complete_checkpoints();
    let SessionOpen::Ready(session) = world.open() else {
        panic!()
    };
    let assets = assets::decompose_asset_calls(
        &world.tiny.subject,
        &world.tiny.plan,
        &world.expected,
        &world.run_id,
    )
    .unwrap();
    let path = world._root.path().join(format!(
        ".vibe/publication-operations/{}.full-plan-{}.json",
        world.run_id,
        assets::asset_suffix(assets[0].ordinal, "landed")
    ));
    std::fs::write(path, b"{\"tampered\":true}").unwrap();
    assert_eq!(
        terminal::inspect_proved(&session).unwrap(),
        TerminalInspection::Uncertain
    );
    drop(session);
    assert!(matches!(world.open(), SessionOpen::Uncertain { .. }));
}

#[test]
fn replaced_vault_root_cannot_cold_attach_a_terminal_result() {
    let world = complete_checkpoints();
    let SessionOpen::Ready(session) = world.open() else {
        panic!()
    };
    assert_eq!(
        terminal::seal(&session, &mut terminal_reader(&world)).unwrap(),
        TerminalInspection::Completed { calls: 42 }
    );
    drop(session);
    let vault = world._root.path().join(format!(
        ".vibe/publication-operations/{}.payloads",
        world.run_id
    ));
    let old = world._root.path().join("vault-retired");
    std::fs::rename(&vault, &old).unwrap();
    std::fs::create_dir(&vault).unwrap();
    assert!(matches!(world.open(), SessionOpen::RecoveryRequired { .. }));
    assert!(!matches!(
        cold::inspect_full_plan_cold_with_home(
            &world.lease,
            world.namespace.id(),
            &world.scope,
            "noop-key",
            &world.home
        )
        .unwrap(),
        Inspection::CallChainCompleted { .. }
    ));
}

#[test]
fn terminal_result_survives_api_exit_and_new_lease() {
    let world = complete_checkpoints();
    let SessionOpen::Ready(session) = world.open() else {
        panic!()
    };
    assert_eq!(
        terminal::seal(&session, &mut terminal_reader(&world)).unwrap(),
        TerminalInspection::Completed { calls: 42 }
    );
    drop(session);
    let World {
        tiny,
        _root,
        lease,
        namespace,
        scope,
        home,
        expected,
        snapshot: _,
        run_id: _,
    } = world;
    drop(home);
    drop(lease);
    let reopened = LifecycleLease::acquire(_root.path()).unwrap();
    let home = tiny.approval_home();
    assert!(matches!(
        cold::inspect_full_plan_cold_with_home(
            &reopened,
            namespace.id(),
            &scope,
            "noop-key",
            &home
        )
        .unwrap(),
        Inspection::CallChainCompleted { calls: 42, .. }
    ));
    assert!(matches!(
        ReadyRun::open(
            &reopened,
            &namespace,
            &scope,
            "noop-key",
            &tiny.subject,
            &tiny.plan,
            &expected,
            &home
        )
        .unwrap(),
        SessionOpen::Completed { .. }
    ));
}

#[test]
fn latest_drift_before_terminal_pending_writes_no_marker() {
    let world = complete_checkpoints();
    let SessionOpen::Ready(session) = world.open() else {
        panic!()
    };
    let mut reader = terminal_reader(&world);
    reader.latest_drift_from = Some(1);
    assert_eq!(
        terminal::seal(&session, &mut reader).unwrap(),
        TerminalInspection::Uncertain
    );
    assert!(
        journal::read_bytes(&world.lease, &world.run_id, "call-terminal-pending")
            .unwrap()
            .is_none()
    );
    assert_eq!(
        terminal::inspect_proved(&session).unwrap(),
        TerminalInspection::Ready { calls: 42 }
    );
}

#[test]
fn prior_draft_replacement_reaches_forty_three_call_terminal_after_cold_attach() {
    use vibe_wire::generated::publication_effect_plan::{
        PublicationObservedLatestRelease, PublicationObservedRelease,
    };
    let world = complete_checkpoints_with_replace(true);
    let prefix = crate::publication_full_plan::calls::decompose(
        &world.tiny.subject,
        &world.tiny.plan,
        &world.expected,
    )
    .unwrap();
    assert_eq!(prefix.calls.len(), 3);
    assert!(matches!(
        prefix.calls[0].action,
        crate::publication_full_plan::calls::CallAction::RetainTag { .. }
    ));
    assert!(
        matches!(prefix.calls[1].action, crate::publication_full_plan::calls::CallAction::DeletePriorRelease { ref release_id, .. } if release_id == "123")
    );
    assert!(matches!(
        prefix.calls[2].action,
        crate::publication_full_plan::calls::CallAction::CreateDraft { .. }
    ));
    assert!(matches!(&world.tiny.plan.observed_latest_release,
        PublicationObservedLatestRelease::Present(latest) if latest.release_id == "101" && latest.tag == "v1.0.1"));
    let deleted: crate::publication_full_plan::calls::Landed = journal::read(
        &world.lease,
        &world.run_id,
        &crate::publication_full_plan::calls::suffix(1, "landed"),
    )
    .unwrap()
    .unwrap();
    assert_eq!(deleted.receipt_id, "123");
    assert!(matches!(
        deleted.after.release,
        PublicationObservedRelease::Absent(_)
    ));
    let created: crate::publication_full_plan::calls::Landed = journal::read(
        &world.lease,
        &world.run_id,
        &crate::publication_full_plan::calls::suffix(2, "landed"),
    )
    .unwrap()
    .unwrap();
    assert_eq!(created.receipt_id, "700");
    assert!(
        matches!(created.after.release, PublicationObservedRelease::Present(ref row) if row.draft && row.release_id == "700")
    );
    let SessionOpen::Ready(session) = world.open() else {
        panic!("43-call ReadyRun absent")
    };
    assert_eq!(
        terminal::inspect_proved(&session).unwrap(),
        TerminalInspection::Ready { calls: 43 }
    );
    assert_eq!(
        terminal::seal(&session, &mut terminal_reader(&world)).unwrap(),
        TerminalInspection::Completed { calls: 43 }
    );
    drop(session);
    assert!(matches!(
        cold::inspect_full_plan_cold_with_home(
            &world.lease,
            world.namespace.id(),
            &world.scope,
            "noop-key",
            &world.home
        )
        .unwrap(),
        Inspection::CallChainCompleted { calls: 43, .. }
    ));
    assert!(matches!(world.open(), SessionOpen::Completed { .. }));
}

#[test]
fn one_lease_reopens_four_distinct_domain_keys_without_inventing_owner_effects() {
    use vibe_lifecycle::retained_mechanism::{
        BuildInspection, DeployColdHostFacts, DeployInspection, PackageInspection,
        ReadOnlyDeployAdmission, RetainedInspection, inspect_bound_vibe_bin_deploy,
        inspect_fixed_cargo_build, inspect_fixed_static_file_package,
    };
    use vibe_lifecycle::{ReservationDecision, SubmissionReservations, SubmissionScope};

    let world = complete_checkpoints();
    let SessionOpen::Ready(session) = world.open() else {
        panic!()
    };
    let mut reader = terminal_reader(&world);
    assert_eq!(
        terminal::seal(&session, &mut reader).unwrap(),
        TerminalInspection::Completed { calls: 42 }
    );
    let publication_reads = reader.reads;
    drop(session);

    let reserve_only = |target: &str, key: &str| {
        let scope = SubmissionScope {
            context: "cross-domain-host".into(),
            target: target.into(),
            principal: "owner".into(),
        };
        let namespace =
            SubmissionReservations::issue_namespace(&world.lease, &scope, 60_000).unwrap();
        let run_id = vibe_lifecycle::process::propose_run_id(world.lease.root()).unwrap();
        assert!(matches!(
            SubmissionReservations::reserve(&world.lease, &namespace, &scope, key,
                b"C1-only request; no domain owner intent", b"C1-only plan", &run_id).unwrap(),
            ReservationDecision::Created { domain_run_id } if domain_run_id == run_id
        ));
        (scope, namespace.id().to_owned(), run_id)
    };
    let (build_scope, build_token, build_run) = reserve_only("fixed-cargo-build", "build-key");
    let (package_scope, package_token, package_run) =
        reserve_only("fixed-static-package", "package-key");
    let (deploy_scope, deploy_token, deploy_run) = reserve_only("instance-a", "deploy-key");
    assert_ne!(build_run, package_run);
    assert_ne!(build_run, deploy_run);
    assert_ne!(package_run, deploy_run);
    assert_ne!(build_token, package_token);
    assert_ne!(build_token, deploy_token);
    assert_ne!(package_token, deploy_token);

    let settings = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let World {
        tiny,
        _root,
        lease,
        namespace: publication_namespace,
        scope: publication_scope,
        home: publication_home,
        expected,
        snapshot: _,
        run_id: publication_run,
    } = world;
    drop(lease); // Simulated API exit: no daemon or retained in-memory lease.
    let reopened = LifecycleLease::acquire(_root.path()).unwrap();
    assert_eq!(
        inspect_fixed_cargo_build(&reopened, &build_token, &build_scope, "build-key").unwrap(),
        BuildInspection::RecoveryRequired { run_id: build_run }
    );
    assert_eq!(
        inspect_fixed_static_file_package(&reopened, &package_token, &package_scope, "package-key")
            .unwrap(),
        PackageInspection::RecoveryRequired {
            run_id: package_run
        }
    );
    let admission = ReadOnlyDeployAdmission::reissue(DeployColdHostFacts {
        host_context_id: "cross-domain-host",
        instance_ref: "instance-a",
        selected_generation_ref: Some("generation-a"),
        principal_ref: "owner",
        admission_epoch: 1,
        binding_nonce: "nonce-a",
        allow_alias_resolution: false,
        target_abi: None,
        target_libc: None,
        project_root: _root.path(),
        settings_root: settings.path(),
        user_home: home.path(),
    })
    .unwrap();
    assert_eq!(
        inspect_bound_vibe_bin_deploy(
            &reopened,
            &deploy_token,
            &deploy_scope,
            "deploy-key",
            &admission
        )
        .unwrap(),
        DeployInspection::RecoveryRequired { run_id: deploy_run }
    );
    assert!(
        matches!(cold::inspect_full_plan_cold_with_home(&reopened, publication_namespace.id(),
        &publication_scope, "noop-key", &publication_home).unwrap(),
        Inspection::CallChainCompleted { run_id, calls: 42 } if run_id == publication_run)
    );
    assert!(matches!(
        ReadyRun::open(
            &reopened,
            &publication_namespace,
            &publication_scope,
            "noop-key",
            &tiny.subject,
            &tiny.plan,
            &expected,
            &publication_home
        )
        .unwrap(),
        SessionOpen::Completed { .. }
    ));
    assert_eq!(
        reader.reads, publication_reads,
        "cold attach invoked a remote reader"
    );

    let foreign = SubmissionScope {
        principal: "foreign".into(),
        ..build_scope.clone()
    };
    assert!(inspect_fixed_cargo_build(&reopened, &build_token, &foreign, "build-key").is_err());
    assert_eq!(
        inspect_fixed_cargo_build(&reopened, &build_token, &build_scope, "wrong-key").unwrap(),
        RetainedInspection::Absent
    );
    assert!(
        cold::inspect_full_plan_cold_with_home(
            &reopened,
            publication_namespace.id(),
            &publication_scope,
            "wrong-key",
            &publication_home
        )
        .is_err()
    );
    // Build/Package/Deploy deliberately have C1 reservations only. Their
    // RecoveryRequired statuses are not post-effect partial or completion.
    assert!(!_root.path().join("target").exists());
}
