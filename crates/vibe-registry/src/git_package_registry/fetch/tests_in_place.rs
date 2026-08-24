//! Tests for the in-place materialisation mode — split from `tests.rs`
//! along the mode seam when the file crossed the length budget.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-model");

use super::*;
use specmark::verifies;
use tempfile::tempdir;

use crate::git_package_registry::test_support::*;

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-022#in-place",
    r = 1
)]
fn fetch_in_place_skips_the_cache_copy_and_keeps_git() {
    let cache = tempdir().unwrap();
    let store_root = tempdir().unwrap();
    let upstream = tempdir().unwrap();
    let pkg_root = upstream.path().join("pkg");
    fs::create_dir_all(&pkg_root).unwrap();
    // A package that declares in-place materialization (PROP-022 §2.4).
    fs::write(
        pkg_root.join("vibe.toml"),
        "[package]\ngroup = \"org.vibevm\"\nname = \"giant\"\nkind = \"feat\"\nversion = \"1.0.0\"\nmaterialization = \"in-place\"\n",
    )
    .unwrap();
    fs::write(pkg_root.join("big.bin"), "lots of files\n").unwrap();

    let fake = Arc::new(FakeBackend::default());
    let url = "git@host:org/org.vibevm.giant.git";
    fake.seed_tags(url, vec!["v1.0.0".into()]);
    fake.seed_bootstrap(url, pkg_root.clone());

    let r = registry_with(
        cache.path(),
        "git@host:org",
        NamingConvention::Fqdn,
        fake.clone(),
    );
    let p = PackageRef::parse("org.vibevm/giant@1.0.0").unwrap();
    let resolved = r.resolve(&p).unwrap();
    let cached = r.fetch(&resolved, store_root.path()).unwrap();

    // In-place hands back the LIVE clone (keeps `.git`), not a stripped copy.
    assert!(
        cached.cache_dir.join(".git").exists(),
        "in-place keeps the clone's .git"
    );
    assert!(cached.cache_dir.join("big.bin").exists());
    // The `.git`-stripped store entry was NOT made — the tree walk
    // the mode exists to avoid never ran.
    let dest_cache = store_root
        .path()
        .join("org.vibevm")
        .join("giant")
        .join("v1.0.0");
    assert!(
        !dest_cache.exists(),
        "no .git-stripped store entry for an in-place package"
    );
    // content_hash is a well-formed sha256 (commit-derived, not a tree walk).
    assert!(cached.content_hash.starts_with("sha256:"));
    assert!(cached.package_meta().materialization.is_in_place());
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-022#in-place",
    r = 1
)]
fn materialise_in_place_clones_then_updates_the_slot() {
    let cache = tempdir().unwrap();
    let slot_parent = tempdir().unwrap();
    let upstream = tempdir().unwrap();
    let pkg_root = upstream.path().join("pkg");
    fs::create_dir_all(&pkg_root).unwrap();
    fs::write(
        pkg_root.join("vibe.toml"),
        "[package]\ngroup = \"org.vibevm\"\nname = \"giant\"\nkind = \"feat\"\nversion = \"1.0.0\"\nmaterialization = \"in-place\"\n",
    )
    .unwrap();

    let fake = Arc::new(FakeBackend::default());
    let url = "git@host:org/org.vibevm.giant.git";
    fake.seed_tags(url, vec!["v1.0.0".into()]);
    fake.seed_bootstrap(url, pkg_root.clone());

    let r = registry_with(
        cache.path(),
        "git@host:org",
        NamingConvention::Fqdn,
        fake.clone(),
    );
    let resolved = r
        .resolve(&PackageRef::parse("org.vibevm/giant@1.0.0").unwrap())
        .unwrap();

    // The slot is absent → a fresh clone lands directly in it. The
    // deps-slot root is named by the layout module (PROP-052 L2), so
    // the R4 flip carries this scaffold to `vibevm/vibedeps/…`.
    let slot = slot_parent
        .path()
        .join(vibe_core::layout::current_vibedeps_root())
        .join("org.vibevm.giant");
    let placed = r.materialise_in_place(&resolved, &slot).unwrap();
    assert_eq!(placed.source_uri, url);
    assert_eq!(placed.source_ref, "v1.0.0");
    assert_eq!(placed.manifest.package.as_ref().unwrap().name, "giant");
    assert!(slot.join("vibe.toml").exists());
    assert!(slot.join(".git").exists());
    assert_eq!(fake.bootstrap_count(), 1);

    // The slot now carries `.git` → a second placement updates incrementally
    // (PROP-022 §2.4), never re-clones — no new bootstrap, one update.
    let again = r.materialise_in_place(&resolved, &slot).unwrap();
    assert_eq!(again.source_ref, "v1.0.0");
    assert_eq!(fake.bootstrap_count(), 1);
    assert_eq!(fake.update_count(), 1);
}
