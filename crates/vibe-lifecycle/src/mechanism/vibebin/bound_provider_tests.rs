use sha2::{Digest, Sha256};
use vibe_core::manifest::ArtifactKind;
use vibe_safefs::Project;
use vibe_wire::generated::artifact_record::ArtifactRecord;

use super::{BoundVibeBinProvider, BoundVibeBinStore};
use crate::mechanism::DeployProvider;
use crate::mechanism::deploy::state::{CheckpointLedger, DeployState, DeploymentHome};
use crate::mechanism::error::DeployProviderError;
use crate::mechanism::vibebin::bound::{BoundDestinationLock, BoundProjectArtifact};
use crate::mechanism::vibebin::support::{World, request, target};
use crate::mechanism::vibebin::{VibeBinProvider, launcher, store};

fn record(project: &std::path::Path, relative: &str, bytes: &[u8]) -> ArtifactRecord {
    serde_json::from_value(serde_json::json!({
        "schema": 1,
        "id": "helper",
        "kind": "executable",
        "shape": "file",
        "path_absolute": project.join(relative).to_string_lossy().replace('\\', "/"),
        "path_relative": {"root":"project", "path":relative},
        "digest": {"algorithm":"sha256", "value":format!("{:x}", Sha256::digest(bytes))},
        "producer": {"target":"helper", "mechanism":"build:cargo",
            "provider":{"key":"org.vibevm/vibe#cargo"}},
        "freshness": {},
        "created_at":"2026-09-30T00:00:00Z",
        "verification":{"status":"verified", "evidence":"fixture source measured"}
    }))
    .unwrap()
}

#[test]
fn bound_plan_and_observation_use_pinned_destination_and_refuse_effects() {
    let world = World::new();
    let bytes = b"executable fixture";
    let artifact = world.artifact("helper", "executable fixture", ArtifactKind::Executable);
    let row = target("local-helper", "helper", Some("command = 'helper'"));
    let record = record(world.project.path(), &artifact.relative, bytes);
    let source = BoundProjectArtifact::from_record(&record, artifact.bytes).unwrap();
    let project = Project::open(world.project.path()).unwrap();
    let settings = Project::open(world.settings.path()).unwrap();
    let request = request(&world, &row, Some(&artifact), false);
    let preplan = BoundVibeBinProvider::for_preplan(&settings, &source);
    let preview = preplan.plan(&request).unwrap();
    assert!(!world.settings.path().join(".vibe").exists());
    assert!(!world.settings.path().join("bin").exists());
    assert!(!world.settings.path().join("store").exists());
    let destination = BoundDestinationLock::acquire(settings).unwrap();
    let store = BoundVibeBinStore::for_test(&project, &destination);
    let provider = BoundVibeBinProvider::for_locked(&store, &source);
    let plan = provider.plan(&request).unwrap();
    assert_eq!(plan, preview);
    let legacy = VibeBinProvider.plan(&request).unwrap();
    assert_eq!(plan.resources, legacy.resources);
    assert_eq!(plan.lock_resources, legacy.lock_resources);
    assert!(!plan.reversible, "bound remove has no admitted inverse yet");
    let observed = provider
        .verify(&request, &[plan.resources[0].resource.clone()])
        .unwrap();
    assert_eq!(observed[0].digest, None);
    assert!(
        provider
            .verify(&request, &["store/foreign".into()])
            .is_err()
    );
    let refusal = provider.remove(&request, &[], None).unwrap_err();
    assert!(matches!(
        refusal,
        crate::mechanism::MechanismError::Deploy(DeployProviderError::BoundEffectNotReady {
            operation: "remove",
            ..
        })
    ));
    assert_eq!(
        store
            .classify_launcher(&plan.resources[0].resource)
            .unwrap(),
        launcher::Occupant::Vacant
    );
}

