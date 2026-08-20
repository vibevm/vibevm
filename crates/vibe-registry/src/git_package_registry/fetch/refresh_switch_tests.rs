//! Refresh-vs-source-switch oracles for the per-package clone
//! (PROP-010 §2.6): a failed refresh of an existing copy never
//! deletes it — the copy is retried or repaired where it stands; only
//! a deliberate source switch downloads from scratch, and even then
//! into a temporary sibling that replaces the existing copy only after
//! the clone has fully succeeded. The ten-gigabyte measure: «delete
//! and re-download» as the response to any hiccup is not a small
//! inefficiency but an unusable tool.
//!
//! Split from `tests.rs` along this seam when the refresh/switch work
//! outgrew the combined file — same harness, same fixtures.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-model");

use super::*;
use tempfile::tempdir;

use crate::git_package_registry::test_support::*;

/// A failed refresh (origin unreachable: `update` fails AND a
/// re-clone would too) must surface an actionable error and LEAVE THE
/// EXISTING COPY ALIVE AND UNTOUCHED — no wipe-and-retry on the
/// refresh path, ever.
#[test]
fn failed_refresh_leaves_the_existing_clone_alive() {
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

    // First fetch lands the working clone.
    let p = PackageRef::parse("org.vibevm/wal@0.1.0").unwrap();
    let resolved = r.resolve(&p).unwrap();
    let _ = r.fetch(&resolved, store_root.path()).unwrap();
    let clone_dir = r.package_clone_dir(&org(), "wal");
    assert!(
        clone_dir.join("vibe.toml").is_file(),
        "fixture: the working clone must exist before the failure"
    );

    // The origin dies: `update` fails and a re-clone would too (no
    // bootstrap seed). No mirrors — nothing to fail over to.
    fake.fail_update_for_url(url);
    fake.bootstrap_seeds.lock().unwrap().remove(url);

    let err = r.fetch(&resolved, store_root.path()).unwrap_err();
    // Actionable: the error carries the dead origin's URL.
    assert!(
        err.to_string().contains(url),
        "the refresh failure must name the origin it could not reach: {err}"
    );

    // THE invariant: the existing copy survives the failed refresh,
    // content untouched.
    assert!(
        clone_dir.join(".git/origin-url").is_file(),
        "a failed refresh must NOT delete the working clone"
    );
    assert!(
        clone_dir.join("vibe.toml").is_file(),
        "a failed refresh must NOT delete the working clone's content"
    );
    assert_eq!(
        fs::read_to_string(clone_dir.join("README.md")).unwrap(),
        "# canonical content\n",
        "a failed refresh must leave the clone's bytes untouched"
    );
}

/// A source switch whose clone FAILS must clean up its temporary
/// sibling and leave the previous copy exactly as it was — an
/// interrupted switch never destroys the thing it was replacing.
#[test]
fn failed_source_switch_keeps_the_old_clone_and_cleans_the_temp() {
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
    let primary_url = "https://primary.example/vibespecs/org.vibevm.wal.git";
    let mirror_url = "https://mirror.example/vibespecs/org.vibevm.wal.git";
    fake.seed_tags(primary_url, vec!["v0.1.0".into()]);
    fake.seed_tags(mirror_url, vec!["v0.1.0".into()]);
    fake.seed_bootstrap(primary_url, pkg_root.clone());

    let r = registry_with_mirrors(
        cache.path(),
        "https://primary.example/vibespecs",
        NamingConvention::Fqdn,
        vec!["https://mirror.example/vibespecs".to_string()],
        fake.clone(),
    );

    let p = PackageRef::parse("org.vibevm/wal@0.1.0").unwrap();
    let resolved = r.resolve(&p).unwrap();
    let _ = r.fetch(&resolved, store_root.path()).unwrap();
    let clone_dir = r.package_clone_dir(&org(), "wal");

    // The primary dies (update fails, re-clone impossible); the mirror
    // is unreachable too (no bootstrap seed) — every switch attempt
    // fails.
    fake.fail_update_for_url(primary_url);
    fake.bootstrap_seeds.lock().unwrap().remove(primary_url);

    let err = r.fetch(&resolved, store_root.path()).unwrap_err();
    assert!(
        err.to_string().contains(primary_url),
        "the surfaced error is the primary's: {err}"
    );

    // The previous copy is exactly as it was…
    assert!(
        clone_dir.join("vibe.toml").is_file(),
        "a failed switch must leave the previous clone in place"
    );
    assert_eq!(
        fs::read_to_string(clone_dir.join("README.md")).unwrap(),
        "# canonical content\n",
        "a failed switch must leave the previous clone's bytes untouched"
    );
    // …and the switch left no temporary sibling behind.
    let parent = clone_dir.parent().unwrap();
    let leftovers: Vec<String> = fs::read_dir(parent)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().into_string().unwrap_or_default())
        .filter(|n| n.contains("switch-tmp"))
        .collect();
    assert!(
        leftovers.is_empty(),
        "a failed switch must clean up its temporary sibling; found {leftovers:?}"
    );
}

