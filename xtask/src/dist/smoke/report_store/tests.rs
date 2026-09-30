use std::fs;
use std::path::PathBuf;

use vibe_safefs::Project;
use vibe_wire::generated::publication_subject::PublicationSubject;

use super::{ReportStoreError, store_report};

fn subject() -> PublicationSubject {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../formats/corpora/publication_subject/e1/valid/complete-five-targets.json");
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

fn report() -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../formats/corpora/distribution_smoke_report/e1/valid/five-unavailable.json");
    fs::read(path).unwrap()
}

#[test]
fn valid_report_is_content_addressed_and_same_bytes_are_idempotent() {
    let output = tempfile::tempdir().unwrap();
    let project = Project::open(output.path()).unwrap();
    let bytes = report();
    let first = store_report(&project, &bytes, &subject()).unwrap();
    assert_eq!(first.digest, vibe_publish::sha256_digest(&bytes));
    assert!(first.relative_path.starts_with("smoke/e1/"));
    assert_eq!(
        fs::read(output.path().join(&first.relative_path)).unwrap(),
        bytes
    );
    let second = store_report(&project, &bytes, &subject()).unwrap();
    assert_eq!(second, first);
    assert_eq!(
        project
            .child_names(&project.dir(&["smoke", "e1"], false).unwrap())
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn conflicting_occupant_is_recovery_required_and_never_replaced() {
    let output = tempfile::tempdir().unwrap();
    let project = Project::open(output.path()).unwrap();
    let bytes = report();
    let digest = vibe_publish::sha256_digest(&bytes);
    let name = format!("{}.json", digest.strip_prefix("sha256:").unwrap());
    let evidence = output.path().join("smoke/e1");
    fs::create_dir_all(&evidence).unwrap();
    let occupied = evidence.join(name);
    fs::write(&occupied, b"different bytes").unwrap();
    let error = store_report(&project, &bytes, &subject()).unwrap_err();
    assert!(matches!(error, ReportStoreError::RecoveryRequired { .. }));
    assert_eq!(fs::read(&occupied).unwrap(), b"different bytes");
}

#[cfg(unix)]
#[test]
fn symlink_occupant_is_recovery_required_and_target_untouched() {
    use std::os::unix::fs::symlink;

    let output = tempfile::tempdir().unwrap();
    let project = Project::open(output.path()).unwrap();
    let bytes = report();
    let digest = vibe_publish::sha256_digest(&bytes);
    let name = format!("{}.json", digest.strip_prefix("sha256:").unwrap());
    let evidence = output.path().join("smoke/e1");
    fs::create_dir_all(&evidence).unwrap();
    let target = output.path().join("real-home-sentinel");
    fs::write(&target, b"untouched").unwrap();
    symlink(&target, evidence.join(name)).unwrap();
    let error = store_report(&project, &bytes, &subject()).unwrap_err();
    assert!(matches!(error, ReportStoreError::RecoveryRequired { .. }));
    assert_eq!(fs::read(&target).unwrap(), b"untouched");
}

#[test]
fn invalid_report_refuses_before_any_directory_is_created() {
    let output = tempfile::tempdir().unwrap();
    let project = Project::open(output.path()).unwrap();
    let mut bytes = report();
    bytes[0] = b'!';
    let error = store_report(&project, &bytes, &subject()).unwrap_err();
    assert!(matches!(error, ReportStoreError::InvalidReport(_)));
    assert!(!output.path().join("smoke").exists());
}
