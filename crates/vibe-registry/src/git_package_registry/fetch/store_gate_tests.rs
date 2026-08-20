//! Store-gate oracles for the per-package fetch path — the write-once
//! insert after the cross-source pin gate, reuse of an already-present
//! entry, the read-side entry verification (§2.5 of the R1-STORE
//! packet; PROP-010 §2.7), and the no-poisoned-entry rule. Split from
//! `tests.rs` along that seam when the store work pushed the combined
//! file past the 600-line budget — same harness, same fixtures.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-model");

use super::*;
use tempfile::tempdir;

use crate::git_package_registry::test_support::*;

/// The §2.5 read gate, green half: an entry already in the store is
/// reused untouched (write-once) and, hashing to the pin, materialises.
/// The fetch still walks the source — reuse is a store property, not a
/// network-skip (that decision is R1-RESOLVER's, not this packet's).
#[test]
fn fetch_with_pin_reuses_a_matching_present_entry() {
    let cache = tempdir().unwrap();
    let store_root = tempdir().unwrap();
    let upstream = tempdir().unwrap();
    let pkg_root = upstream.path().join("pkg");
    fs::create_dir_all(&pkg_root).unwrap();
    fs::write(
        pkg_root.join("vibe.toml"),
        manifest_text("wal", "flow", "0.1.0"),
    )
    .unwrap();
    fs::write(pkg_root.join("README.md"), "# canonical content\n").unwrap();

    let fake = Arc::new(FakeBackend::default());
    let url = "git@host:org/org.vibevm.wal.git";
    fake.seed_tags(url, vec!["v0.1.0".into()]);
    fake.seed_bootstrap(url, pkg_root.clone());

    let r = registry_with(
        cache.path(),
        "git@host:org",
        NamingConvention::Fqdn,
        fake.clone(),
    );

    let p = PackageRef::parse("org.vibevm/wal@0.1.0").unwrap();
    let resolved = r.resolve(&p).unwrap();
    let pin = compute_content_hash(&pkg_root).unwrap();

    let first = r
        .fetch_with_expected_hash(&resolved, store_root.path(), Some(&pin))
        .unwrap();
    let entry = store_root
        .path()
        .join("org.vibevm")
        .join("wal")
        .join("v0.1.0");
    assert_eq!(first.cache_dir, entry);
    let sentinel_before = fs::read(entry.join("README.md")).unwrap();

    // Second fetch: the entry is present, the fresh clone hashes to the
    // pin, the entry itself hashes to the pin → reused, nothing rewritten.
    let second = r
        .fetch_with_expected_hash(&resolved, store_root.path(), Some(&pin))
        .unwrap();
    assert_eq!(
        second.cache_dir, entry,
        "a present entry is reused, not rewritten"
    );
    assert_eq!(
        fs::read(entry.join("README.md")).unwrap(),
        sentinel_before,
        "write-once: the second fetch must not touch the entry's bytes"
    );
    assert_eq!(second.content_hash, pin);
}

/// The §2.5 read gate, red half: an entry tampered with on disk (a byte
/// swapped outside vibevm) no longer hashes to the lockfile pin — the
/// fetch that would materialise from it must refuse and NAME the
/// package (PROP-010 §2.7, mismatch-is-named), never silently use the
/// altered bytes.
#[test]
fn fetch_with_pin_names_a_tampered_store_entry() {
    let cache = tempdir().unwrap();
    let store_root = tempdir().unwrap();
    let upstream = tempdir().unwrap();
    let pkg_root = upstream.path().join("pkg");
    fs::create_dir_all(&pkg_root).unwrap();
    fs::write(
        pkg_root.join("vibe.toml"),
        manifest_text("wal", "flow", "0.1.0"),
    )
    .unwrap();
    fs::write(pkg_root.join("README.md"), "# canonical content\n").unwrap();

    let fake = Arc::new(FakeBackend::default());
    let url = "git@host:org/org.vibevm.wal.git";
    fake.seed_tags(url, vec!["v0.1.0".into()]);
    fake.seed_bootstrap(url, pkg_root.clone());

    let r = registry_with(
        cache.path(),
        "git@host:org",
        NamingConvention::Fqdn,
        fake.clone(),
    );

    let p = PackageRef::parse("org.vibevm/wal@0.1.0").unwrap();
    let resolved = r.resolve(&p).unwrap();
    let pin = compute_content_hash(&pkg_root).unwrap();

    // Land a good entry, then tamper with it the way only an outside
    // edit can (our code never rewrites an entry).
    r.fetch_with_expected_hash(&resolved, store_root.path(), Some(&pin))
        .unwrap();
    let entry = store_root
        .path()
        .join("org.vibevm")
        .join("wal")
        .join("v0.1.0");
    fs::write(entry.join("README.md"), "# TAMPERED content\n").unwrap();

    let err = r
        .fetch_with_expected_hash(&resolved, store_root.path(), Some(&pin))
        .unwrap_err();
    match &err {
        RegistryError::StoreEntryMismatch { detail } => {
            assert_eq!(detail.group.as_str(), "org.vibevm");
            assert_eq!(detail.name, "wal");
            assert_eq!(detail.version, semver::Version::parse("0.1.0").unwrap());
            assert_eq!(detail.path, entry);
        }
        other => panic!("expected StoreEntryMismatch, got {other:?}"),
    }
    // The message names the package and version — the operator's
    // actionable handle on which entry to remove.
    let msg = err.to_string();
    assert!(
        msg.contains("org.vibevm/wal@0.1.0"),
        "message must name the package: {msg}"
    );
}

/// A source the pin rejects must leave NO store entry behind — the
/// write-once store would otherwise pin a poisoned mirror's bytes
/// forever. The insert happens only for an accepted source.
#[test]
fn fetch_rejected_by_the_pin_leaves_no_store_entry() {
    let cache = tempdir().unwrap();
    let store_root = tempdir().unwrap();
    let upstream = tempdir().unwrap();
    let pkg_root = upstream.path().join("pkg");
    fs::create_dir_all(&pkg_root).unwrap();
    fs::write(
        pkg_root.join("vibe.toml"),
        manifest_text("wal", "flow", "0.1.0"),
    )
    .unwrap();

    let fake = Arc::new(FakeBackend::default());
    let url = "git@host:org/org.vibevm.wal.git";
    fake.seed_tags(url, vec!["v0.1.0".into()]);
    fake.seed_bootstrap(url, pkg_root.clone());

    let r = registry_with(
        cache.path(),
        "git@host:org",
        NamingConvention::Fqdn,
        fake.clone(),
    );

    let p = PackageRef::parse("org.vibevm/wal@0.1.0").unwrap();
    let resolved = r.resolve(&p).unwrap();
    let bogus_pin = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

    // Every source disagrees with the pin → the disagreeing cached is
    // returned (the caller renders drift), and the store stays empty.
    let cached = r
        .fetch_with_expected_hash(&resolved, store_root.path(), Some(bogus_pin))
        .unwrap();
    assert_ne!(cached.content_hash, bogus_pin);
    let group_dir = store_root.path().join("org.vibevm");
    assert!(
        !group_dir.exists(),
        "a pin-rejected source must not leave a store entry behind"
    );
}
