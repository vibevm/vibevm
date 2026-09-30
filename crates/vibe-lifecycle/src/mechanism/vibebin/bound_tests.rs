use sha2::{Digest, Sha256};
use vibe_safefs::Project;
use vibe_wire::generated::artifact_record::{ArtifactRecord, RelativeRoot};

use super::{BoundDestinationLock, BoundProjectArtifact, BoundVibeBinError, BoundVibeBinStore};
use crate::mechanism::vibebin::{launcher, store};

fn record(source: &std::path::Path, bytes: &[u8]) -> ArtifactRecord {
    let absolute = source
        .join("artifact.bin")
        .to_string_lossy()
        .replace('\\', "/");
    serde_json::from_value(serde_json::json!({
        "schema": 1,
        "id": "artifact.bin",
        "kind": "executable",
        "shape": "file",
        "path_absolute": absolute,
        "path_relative": {"root":"project", "path":"artifact.bin"},
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
fn pinned_store_copies_once_and_handles_launcher_pointer_and_remove() {
    let source = tempfile::tempdir().unwrap();
    let settings = tempfile::tempdir().unwrap();
    let bytes = b"payload from pinned project";
    std::fs::write(source.path().join("artifact.bin"), bytes).unwrap();
    let record = record(source.path(), bytes);
    let artifact = BoundProjectArtifact::from_record(&record, bytes.len() as u64).unwrap();
    let source = Project::open(source.path()).unwrap();
    let settings_path = settings.path().to_path_buf();
    let destination =
        BoundDestinationLock::acquire(Project::open(&settings_path).unwrap()).unwrap();
    let bound = BoundVibeBinStore::new(&source, &destination);
    let flavour = launcher::LauncherFlavour::NATIVE;
    assert_eq!(
        bound.place_payload(&artifact, flavour).unwrap(),
        store::PayloadPlacement::Written
    );
    assert_eq!(
        bound.place_payload(&artifact, flavour).unwrap(),
        store::PayloadPlacement::AlreadyPresent
    );
    let relative = store::payload_relative(flavour, &record.digest.value);
    assert_eq!(
        destination
            .settings()
            .read_file(&relative)
            .unwrap()
            .unwrap(),
        bytes
    );
    let launcher_path = format!("bin/helper{}", flavour.launcher_suffix());
    assert_eq!(
        bound.classify_launcher(&launcher_path).unwrap(),
        launcher::Occupant::Vacant
    );
    bound
        .write_resource(
            &launcher_path,
            &launcher::render(flavour, "helper"),
            flavour.needs_executable_bit(),
        )
        .unwrap();
    assert_eq!(
        bound.classify_launcher(&launcher_path).unwrap(),
        launcher::Occupant::OurGenre
    );
    let pointer = "bin/helper.current";
    assert_eq!(bound.prior_pointer(pointer).unwrap(), None);
    bound
        .write_resource(pointer, b"malformed pointer", false)
        .unwrap();
    assert!(matches!(
        bound.prior_pointer(pointer),
        Err(BoundVibeBinError::InvalidPointer)
    ));
    bound
        .write_resource(
            pointer,
            &launcher::pointer_body(&record.digest.value),
            false,
        )
        .unwrap();
    assert_eq!(
        bound.prior_pointer(pointer).unwrap().as_deref(),
        Some(record.digest.value.as_str())
    );
    assert!(bound.state(pointer).unwrap().is_some());
    assert!(bound.remove_resource(pointer).unwrap());
    assert!(bound.state(pointer).unwrap().is_none());
    let second = Project::open(&settings_path).unwrap();
    let contender = second.dir(&[".vibe"], false).unwrap();
    assert!(
        contender
            .try_lock_file("bound-vibe-bin-physical.lock")
            .unwrap()
            .is_none()
    );
    drop(bound);
    drop(destination);
    assert!(
        contender
            .try_lock_file("bound-vibe-bin-physical.lock")
            .unwrap()
            .is_some()
    );
}

#[test]
fn pinned_store_refuses_nonproject_source_changed_bytes_and_cas_third_state() {
    let source = tempfile::tempdir().unwrap();
    let settings = tempfile::tempdir().unwrap();
    let bytes = b"expected";
    std::fs::write(source.path().join("artifact.bin"), bytes).unwrap();
    let mut record = record(source.path(), bytes);
    record.path_relative.root = RelativeRoot::Slot;
    assert!(matches!(
        BoundProjectArtifact::from_record(&record, bytes.len() as u64),
        Err(BoundVibeBinError::UnsupportedArtifact)
    ));
    record.path_relative.root = RelativeRoot::Project;
    let artifact = BoundProjectArtifact::from_record(&record, bytes.len() as u64).unwrap();
    let source = Project::open(source.path()).unwrap();
    let destination =
        BoundDestinationLock::acquire(Project::open(settings.path()).unwrap()).unwrap();
    let bound = BoundVibeBinStore::new(&source, &destination);
    std::fs::write(source.root_path().join("artifact.bin"), b"changed").unwrap();
    assert!(matches!(
        bound.place_payload(&artifact, launcher::LauncherFlavour::NATIVE),
        Err(BoundVibeBinError::SourceChanged)
    ));
    std::fs::write(source.root_path().join("artifact.bin"), bytes).unwrap();
    let relative = store::payload_relative(launcher::LauncherFlavour::NATIVE, &record.digest.value);
    destination
        .settings()
        .write_atomic(&relative, b"foreign bytes")
        .unwrap();
    assert!(matches!(
        bound.place_payload(&artifact, launcher::LauncherFlavour::NATIVE),
        Err(BoundVibeBinError::CasMismatch)
    ));
    assert_eq!(
        destination
            .settings()
            .read_file(&relative)
            .unwrap()
            .unwrap(),
        b"foreign bytes"
    );
}

#[test]
fn postpublication_fault_is_uncertain_then_matching_replay_is_noop() {
    let source = tempfile::tempdir().unwrap();
    let settings = tempfile::tempdir().unwrap();
    let bytes = b"exact payload";
    std::fs::write(source.path().join("artifact.bin"), bytes).unwrap();
    let record = record(source.path(), bytes);
    let artifact = BoundProjectArtifact::from_record(&record, bytes.len() as u64).unwrap();
    let source = Project::open(source.path()).unwrap();
    let destination =
        BoundDestinationLock::acquire(Project::open(settings.path()).unwrap()).unwrap();
    let bound = BoundVibeBinStore::new(&source, &destination);
    let flavour = launcher::LauncherFlavour::NATIVE;
    let relative = store::payload_relative(flavour, &record.digest.value);
    vibe_safefs::fail_after_publish(Some(&relative));
    let result = bound.place_payload(&artifact, flavour);
    vibe_safefs::fail_after_publish(None);
    assert!(matches!(
        result,
        Err(BoundVibeBinError::PublicationUncertain(_))
    ));
    assert_eq!(
        destination
            .settings()
            .read_file(&relative)
            .unwrap()
            .unwrap(),
        bytes
    );
    assert_eq!(
        bound.place_payload(&artifact, flavour).unwrap(),
        store::PayloadPlacement::AlreadyPresent
    );
}
