//! Tests for the clone-free lookup half — tag-shaped version listing,
//! version resolution, and the archive-first dep-manifest read with
//! its clone fallback, all mirror-aware.

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-002#registry-model");

use super::*;
use tempfile::tempdir;

use crate::git_package_registry::test_support::*;

#[test]
fn list_versions_filters_non_semver_and_sorts() {
    let cache = tempdir().unwrap();
    let fake = Arc::new(FakeBackend::default());
    let url = "git@host:org/org.vibevm_wal.git";
    fake.seed_tags(
        url,
        vec![
            "v0.2.0".into(),
            "v0.1.0".into(),
            "v0.10.0".into(),
            "release-foo".into(),
            "v1.0.0-rc.1".into(),
            "draft".into(),
            "1.2.3".into(), // missing `v` prefix — dropped
        ],
    );
    let r = registry_with(cache.path(), "git@host:org", NamingConvention::Fqdn, fake);
    let versions = r.list_versions(&org(), "wal").unwrap();
    let strs: Vec<String> = versions.iter().map(|v| v.to_string()).collect();
    assert_eq!(strs, vec!["0.1.0", "0.2.0", "0.10.0", "1.0.0-rc.1"]);
}

#[test]
fn list_versions_empty_when_repo_has_no_tags() {
    let cache = tempdir().unwrap();
    let fake = Arc::new(FakeBackend::default());
    fake.seed_tags("git@host:org/org.vibevm_wal.git", vec![]);
    let r = registry_with(cache.path(), "git@host:org", NamingConvention::Fqdn, fake);
    let v = r.list_versions(&org(), "wal").unwrap();
    assert!(v.is_empty());
}

#[test]
fn list_versions_repo_not_found_translates_to_unknown_package() {
    let cache = tempdir().unwrap();
    let fake = Arc::new(FakeBackend::default());
    // No seed for the URL → FakeBackend returns RepoNotFound.
    let r = registry_with(cache.path(), "git@host:org", NamingConvention::Fqdn, fake);
    let err = r.list_versions(&org(), "ghost").unwrap_err();
    assert!(matches!(err, RegistryError::UnknownPackage { .. }));
}

#[test]
fn list_versions_falls_through_to_mirror_when_primary_unreachable() {
    // Primary's per-package URL is NOT seeded — primary returns
    // RepoNotFound. Mirror's URL IS seeded with tags. Mirror
    // dispatch should pick up the tag list from the mirror.
    let cache = tempdir().unwrap();
    let fake = Arc::new(FakeBackend::default());
    fake.seed_tags(
        "https://mirror.example/vibespecs/org.vibevm_wal.git",
        vec!["v0.1.0".into(), "v0.2.0".into()],
    );
    let r = registry_with_mirrors(
        cache.path(),
        "https://primary.example/vibespecs",
        NamingConvention::Fqdn,
        vec!["https://mirror.example/vibespecs".to_string()],
        fake,
    );
    let versions = r.list_versions(&org(), "wal").unwrap();
    assert_eq!(versions.len(), 2);
    assert_eq!(versions[0].to_string(), "0.1.0");
    assert_eq!(versions[1].to_string(), "0.2.0");
}

#[test]
fn list_versions_prefers_primary_when_both_seeded() {
    // Primary has [v0.1.0] only; mirror has [v0.1.0, v0.2.0].
    // Primary wins because it answers first; the mirror is never
    // consulted. The user's lockfile thus reflects what the
    // canonical source publishes — mirrors don't get to introduce
    // versions the primary doesn't carry.
    let cache = tempdir().unwrap();
    let fake = Arc::new(FakeBackend::default());
    fake.seed_tags(
        "https://primary.example/vibespecs/org.vibevm_wal.git",
        vec!["v0.1.0".into()],
    );
    fake.seed_tags(
        "https://mirror.example/vibespecs/org.vibevm_wal.git",
        vec!["v0.1.0".into(), "v0.2.0".into()],
    );
    let r = registry_with_mirrors(
        cache.path(),
        "https://primary.example/vibespecs",
        NamingConvention::Fqdn,
        vec!["https://mirror.example/vibespecs".to_string()],
        fake,
    );
    let versions = r.list_versions(&org(), "wal").unwrap();
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].to_string(), "0.1.0");
}

#[test]
fn list_versions_returns_primary_error_when_all_urls_fail() {
    // Neither primary nor mirror is seeded. The result is
    // UnknownPackage from the *primary's* `RepoNotFound` — that's
    // the canonical "the package doesn't exist" diagnostic.
    let cache = tempdir().unwrap();
    let fake = Arc::new(FakeBackend::default());
    let r = registry_with_mirrors(
        cache.path(),
        "https://primary.example/vibespecs",
        NamingConvention::Fqdn,
        vec!["https://mirror.example/vibespecs".to_string()],
        fake,
    );
    let err = r.list_versions(&org(), "ghost").unwrap_err();
    assert!(matches!(err, RegistryError::UnknownPackage { .. }));
}