#[test]
fn locked_bound_apply_publishes_payload_then_launcher_then_pointer_and_checkpoints() {
    let world = World::new();
    let bytes = b"bound executable";
    let artifact = world.artifact("helper", "bound executable", ArtifactKind::Executable);
    let row = target("local-helper", "helper", Some("command = 'helper'"));
    let record = record(world.project.path(), &artifact.relative, bytes);
    let source = BoundProjectArtifact::from_record(&record, artifact.bytes).unwrap();
    let project = Project::open(world.project.path()).unwrap();
    let destination =
        BoundDestinationLock::acquire(Project::open(world.settings.path()).unwrap()).unwrap();
    let store = BoundVibeBinStore::for_test(&project, &destination);
    let provider = BoundVibeBinProvider::for_locked(&store, &source);
    let call = request(&world, &row, Some(&artifact), false);
    let plan = provider.plan(&call).unwrap();
    let state = DeployState::open(world.state.path()).unwrap();
    let home = DeploymentHome::new(world.state.path(), "org.example/demo", None, &row.id);
    let mut ledger = CheckpointLedger::open(&state, &home, &"a".repeat(64)).unwrap();

    let report = provider.apply(&call, &plan, &mut ledger).unwrap();
    assert!(report.evidence.contains("payload written"), "{report:?}");
    assert_eq!(report.prior_state_handle, None);
    let payload = store::payload_relative(launcher::LauncherFlavour::NATIVE, &artifact.digest);
    assert_eq!(std::fs::read(world.at(&payload)).unwrap(), bytes);
    assert_eq!(
        std::fs::read(world.at(&plan.resources[1].resource)).unwrap(),
        launcher::pointer_body(&artifact.digest)
    );
    let observed = provider
        .verify(
            &call,
            &plan
                .resources
                .iter()
                .map(|resource| resource.resource.clone())
                .collect::<Vec<_>>(),
        )
        .unwrap();
    assert!(observed.iter().zip(&plan.resources).all(
        |(actual, expected)| actual.digest.as_deref() == Some(expected.desired_digest.as_str())
    ));
    let checkpoints = state
        .read_checkpoints(&home, &"a".repeat(64))
        .unwrap()
        .unwrap();
    assert_eq!(
        checkpoints.completed,
        [
            payload,
            plan.resources[0].resource.clone(),
            plan.resources[1].resource.clone()
        ]
    );
}

#[test]
fn preplan_view_refuses_apply_before_any_destination_write() {
    let world = World::new();
    let artifact = world.artifact("helper", "fixture", ArtifactKind::Executable);
    let row = target("local-helper", "helper", Some("command = 'helper'"));
    let record = record(world.project.path(), &artifact.relative, b"fixture");
    let source = BoundProjectArtifact::from_record(&record, artifact.bytes).unwrap();
    let settings = Project::open(world.settings.path()).unwrap();
    let provider = BoundVibeBinProvider::for_preplan(&settings, &source);
    let call = request(&world, &row, Some(&artifact), false);
    let plan = provider.plan(&call).unwrap();
    let state = DeployState::open(world.state.path()).unwrap();
    let home = DeploymentHome::new(world.state.path(), "org.example/demo", None, &row.id);
    let mut ledger = CheckpointLedger::open(&state, &home, &"b".repeat(64)).unwrap();
    let refusal = provider.apply(&call, &plan, &mut ledger).unwrap_err();
    assert!(matches!(
        refusal,
        crate::mechanism::MechanismError::Deploy(DeployProviderError::BoundEffectNotReady {
            operation: "apply",
            ..
        })
    ));
    assert!(!world.settings.path().join("bin").exists());
    assert!(!world.settings.path().join("store").exists());
}

#[test]
fn first_bound_plan_refuses_an_unowned_existing_pointer() {
    let world = World::new();
    let artifact = world.artifact("helper", "fixture", ArtifactKind::Executable);
    let row = target("local-helper", "helper", Some("command = 'helper'"));
    let record = record(world.project.path(), &artifact.relative, b"fixture");
    let source = BoundProjectArtifact::from_record(&record, artifact.bytes).unwrap();
    let pointer = world.at("bin/helper.current");
    std::fs::create_dir_all(pointer.parent().unwrap()).unwrap();
    std::fs::write(&pointer, launcher::pointer_body(&artifact.digest)).unwrap();
    let settings = Project::open(world.settings.path()).unwrap();
    let provider = BoundVibeBinProvider::for_preplan(&settings, &source);
    let refusal = provider
        .plan(&request(&world, &row, Some(&artifact), false))
        .unwrap_err();
    assert!(matches!(
        refusal,
        crate::mechanism::MechanismError::Deploy(DeployProviderError::BoundEffectNotReady {
            operation: "update of an unowned destination",
            ..
        })
    ));
    assert!(!world.settings.path().join("store").exists());
}

#[test]
fn bound_plan_refuses_a_resolved_artifact_outside_its_pinned_record() {
    let world = World::new();
    let mut artifact = world.artifact("helper", "fixture", ArtifactKind::Executable);
    let row = target("local-helper", "helper", Some("command = 'helper'"));
    let record = record(world.project.path(), &artifact.relative, b"fixture");
    let source = BoundProjectArtifact::from_record(&record, artifact.bytes).unwrap();
    artifact.digest = format!("{:x}", Sha256::digest(b"another"));
    let project = Project::open(world.project.path()).unwrap();
    let destination =
        BoundDestinationLock::acquire(Project::open(world.settings.path()).unwrap()).unwrap();
    let store = BoundVibeBinStore::for_test(&project, &destination);
    let provider = BoundVibeBinProvider::for_locked(&store, &source);
    let refusal = provider
        .plan(&request(&world, &row, Some(&artifact), false))
        .unwrap_err();
    assert!(matches!(
        refusal,
        crate::mechanism::MechanismError::Deploy(DeployProviderError::BoundArtifactMismatch { .. })
    ));
}
