//! Tests for the clone / materialise half — per-project cache
//! population, mirror fall-through on the fetch path, the cross-source
//! content-hash gate, and clone reuse via `update`.

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-002#registry-model");

use super::*;
use specmark::verifies;
use tempfile::tempdir;

use crate::git_package_registry::test_support::*;

#[test]
fn fetch_clones_and_populates_per_project_cache() {
    let cache = tempdir().unwrap();
    let pkg_cache = tempdir().unwrap();
    let upstream = tempdir().unwrap();
    // Build a fake upstream tree at the seeded URL: vibe.toml
    // plus a spec file and a stray `.git/` to make sure the copy
    // strips it on the way to the cache.
    let pkg_root = upstream.path().join("pkg");
    fs::create_dir_all(pkg_root.join("spec")).unwrap();
    fs::write(
        pkg_root.join("vibe.toml"),
        manifest_text("wal", "flow", "0.1.0"),
    )
    .unwrap();
    fs::write(pkg_root.join("spec/foo.md"), "content\n").unwrap();
    // Upstream tree has no .git/ — the FakeBackend creates one in
    // dest after copying; we want to verify our extractor strips it.

    let fake = Arc::new(FakeBackend::default());
    let url = "git@host:org/org.vibevm_wal.git";
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
    let cached = r.fetch(&resolved, pkg_cache.path()).unwrap();

    // Cache populated, no .git/ dragged through.
    assert!(cached.cache_dir.join("vibe.toml").exists());
    assert!(cached.cache_dir.join("spec/foo.md").exists());
    assert!(!cached.cache_dir.join(".git").exists());

    // Manifest parsed and content_hash populated.
    assert_eq!(cached.package_meta().name, "wal");
    assert!(cached.content_hash.starts_with("sha256:"));

    // source_uri is the canonical per-package repo URL.
    assert_eq!(cached.source_uri, url);

    // Bootstrap was called exactly once.
    assert_eq!(fake.bootstrap_count(), 1);
}

#[test]
fn fetch_falls_through_to_mirror_when_primary_unreachable() {
    // Primary URL has tags seeded (so list_versions finds the
    // version) but no bootstrap seed → primary's clone fails.
    // Mirror has BOTH tags and bootstrap seeded. The fetch
    // walk should land on the mirror, materialise from there,
    // and still record the canonical primary URL as
    // `cached.source_uri` per PROP-002 §2.3 step 3.
    let cache = tempdir().unwrap();
    let pkg_cache = tempdir().unwrap();
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

    let fake = Arc::new(FakeBackend::default());
    // Tags on both — list_versions hits primary first and finds it.
    fake.seed_tags(primary_url, vec!["v0.1.0".into()]);
    fake.seed_tags(mirror_url, vec!["v0.1.0".into()]);
    // Bootstrap only on mirror — primary's clone path fails.
    fake.seed_bootstrap(mirror_url, pkg_root.clone());

    let r = registry_with_mirrors(
        cache.path(),
        "https://primary.example/vibespecs",
        NamingConvention::Fqdn,
        vec!["https://mirror.example/vibespecs".to_string()],
        fake.clone(),
    );

    let p = PackageRef::parse("org.vibevm/wal@0.1.0").unwrap();
    let resolved = r.resolve(&p).unwrap();
    let cached = r.fetch(&resolved, pkg_cache.path()).unwrap();

    // Materialised from the mirror.
    assert_eq!(cached.package_meta().name, "wal");
    assert_eq!(cached.package_meta().version.to_string(), "0.1.0");
    // PROP-002 §2.3 step 3: source_uri is canonical primary URL,
    // regardless of which source actually served the bytes.
    assert_eq!(cached.source_uri, primary_url);
    assert_eq!(cached.source_ref.as_deref(), Some("v0.1.0"));
    assert_eq!(cached.registry_name.as_deref(), Some("vibespecs"));

    // Bootstrap was attempted twice: primary (fail) + mirror (ok).
    assert_eq!(fake.bootstrap_count(), 2);
    assert_eq!(
        fake.bootstrap_urls(),
        vec![primary_url.to_string(), mirror_url.to_string()]
    );
}