#[test]
fn list_versions_walks_mirrors_in_priority_order() {
    // Three mirrors, only the third is seeded. The dispatcher
    // should iterate through mirrors[0], mirrors[1], mirrors[2]
    // and find the answer on mirrors[2]. Mirror order is the
    // caller's responsibility (MultiRegistryResolver does the
    // priority sort) — at this layer we only verify left-to-right
    // dispatch.
    let cache = tempdir().unwrap();
    let fake = Arc::new(FakeBackend::default());
    fake.seed_tags(
        "https://m3.example/vibespecs/org.vibevm_wal.git",
        vec!["v0.3.0".into()],
    );
    let r = registry_with_mirrors(
        cache.path(),
        "https://primary.example/vibespecs",
        NamingConvention::Fqdn,
        vec![
            "https://m1.example/vibespecs".to_string(),
            "https://m2.example/vibespecs".to_string(),
            "https://m3.example/vibespecs".to_string(),
        ],
        fake,
    );
    let versions = r.list_versions(&org(), "wal").unwrap();
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].to_string(), "0.3.0");
}

#[test]
fn resolve_picks_latest_stable() {
    let cache = tempdir().unwrap();
    let fake = Arc::new(FakeBackend::default());
    fake.seed_tags(
        "git@host:org/org.vibevm_wal.git",
        vec!["v0.1.0".into(), "v0.2.0".into(), "v1.0.0-rc.1".into()],
    );
    let r = registry_with(cache.path(), "git@host:org", NamingConvention::Fqdn, fake);
    let p = PackageRef::parse("org.vibevm/wal").unwrap();
    let resolved = r.resolve(&p).unwrap();
    // 1.0.0-rc.1 is pre-release; latest stable wins.
    assert_eq!(resolved.version.to_string(), "0.2.0");
}

#[test]
fn resolve_picks_exact_version() {
    let cache = tempdir().unwrap();
    let fake = Arc::new(FakeBackend::default());
    fake.seed_tags(
        "git@host:org/org.vibevm_wal.git",
        vec!["v0.1.0".into(), "v0.2.0".into(), "v0.3.0".into()],
    );
    let r = registry_with(cache.path(), "git@host:org", NamingConvention::Fqdn, fake);
    let p = PackageRef::parse("org.vibevm/wal@0.2.0").unwrap();
    let resolved = r.resolve(&p).unwrap();
    assert_eq!(resolved.version.to_string(), "0.2.0");
}

#[test]
fn resolve_picks_range() {
    let cache = tempdir().unwrap();
    let fake = Arc::new(FakeBackend::default());
    fake.seed_tags(
        "git@host:org/org.vibevm_wal.git",
        vec!["v0.1.0".into(), "v0.1.5".into(), "v0.2.0".into()],
    );
    let r = registry_with(cache.path(), "git@host:org", NamingConvention::Fqdn, fake);
    let p = PackageRef::parse("org.vibevm/wal@^0.1").unwrap();
    let resolved = r.resolve(&p).unwrap();
    assert_eq!(resolved.version.to_string(), "0.1.5");
}

#[test]
fn resolve_no_match_errors() {
    let cache = tempdir().unwrap();
    let fake = Arc::new(FakeBackend::default());
    fake.seed_tags("git@host:org/org.vibevm_wal.git", vec!["v0.1.0".into()]);
    let r = registry_with(cache.path(), "git@host:org", NamingConvention::Fqdn, fake);
    let p = PackageRef::parse("org.vibevm/wal@^9.0").unwrap();
    let err = r.resolve(&p).unwrap_err();
    assert!(matches!(err, RegistryError::NoMatchingVersion { .. }));
}

#[test]
fn fetch_dep_manifest_falls_through_to_mirror_on_archive_path() {
    // Primary's archive endpoint is empty (FakeBackend returns
    // FileNotFoundInRef). Mirror's archive endpoint has the
    // manifest. Dispatch should hit the mirror and return the
    // manifest without a clone.
    let cache = tempdir().unwrap();
    let fake = Arc::new(FakeBackend::default());
    let primary_url = "https://primary.example/vibespecs/org.vibevm_wal.git";
    let mirror_url = "https://mirror.example/vibespecs/org.vibevm_wal.git";
    // Tag list seeded only on the mirror — list_versions will land
    // on the mirror first too.
    fake.seed_tags(mirror_url, vec!["v0.1.0".into()]);
    fake.seed_file(
        mirror_url,
        "v0.1.0",
        "vibe.toml",
        manifest_text("wal", "flow", "0.1.0").into_bytes(),
    );
    let _ = primary_url; // documented for reading the test
    let r = registry_with_mirrors(
        cache.path(),
        "https://primary.example/vibespecs",
        NamingConvention::Fqdn,
        vec!["https://mirror.example/vibespecs".to_string()],
        fake.clone(),
    );
    let v = semver::Version::parse("0.1.0").unwrap();
    let manifest = r.fetch_dep_manifest(&org(), "wal", &v).unwrap();
    assert_eq!(manifest.require_package().unwrap().name, "wal");
    // No clone — the mirror served the manifest via the archive
    // path, same as the primary-only test asserts.
    assert_eq!(fake.bootstrap_count(), 0);
    assert_eq!(fake.update_count(), 0);
}

