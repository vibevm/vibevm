//! Committed installer bytes retained for the future local payload vault.

use super::*;

#[test]
fn retained_installers_are_exact_selected_commit_bytes_after_working_edit() {
    let world = World::new();
    let identity = snapshot::committed_identity(world.repo.path(), true).unwrap();
    std::fs::write(
        world.repo.path().join("distribution/install/install.sh"),
        b"dirty working installer\n",
    )
    .unwrap();
    assert!(
        world.capture().is_err(),
        "path-taking capture requires clean HEAD"
    );
    let output = Project::open(&world.out).unwrap();
    let capture =
        capture_prepared_subject_at_identity(world.repo.path(), &output, identity, smoke())
            .unwrap();
    let final_bytes = capture.final_payload_bytes();
    assert_eq!(final_bytes.bash, b"committed bash\n");
    assert_eq!(final_bytes.powershell, b"committed powershell\n");
    assert_eq!(final_bytes.cmd, b"committed cmd\n");
    assert_eq!(
        sha(final_bytes.bash),
        capture.subject.installers[0].asset.sha256
    );
    assert_eq!(
        sha(final_bytes.powershell),
        capture.subject.installers[1].asset.sha256
    );
    assert_eq!(
        sha(final_bytes.cmd),
        capture.subject.installers[2].asset.sha256
    );
    assert_eq!(sha(final_bytes.aggregate), capture.subject.aggregate.sha256);
    assert_eq!(
        final_bytes.bash.len().to_string(),
        capture.subject.installers[0].asset.size_bytes,
    );
}

#[test]
fn absent_installer_in_selected_commit_refuses_before_subject_capture() {
    let world = World::new();
    std::fs::remove_file(world.repo.path().join("distribution/install/install.ps1")).unwrap();
    run_git(world.repo.path(), &["add", "-u"]);
    run_git(
        world.repo.path(),
        &["commit", "--quiet", "-m", "remove fixture installer"],
    );
    let identity = snapshot::committed_identity(world.repo.path(), true).unwrap();
    let output = Project::open(&world.out).unwrap();
    let result =
        capture_prepared_subject_at_identity(world.repo.path(), &output, identity, smoke());
    let error = match result {
        Ok(_) => panic!("missing selected commit installer was accepted"),
        Err(error) => error.to_string(),
    };
    assert!(
        error.contains("distribution/install/install.ps1"),
        "{error}"
    );
}
