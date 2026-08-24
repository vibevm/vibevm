//! End-to-end oracles for store-backed resolution (PROP-010 §2.6):
//!
//! - the offline posture resolves and materialises from the machine
//!   store with ZERO registries configured (`RESOLVER-OFFLINE-MODE`);
//! - an offline miss is the hard error naming the package and the
//!   recovery recipes (`OFFLINE-HARD-ERROR`);
//! - the canonical availability rule: a version deleted from its
//!   serving registry after the install that warmed the store still
//!   re-installs — a store hit outranks the silent registry
//!   (`A-CACHE-HIT-IS-AUTHORITATIVE-FOR-AVAILABILITY`).
//!
//! Every run is isolated through `UserScratch` (its `VIBE_SETTINGS`
//! stands in for `~/.vibe`), so the store under test is the scratch
//! home's `cache/` — never the operator's real one.

mod common;

use std::fs;
use std::path::Path;

use common::{UserScratch, make_wal_dir_registry, write_project_with_per_package_registry};

/// `file://` URL for a directory, forward-slashed, Windows-safe — the
/// same rule `vibe_registry::vendor::file_url_for_dir` pins (not a
/// dev-dependency of this test crate, hence the local copy of the
/// five-line rule).
fn file_url(dir: &Path) -> String {
    let mut s = dir.to_string_lossy().replace('\\', "/");
    if let Some(stripped) = s.strip_prefix("//?/") {
        s = stripped.to_string();
    }
    if !s.starts_with('/') {
        s.insert(0, '/');
    }
    format!("file://{s}")
}

/// Warm the store: install `org.vibevm.world/wal@=0.2.0` from a
/// one-shot directory registry (`--registry`), then re-install the
/// same pinned pkgref under `--offline` with ZERO registries (no
/// `--registry`, no `[[registry]]` in `vibe.toml`, the scratch home
/// carries no global registries, and `vibe()`'s
/// `VIBE_NO_DEFAULT_REGISTRY=1` keeps the embedded family out). The
/// offline run must resolve from the store and materialise `vibedeps/`
/// — no network path exists to satisfy it any other way.
#[test]
fn offline_install_resolves_from_the_store_with_zero_registries() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());

    // Warm: the store entry lands under the scratch settings home.
    let reg = make_wal_dir_registry(project.path());
    user.vibe()
        .arg("install")
        .arg("org.vibevm.world/wal@=0.2.0")
        .arg("--path")
        .arg(project.path())
        .arg("--registry")
        .arg(&reg)
        .arg("--assume-yes")
        .assert()
        .success();
    let entry = user
        .settings
        .join("cache/org.vibevm.world/wal/v0.2.0/vibe.toml");
    assert!(
        entry.is_file(),
        "fixture: the warm install must land the store entry"
    );

    // The offline re-install: zero registries, an explicit pkgref (so
    // the freshness fast path cannot skip the resolution).
    user.vibe()
        .arg("install")
        .arg("org.vibevm.world/wal@=0.2.0")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .arg("--offline")
        .assert()
        .success();

    assert!(
        project
            .path()
            .join(common::slot_rel(
                "org.vibevm.world.wal",
                "0.2.0",
                "vibe.toml"
            ))
            .is_file(),
        "the offline run must materialise vibedeps from the store"
    );
}

/// An offline MISS — the store is warm (something IS cached) but not
/// for THIS package — is a hard error that names the pkgref and hands
/// the operator the three recovery recipes (PROP-010 §2.5,
/// `OFFLINE-HARD-ERROR`). A fully cold store is the OLD bail's case
/// ("no local registry AND no store") and stays covered by the
/// flag-clause tests.
#[test]
fn offline_miss_names_the_package_and_the_recipes() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());

    // Warm the store with one package — so the miss is "reached the
    // store, store has no such version", not the cold-store bail.
    let reg = make_wal_dir_registry(project.path());
    user.vibe()
        .arg("install")
        .arg("org.vibevm.world/wal@=0.2.0")
        .arg("--path")
        .arg(project.path())
        .arg("--registry")
        .arg(&reg)
        .arg("--assume-yes")
        .assert()
        .success();

    let out = user
        .vibe()
        .arg("install")
        .arg("org.vibevm/nope@1.0.0")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .arg("--offline")
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "an offline miss must fail loudly\nstdout:\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    // The version label is whatever the solver's node requested
    // (`@latest` for a bare pkgref) — the load-bearing part is that
    // the package is NAMED; the req label itself is pinned by the
    // in-crate offline tests.
    assert!(
        stderr.contains("org.vibevm/nope@"),
        "the miss must name the pkgref:\n{stderr}"
    );
    assert!(
        stderr.contains("vibe cache add org.vibevm/nope"),
        "recipe — pre-warm:\n{stderr}"
    );
    assert!(
        stderr.contains("vibe registry vendor"),
        "recipe — vendor a mirror:\n{stderr}"
    );
    assert!(
        stderr.contains("once online"),
        "recipe — run once online:\n{stderr}"
    );
}

/// THE canonical availability scenario: install a version from a
/// `file://` registry (store + lock warm), DELETE the version from the
/// registry directory, then re-install online — the walk answers "no
/// such version", the store outranks that silence, and the install
/// succeeds from the store (PROP-010 §2.6,
/// `A-CACHE-HIT-IS-AUTHORITATIVE-FOR-AVAILABILITY`).
#[test]
fn a_version_deleted_from_the_registry_still_installs_from_the_store() {
    let user = UserScratch::new();
    let outer = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());

    // A declared file:// registry serving wal v0.2.0.
    let reg = make_wal_dir_registry(outer.path());
    write_project_with_per_package_registry(project.path(), &file_url(&reg));

    // First install: online, from the registry — warms store + lock.
    user.vibe()
        .arg("install")
        .arg("org.vibevm.world/wal@=0.2.0")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();

    // The registry "goes silent": the version directory is deleted
    // upstream.
    fs::remove_dir_all(reg.join("org.vibevm.world").join("wal").join("v0.2.0")).unwrap();

    // The re-install (online; explicit pkgref forces the full
    // resolution): the registry no longer lists the version, the store
    // still holds it — the install must succeed.
    user.vibe()
        .arg("install")
        .arg("org.vibevm.world/wal@=0.2.0")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();

    assert!(
        project
            .path()
            .join(common::slot_rel(
                "org.vibevm.world.wal",
                "0.2.0",
                "vibe.toml"
            ))
            .is_file(),
        "the re-install must materialise the store's copy"
    );
}