#[test]
fn fetch_prefers_primary_when_both_reachable() {
    let cache = tempdir().unwrap();
    let pkg_cache = tempdir().unwrap();
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

    let fake = Arc::new(FakeBackend::default());
    fake.seed_tags(primary_url, vec!["v0.1.0".into()]);
    fake.seed_tags(mirror_url, vec!["v0.1.0".into()]);
    // Both URLs serve the same content from the same source dir.
    fake.seed_bootstrap(primary_url, pkg_root.clone());
    fake.seed_bootstrap(mirror_url, pkg_root.clone());

    let r = registry_with_mirrors(
        cache.path(),
        "https://primary.example/vibespecs",
        NamingConvention::Fqdn,
        vec!["https://mirror.example/vibespecs".to_string()],
        fake.clone(),
    );

    let p = PackageRef::parse("org.vibevm/wal@0.1.0").unwrap();
    let resolved = r.resolve(&p).unwrap();
    let _ = r.fetch(&resolved, pkg_cache.path()).unwrap();

    // Bootstrap exactly once — primary won, mirror untouched.
    assert_eq!(fake.bootstrap_count(), 1);
    assert_eq!(fake.bootstrap_urls(), vec![primary_url.to_string()]);
}

#[test]
fn fetch_falls_through_when_primary_update_fails() {
    // First fetch lands a working clone via primary. Then the
    // primary's tag goes missing (we wire `fail_update_for_url`).
    // Second fetch tries `update` against primary, fails,
    // wipes-and-rebootstraps from primary (still fails — no seed
    // after wipe? actually bootstrap is still seeded), …
    //
    // Actually — once `update` fails on the primary's existing
    // clone, `bootstrap_or_update_at` wipes the clone and retries
    // bootstrap on the SAME URL. The bootstrap then re-seeds
    // (primary IS seeded), so the SAME URL succeeds. To force
    // fall-through, primary must fail BOTH update AND bootstrap.
    // Drop primary's bootstrap seed before the second fetch.
    let cache = tempdir().unwrap();
    let pkg_cache = tempdir().unwrap();
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

    let fake = Arc::new(FakeBackend::default());
    fake.seed_tags(primary_url, vec!["v0.1.0".into()]);
    fake.seed_tags(mirror_url, vec!["v0.1.0".into()]);
    fake.seed_bootstrap(primary_url, pkg_root.clone());
    fake.seed_bootstrap(mirror_url, pkg_root.clone());

    let r = registry_with_mirrors(
        cache.path(),
        "https://primary.example/vibespecs",
        NamingConvention::Fqdn,
        vec!["https://mirror.example/vibespecs".to_string()],
        fake.clone(),
    );

    // First fetch lands the clone via primary.
    let p = PackageRef::parse("org.vibevm/wal@0.1.0").unwrap();
    let resolved = r.resolve(&p).unwrap();
    let _ = r.fetch(&resolved, pkg_cache.path()).unwrap();
    assert_eq!(fake.bootstrap_count(), 1);
    assert_eq!(fake.update_count(), 0);

    // Now make primary's update + bootstrap both fail. Mirror
    // remains seeded.
    fake.fail_update_for_url(primary_url);
    fake.bootstrap_seeds.lock().unwrap().remove(primary_url);

    // Second fetch: update primary fails → wipe+re-bootstrap from
    // primary fails → fall through to mirror, which seeds a fresh
    // clone via bootstrap.
    let _ = r.fetch(&resolved, pkg_cache.path()).unwrap();
    // Update was tried once (against primary, failed). Bootstrap
    // counts: 1 (initial primary) + 1 (re-bootstrap primary, fails
    // RepoNotFound after seed removed) + 1 (mirror, succeeds).
    assert_eq!(fake.update_count(), 1);
    assert_eq!(fake.bootstrap_count(), 3);
    assert_eq!(
        fake.bootstrap_urls(),
        vec![
            primary_url.to_string(), // initial fetch
            primary_url.to_string(), // retry after update fail
            mirror_url.to_string(),  // mirror takes over
        ]
    );
}

#[test]
fn fetch_with_expected_hash_passes_through_when_no_pin() {
    // expected_hash = None — equivalent to `fetch`. Just verifies
    // the trait/wrapper plumbing is wired and identical to the
    // existing single-source fetch behaviour.
    let cache = tempdir().unwrap();
    let pkg_cache = tempdir().unwrap();
    let upstream = tempdir().unwrap();
    let pkg_root = upstream.path().join("pkg");
    fs::create_dir_all(&pkg_root).unwrap();
    fs::write(
        pkg_root.join("vibe.toml"),
        manifest_text("wal", "flow", "0.1.0"),
    )
    .unwrap();

    let fake = Arc::new(FakeBackend::default());
    let url = "git@host:org/org.vibevm_wal.git";
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
    let cached = r
        .fetch_with_expected_hash(&resolved, pkg_cache.path(), None)
        .unwrap();
    assert!(cached.content_hash.starts_with("sha256:"));
    assert_eq!(cached.package_meta().name, "wal");
}

