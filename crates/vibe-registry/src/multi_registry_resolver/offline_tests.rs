//! Store-backed resolution oracles (PROP-010 §2.6): the offline
//! posture (versions and manifests from the machine store, zero
//! network), the hard offline miss with recovery recipes, the
//! availability fallback (a store hit outranks a registry that no
//! longer lists the version), and the error-semantics guard (an
//! operational failure is never masked by the store).
//!
//! The store rides in as a builder parameter (`with_store_root`) — the
//! same parameter isolation the whole store layer uses — so no test
//! here touches the real `~/.vibe`.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-model");

use std::fs;
use std::path::{Path, PathBuf};

use tempfile::tempdir;

use super::test_support::*;
use super::*;
use crate::vendor::file_url_for_dir;

/// A lockfile entry for `org.vibevm/wal` at `version`, served from
/// `source_url` — the provenance the store-backed resolution re-uses.
/// Built by TOML so the test states the wire shape, not 20 struct
/// fields.
fn locked_wal(version: &str, source_url: &str) -> vibe_core::manifest::LockedPackage {
    toml::from_str(&format!(
        r#"
kind = "flow"
group = "org.vibevm"
name = "wal"
version = "{version}"
registry = "vibespecs"
source_url = "{source_url}"
source_ref = "v{version}"
content_hash = "sha256:1111111111111111111111111111111111111111111111111111111111111111"
"#
    ))
    .unwrap()
}

/// Lay down a store entry for `(org.vibevm, wal) at `version` under
/// `root` — the documented layout `<root>/<group>/<name>/v<version>/`
/// with a manifest and a payload file.
fn seed_store_entry(root: &Path, version: &str, body: &str) -> PathBuf {
    let entry = root
        .join("org.vibevm")
        .join("wal")
        .join(format!("v{version}"));
    fs::create_dir_all(&entry).unwrap();
    fs::write(
        entry.join("vibe.toml"),
        manifest_text("wal", "flow", version),
    )
    .unwrap();
    fs::write(entry.join("PAYLOAD.md"), body).unwrap();
    entry
}

/// Lay down a local-directory registry carrying `org.vibevm/wal` at
/// the listed versions, and return its `file://` URL.
fn seed_local_registry(root: &Path, versions: &[&str]) -> String {
    for v in versions {
        let pkg = root
            .join("registry")
            .join("org.vibevm")
            .join("wal")
            .join(format!("v{v}"));
        fs::create_dir_all(&pkg).unwrap();
        fs::write(pkg.join("vibe.toml"), manifest_text("wal", "flow", v)).unwrap();
    }
    file_url_for_dir(&root.join("registry"))
}

/// PROP-010 §2.6 (`RESOLVER-OFFLINE-MODE`): with ZERO registries
/// configured, the offline posture resolves from the machine store —
/// version, manifest provenance off the lock entry — and the fetch
/// short-circuits to the entry's bytes. No git backend call happens
/// (the fake's counters stay at zero).
#[test]
fn offline_resolves_and_fetches_from_the_store_with_zero_registries() {
    let cache = tempdir().unwrap();
    let store_root = tempdir().unwrap();
    let entry = seed_store_entry(store_root.path(), "0.2.0", "store bytes\n");
    let fake = Arc::new(FakeBackend::default());

    let r = build_resolver(cache.path(), vec![], vec![], vec![], fake.clone())
        .with_offline(true)
        .with_store_root(store_root.path().to_path_buf())
        .with_locked_packages(vec![locked_wal(
            "0.2.0",
            "https://registry.example/wal.git",
        )]);

    // The solver's candidate set: the store's locked version.
    assert_eq!(r.list_versions(&org(), "wal").unwrap(), vec![v("0.2.0")]);

    let resolution = r
        .resolve(&PackageRef::parse("org.vibevm/wal@=0.2.0").unwrap())
        .unwrap();
    assert!(resolution.from_store, "the resolution must be store-backed");
    assert_eq!(resolution.resolved.version, v("0.2.0"));
    assert_eq!(resolution.resolved.source_dir, entry);
    assert_eq!(
        resolution.source_url, "https://registry.example/wal.git",
        "provenance comes from the lock entry, never minted"
    );

    // Fetch short-circuits to the entry — bytes off disk, no source walk.
    let cached = r
        .fetch_with_expected_hash(&resolution, store_root.path(), None)
        .unwrap();
    assert_eq!(cached.cache_dir, entry);
    assert_eq!(
        fs::read_to_string(cached.cache_dir.join("PAYLOAD.md")).unwrap(),
        "store bytes\n"
    );
    assert!(cached.content_hash.starts_with("sha256:"));
    // Zero network: the fake backend was never consulted.
    assert_eq!(fake.bootstrap_count(), 0);
}

