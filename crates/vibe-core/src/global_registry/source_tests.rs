//! Demand boundaries of the operation-owned registry document.
use super::GlobalRegistryConfig;
use super::source::GlobalRegistrySource;

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE")]
fn deferred_source_does_not_read_and_policy_ignores_unrelated_invalid_auth() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("registry.toml");
    let source =
        GlobalRegistrySource::deferred(Some(path.clone()), GlobalRegistryConfig::default());
    assert!(source.known().is_none());
    std::fs::write(
        &path,
        "[[registry]]\nname='broken'\nurl=42\nauth={wrong=true}\n",
    )
    .unwrap();
    assert!(source.policy().unwrap().exclude_hosting.is_empty());
    assert!(source.catalog().is_err());
    // Both demand paths consume one frozen document, not a later file version.
    std::fs::write(&path, "").unwrap();
    assert!(source.catalog().is_err());
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE")]
fn missing_catalog_uses_memory_defaults_without_creating_file() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("missing/registry.toml");
    let source =
        GlobalRegistrySource::deferred(Some(path.clone()), GlobalRegistryConfig::default());
    assert!(source.catalog().unwrap().registries.is_empty());
    assert!(!path.exists());
}

#[cfg(unix)]
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE")]
fn ordinary_group_writable_registry_remains_valid_but_special_file_refuses() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("registry.toml");
    std::fs::write(&path, "").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o664)).unwrap();
    assert!(
        GlobalRegistrySource::deferred(Some(path.clone()), Default::default())
            .policy()
            .is_ok()
    );
    std::fs::remove_file(&path).unwrap();
    // The owned FIFO has no writer; a blocking open would hang this fixture.
    let created = std::process::Command::new("mkfifo")
        .args(["-m", "600"])
        .arg(&path)
        .status()
        .unwrap();
    assert!(created.success());
    assert!(
        GlobalRegistrySource::deferred(Some(path), Default::default())
            .policy()
            .is_err()
    );
}
