use std::path::PathBuf;

use specmark::verifies;
use tempfile::TempDir;
use vibe_core::manifest::{ArtifactKind, DeployTarget, MechanismRoutes};
use vibe_extension_registry::SelectionStep;
use vibe_wire::generated::artifact_record::ArtifactShape;

use super::*;
use crate::mechanism::deploy::model::{
    ClientExecutables, DeployExecution, DeploySelection, deploy_state_home,
};
use crate::mechanism::deploy::protocol::{
    ApplyReport, DeployDescriptor, DeployFingerprint, DeployPlan, ObservedResource, RemoveReport,
};
use crate::mechanism::deploy::state::CheckpointLedger;
use crate::mechanism::deploy::{Selected, apply_selection};
use crate::mechanism::package::support::{config, empty_world, key, registry, temp};
use crate::mechanism::record::{RecordFreshness, RecordInputs, build_record, write_record};
use crate::mechanism::vibebin::support::fake_clients;
use crate::mechanism::{
    BUILTIN_STATIC_FILE_PIN, DeployProvider, DeployTargetRequest, EffectClass, NetworkUse,
    PrivilegeNeed, ProviderDescriptor, ProviderOperation, Reversibility,
};
use crate::{DeployError, execute_deploy_targets, undeploy_targets};

struct World {
    project: TempDir,
    settings: TempDir,
    home: TempDir,
    state_home: PathBuf,
    clients: ClientExecutables,
    registry: vibe_extension_registry::MechanismRegistry,
    routes: MechanismRoutes,
}

impl World {
    fn new() -> Self {
        let settings = temp();
        let home = temp();
        let state_home = deploy_state_home(settings.path());
        let clients = fake_clients(home.path());
        Self {
            project: temp(),
            settings,
            home,
            state_home,
            clients,
            registry: registry(&empty_world()),
            routes: MechanismRoutes::default(),
        }
    }

    fn record_file(&self, id: &str, bytes: &[u8]) -> String {
        let relative = format!("target/vibe-package/launcher/{id}");
        let absolute = self.project.path().join(&relative);
        std::fs::create_dir_all(absolute.parent().expect("a parent"))
            .expect("the artifact directory creates");
        std::fs::write(&absolute, bytes).expect("the artifact writes");
        let digest = digest_of(bytes);
        let record = build_record(&RecordInputs {
            target: "launcher-package",
            mechanism: &key("package:static-file"),
            provider_key: BUILTIN_STATIC_FILE_PIN,
            provider_version: None,
            provider_hash: None,
            output_id: id,
            kind: ArtifactKind::File,
            shape: ArtifactShape::File,
            digest: &digest,
            path_absolute: &crate::mechanism::contain::forward_slashed(&absolute),
            path_relative: &relative,
            freshness: RecordFreshness::default(),
            platform: None,
            media_type: None,
            created_at: "2026-09-04T00:00:00Z",
            evidence: "fixture opaque file".to_owned(),
        })
        .expect("the artifact record builds");
        write_record(self.project.path(), &record).expect("the artifact record writes");
        digest
    }

    fn execution<'a>(
        &'a self,
        targets: &'a [DeployTarget],
        selection: &'a DeploySelection,
    ) -> DeployExecution<'a> {
        DeployExecution {
            project_root: self.project.path(),
            targets,
            selection,
            registry: &self.registry,
            routes: &self.routes,
            state_home: &self.state_home,
            settings_root: self.settings.path(),
            user_home: self.home.path(),
            clients: &self.clients,
            project: "org.example/demo",
            package: Some("org.example/launchers"),
            created_at: "2026-09-04T00:00:00Z",
        }
    }

    fn destination(&self, name: &str) -> PathBuf {
        self.settings.path().join("opt").join("bin").join(name)
    }
}

fn named_target(id: &str, artifact: &str) -> DeployTarget {
    DeployTarget {
        id: id.to_owned(),
        artifact: artifact.to_owned(),
        mechanism: key("deploy:vibe-opt-launcher"),
        provider: None,
        depends_on: None,
        config: Some(config("")),
    }
}

fn target(artifact: &str) -> DeployTarget {
    named_target("install-launcher", artifact)
}

