use sha2::{Digest, Sha256};
use vibe_core::manifest::ArtifactKind;
use vibe_safefs::Project;
use vibe_wire::generated::artifact_record::ArtifactRecord;

use super::{BoundVibeBinProvider, BoundVibeBinStore};
use crate::mechanism::DeployProvider;
use crate::mechanism::error::DeployProviderError;
use crate::mechanism::vibebin::bound::{BoundDestinationLock, BoundProjectArtifact};
use crate::mechanism::vibebin::support::{World, request, target};
use crate::mechanism::vibebin::{VibeBinProvider, launcher};

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
    let store = BoundVibeBinStore::new(&project, &destination);
    let provider = BoundVibeBinProvider::for_locked(&store, &source);
    let plan = provider.plan(&request).unwrap();
    assert_eq!(plan, preview);
    assert_eq!(plan, VibeBinProvider.plan(&request).unwrap());
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
    let store = BoundVibeBinStore::new(&project, &destination);
    let provider = BoundVibeBinProvider::for_locked(&store, &source);
    let refusal = provider
        .plan(&request(&world, &row, Some(&artifact), false))
        .unwrap_err();
    assert!(matches!(
        refusal,
        crate::mechanism::MechanismError::Deploy(DeployProviderError::BoundArtifactMismatch { .. })
    ));
}
