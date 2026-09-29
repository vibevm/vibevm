specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#MANIFEST-MIGRATION-CELL");
use super::*;
use std::path::Path;
use vibe_wire::generated::migration::e1::operation_request::{Authority, AuthorityScope};

fn fixture() -> (tempfile::TempDir, PathBuf, PathBuf, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let project = root.path().join("project");
    std::fs::create_dir(&project).unwrap();
    std::fs::write(project.join("sentinel"), b"unchanged").unwrap();
    let cache = root.path().join("cache");
    let recovery = root.path().join("recovery");
    (root, project, cache, recovery)
}
fn authority(project: &Path, recovery: &Path, file: &str) -> Authority {
    Authority {
        project_root: project.to_str().unwrap().into(),
        recovery_store_root: recovery.to_str().unwrap().into(),
        target_relative: file.into(),
        scope: AuthorityScope::OwnedAuthoredFile,
    }
}
fn snapshot(root: &Path) -> Vec<(PathBuf, String, Vec<u8>)> {
    fn walk(root: &Path, current: &Path, rows: &mut Vec<(PathBuf, String, Vec<u8>)>) {
        for entry in std::fs::read_dir(current).unwrap() {
            let path = entry.unwrap().path();
            let metadata = std::fs::symlink_metadata(&path).unwrap();
            let relative = path.strip_prefix(root).unwrap().to_owned();
            if metadata.file_type().is_symlink() {
                rows.push((
                    relative,
                    "link".into(),
                    std::fs::read_link(&path)
                        .unwrap()
                        .to_string_lossy()
                        .as_bytes()
                        .to_vec(),
                ));
            } else if metadata.is_dir() {
                rows.push((relative, "directory".into(), Vec::new()));
                walk(root, &path, rows);
            } else {
                rows.push((relative, "file".into(), std::fs::read(&path).unwrap()));
            }
        }
    }
    let mut rows = Vec::new();
    walk(root, root, &mut rows);
    rows.sort();
    rows
}
fn link_dir(target: &Path, link: &Path) -> bool {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link).unwrap();
        true
    }
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_dir(target, link).is_ok()
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (target, link);
        false
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#MANIFEST-MIGRATION-CELL",
    r = 1
)]
fn default_context_refuses_without_creation() {
    let (root, project, cache, recovery) = fixture();
    let before = snapshot(root.path());
    let ctx = ServerContext::with_store_root(&project, &cache);
    let err = ctx
        .grant_for_manifest_migration(&authority(&project, &recovery, "vibe.toml"))
        .unwrap_err();
    assert_eq!(err.field, "authority");
    assert_eq!(snapshot(root.path()), before);
    assert!(!cache.exists());
    assert!(!recovery.exists());
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#MANIFEST-MIGRATION-CELL",
    r = 1
)]
fn configured_default_and_explicit_members_admit_only_their_files() {
    let (root, project, cache, recovery) = fixture();
    let before = snapshot(root.path());
    let ctx = ServerContext::with_store_root(&project, &cache)
        .with_manifest_migration_authority(&recovery, Vec::<String>::new())
        .unwrap();
    let grant = ctx
        .grant_for_manifest_migration(&authority(&project, &recovery, "vibe.toml"))
        .unwrap();
    assert_eq!(grant.project().root_path(), project);
    assert_eq!(grant.recovery_store_root(), recovery);
    assert_eq!(grant.target_relative(), "vibe.toml");
    assert!(
        ctx.grant_for_manifest_migration(&authority(&project, &recovery, "member/vibe.toml"))
            .is_err()
    );
    let ctx = ServerContext::with_store_root(&project, &cache)
        .with_manifest_migration_authority(&recovery, ["member/vibe.toml", "member/vibe.toml"])
        .unwrap();
    assert!(
        ctx.grant_for_manifest_migration(&authority(&project, &recovery, "member/vibe.toml"))
            .is_ok()
    );
    assert!(
        ctx.grant_for_manifest_migration(&authority(&project, &recovery, "vibe.toml"))
            .is_err()
    );
    assert_eq!(snapshot(root.path()), before);
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#MANIFEST-MIGRATION-CELL",
    r = 1
)]
fn foreign_and_relative_authority_refuses_before_open() {
    let (root, project, cache, recovery) = fixture();
    let before = snapshot(root.path());
    let ctx = ServerContext::with_store_root(&project, &cache)
        .with_manifest_migration_authority(&recovery, Vec::<String>::new())
        .unwrap();
    let mut a = authority(&project, &recovery, "vibe.toml");
    a.project_root = root.path().join("foreign-absent").to_str().unwrap().into();
    assert!(ctx.grant_for_manifest_migration(&a).is_err());
    let mut a = authority(&project, &recovery, "vibe.toml");
    a.recovery_store_root = root.path().join("foreign-store").to_str().unwrap().into();
    assert!(ctx.grant_for_manifest_migration(&a).is_err());
    for file in [
        "other.toml",
        "../vibe.toml",
        "/vibe.toml",
        "docs/../../escape",
        "COM1.json",
    ] {
        assert!(
            ctx.grant_for_manifest_migration(&authority(&project, &recovery, file))
                .is_err()
        );
    }
    for (p, c, r) in [
        (PathBuf::from("relative"), cache.clone(), recovery.clone()),
        (project.clone(), PathBuf::from("relative"), recovery.clone()),
        (project.clone(), cache.clone(), PathBuf::from("relative")),
    ] {
        assert!(
            ServerContext::with_store_root(p, c)
                .with_manifest_migration_authority(r, Vec::<String>::new())
                .is_err()
        );
    }
    for file in ["../escape", "/escape", "COM1.json", ".vibe-stage-foreign"] {
        assert!(
            ServerContext::with_store_root(&project, &cache)
                .with_manifest_migration_authority(&recovery, [file])
                .is_err()
        );
    }
    assert_eq!(snapshot(root.path()), before);
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#MANIFEST-MIGRATION-CELL",
    r = 1
)]
fn cache_overlap_and_nofollow_aliases_refuse_without_effects() {
    let (root, project, cache, recovery) = fixture();
    std::fs::create_dir(&cache).unwrap();
    std::fs::create_dir(&recovery).unwrap();
    let before = snapshot(root.path());
    assert!(
        ServerContext::with_store_root(&project, &cache)
            .with_manifest_migration_authority(&recovery, ["vibe.toml"])
            .is_ok()
    );
    for r in [
        cache.clone(),
        cache.join("missing/suffix"),
        root.path().to_owned(),
    ] {
        assert!(
            ServerContext::with_store_root(&project, &cache)
                .with_manifest_migration_authority(r, ["vibe.toml"])
                .is_err()
        );
    }
    assert_eq!(snapshot(root.path()), before);
    let alias = root.path().join("cache-alias");
    if link_dir(&cache, &alias) {
        let before = snapshot(root.path());
        assert!(
            ServerContext::with_store_root(&project, &alias)
                .with_manifest_migration_authority(&recovery, ["vibe.toml"])
                .is_err()
        );
        assert!(
            ServerContext::with_store_root(&project, &cache)
                .with_manifest_migration_authority(alias.join("missing"), ["vibe.toml"])
                .is_err()
        );
        assert_eq!(snapshot(root.path()), before);
    }
    let bad_cache = root.path().join("cache-file");
    std::fs::write(&bad_cache, b"foreign").unwrap();
    let before = snapshot(root.path());
    assert!(
        ServerContext::with_store_root(&project, &bad_cache)
            .with_manifest_migration_authority(&recovery, ["vibe.toml"])
            .is_err()
    );
    assert_eq!(snapshot(root.path()), before);
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#MANIFEST-MIGRATION-CELL",
    r = 1
)]
fn missing_cache_is_not_initialized_and_policy_rechecks() {
    let (root, project, cache, recovery) = fixture();
    let before = snapshot(root.path());
    let ctx = ServerContext::with_store_root(&project, &cache)
        .with_manifest_migration_authority(&recovery, ["vibe.toml"])
        .unwrap();
    let grant = ctx
        .grant_for_manifest_migration(&authority(&project, &recovery, "vibe.toml"))
        .unwrap();
    grant.validate_recovery_store_policy().unwrap();
    assert_eq!(snapshot(root.path()), before);
    assert!(!cache.exists());
    assert!(!recovery.exists());
    std::fs::create_dir(&cache).unwrap();
    grant.validate_recovery_store_policy().unwrap();
    if link_dir(&cache, &recovery) {
        let before = snapshot(root.path());
        assert!(grant.validate_recovery_store_policy().is_err());
        assert_eq!(snapshot(root.path()), before);
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#MANIFEST-MIGRATION-CELL",
    r = 1
)]
fn lifecycle_builders_preserve_explicit_migration_authority() {
    let (root, project, cache, recovery) = fixture();
    let before = snapshot(root.path());
    let ctx = ServerContext::with_store_root(&project, &cache)
        .with_manifest_migration_authority(&recovery, ["vibe.toml"])
        .unwrap()
        .with_lifecycle_execution(
            InstallPolicy {
                offline: true,
                ..InstallPolicy::default()
            },
            Some(root.path().join("embedded")),
            true,
        );
    assert!(ctx.lifecycle_policy.offline);
    assert!(ctx.seed_default_registry);
    assert!(
        ctx.grant_for_manifest_migration(&authority(&project, &recovery, "vibe.toml"))
            .is_ok()
    );
    let mut ctx = ctx;
    ctx.store_root = root.path().join("changed-cache");
    assert!(
        ctx.grant_for_manifest_migration(&authority(&project, &recovery, "vibe.toml"))
            .is_err()
    );
    let ctx = ServerContext::with_store_root(&project, &cache)
        .with_lifecycle_execution(InstallPolicy::default(), None, false)
        .with_manifest_migration_authority(&recovery, ["vibe.toml"])
        .unwrap();
    assert!(
        ctx.grant_for_manifest_migration(&authority(&project, &recovery, "vibe.toml"))
            .is_ok()
    );
    assert_eq!(snapshot(root.path()), before);
}
