//! Tests for path-source and git-source dispatch — the short-circuit
//! resolution order, identity / version-constraint refusals, and the
//! clone-free path-source fetch.

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-002#git-source");

use super::*;
use std::fs;
use tempfile::tempdir;

use crate::multi_registry_resolver::test_support::*;

use fixtures::*;

#[test]
fn resolve_dispatches_to_git_source_short_circuiting_registries() {
    // M1.15: a `[requires.packages]` git-source declaration bypasses
    // the registry walk for that pkgref. The resolver synthesises a
    // single-package registry pointing at `dep.url`, fetches the
    // manifest at the declared ref, returns
    // `MultiResolution { is_git_source: true, ... }`.
    let cache = tempdir().unwrap();
    let fake = Arc::new(FakeBackend::default());
    // Registry has nothing — would fail without git-source dispatch.
    // git-source URL has the manifest at v0.3.0 tag.
    let url = "git@host:owner/flow-internal.git";
    fake.seed_file(
        url,
        "v0.3.0",
        "vibe.toml",
        manifest_text("internal", "flow", "0.3.0").into_bytes(),
    );

    let dep = vibe_core::manifest::GitPackageDep {
        kind: Some(vibe_core::PackageKind::Flow),
        group: org(),
        name: "internal".to_string(),
        url: url.to_string(),
        ref_kind: vibe_core::manifest::GitRefKind::Tag("v0.3.0".to_string()),
        version: None,
        auth: vibe_core::manifest::AuthKind::None,
        token_env: None,
    };
    let r = build_resolver(cache.path(), vec![], vec![], vec![], fake).with_git_packages(vec![dep]);

    let p = PackageRef::parse("org.vibevm/internal").unwrap();
    let m = r.resolve(&p).expect("git-source resolution must succeed");
    assert!(m.is_git_source);
    assert!(!m.overridden);
    assert_eq!(m.registry_name, None);
    assert_eq!(m.source_url, url);
    assert_eq!(m.source_ref.as_deref(), Some("v0.3.0"));
    assert_eq!(m.resolved.version.to_string(), "0.3.0");
}

#[test]
fn resolve_git_source_rejects_name_mismatch() {
    // The repo's `vibe.toml` declares `org.vibevm/something-else`,
    // but the consumer's `[requires.packages]` declared
    // `org.vibevm/internal` pointing at this URL. Refuse — pulling
    // code under the wrong pkgref slot would silently misroute on
    // disk. `kind` is metadata (PROP-008 §2.3); the `name` mismatch
    // is what catches this.
    let cache = tempdir().unwrap();
    let fake = Arc::new(FakeBackend::default());
    let url = "git@host:owner/wrong-pkg.git";
    fake.seed_file(
        url,
        "v0.1.0",
        "vibe.toml",
        manifest_text("something-else", "feat", "0.1.0").into_bytes(),
    );
    let dep = vibe_core::manifest::GitPackageDep {
        kind: Some(vibe_core::PackageKind::Flow),
        group: org(),
        name: "internal".to_string(),
        url: url.to_string(),
        ref_kind: vibe_core::manifest::GitRefKind::Tag("v0.1.0".to_string()),
        version: None,
        auth: vibe_core::manifest::AuthKind::None,
        token_env: None,
    };
    let r = build_resolver(cache.path(), vec![], vec![], vec![], fake).with_git_packages(vec![dep]);

    let p = PackageRef::parse("org.vibevm/internal").unwrap();
    let err = r.resolve(&p).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("refusing to install"),
        "expected identity-mismatch refusal, got: {msg}"
    );
}

// ----- path-source (PROP-007 §2.5) ------------------------------