#[test]
fn fetch_dep_manifest_reads_via_archive_without_clone() {
    let cache = tempdir().unwrap();
    let fake = Arc::new(FakeBackend::default());
    let url = "git@host:org/org.vibevm_wal.git";
    fake.seed_tags(url, vec!["v0.1.0".into()]);
    fake.seed_file(
        url,
        "v0.1.0",
        "vibe.toml",
        manifest_text("wal", "flow", "0.1.0").into_bytes(),
    );
    let r = registry_with(
        cache.path(),
        "git@host:org",
        NamingConvention::Fqdn,
        fake.clone(),
    );
    let v = semver::Version::parse("0.1.0").unwrap();
    let manifest = r.fetch_dep_manifest(&org(), "wal", &v).unwrap();
    assert_eq!(manifest.require_package().unwrap().name, "wal");
    assert_eq!(
        manifest.require_package().unwrap().version.to_string(),
        "0.1.0"
    );
    // Critically: no clone was triggered for this manifest read.
    assert_eq!(fake.bootstrap_count(), 0);
    assert_eq!(fake.update_count(), 0);
}

#[test]
fn fetch_dep_manifest_clone_fallback_uses_mirror_dispatch() {
    // GitHub-shape host: archive endpoint is unsupported. The
    // dep-manifest fetch falls back to the per-package clone via
    // refresh_package. With mirror dispatch wired, the clone walk
    // tries primary then mirror — mirror seeded, primary not.
    let cache = tempdir().unwrap();
    let upstream = tempdir().unwrap();
    let pkg_root = upstream.path().join("pkg");
    fs::create_dir_all(&pkg_root).unwrap();
    fs::write(
        pkg_root.join("vibe.toml"),
        manifest_text("wal", "flow", "0.1.0"),
    )
    .unwrap();

    let primary_url = "https://primary.example/vibespecs/org.vibevm_wal.git";
    let mirror_url = "https://mirror.example/vibespecs/org.vibevm_wal.git";

    // FakeBackend's `fetch_file_at_ref` returns FileNotFoundInRef
    // when not seeded. To trigger the clone fallback we need an
    // ArchiveUnsupported. Build a dedicated backend variant.
    struct NoArchiveBackend(Arc<FakeBackend>);
    impl GitBackend for NoArchiveBackend {
        fn bootstrap(&self, url: &str, refname: &str, dest: &Path) -> Result<(), GitError> {
            self.0.bootstrap(url, refname, dest)
        }
        fn update(&self, dest: &Path, refname: &str) -> Result<(), GitError> {
            self.0.update(dest, refname)
        }
        fn list_tags(&self, url: &str) -> Result<Vec<String>, GitError> {
            self.0.list_tags(url)
        }
        fn fetch_file_at_ref(
            &self,
            url: &str,
            _refname: &str,
            _path: &str,
        ) -> Result<Vec<u8>, GitError> {
            Err(GitError::ArchiveUnsupported {
                url: url.to_string(),
            })
        }
    }

    let inner = Arc::new(FakeBackend::default());
    inner.seed_tags(primary_url, vec!["v0.1.0".into()]);
    inner.seed_tags(mirror_url, vec!["v0.1.0".into()]);
    inner.seed_bootstrap(mirror_url, pkg_root.clone());
    // Primary has no bootstrap seed → primary's clone fails →
    // mirror takes over.

    let backend: Arc<dyn GitBackend> = Arc::new(NoArchiveBackend(inner.clone()));
    let r = GitPackageRegistry::open_with_mirrors(
        "vibespecs",
        "https://primary.example/vibespecs",
        "main",
        NamingConvention::Fqdn,
        vec!["https://mirror.example/vibespecs".to_string()],
        cache.path(),
        backend,
        DEFAULT_FRESHNESS_SECS,
    )
    .unwrap();

    let v = semver::Version::parse("0.1.0").unwrap();
    let manifest = r.fetch_dep_manifest(&org(), "wal", &v).unwrap();
    assert_eq!(manifest.require_package().unwrap().name, "wal");

    // Clone-fallback walked primary (fail) + mirror (ok).
    assert_eq!(
        inner.bootstrap_urls(),
        vec![primary_url.to_string(), mirror_url.to_string()]
    );
}