/// PROP-010 §2.5 (`OFFLINE-HARD-ERROR`): an offline miss is a hard
/// error that names the package and version and hands the operator the
/// three recovery recipes — never a silent degrade.
#[test]
fn offline_miss_is_a_hard_error_naming_the_package_and_recipes() {
    let cache = tempdir().unwrap();
    let store_root = tempdir().unwrap(); // warm for nothing: no entries
    let fake = Arc::new(FakeBackend::default());

    let r = build_resolver(cache.path(), vec![], vec![], vec![], fake)
        .with_offline(true)
        .with_store_root(store_root.path().to_path_buf())
        .with_locked_packages(vec![locked_wal(
            "0.2.0",
            "https://registry.example/wal.git",
        )]);

    let err = r
        .resolve(&PackageRef::parse("org.vibevm/wal@=0.2.0").unwrap())
        .unwrap_err();
    match &err {
        RegistryError::OfflinePackageUnavailable { group, name, req } => {
            assert_eq!(group.as_str(), "org.vibevm");
            assert_eq!(name, "wal");
            assert_eq!(req, "=0.2.0");
        }
        other => panic!("expected OfflinePackageUnavailable, got {other:?}"),
    }
    let msg = err.to_string();
    assert!(
        msg.contains("org.vibevm/wal@=0.2.0"),
        "names the package: {msg}"
    );
    assert!(
        msg.contains("once online"),
        "recipe: run once online — {msg}"
    );
    assert!(
        msg.contains("vibe cache add org.vibevm/wal"),
        "recipe: cache add — {msg}"
    );
    assert!(
        msg.contains("vibe registry vendor"),
        "recipe: vendor — {msg}"
    );
}

/// THE canonical rule of `A-CACHE-HIT-IS-AUTHORITATIVE-FOR-AVAILABILITY`
/// (PROP-010 §2.6): a version that was installed earlier (lock entry +
/// store entry), then deleted from the serving registry, still
/// installs — the registry answered "no such version", and the store
/// outranks that silence. Online posture throughout.
#[test]
fn availability_fallback_resolves_a_version_the_registry_dropped() {
    let cache = tempdir().unwrap();
    let store_root = tempdir().unwrap();
    let outer = tempdir().unwrap();
    let entry = seed_store_entry(store_root.path(), "0.2.0", "store bytes\n");
    // The registry now serves only 0.1.0 — 0.2.0 was deleted upstream
    // after the install that warmed the store and wrote the lock.
    let reg_url = seed_local_registry(outer.path(), &["0.1.0"]);
    let fake = Arc::new(FakeBackend::default());

    let r = build_resolver(
        cache.path(),
        vec![registry_section("local", &reg_url)],
        vec![],
        vec![],
        fake,
    )
    .with_store_root(store_root.path().to_path_buf())
    .with_locked_packages(vec![locked_wal(
        "0.2.0",
        "https://registry.example/wal.git",
    )]);

    let resolution = r
        .resolve(&PackageRef::parse("org.vibevm/wal@=0.2.0").unwrap())
        .unwrap();
    assert!(
        resolution.from_store,
        "the dropped version resolves from the store, not the registry"
    );
    assert_eq!(resolution.resolved.version, v("0.2.0"));
    assert_eq!(resolution.resolved.source_dir, entry);

    let cached = r
        .fetch_with_expected_hash(&resolution, store_root.path(), None)
        .unwrap();
    assert_eq!(
        fs::read_to_string(cached.cache_dir.join("PAYLOAD.md")).unwrap(),
        "store bytes\n"
    );
}

/// The error-semantics guard: an OPERATIONAL failure is NOT an
/// absence — the store must not mask it. Here the registry host
/// answers 401/403 (`AuthFailed`) and the resolver runs with
/// `strict_auth`, so the walk HALTS on the auth error exactly as it
/// did before the store existed — even though the store + lock entry
/// could serve the package. (A `RepoNotFound` host, by contrast, is
/// classified "no answer here" and walks on — on THAT shape the store
/// fallback is contractually correct, and the canonical test above
/// covers it.)
#[test]
fn an_operational_error_is_not_masked_by_the_store() {
    let cache = tempdir().unwrap();
    let store_root = tempdir().unwrap();
    seed_store_entry(store_root.path(), "0.2.0", "store bytes\n");
    let fake = Arc::new(FakeBackend::default());
    let url = "https://auth.example/vibespecs";
    // `list_tags` is called with the per-package repo URL (Fqdn naming
    // appends `<group>.<name>.git` to the org URL) — the failure must
    // be seeded there.
    fake.seed_auth_failure("https://auth.example/vibespecs/org.vibevm.wal.git");

    let r = build_resolver(
        cache.path(),
        vec![registry_section("vibespecs", url)],
        vec![],
        vec![],
        fake,
    )
    .with_strict_auth(true)
    .with_store_root(store_root.path().to_path_buf())
    .with_locked_packages(vec![locked_wal("0.2.0", "https://auth.example/wal.git")]);

    let err = r
        .resolve(&PackageRef::parse("org.vibevm/wal@=0.2.0").unwrap())
        .unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("auth.example"),
        "the operational error surfaces verbatim: {msg}"
    );
    assert!(
        !matches!(err, RegistryError::OfflinePackageUnavailable { .. }),
        "not the offline-miss shape"
    );
    assert!(
        !msg.contains("vibe cache add"),
        "an auth failure is not answered with store recipes: {msg}"
    );
}

fn v(raw: &str) -> semver::Version {
    semver::Version::parse(raw).unwrap()
}