#[test]
fn fetch_with_expected_hash_skips_mirror_with_disagreeing_content() {
    // Two seeded mirrors. Primary serves content A; mirror[0]
    // serves content B (disagreeing); mirror[1] serves content A
    // (matches the lockfile pin which is the hash of A).
    // Expected: primary wins on the first iteration because A
    // matches the pin — mirror walk never runs.
    //
    // To make the cross-source check actually fire, we make
    // primary unreachable so the walk reaches the mirrors. Then
    // mirror[0] serves B, hash check fails, fall through to
    // mirror[1] which serves A and matches.
    let cache = tempdir().unwrap();
    let pkg_cache = tempdir().unwrap();
    let upstream = tempdir().unwrap();

    // Two distinct fixture trees → distinct content_hashes.
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
    fs::write(pkg_b.join("README.md"), "# DIVERGENT content\n").unwrap();

    // Compute the expected hash of pkg_a for the lockfile pin.
    let temp_for_hash = tempdir().unwrap();
    copy_dir_excluding_git(&pkg_a, temp_for_hash.path()).unwrap();
    let expected_hash = compute_content_hash(temp_for_hash.path()).unwrap();

    let primary_url = "https://primary.example/vibespecs/org.vibevm_wal.git";
    let mirror_a_url = "https://mirror-bad.example/vibespecs/org.vibevm_wal.git";
    let mirror_b_url = "https://mirror-ok.example/vibespecs/org.vibevm_wal.git";

    let fake = Arc::new(FakeBackend::default());
    // All sources seed tags so the resolver reaches them in order.
    fake.seed_tags(primary_url, vec!["v0.1.0".into()]);
    fake.seed_tags(mirror_a_url, vec!["v0.1.0".into()]);
    fake.seed_tags(mirror_b_url, vec!["v0.1.0".into()]);
    // Primary unreachable for clone (no bootstrap seed).
    // mirror_a serves divergent content.
    fake.seed_bootstrap(mirror_a_url, pkg_b.clone());
    // mirror_b serves canonical content.
    fake.seed_bootstrap(mirror_b_url, pkg_a.clone());

    let r = registry_with_mirrors(
        cache.path(),
        "https://primary.example/vibespecs",
        NamingConvention::Fqdn,
        vec![
            "https://mirror-bad.example/vibespecs".to_string(),
            "https://mirror-ok.example/vibespecs".to_string(),
        ],
        fake.clone(),
    );

    let p = PackageRef::parse("org.vibevm/wal@0.1.0").unwrap();
    let resolved = r.resolve(&p).unwrap();
    let cached = r
        .fetch_with_expected_hash(&resolved, pkg_cache.path(), Some(&expected_hash))
        .unwrap();

    // Final material is canonical content.
    assert_eq!(cached.content_hash, expected_hash);
    // source_uri remains the canonical primary URL.
    assert_eq!(cached.source_uri, primary_url);

    // Walk: primary (bootstrap fail) → mirror_a (succeed, hash mismatch,
    // fall through) → mirror_b (succeed, hash match).
    assert_eq!(
        fake.bootstrap_urls(),
        vec![
            primary_url.to_string(),
            mirror_a_url.to_string(),
            mirror_b_url.to_string(),
        ]
    );
}

