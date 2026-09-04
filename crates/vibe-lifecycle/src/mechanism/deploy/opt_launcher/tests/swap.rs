use super::*;

struct SwappingBeforeApply;

impl DeployProvider for SwappingBeforeApply {
    fn descriptor(&self) -> DeployDescriptor {
        VibeOptLauncherProvider.descriptor()
    }

    fn plan(&self, request: &DeployTargetRequest<'_>) -> Result<DeployPlan, MechanismError> {
        VibeOptLauncherProvider.plan(request)
    }

    fn fingerprint(
        &self,
        request: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
    ) -> Result<DeployFingerprint, MechanismError> {
        VibeOptLauncherProvider.fingerprint(request, plan)
    }

    fn apply(
        &self,
        request: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
        checkpoint: &mut CheckpointLedger<'_>,
    ) -> Result<ApplyReport, MechanismError> {
        let artifact = request.artifact.expect("the engine resolved the artifact");
        std::fs::write(&artifact.absolute, b"wrong!")
            .expect("the test swaps the source after resolution");
        VibeOptLauncherProvider.apply(request, plan, checkpoint)
    }

    fn verify(
        &self,
        request: &DeployTargetRequest<'_>,
        resources: &[String],
    ) -> Result<Vec<ObservedResource>, MechanismError> {
        VibeOptLauncherProvider.verify(request, resources)
    }

    fn remove(
        &self,
        request: &DeployTargetRequest<'_>,
        resources: &[String],
        handle: Option<&str>,
    ) -> Result<RemoveReport, MechanismError> {
        VibeOptLauncherProvider.remove(request, resources, handle)
    }

    fn recover(
        &self,
        request: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
        observed: &[ObservedResource],
        checkpoint: &mut CheckpointLedger<'_>,
    ) -> Result<ApplyReport, MechanismError> {
        VibeOptLauncherProvider.recover(request, plan, observed, checkpoint)
    }
}

struct FailingAfterRestore;

impl DeployProvider for FailingAfterRestore {
    fn descriptor(&self) -> DeployDescriptor {
        VibeOptLauncherProvider.descriptor()
    }
    fn plan(&self, request: &DeployTargetRequest<'_>) -> Result<DeployPlan, MechanismError> {
        VibeOptLauncherProvider.plan(request)
    }
    fn fingerprint(
        &self,
        request: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
    ) -> Result<DeployFingerprint, MechanismError> {
        VibeOptLauncherProvider.fingerprint(request, plan)
    }
    fn apply(
        &self,
        request: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
        checkpoint: &mut CheckpointLedger<'_>,
    ) -> Result<ApplyReport, MechanismError> {
        VibeOptLauncherProvider.apply(request, plan, checkpoint)
    }
    fn verify(
        &self,
        request: &DeployTargetRequest<'_>,
        resources: &[String],
    ) -> Result<Vec<ObservedResource>, MechanismError> {
        VibeOptLauncherProvider.verify(request, resources)
    }
    fn remove(
        &self,
        request: &DeployTargetRequest<'_>,
        resources: &[String],
        handle: Option<&str>,
    ) -> Result<RemoveReport, MechanismError> {
        VibeOptLauncherProvider.remove(request, resources, handle)?;
        Err(MechanismError::Deploy(DeployProviderError::Write {
            target: request.target.id.clone(),
            path: "injected-after-restore".to_owned(),
            reason: "the injected crash fired before rolled-back receipt publication".to_owned(),
        }))
    }
    fn recover(
        &self,
        request: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
        observed: &[ObservedResource],
        checkpoint: &mut CheckpointLedger<'_>,
    ) -> Result<ApplyReport, MechanismError> {
        VibeOptLauncherProvider.recover(request, plan, observed, checkpoint)
    }
}

struct FailingBeforeDelegate;

