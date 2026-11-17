use super::*;

#[test]
fn continued_payload_cannot_replay_same_manifest_and_hash_from_a_different_commit() {
    // Two real Git commits keep identical package bytes: content equality is
    // not permission to substitute the commit pinned by the personal lock.
    let repo = tempfile::tempdir().unwrap();
    let git = |args: &[&str]| {
        let result = std::process::Command::new("git")
            .current_dir(repo.path())
            .args(args)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        String::from_utf8(result.stdout).unwrap().trim().to_owned()
    };
    git(&["init", "--quiet"]);
    std::fs::write(
        repo.path().join("vibe.toml"),
        "[package]\ngroup='org.continuation'\nname='demo'\nkind='flow'\nversion='1.0.0'\n",
    )
    .unwrap();
    git(&["add", "vibe.toml"]);
    git(&[
        "-c",
        "user.name=Fixture",
        "-c",
        "user.email=fixture@example.invalid",
        "commit",
        "--quiet",
        "-m",
        "first",
    ]);
    let first = git(&["rev-parse", "HEAD"]);
    git(&[
        "-c",
        "user.name=Fixture",
        "-c",
        "user.email=fixture@example.invalid",
        "commit",
        "--quiet",
        "--allow-empty",
        "-m",
        "second",
    ]);
    let second = git(&["rev-parse", "HEAD"]);
    let row: LockedPackage = toml::from_str(&format!("group='org.continuation'\nname='demo'\nkind='flow'\nversion='1.0.0'\nsource_url='file:///fixture'\nsource_ref='v1.0.0'\ncontent_hash='sha256:aabbccdd'\nresolved_commit='{first}'\n")).unwrap();
    let held = LockedSelection {
        original: Arc::new(row.clone()),
        row,
        revision: LockedRevision::Replay,
    };
    assert!(
        held.verify("sha256:aabbccdd", Some(&first))
            .is_ok()
    );
    let error = held
        .verify("sha256:aabbccdd", Some(&second))
        .unwrap_err();
    assert!(error.to_string().contains("commit differs"));
    assert!(held.verify("sha256:11223344", Some(&first)).is_err());
}