#[test]
fn resolve_dispatches_to_path_source_short_circuiting_registries() {
    // PROP-007 §2.5: a `[requires.packages]` path-source declaration
    // bypasses the registry walk for that pkgref. The resolver reads
    // the package's `vibe.toml` straight off the local directory and
    // returns `MultiResolution { is_path_source: true, ... }`.
    let cache = tempdir().unwrap();
    let ws = tempdir().unwrap();
    let fake = Arc::new(FakeBackend::default());
    // Registry has nothing — would fail without path-source dispatch.
    let pkg_dir = seed_path_package(ws.path(), "flow-internal", "internal", "flow", "0.3.0");

    let dep = ResolvedPathDep {
        kind: Some(vibe_core::PackageKind::Flow),
        group: org(),
        name: "internal".to_string(),
        version: None,
        package_dir: pkg_dir.clone(),
        workspace_rel: "flow-internal".to_string(),
    };
    let r =
        build_resolver(cache.path(), vec![], vec![], vec![], fake).with_path_packages(vec![dep]);

    let p = PackageRef::parse("org.vibevm/internal").unwrap();
    let m = r.resolve(&p).expect("path-source resolution must succeed");
    assert!(m.is_path_source);
    assert!(!m.is_git_source);
    assert!(!m.overridden);
    assert_eq!(m.registry_name, None);
    // source_url carries the workspace-relative path, never an
    // absolute path and never a URL.
    assert_eq!(m.source_url, "flow-internal");
    assert_eq!(m.source_ref, None);
    assert_eq!(m.resolved.version.to_string(), "0.3.0");
}

#[test]
fn resolve_path_source_rejects_name_mismatch() {
    // The package's `vibe.toml` declares `org.vibevm/something-else`,
    // but the consumer's `[requires.packages]` declared
    // `org.vibevm/internal` pointing at this directory. Refuse —
    // installing code under a misnamed slot would silently misroute
    // on disk. `kind` is metadata (PROP-008 §2.3); the `name`
    // mismatch is what catches this.
    let cache = tempdir().unwrap();
    let ws = tempdir().unwrap();
    let fake = Arc::new(FakeBackend::default());
    let pkg_dir = seed_path_package(ws.path(), "wrong-pkg", "something-else", "feat", "0.1.0");

    let dep = ResolvedPathDep {
        kind: Some(vibe_core::PackageKind::Flow),
        group: org(),
        name: "internal".to_string(),
        version: None,
        package_dir: pkg_dir,
        workspace_rel: "wrong-pkg".to_string(),
    };
    let r =
        build_resolver(cache.path(), vec![], vec![], vec![], fake).with_path_packages(vec![dep]);

    let p = PackageRef::parse("org.vibevm/internal").unwrap();
    let err = r.resolve(&p).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("refusing to install"),
        "expected identity-mismatch refusal, got: {msg}"
    );
}

#[test]
fn resolve_path_source_rejects_version_constraint_mismatch() {
    // The path-dep carried a dual-form `{ path, version }` constraint
    // that the package's own `[package].version` does not satisfy.
    // Refuse — same shape as the git-source version check.
    let cache = tempdir().unwrap();
    let ws = tempdir().unwrap();
    let fake = Arc::new(FakeBackend::default());
    let pkg_dir = seed_path_package(ws.path(), "flow-wal", "wal", "flow", "0.1.0");

    let dep = ResolvedPathDep {
        kind: Some(vibe_core::PackageKind::Flow),
        group: org(),
        name: "wal".to_string(),
        // Package is 0.1.0; constraint demands ^0.3 — mismatch.
        version: Some(VersionSpec::parse("^0.3").unwrap()),
        package_dir: pkg_dir,
        workspace_rel: "flow-wal".to_string(),
    };
    let r =
        build_resolver(cache.path(), vec![], vec![], vec![], fake).with_path_packages(vec![dep]);

    let p = PackageRef::parse("org.vibevm/wal").unwrap();
    let err = r.resolve(&p).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("does not satisfy the constraint"),
        "expected version-constraint refusal, got: {msg}"
    );
}

