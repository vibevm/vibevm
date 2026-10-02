use super::*;
use std::path::Path;

fn scope() -> SubmissionScope {
    SubmissionScope {
        context: "fixed-package-test-context".into(),
        target: "launcher".into(),
        principal: "fixed-package-test-principal".into(),
    }
}

fn issued(root: &Path) -> (Arc<Project>, SubmissionNamespace, SubmissionScope) {
    let project = Arc::new(Project::open(root).unwrap());
    let scope = scope();
    let lease = LifecycleLease::acquire(root).unwrap();
    let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
    (project, namespace, scope)
}

#[test]
fn host_replay_and_cold_reconstruction_keep_one_output_and_run() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("source.bin"), b"copy once").unwrap();
    let (project, namespace, scope) = issued(root.path());
    let host =
        FixedPackageHost::from_trusted_host(project.clone(), namespace.clone(), scope.clone())
            .unwrap();
    assert_eq!(host.inspect("copy").unwrap(), None);
    let first = host.submit("copy").unwrap();
    assert!(matches!(
        first,
        StaticFileJournalInspection::Completed { .. }
    ));
    let output = root
        .path()
        .join("target/vibe-package/launcher/launcher.bin");
    let first_metadata = std::fs::metadata(&output).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(1100));
    assert_eq!(host.submit("copy").unwrap(), first);
    let second_metadata = std::fs::metadata(&output).unwrap();
    assert_eq!(
        second_metadata.modified().unwrap(),
        first_metadata.modified().unwrap()
    );
    drop(host);
    let cold = FixedPackageHost::from_trusted_host(project, namespace, scope).unwrap();
    assert_eq!(cold.inspect("copy").unwrap(), Some(first.clone()));
    assert_eq!(cold.submit("copy").unwrap(), first);
    assert_eq!(std::fs::read(output).unwrap(), b"copy once");
}

#[test]
fn changed_source_preserves_typed_owner_refusal_and_prior_output() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("source.bin"), b"first").unwrap();
    let (project, namespace, scope) = issued(root.path());
    let host = FixedPackageHost::from_trusted_host(project, namespace, scope).unwrap();
    let first = host.submit("copy").unwrap();
    std::fs::write(root.path().join("source.bin"), b"changed").unwrap();
    assert!(matches!(
        host.submit("copy"),
        Err(FixedPackageHostError::Owner(_))
    ));
    assert_eq!(host.inspect("copy").unwrap(), Some(first));
    assert_eq!(
        std::fs::read(
            root.path()
                .join("target/vibe-package/launcher/launcher.bin")
        )
        .unwrap(),
        b"first"
    );
}

#[test]
fn foreign_scope_and_replaced_physical_root_refuse_before_owner() {
    let outer = tempfile::tempdir().unwrap();
    let selected = outer.path().join("selected");
    let moved = outer.path().join("moved");
    std::fs::create_dir(&selected).unwrap();
    std::fs::write(selected.join("source.bin"), b"first").unwrap();
    let (project, namespace, scope) = issued(&selected);
    let foreign = SubmissionScope {
        principal: "other".into(),
        ..scope.clone()
    };
    assert!(matches!(
        FixedPackageHost::from_trusted_host(project.clone(), namespace.clone(), foreign),
        Err(FixedPackageHostError::Namespace(
            SubmissionError::ForeignScope
        ))
    ));
    let host = FixedPackageHost::from_trusted_host(project, namespace, scope).unwrap();
    std::fs::rename(&selected, &moved).unwrap();
    std::fs::create_dir(&selected).unwrap();
    std::fs::write(selected.join("source.bin"), b"replacement").unwrap();
    assert!(matches!(
        host.submit("copy"),
        Err(FixedPackageHostError::ForeignRoot)
    ));
    assert!(matches!(
        host.inspect("copy"),
        Err(FixedPackageHostError::ForeignRoot)
    ));
    assert!(!selected.join("target").exists());
    assert!(!moved.join("target").exists());
}

#[test]
fn damaged_completion_is_cold_uncertainty_and_never_recopies() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("source.bin"), b"first").unwrap();
    let (project, namespace, scope) = issued(root.path());
    let host =
        FixedPackageHost::from_trusted_host(project.clone(), namespace.clone(), scope.clone())
            .unwrap();
    let StaticFileJournalInspection::Completed { run_id, .. } = host.submit("copy").unwrap() else {
        panic!("owner did not complete the fixed copy")
    };
    let output = root
        .path()
        .join("target/vibe-package/launcher/launcher.bin");
    let original = std::fs::metadata(&output).unwrap().modified().unwrap();
    let completion = root
        .path()
        .join(".vibe/package-operations")
        .join(format!("{run_id}.static-file-result.json"));
    std::fs::write(completion, b"{\"corrupt\":true}").unwrap();
    drop(host);
    let cold = FixedPackageHost::from_trusted_host(project, namespace, scope).unwrap();
    assert!(matches!(cold.inspect("copy").unwrap(),
        Some(StaticFileJournalInspection::Uncertain { run_id: seen, .. }) if seen == run_id));
    assert!(matches!(cold.submit("copy").unwrap(),
        StaticFileJournalInspection::Uncertain { run_id: seen, .. } if seen == run_id));
    assert_eq!(
        std::fs::metadata(&output).unwrap().modified().unwrap(),
        original
    );
    assert_eq!(std::fs::read(output).unwrap(), b"first");
}
