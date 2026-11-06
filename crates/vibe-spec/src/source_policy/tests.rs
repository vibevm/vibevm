use super::*;
use std::fs;
const PACKAGE: &str = "[package]\nname='demo'\ngroup='org.example'\nkind='flow'\nversion='1.0.0'\n";
#[test]
fn absence_legacy_empty_and_declared_are_distinct() {
    let root = tempfile::tempdir().unwrap();
    assert_eq!(
        observe_package_policy(root.path()).unwrap().mode(),
        PackageSourceMode::MissingManifest
    );
    fs::write(root.path().join("vibe.toml"), PACKAGE).unwrap();
    assert_eq!(
        observe_package_policy(root.path()).unwrap().mode(),
        PackageSourceMode::Legacy
    );
    fs::write(
        root.path().join("vibe.toml"),
        format!("{PACKAGE}sources=[]\n"),
    )
    .unwrap();
    let empty = observe_package_policy(root.path()).unwrap();
    assert_eq!(empty.mode(), PackageSourceMode::ExplicitEmpty);
    assert_eq!(empty.package_sources().unwrap(), []);
    fs::write(root.path().join("vibe.toml"), format!("{PACKAGE}[[package.sources]]\nfile='data.xml'\nmedia='application/xml'\ninterpretation='xml-data'\nrole='resource'\nloading='explicit'\n")).unwrap();
    assert!(empty.recheck_unchanged().is_err());
    assert_eq!(
        observe_package_policy(root.path()).unwrap().mode(),
        PackageSourceMode::Explicit
    );
}
#[test]
fn malformed_present_manifest_and_missing_becoming_present_refuse() {
    let root = tempfile::tempdir().unwrap();
    let absent = observe_package_policy(root.path()).unwrap();
    fs::write(root.path().join("vibe.toml"), PACKAGE).unwrap();
    assert_eq!(
        absent.recheck_unchanged().unwrap_err().kind,
        SourcePolicyErrorKind::Changed
    );
    fs::write(root.path().join("vibe.toml"), "[invalid").unwrap();
    assert_eq!(
        observe_package_policy(root.path()).unwrap_err().kind,
        SourcePolicyErrorKind::InvalidManifest
    );
}
#[test]
fn replacing_selected_logical_root_cannot_reuse_detached_old_handle() {
    let scope = tempfile::tempdir().unwrap();
    let root = scope.path().join("root");
    fs::create_dir(&root).unwrap();
    let observed = observe_package_policy(&root).unwrap();
    fs::rename(&root, scope.path().join("detached")).unwrap();
    fs::create_dir(&root).unwrap();
    assert_eq!(
        observed.recheck_unchanged().unwrap_err().kind,
        SourcePolicyErrorKind::Changed
    );
    assert_eq!(
        observed.read_source_bytes("data.md").unwrap_err().kind,
        SourcePolicyErrorKind::Changed
    );
}
#[test]
fn held_sources_allow_hardlinks_and_project_only_manifest_is_legacy() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("vibe.toml"),
        "[project]\nname='demo'\nversion='1.0.0'\n",
    )
    .unwrap();
    fs::write(root.path().join("original.md"), "authored").unwrap();
    fs::hard_link(
        root.path().join("original.md"),
        root.path().join("shared.md"),
    )
    .unwrap();
    let observed = observe_package_policy(root.path()).unwrap();
    assert_eq!(observed.mode(), PackageSourceMode::Legacy);
    assert_eq!(
        observed.read_source_bytes("shared.md").unwrap().unwrap(),
        b"authored"
    );
    observed.recheck_unchanged().unwrap();
}
#[cfg(unix)]
#[test]
fn selected_root_alias_retarget_and_symlink_source_refuse() {
    let scope = tempfile::tempdir().unwrap();
    let first = scope.path().join("first");
    let second = scope.path().join("second");
    fs::create_dir(&first).unwrap();
    fs::create_dir(&second).unwrap();
    let alias = scope.path().join("alias");
    std::os::unix::fs::symlink(&first, &alias).unwrap();
    let observed = observe_package_policy(&alias).unwrap();
    fs::write(first.join("real.md"), "raw").unwrap();
    std::os::unix::fs::symlink("real.md", first.join("link.md")).unwrap();
    assert!(observed.read_source_bytes("link.md").is_err());
    fs::remove_file(&alias).unwrap();
    std::os::unix::fs::symlink(&second, &alias).unwrap();
    assert_eq!(
        observed.recheck_unchanged().unwrap_err().kind,
        SourcePolicyErrorKind::Changed
    );
}

#[test]
fn detached_checkpoints_still_refuse_name_and_manifest_drift() {
    let scope = tempfile::tempdir().unwrap();
    let root = scope.path().join("root");
    fs::create_dir(&root).unwrap();
    let absent = observe_package_policy(&root).unwrap().into_checkpoint();
    fs::write(root.join("vibe.toml"), PACKAGE).unwrap();
    assert_eq!(
        absent.recheck_unchanged().unwrap_err().kind,
        SourcePolicyErrorKind::Changed
    );
    let observed = observe_package_policy(&root).unwrap().into_checkpoint();
    fs::rename(&root, scope.path().join("detached")).unwrap();
    fs::create_dir(&root).unwrap();
    fs::write(root.join("vibe.toml"), PACKAGE).unwrap();
    assert_eq!(
        observed.recheck_unchanged().unwrap_err().kind,
        SourcePolicyErrorKind::Changed
    );
}
#[cfg(unix)]
#[test]
fn detached_checkpoint_detects_selected_root_alias_retarget() {
    let scope = tempfile::tempdir().unwrap();
    let a = scope.path().join("a");
    let b = scope.path().join("b");
    fs::create_dir(&a).unwrap();
    fs::create_dir(&b).unwrap();
    let alias = scope.path().join("alias");
    std::os::unix::fs::symlink(&a, &alias).unwrap();
    let observed = observe_package_policy(&alias).unwrap().into_checkpoint();
    fs::remove_file(&alias).unwrap();
    std::os::unix::fs::symlink(&b, &alias).unwrap();
    assert_eq!(
        observed.recheck_unchanged().unwrap_err().kind,
        SourcePolicyErrorKind::Changed
    );
}