#[test]
fn fetch_with_expected_hash_returns_last_attempt_when_no_match() {
    // Every source serves disagreeing content; lockfile pins
    // something else. Per the contract, the registry returns the
    // last successful CachedPackage with its (non-matching) hash;
    // vibe-install's `plan_install` then renders ContentDrift.
    let cache = tempdir().unwrap();
    let pkg_cache = tempdir().unwrap();
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

    let fake = Arc::new(FakeBackend::default());
    fake.seed_tags(primary_url, vec!["v0.1.0".into()]);
    fake.seed_tags(mirror_url, vec!["v0.1.0".into()]);
    fake.seed_bootstrap(primary_url, pkg_root.clone());
    fake.seed_bootstrap(mirror_url, pkg_root.clone());

    let r = registry_with_mirrors(
        cache.path(),
        "https://primary.example/vibespecs",
        NamingConvention::Fqdn,
        vec!["https://mirror.example/vibespecs".to_string()],
        fake.clone(),
    );

    let bogus_pin = "sha256:0000000000000000000000000000000000000000000000000000000000000000";
    let p = PackageRef::parse("org.vibevm/wal@0.1.0").unwrap();
    let resolved = r.resolve(&p).unwrap();
    let cached = r
        .fetch_with_expected_hash(&resolved, pkg_cache.path(), Some(bogus_pin))
        .unwrap();

    // Returned cached carries the actual (non-matching) hash —
    // not the pin. vibe-install's plan_install lifts this into
    // ContentDrift.
    assert_ne!(cached.content_hash, bogus_pin);
    assert!(cached.content_hash.starts_with("sha256:"));
    // Both URLs were tried.
    assert_eq!(fake.bootstrap_count(), 2);
}

#[test]
fn refresh_package_falls_through_to_mirror_when_primary_unreachable() {
    // refresh_package walks primary then mirror, same as fetch.
    // Used by `vibe registry sync`. Test that a fresh sync against
    // an unreachable primary lands the clone via mirror.
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

    let fake = Arc::new(FakeBackend::default());
    fake.seed_bootstrap(mirror_url, pkg_root.clone());

    let r = registry_with_mirrors(
        cache.path(),
        "https://primary.example/vibespecs",
        NamingConvention::Fqdn,
        vec!["https://mirror.example/vibespecs".to_string()],
        fake.clone(),
    );

    r.refresh_package(&org(), "wal", "v0.1.0").unwrap();

    // Primary (fail) + mirror (succeed).
    assert_eq!(
        fake.bootstrap_urls(),
        vec![primary_url.to_string(), mirror_url.to_string()]
    );
}

#[test]
fn fetch_reuses_existing_clone_via_update() {
    let cache = tempdir().unwrap();
    let pkg_cache = tempdir().unwrap();
    let upstream = tempdir().unwrap();
    let pkg_root = upstream.path().join("pkg");
    fs::create_dir_all(&pkg_root).unwrap();
    fs::write(
        pkg_root.join("vibe.toml"),
        manifest_text("wal", "flow", "0.1.0"),
    )
    .unwrap();

    let fake = Arc::new(FakeBackend::default());
    let url = "git@host:org/org.vibevm_wal.git";
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

    let _ = r.fetch(&resolved, pkg_cache.path()).unwrap();
    let _ = r.fetch(&resolved, pkg_cache.path()).unwrap();

    // First fetch: bootstrap; second: update (clone exists from first).
    assert_eq!(fake.bootstrap_count(), 1);
    assert_eq!(fake.update_count(), 1);
}

#[test]
#[verifies("spec://vibevm/modules/vibe-workspace/PROP-022#in-place", r = 1)]
fn fetch_in_place_skips_the_cache_copy_and_keeps_git() {
    let cache = tempdir().unwrap();
    let pkg_cache = tempdir().unwrap();
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
    let url = "git@host:org/org.vibevm_giant.git";
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
    let cached = r.fetch(&resolved, pkg_cache.path()).unwrap();

    // In-place hands back the LIVE clone (keeps `.git`), not a stripped copy.
    assert!(
        cached.cache_dir.join(".git").exists(),
        "in-place keeps the clone's .git"
    );
    assert!(cached.cache_dir.join("big.bin").exists());
    // The `.git`-stripped per-project cache copy was NOT made — the tree
    // walk the mode exists to avoid never ran.
    let dest_cache = pkg_cache
        .path()
        .join("org.vibevm")
        .join("giant")
        .join("v1.0.0");
    assert!(
        !dest_cache.exists(),
        "no .git-stripped cache copy for an in-place package"
    );
    // content_hash is a well-formed sha256 (commit-derived, not a tree walk).
    assert!(cached.content_hash.starts_with("sha256:"));
    assert!(cached.package_meta().materialization.is_in_place());
}

#[test]
#[verifies("spec://vibevm/modules/vibe-workspace/PROP-022#in-place", r = 1)]
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
    let url = "git@host:org/org.vibevm_giant.git";
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

    // The slot is absent → a fresh clone lands directly in it.
    let slot = slot_parent.path().join("vibedeps/feat-giant");
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