#[test]
fn resolve_path_source_wins_over_same_pkgref_git_source() {
    // PROP-007 §2.5 priority: a pkgref declared as BOTH path-source
    // and git-source resolves via path-source — path-source sits one
    // notch above git-source in the resolution order.
    let cache = tempdir().unwrap();
    let ws = tempdir().unwrap();
    let fake = Arc::new(FakeBackend::default());
    // path-source package: version 0.5.0.
    let pkg_dir = seed_path_package(ws.path(), "flow-dual", "dual", "flow", "0.5.0");
    // git-source for the SAME pkgref: a different version on a URL.
    let git_url = "git@host:owner/flow-dual.git";
    fake.seed_file(
        git_url,
        "v9.9.9",
        "vibe.toml",
        manifest_text("dual", "flow", "9.9.9").into_bytes(),
    );

    let path_dep = ResolvedPathDep {
        kind: Some(vibe_core::PackageKind::Flow),
        group: org(),
        name: "dual".to_string(),
        version: None,
        package_dir: pkg_dir,
        workspace_rel: "flow-dual".to_string(),
    };
    let git_dep = vibe_core::manifest::GitPackageDep {
        kind: Some(vibe_core::PackageKind::Flow),
        group: org(),
        name: "dual".to_string(),
        url: git_url.to_string(),
        ref_kind: vibe_core::manifest::GitRefKind::Tag("v9.9.9".to_string()),
        version: None,
        auth: vibe_core::manifest::AuthKind::None,
        token_env: None,
    };
    let r = build_resolver(cache.path(), vec![], vec![], vec![], fake)
        .with_git_packages(vec![git_dep])
        .with_path_packages(vec![path_dep]);

    let p = PackageRef::parse("org.vibevm/dual").unwrap();
    let m = r.resolve(&p).expect("path-source must win and resolve");
    assert!(m.is_path_source, "path-source must win over git-source");
    assert!(!m.is_git_source);
    // The path-source version (0.5.0), not the git-source (9.9.9).
    assert_eq!(m.resolved.version.to_string(), "0.5.0");
    assert_eq!(m.source_url, "flow-dual");
}

#[test]
fn fetch_path_source_copies_local_dir_and_computes_hash() {
    // PROP-007 §2.5: fetching a path-source package copies the local
    // directory's content into the per-project package cache,
    // excludes any `.git/`, and computes a content_hash over the
    // copied tree. No git clone happens.
    let cache = tempdir().unwrap();
    let pkg_cache = tempdir().unwrap();
    let ws = tempdir().unwrap();
    let fake = Arc::new(FakeBackend::default());

    // Path-source package with a regular file AND a `.git/` subtree
    // that must NOT make it into the cache.
    let pkg_dir = seed_path_package(ws.path(), "flow-local", "local", "flow", "0.2.0");
    fs::write(pkg_dir.join("README.md"), "# local package\n").unwrap();
    let git_dir = pkg_dir.join(".git");
    fs::create_dir_all(&git_dir).unwrap();
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();

    let dep = ResolvedPathDep {
        kind: Some(vibe_core::PackageKind::Flow),
        group: org(),
        name: "local".to_string(),
        version: None,
        package_dir: pkg_dir,
        workspace_rel: "flow-local".to_string(),
    };
    let r = build_resolver(cache.path(), vec![], vec![], vec![], fake.clone())
        .with_path_packages(vec![dep]);

    let p = PackageRef::parse("org.vibevm/local").unwrap();
    let resolution = r.resolve(&p).unwrap();
    let cached = r.fetch(&resolution, pkg_cache.path()).unwrap();

    assert!(cached.is_path_source);
    assert!(!cached.is_git_source);
    assert!(!cached.overridden);
    assert_eq!(cached.registry_name, None);
    assert_eq!(cached.source_ref, None);
    // source_uri is the workspace-relative path, recorded verbatim
    // as the lockfile `source_url` for a path entry.
    assert_eq!(cached.source_uri, "flow-local");
    assert_eq!(cached.package_meta().version.to_string(), "0.2.0");
    // Cache is populated with the package payload.
    assert!(cached.cache_dir.join("vibe.toml").exists());
    assert!(cached.cache_dir.join("README.md").exists());
    // `.git/` was excluded.
    assert!(!cached.cache_dir.join(".git").exists());
    // content_hash computed over the copied tree.
    assert!(cached.content_hash.starts_with("sha256:"));
    // No git clone — `bootstrap` was never invoked.
    assert_eq!(fake.bootstrap_count(), 0);
}

/// Test-only fixtures behind their own `#[cfg(test)]` marker: fact
/// extraction is per-file, and the no-unwrap rule scopes test code by
/// the enclosing `#[cfg(test)]` item — the marker keeps this helper
/// reading as test code now that the tests live outside the parent
/// module's inline `mod tests`.
#[cfg(test)]
mod fixtures {
    use super::*;

    /// Lay down a path-source package directory under `parent`:
    /// `<parent>/<dirname>/vibe.toml` carrying a `[package]` table.
    /// Returns the package directory.
    pub(super) fn seed_path_package(
        parent: &Path,
        dirname: &str,
        name: &str,
        kind: &str,
        version: &str,
    ) -> PathBuf {
        let dir = parent.join(dirname);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("vibe.toml"), manifest_text(name, kind, version)).unwrap();
        dir
    }
}