impl DeployProvider for FailingBeforeDelegate {
    fn descriptor(&self) -> DeployDescriptor {
        VibeOptLauncherProvider.descriptor()
    }
    fn plan(&self, request: &DeployTargetRequest<'_>) -> Result<DeployPlan, MechanismError> {
        VibeOptLauncherProvider.plan(request)
    }
    fn fingerprint(
        &self,
        request: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
    ) -> Result<DeployFingerprint, MechanismError> {
        VibeOptLauncherProvider.fingerprint(request, plan)
    }
    fn apply(
        &self,
        request: &DeployTargetRequest<'_>,
        _plan: &DeployPlan,
        _checkpoint: &mut CheckpointLedger<'_>,
    ) -> Result<ApplyReport, MechanismError> {
        Err(MechanismError::Deploy(DeployProviderError::Write {
            target: request.target.id.clone(),
            path: "injected-before-delegate".to_owned(),
            reason: "the injected crash fired before provider apply".to_owned(),
        }))
    }
    fn verify(
        &self,
        request: &DeployTargetRequest<'_>,
        resources: &[String],
    ) -> Result<Vec<ObservedResource>, MechanismError> {
        VibeOptLauncherProvider.verify(request, resources)
    }
    fn remove(
        &self,
        request: &DeployTargetRequest<'_>,
        resources: &[String],
        handle: Option<&str>,
    ) -> Result<RemoveReport, MechanismError> {
        VibeOptLauncherProvider.remove(request, resources, handle)
    }
    fn recover(
        &self,
        request: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
        observed: &[ObservedResource],
        checkpoint: &mut CheckpointLedger<'_>,
    ) -> Result<ApplyReport, MechanismError> {
        VibeOptLauncherProvider.recover(request, plan, observed, checkpoint)
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ZAI-GLM-LAUNCHER-DELIVERY")]
fn artifact_swap_after_resolution_leaves_owned_destination_byte_and_mode_exact() {
    let world = World::new();
    world.record_file("claudez.ps1", b"first");
    let targets = [target("claudez.ps1")];
    let profile = selection();
    let execution = world.execution(&targets, &profile);
    execute_deploy_targets(&execution).expect("the prior launcher deploys");
    let before = store::resource_state(
        "install-launcher",
        world.settings.path(),
        "opt/bin/claudez.ps1",
    )
    .expect("the prior launcher observes")
    .expect("the prior launcher exists");
    world.record_file("claudez.ps1", b"second");

    let error = apply_selection(
        &execution,
        &[selected(
            &targets[0],
            Box::new(SwappingBeforeApply),
            BUILTIN_VIBE_OPT_LAUNCHER_PIN,
        )],
    )
    .expect_err("the expected artifact state refuses before destination mutation");

    assert!(matches!(error, DeployError::Provider(_)));
    let after = store::resource_state(
        "install-launcher",
        world.settings.path(),
        "opt/bin/claudez.ps1",
    )
    .expect("the prior launcher re-observes")
    .expect("the prior launcher remains");
    assert_eq!(after, before, "bytes and Unix mode remain exact");
    assert_eq!(
        std::fs::read(world.destination("claudez.ps1")).unwrap(),
        b"first",
    );
}

fn interrupt_update(world: &World, body: &[u8]) {
    world.record_file("claudez.ps1", body);
    let targets = [target("claudez.ps1")];
    let profile = selection();
    let execution = world.execution(&targets, &profile);
    apply_selection(
        &execution,
        &[selected(
            &targets[0],
            Box::new(FailingAfterWrite),
            BUILTIN_VIBE_OPT_LAUNCHER_PIN,
        )],
    )
    .expect_err("the update is interrupted after publication");
}

fn backup_path(world: &World) -> std::path::PathBuf {
    crate::mechanism::deploy::state::DeploymentHome::new(
        &world.state_home,
        "org.example/demo",
        Some("org.example/launchers"),
        "install-launcher",
    )
    .staging()
    .join("vibe-opt-launcher-prior")
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ZAI-GLM-LAUNCHER-DELIVERY")]
fn modified_after_crash_backup_refuses_recovery() {
    let world = World::new();
    world.record_file("claudez.ps1", b"first");
    let targets = [target("claudez.ps1")];
    let profile = selection();
    execute_deploy_targets(&world.execution(&targets, &profile)).expect("generation zero deploys");
    interrupt_update(&world, b"second");
    std::fs::write(backup_path(&world), b"tampered").expect("the backup is modified");

    let error = execute_deploy_targets(&world.execution(&targets, &profile))
        .expect_err("a modified rollback backup cannot mint a handle");

    assert!(matches!(error, DeployError::Provider(_)));
    assert_eq!(
        std::fs::read(world.destination("claudez.ps1")).unwrap(),
        b"second"
    );
    let state = crate::mechanism::deploy::state::DeployState::open(&world.state_home)
        .expect("the state opens");
    let home = crate::mechanism::deploy::state::DeploymentHome::new(
        &world.state_home,
        "org.example/demo",
        Some("org.example/launchers"),
        "install-launcher",
    );
    let receipt = state
        .read_receipt(&home)
        .expect("the receipt reads")
        .expect("the deploy owns a receipt");
    assert_eq!(receipt.resources.len(), 1);
    assert_eq!(receipt.resources[0].resource, "opt/bin/claudez.ps1");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ZAI-GLM-LAUNCHER-DELIVERY")]
fn stale_prior_generation_backup_refuses_recovery() {
    let world = World::new();
    world.record_file("claudez.ps1", b"zero");
    let targets = [target("claudez.ps1")];
    let profile = selection();
    let execution = world.execution(&targets, &profile);
    execute_deploy_targets(&execution).expect("generation zero deploys");
    world.record_file("claudez.ps1", b"one");
    execute_deploy_targets(&execution).expect("generation one deploys");
    interrupt_update(&world, b"two");
    std::fs::write(backup_path(&world), b"zero").expect("a stale generation replaces the backup");

    let error = execute_deploy_targets(&execution)
        .expect_err("a stale prior-generation backup cannot mint a handle");

    assert!(matches!(error, DeployError::Provider(_)));
    assert_eq!(
        std::fs::read(world.destination("claudez.ps1")).unwrap(),
        b"two"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ZAI-GLM-LAUNCHER-DELIVERY")]
fn retry_after_restore_before_receipt_publication_is_idempotent() {
    let world = World::new();
    world.record_file("claudez.ps1", b"first");
    let initial = [target("claudez.ps1")];
    let initial_profile = selection();
    execute_deploy_targets(&world.execution(&initial, &initial_profile))
        .expect("the prior launcher deploys");
    world.record_file("claudez.ps1", b"second");
    let targets = [
        target("claudez.ps1"),
        named_target("fail-later", "claudez.ps1"),
    ];
    let profile = DeploySelection {
        profile: "windows".to_owned(),
        targets: vec!["install-launcher".to_owned(), "fail-later".to_owned()],
    };
    let execution = world.execution(&targets, &profile);
    let resolved = [
        selected(
            &targets[0],
            Box::new(FailingAfterRestore),
            BUILTIN_VIBE_OPT_LAUNCHER_PIN,
        ),
        selected(
            &targets[1],
            Box::new(AlwaysFail),
            "org.example/tests#always-fail",
        ),
    ];
    apply_selection(&execution, &resolved).expect_err("rollback crashes after restore");
    assert_eq!(
        std::fs::read(world.destination("claudez.ps1")).unwrap(),
        b"first"
    );
    let pending_state = crate::mechanism::deploy::state::DeployState::open(&world.state_home)
        .expect("pending state opens");
    let pending_home = crate::mechanism::deploy::state::DeploymentHome::new(
        &world.state_home,
        "org.example/demo",
        Some("org.example/launchers"),
        "install-launcher",
    );
    pending_state
        .cleanup_staging(&pending_home)
        .expect("cleanup observes the inverse marker");
    assert!(
        backup_path(&world).is_file(),
        "marker preserves rollback bytes"
    );
    assert!(pending_home.directory().join("inverse.json").is_file());

    let retried = [target("claudez.ps1")];
    let retried_profile = selection();
    let retried_execution = world.execution(&retried, &retried_profile);
    let mismatch = apply_selection(
        &retried_execution,
        &[selected(
            &retried[0],
            Box::new(VibeOptLauncherProvider),
            "org.example/wrong#provider",
        )],
    )
    .expect_err("the marker never crosses into another selected provider");
    assert!(matches!(mismatch, DeployError::RecordInvalid { .. }));
    let outcome = execute_deploy_targets(&world.execution(&retried, &retried_profile))
        .expect("the inverse finalizes and ordinary deployment continues");
    assert_eq!(outcome.len(), 1, "the requested deploy returns its outcome");
    assert_eq!(
        std::fs::read(world.destination("claudez.ps1")).unwrap(),
        b"second"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ZAI-GLM-LAUNCHER-DELIVERY")]
fn manual_reversion_to_prior_bytes_without_inverse_marker_still_refuses() {
    let world = World::new();
    world.record_file("claudez.ps1", b"first");
    let targets = [target("claudez.ps1")];
    let profile = selection();
    let execution = world.execution(&targets, &profile);
    execute_deploy_targets(&execution).expect("generation zero deploys");
    world.record_file("claudez.ps1", b"second");
    execute_deploy_targets(&execution).expect("generation one deploys");
    std::fs::write(world.destination("claudez.ps1"), b"first")
        .expect("a user manually reverts the bytes");

    let error = execute_deploy_targets(&execution)
        .expect_err("prior-looking bytes without a causal marker are drift");

    assert!(matches!(
        error,
        DeployError::Provider(MechanismError::Deploy(
            DeployProviderError::OccupantDrifted { .. }
        ))
    ));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ZAI-GLM-LAUNCHER-DELIVERY")]
fn recovery_before_prior_save_remains_reversible_for_a_later_saga_failure() {
    let world = World::new();
    world.record_file("claudez.ps1", b"first");
    let initial = [target("claudez.ps1")];
    let initial_profile = selection();
    let initial_execution = world.execution(&initial, &initial_profile);
    execute_deploy_targets(&initial_execution).expect("generation zero deploys");
    world.record_file("claudez.ps1", b"second");
    apply_selection(
        &initial_execution,
        &[selected(
            &initial[0],
            Box::new(FailingBeforeDelegate),
            BUILTIN_VIBE_OPT_LAUNCHER_PIN,
        )],
    )
    .expect_err("the intent survives a pre-provider crash");
    assert!(!backup_path(&world).exists(), "no backup was saved yet");

    let targets = [
        target("claudez.ps1"),
        named_target("fail-later", "claudez.ps1"),
    ];
    let profile = DeploySelection {
        profile: "windows".to_owned(),
        targets: vec!["install-launcher".to_owned(), "fail-later".to_owned()],
    };
    let execution = world.execution(&targets, &profile);
    let error = apply_selection(
        &execution,
        &[
            selected(
                &targets[0],
                Box::new(VibeOptLauncherProvider),
                BUILTIN_VIBE_OPT_LAUNCHER_PIN,
            ),
            selected(
                &targets[1],
                Box::new(AlwaysFail),
                "org.example/tests#always-fail",
            ),
        ],
    )
    .expect_err("the recovered update is rolled back by the later failure");

    assert!(matches!(error, DeployError::Saga { .. }));
    assert_eq!(
        std::fs::read(world.destination("claudez.ps1")).unwrap(),
        b"first"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ZAI-GLM-LAUNCHER-DELIVERY")]
fn raced_destination_is_not_trusted_as_the_receipt_prior_backup() {
    let world = World::new();
    world.record_file("claudez.ps1", b"first");
    let targets = [target("claudez.ps1")];
    let profile = selection();
    execute_deploy_targets(&world.execution(&targets, &profile)).expect("the prior deploys");
    let state =
        crate::mechanism::deploy::state::DeployState::open(&world.state_home).expect("state opens");
    let home = crate::mechanism::deploy::state::DeploymentHome::new(
        &world.state_home,
        "org.example/demo",
        Some("org.example/launchers"),
        "install-launcher",
    );
    let receipt = state
        .read_receipt(&home)
        .expect("receipt reads")
        .expect("receipt exists");
    let staging = home.staging();
    std::fs::create_dir_all(&staging).expect("staging creates");
    std::fs::write(world.destination("claudez.ps1"), b"raced")
        .expect("the destination races after occupancy");
    let request = DeployTargetRequest {
        target: &targets[0],
        profile: "windows",
        project_root: world.project.path(),
        settings_root: world.settings.path(),
        user_home: world.home.path(),
        clients: &world.clients,
        prior_receipt: Some(&receipt),
        recovery_intent: None,
        artifact: None,
        staging: Some(&staging),
    };

    let error = VibeOptLauncherProvider
        .save_prior(
            &request,
            &destination("claudez.ps1"),
            &receipt.resources[0].post_digest,
        )
        .expect_err("raced bytes are not the receipt prior");

    assert!(matches!(error, DeployProviderError::OccupantDrifted { .. }));
    assert!(!backup_path(&world).exists());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ZAI-GLM-LAUNCHER-DELIVERY")]
fn backup_swap_after_handle_load_refuses_before_destination_mutation() {
    let world = World::new();
    world.record_file("claudez.ps1", b"current");
    let targets = [target("claudez.ps1")];
    let profile = selection();
    execute_deploy_targets(&world.execution(&targets, &profile)).expect("current deploys");
    let state =
        crate::mechanism::deploy::state::DeployState::open(&world.state_home).expect("state opens");
    let home = crate::mechanism::deploy::state::DeploymentHome::new(
        &world.state_home,
        "org.example/demo",
        Some("org.example/launchers"),
        "install-launcher",
    );
    let receipt = state.read_receipt(&home).unwrap().unwrap();
    let staging = home.staging();
    std::fs::create_dir_all(&staging).expect("staging creates");
    let request = DeployTargetRequest {
        target: &targets[0],
        profile: "windows",
        project_root: world.project.path(),
        settings_root: world.settings.path(),
        user_home: world.home.path(),
        clients: &world.clients,
        prior_receipt: Some(&receipt),
        recovery_intent: None,
        artifact: None,
        staging: Some(&staging),
    };
    let backup = backup_relative(&request).expect("backup identity resolves");
    store::place_resource(
        "install-launcher",
        world.settings.path(),
        Some(&staging),
        &backup,
        b"prior",
        cfg!(unix),
    )
    .expect("prior backup writes");
    let backup_state = store::resource_state("install-launcher", world.settings.path(), &backup)
        .unwrap()
        .unwrap();
    let encoded = render_handle(&PriorHandle {
        path: backup.clone(),
        sha256: backup_state.sha256,
        bytes: backup_state.bytes,
        unix_mode: backup_state.unix_mode,
    });
    let (handle, loaded) = VibeOptLauncherProvider
        .load_prior(&request, &encoded)
        .expect("the prior handle loads");
    let before = store::resource_state(
        "install-launcher",
        world.settings.path(),
        "opt/bin/claudez.ps1",
    )
    .unwrap()
    .unwrap();
    store::place_resource(
        "install-launcher",
        world.settings.path(),
        Some(&staging),
        &backup,
        b"raced",
        cfg!(unix),
    )
    .expect("the backup races after load");

    VibeOptLauncherProvider
        .restore_prior(&request, &destination("claudez.ps1"), &handle, &loaded)
        .expect_err("expected state refuses before destination mutation");

    let after = store::resource_state(
        "install-launcher",
        world.settings.path(),
        "opt/bin/claudez.ps1",
    )
    .unwrap()
    .unwrap();
    assert_eq!(after, before);
}