fn selection() -> DeploySelection {
    DeploySelection {
        profile: "windows".to_owned(),
        targets: vec!["install-launcher".to_owned()],
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ZAI-GLM-LAUNCHER-DELIVERY")]
fn first_apply_refuses_an_unowned_occupant_without_mutating_it() {
    let world = World::new();
    world.record_file("claudez.ps1", b"desired");
    let destination = world.destination("claudez.ps1");
    std::fs::create_dir_all(destination.parent().expect("a parent"))
        .expect("the destination directory creates");
    std::fs::write(&destination, b"foreign").expect("the foreign occupant writes");
    let targets = [target("claudez.ps1")];
    let selected = selection();

    let error = execute_deploy_targets(&world.execution(&targets, &selected))
        .expect_err("an unowned occupant refuses");

    assert!(matches!(
        error,
        DeployError::Provider(MechanismError::Deploy(
            DeployProviderError::OccupantUnowned { .. }
        ))
    ));
    assert_eq!(
        std::fs::read(destination).expect("the occupant reads"),
        b"foreign"
    );
    assert!(!world.state_home.exists());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ZAI-GLM-LAUNCHER-DELIVERY")]
fn owned_update_is_verified_and_undeploy_removes_only_the_launcher() {
    let world = World::new();
    world.record_file("claudez.ps1", b"first");
    let targets = [target("claudez.ps1")];
    let selected = selection();
    let execution = world.execution(&targets, &selected);

    let first = execute_deploy_targets(&execution).expect("the first generation deploys");
    assert_eq!(first[0].provider, BUILTIN_VIBE_OPT_LAUNCHER_PIN);
    assert_eq!(first[0].resources[0].resource, "opt/bin/claudez.ps1");
    assert_eq!(
        std::fs::read(world.destination("claudez.ps1")).unwrap(),
        b"first"
    );

    world.record_file("claudez.ps1", b"second");
    let second = execute_deploy_targets(&execution).expect("the owned update deploys");
    assert_eq!(second[0].generation, 1);
    assert!(
        second[0].reversible,
        "the update retained exact prior state"
    );
    assert_eq!(
        std::fs::read(world.destination("claudez.ps1")).unwrap(),
        b"second"
    );

    std::fs::write(world.settings.path().join("opt/bin/neighbour"), b"keep")
        .expect("the neighbour writes");
    undeploy_targets(&execution).expect("the receipt-owned file removes");
    assert!(!world.destination("claudez.ps1").exists());
    assert_eq!(
        std::fs::read(world.settings.path().join("opt/bin/neighbour")).unwrap(),
        b"keep",
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ZAI-GLM-LAUNCHER-DELIVERY")]
fn an_unsafe_artifact_filename_refuses_before_the_destination_exists() {
    let world = World::new();
    let path = world.project.path().join("opaque");
    std::fs::write(&path, b"opaque").expect("the artifact writes");
    let artifact = ResolvedDeployArtifact {
        id: "../escape".to_owned(),
        kind: ArtifactKind::File,
        shape: ArtifactShape::File,
        absolute: path,
        relative: "opaque".to_owned(),
        digest: digest_of(b"opaque"),
        bytes: 6,
    };
    let row = target("../escape");
    let request = DeployTargetRequest {
        target: &row,
        profile: "windows",
        project_root: world.project.path(),
        settings_root: world.settings.path(),
        user_home: world.home.path(),
        clients: &world.clients,
        prior_receipt: None,
        recovery_intent: None,
        artifact: Some(&artifact),
        staging: None,
    };

    let error = VibeOptLauncherProvider
        .plan(&request)
        .expect_err("a non-portable filename refuses on every platform");

    assert!(matches!(
        error,
        MechanismError::Deploy(DeployProviderError::Config { .. })
    ));
    assert!(!world.settings.path().join("opt").exists());
    assert!(!world.state_home.exists());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ZAI-GLM-LAUNCHER-DELIVERY")]
fn the_adapter_epoch_is_part_of_config_and_therefore_the_plan_hash() {
    let world = World::new();
    let path = world.project.path().join("launcher");
    std::fs::write(&path, b"opaque").expect("the artifact writes");
    let artifact = ResolvedDeployArtifact {
        id: "claudez".to_owned(),
        kind: ArtifactKind::File,
        shape: ArtifactShape::File,
        absolute: path,
        relative: "launcher".to_owned(),
        digest: digest_of(b"opaque"),
        bytes: 6,
    };
    let row = target("claudez");
    let request = DeployTargetRequest {
        target: &row,
        profile: "posix",
        project_root: world.project.path(),
        settings_root: world.settings.path(),
        user_home: world.home.path(),
        clients: &world.clients,
        prior_receipt: None,
        recovery_intent: None,
        artifact: Some(&artifact),
        staging: None,
    };
    let plan = VibeOptLauncherProvider
        .plan(&request)
        .expect("the target plans");
    assert_eq!(plan.config_digest, config_digest());
    let current = crate::mechanism::deploy::transaction::plan_hash(
        &request,
        BUILTIN_VIBE_OPT_LAUNCHER_PIN,
        &plan,
    );
    let mut changed = plan.clone();
    changed.config_digest = digest_of(b"different-adapter-epoch");
    assert_ne!(
        current,
        crate::mechanism::deploy::transaction::plan_hash(
            &request,
            BUILTIN_VIBE_OPT_LAUNCHER_PIN,
            &changed,
        ),
    );
}

struct FailingAfterWrite;

impl DeployProvider for FailingAfterWrite {
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
        VibeOptLauncherProvider.apply(request, plan, checkpoint)?;
        Err(MechanismError::Deploy(DeployProviderError::Write {
            target: request.target.id.clone(),
            path: "injected-after-write".to_owned(),
            reason: "the injected crash fired after publication".to_owned(),
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

struct AlwaysFail;

impl DeployProvider for AlwaysFail {
    fn descriptor(&self) -> DeployDescriptor {
        DeployDescriptor {
            provider: ProviderDescriptor {
                key: "org.example/tests#always-fail",
                kinds: &[ArtifactKind::File],
                effect: EffectClass::User,
                network: NetworkUse::Never,
                privilege: PrivilegeNeed::None,
                reversibility: Reversibility::Irreversible,
                operations: &[
                    ProviderOperation::Plan,
                    ProviderOperation::Fingerprint,
                    ProviderOperation::Apply,
                    ProviderOperation::Verify,
                    ProviderOperation::Remove,
                    ProviderOperation::Recover,
                ],
            },
            atomic_replacement: false,
            reference_ownership: false,
        }
    }

    fn plan(&self, _request: &DeployTargetRequest<'_>) -> Result<DeployPlan, MechanismError> {
        Ok(DeployPlan {
            resources: Vec::new(),
            lock_resources: Vec::new(),
            config_digest: "0".repeat(64),
            reversible: false,
            summary: "injected later failure".to_owned(),
        })
    }

    fn fingerprint(
        &self,
        _request: &DeployTargetRequest<'_>,
        _plan: &DeployPlan,
    ) -> Result<DeployFingerprint, MechanismError> {
        Ok(DeployFingerprint {
            digest: "0".repeat(64),
            summary: "injected failure".to_owned(),
        })
    }

    fn apply(
        &self,
        request: &DeployTargetRequest<'_>,
        _plan: &DeployPlan,
        _checkpoint: &mut CheckpointLedger<'_>,
    ) -> Result<ApplyReport, MechanismError> {
        Err(MechanismError::Deploy(DeployProviderError::Write {
            target: request.target.id.clone(),
            path: "injected-later-target".to_owned(),
            reason: "the later target failed".to_owned(),
        }))
    }

    fn verify(
        &self,
        _request: &DeployTargetRequest<'_>,
        _resources: &[String],
    ) -> Result<Vec<ObservedResource>, MechanismError> {
        Ok(Vec::new())
    }

    fn remove(
        &self,
        _request: &DeployTargetRequest<'_>,
        _resources: &[String],
        _handle: Option<&str>,
    ) -> Result<RemoveReport, MechanismError> {
        Ok(RemoveReport::default())
    }

    fn recover(
        &self,
        request: &DeployTargetRequest<'_>,
        _plan: &DeployPlan,
        _observed: &[ObservedResource],
        _checkpoint: &mut CheckpointLedger<'_>,
    ) -> Result<ApplyReport, MechanismError> {
        Err(MechanismError::Deploy(DeployProviderError::Write {
            target: request.target.id.clone(),
            path: "injected-recover".to_owned(),
            reason: "the failure fixture cannot recover".to_owned(),
        }))
    }
}

fn selected<'a>(
    target: &'a DeployTarget,
    provider: Box<dyn DeployProvider>,
    pin: &str,
) -> Selected<'a> {
    Selected {
        target,
        provider,
        pin: pin.to_owned(),
        via: SelectionStep::BuiltinDefault,
        displaced: None,
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ZAI-GLM-LAUNCHER-DELIVERY")]
fn a_later_target_failure_restores_the_exact_prior_launcher() {
    let world = World::new();
    world.record_file("claudez.ps1", b"first");
    let initial = [target("claudez.ps1")];
    let initial_selection = selection();
    execute_deploy_targets(&world.execution(&initial, &initial_selection))
        .expect("the prior generation deploys");
    world.record_file("claudez.ps1", b"second");
    let targets = [
        target("claudez.ps1"),
        named_target("fail-later", "claudez.ps1"),
    ];
    let selection = DeploySelection {
        profile: "windows".to_owned(),
        targets: vec!["install-launcher".to_owned(), "fail-later".to_owned()],
    };
    let execution = world.execution(&targets, &selection);
    let resolved = vec![
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
    ];

    let error = apply_selection(&execution, &resolved).expect_err("the saga unwinds");

    assert!(matches!(error, DeployError::Saga { .. }));
    assert_eq!(
        std::fs::read(world.destination("claudez.ps1")).unwrap(),
        b"first",
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ZAI-GLM-LAUNCHER-DELIVERY")]
fn an_interrupted_update_keeps_rollback_state_and_recovers_its_generation() {
    let world = World::new();
    world.record_file("claudez.ps1", b"first");
    let targets = [target("claudez.ps1")];
    let selected_profile = selection();
    let execution = world.execution(&targets, &selected_profile);
    execute_deploy_targets(&execution).expect("the prior generation deploys");
    world.record_file("claudez.ps1", b"second");
    apply_selection(
        &execution,
        &[selected(
            &targets[0],
            Box::new(FailingAfterWrite),
            BUILTIN_VIBE_OPT_LAUNCHER_PIN,
        )],
    )
    .expect_err("the update is interrupted after publication");
    assert_eq!(
        std::fs::read(world.destination("claudez.ps1")).unwrap(),
        b"second",
    );

    let recovered = execute_deploy_targets(&execution).expect("the update recovers");

    assert_eq!(recovered[0].generation, 1);
    assert_eq!(recovered[0].settlement, "recovered");
    assert!(recovered[0].reversible);
    assert_eq!(
        std::fs::read(world.destination("claudez.ps1")).unwrap(),
        b"second",
    );
}

#[cfg(unix)]
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ZAI-GLM-LAUNCHER-DELIVERY")]
fn a_posix_launcher_is_executable_and_mode_drift_is_not_content_only_success() {
    use std::os::unix::fs::PermissionsExt;

    let world = World::new();
    world.record_file("claudez", b"#!/bin/sh\nexit 0\n");
    let targets = [target("claudez")];
    let selected = selection();
    let execution = world.execution(&targets, &selected);
    execute_deploy_targets(&execution).expect("the POSIX launcher deploys");
    let destination = world.destination("claudez");
    let metadata = std::fs::metadata(&destination).expect("the launcher metadata reads");
    assert_eq!(metadata.permissions().mode() & 0o7777, 0o755);

    std::fs::set_permissions(&destination, std::fs::Permissions::from_mode(0o644))
        .expect("the executable bit is removed");
    let error = execute_deploy_targets(&execution)
        .expect_err("content equality without exact mode is not fresh");
    let DeployError::Provider(MechanismError::Deploy(DeployProviderError::OccupantDrifted {
        recorded,
        observed,
        ..
    })) = error
    else {
        panic!("expected full-state drift, got {error}");
    };
    assert_ne!(recorded, observed, "mode changes the resource identity");
}

#[path = "tests/swap.rs"]
mod swap;

#[path = "tests/inverse_drift.rs"]
mod inverse_drift;