/// A SUCCESSFUL source switch replaces the clone's content wholesale —
/// content from the new source, none of the old, and a stray file in
/// the old copy does NOT survive (the swap is a whole-directory
/// replacement, not an in-place update).
#[test]
fn successful_source_switch_replaces_the_clone_content() {
    let cache = tempdir().unwrap();
    let store_root = tempdir().unwrap();
    let upstream = tempdir().unwrap();

    let pkg_a = upstream.path().join("pkg-a");
    fs::create_dir_all(&pkg_a).unwrap();
    fs::write(
        pkg_a.join("vibe.toml"),
        manifest_text("wal", "flow", "0.1.0"),
    )
    .unwrap();
    fs::write(pkg_a.join("README.md"), "# canonical content\n").unwrap();

    let pkg_b = upstream.path().join("pkg-b");
    fs::create_dir_all(&pkg_b).unwrap();
    fs::write(
        pkg_b.join("vibe.toml"),
        manifest_text("wal", "flow", "0.1.0"),
    )
    .unwrap();
    fs::write(pkg_b.join("README.md"), "# mirror content\n").unwrap();

    let fake = Arc::new(FakeBackend::default());
    let primary_url = "https://primary.example/vibespecs/org.vibevm.wal.git";
    let mirror_url = "https://mirror.example/vibespecs/org.vibevm.wal.git";
    fake.seed_tags(primary_url, vec!["v0.1.0".into()]);
    fake.seed_tags(mirror_url, vec!["v0.1.0".into()]);
    fake.seed_bootstrap(primary_url, pkg_a.clone());
    fake.seed_bootstrap(mirror_url, pkg_b.clone());

    let r = registry_with_mirrors(
        cache.path(),
        "https://primary.example/vibespecs",
        NamingConvention::Fqdn,
        vec!["https://mirror.example/vibespecs".to_string()],
        fake.clone(),
    );

    let p = PackageRef::parse("org.vibevm/wal@0.1.0").unwrap();
    let resolved = r.resolve(&p).unwrap();
    let _ = r.fetch(&resolved, store_root.path()).unwrap();
    let clone_dir = r.package_clone_dir(&org(), "wal");
    // A stray file in the old copy — the sentinel that must survive a
    // refresh but NOT a switch.
    fs::write(clone_dir.join("STRAY-SENTINEL"), "local dirt\n").unwrap();

    // Primary dies; the mirror serves DIFFERENT content.
    fake.fail_update_for_url(primary_url);
    fake.bootstrap_seeds.lock().unwrap().remove(primary_url);

    let _ = r.fetch(&resolved, store_root.path()).unwrap();

    // The clone now carries the mirror's bytes…
    assert_eq!(
        fs::read_to_string(clone_dir.join("README.md")).unwrap(),
        "# mirror content\n",
        "a successful switch must serve the new source's content"
    );
    assert_eq!(
        fs::read_to_string(clone_dir.join(".git/origin-url")).unwrap(),
        mirror_url,
        "the switched clone's recorded origin is the mirror that served it"
    );
    // …and none of the old copy survives the swap — sentinel included.
    assert!(
        !clone_dir.join("STRAY-SENTINEL").exists(),
        "a switch replaces the whole directory; a stray file must not survive it"
    );
}

/// A SUCCESSFUL refresh happens IN PLACE — the existing copy is
/// updated where it stands, and a stray file in it survives (the
/// counterpart of the switch oracle above: the sentinel separates
/// «updated in place» from «replaced wholesale»).
#[test]
fn successful_refresh_updates_in_place_sentinel_survives() {
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
    let _ = r.fetch(&resolved, store_root.path()).unwrap();
    let clone_dir = r.package_clone_dir(&org(), "wal");
    fs::write(clone_dir.join("STRAY-SENTINEL"), "local dirt\n").unwrap();

    // A healthy origin: the second fetch refreshes the existing copy.
    let _ = r.fetch(&resolved, store_root.path()).unwrap();

    assert_eq!(
        fs::read_to_string(clone_dir.join("STRAY-SENTINEL")).unwrap(),
        "local dirt\n",
        "a refresh updates the copy in place; the stray file survives it"
    );
    assert_eq!(
        fs::read_to_string(clone_dir.join("README.md")).unwrap(),
        "# canonical content\n",
        "a refresh keeps the clone's content"
    );
    // And the refresh path took `update`, not a re-clone.
    assert_eq!(
        fake.bootstrap_count(),
        1,
        "one bootstrap ever; the second fetch updated"
    );
    assert_eq!(
        fake.update_count(),
        1,
        "the second fetch refreshed via update"
    );
}
