use super::*;

struct DriftingFail(std::path::PathBuf);

struct DriftAfterRestore(std::path::PathBuf);

impl DeployProvider for DriftAfterRestore {
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
        let report = VibeOptLauncherProvider.remove(request, resources, handle)?;
        std::fs::write(&self.0, b"post-restore-drift").expect("inject post-restore drift");
        Ok(report)
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

impl DeployProvider for DriftingFail {
    fn descriptor(&self) -> DeployDescriptor {
        AlwaysFail.descriptor()
    }
    fn plan(&self, request: &DeployTargetRequest<'_>) -> Result<DeployPlan, MechanismError> {
        AlwaysFail.plan(request)
    }
    fn fingerprint(
        &self,
        request: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
    ) -> Result<DeployFingerprint, MechanismError> {
        AlwaysFail.fingerprint(request, plan)
    }
    fn apply(
        &self,
        request: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
        checkpoint: &mut CheckpointLedger<'_>,
    ) -> Result<ApplyReport, MechanismError> {
        std::fs::write(&self.0, b"user-drift").expect("inject drift");
        AlwaysFail.apply(request, plan, checkpoint)
    }
    fn verify(
        &self,
        request: &DeployTargetRequest<'_>,
        resources: &[String],
    ) -> Result<Vec<ObservedResource>, MechanismError> {
        AlwaysFail.verify(request, resources)
    }
    fn remove(
        &self,
        request: &DeployTargetRequest<'_>,
        resources: &[String],
        handle: Option<&str>,
    ) -> Result<RemoveReport, MechanismError> {
        AlwaysFail.remove(request, resources, handle)
    }
    fn recover(
        &self,
        request: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
        observed: &[ObservedResource],
        checkpoint: &mut CheckpointLedger<'_>,
    ) -> Result<ApplyReport, MechanismError> {
        AlwaysFail.recover(request, plan, observed, checkpoint)
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ZAI-GLM-LAUNCHER-DELIVERY")]
fn drift_before_rollback_leaves_no_marker_and_retry_never_overwrites() {
    let world = World::new();
    world.record_file("claudez.ps1", b"first");
    let initial = [target("claudez.ps1")];
    let initial_profile = selection();
    execute_deploy_targets(&world.execution(&initial, &initial_profile)).expect("initial deploy");
    world.record_file("claudez.ps1", b"second");
    let targets = [
        target("claudez.ps1"),
        named_target("fail-later", "claudez.ps1"),
    ];
    let profile = DeploySelection {
        profile: "windows".into(),
        targets: vec!["install-launcher".into(), "fail-later".into()],
    };
    let execution = world.execution(&targets, &profile);
    apply_selection(
        &execution,
        &[
            selected(
                &targets[0],
                Box::new(VibeOptLauncherProvider),
                BUILTIN_VIBE_OPT_LAUNCHER_PIN,
            ),
            selected(
                &targets[1],
                Box::new(DriftingFail(world.destination("claudez.ps1"))),
                "org.example/tests#always-fail",
            ),
        ],
    )
    .expect_err("rollback drift refuses");
    let home = crate::mechanism::deploy::state::DeploymentHome::new(
        &world.state_home,
        "org.example/demo",
        Some("org.example/launchers"),
        "install-launcher",
    );
    assert!(!home.directory().join("inverse.json").exists());
    execute_deploy_targets(&world.execution(&initial, &initial_profile))
        .expect_err("retry refuses without marker");
    assert_eq!(
        std::fs::read(world.destination("claudez.ps1")).unwrap(),
        b"user-drift"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ZAI-GLM-LAUNCHER-DELIVERY")]
fn post_restore_drift_is_not_laundered_into_rolled_back_ownership() {
    let world = World::new();
    world.record_file("claudez.ps1", b"first");
    let initial = [target("claudez.ps1")];
    let initial_profile = selection();
    execute_deploy_targets(&world.execution(&initial, &initial_profile)).expect("initial deploy");
    world.record_file("claudez.ps1", b"second");
    let targets = [
        target("claudez.ps1"),
        named_target("fail-later", "claudez.ps1"),
    ];
    let profile = DeploySelection {
        profile: "windows".into(),
        targets: vec!["install-launcher".into(), "fail-later".into()],
    };
    let execution = world.execution(&targets, &profile);
    apply_selection(
        &execution,
        &[
            selected(
                &targets[0],
                Box::new(DriftAfterRestore(world.destination("claudez.ps1"))),
                BUILTIN_VIBE_OPT_LAUNCHER_PIN,
            ),
            selected(
                &targets[1],
                Box::new(AlwaysFail),
                "org.example/tests#always-fail",
            ),
        ],
    )
    .expect_err("independent observation rejects post-restore drift");
    let home = crate::mechanism::deploy::state::DeploymentHome::new(
        &world.state_home,
        "org.example/demo",
        Some("org.example/launchers"),
        "install-launcher",
    );
    assert!(
        home.directory().join("inverse.json").exists(),
        "marker remains retry evidence"
    );
    let state = crate::mechanism::deploy::state::DeployState::open(&world.state_home).unwrap();
    let receipt = state.read_receipt(&home).unwrap().unwrap();
    assert_eq!(
        receipt.status,
        vibe_wire::generated::deploy_receipt::ReceiptStatus::Verified
    );
    assert_ne!(
        receipt.resources[0].post_digest,
        resource_digest(
            &store::resource_state(
                "install-launcher",
                world.settings.path(),
                "opt/bin/claudez.ps1"
            )
            .unwrap()
            .unwrap()
        )
    );
}
