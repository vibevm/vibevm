//! Tests for the priority-ordered registry walk — first-match wins,
//! `UnknownPackage` fall-through, the per-`auth` walk-vs-halt rules,
//! override short-circuits, and the fetch-side dispatch they feed.

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-002#registry-model");

use super::*;
use std::fs;
use tempfile::tempdir;
use vibe_core::manifest::NamingConvention;

use crate::multi_registry_resolver::test_support::*;

#[test]
fn resolve_picks_first_registry_with_match() {
    let cache = tempdir().unwrap();
    let fake = Arc::new(FakeBackend::default());
    // Both registries have the package; first wins.
    fake.seed_tags(
        "git@host:org-a/org.vibevm.wal.git",
        vec!["v0.1.0".into(), "v0.2.0".into()],
    );
    fake.seed_tags("git@host:org-b/org.vibevm.wal.git", vec!["v0.5.0".into()]);

    let r = build_resolver(
        cache.path(),
        vec![
            registry_section("a", "git@host:org-a"),
            registry_section("b", "git@host:org-b"),
        ],
        vec![],
        vec![],
        fake,
    );

    let p = PackageRef::parse("org.vibevm/wal").unwrap();
    let m = r.resolve(&p).unwrap();
    assert_eq!(m.registry_name.as_deref(), Some("a"));
    assert_eq!(m.resolved.version.to_string(), "0.2.0");
    assert!(!m.overridden);
    assert_eq!(m.source_url, "git@host:org-a/org.vibevm.wal.git");
    assert_eq!(m.source_ref.as_deref(), Some("v0.2.0"));
}

#[test]
fn resolve_falls_through_to_next_registry_on_unknown_package() {
    let cache = tempdir().unwrap();
    let fake = Arc::new(FakeBackend::default());
    // First registry: no seed for this URL → RepoNotFound → fall through.
    fake.seed_tags("git@host:org-b/org.vibevm.wal.git", vec!["v0.5.0".into()]);

    let r = build_resolver(
        cache.path(),
        vec![
            registry_section("a", "git@host:org-a"),
            registry_section("b", "git@host:org-b"),
        ],
        vec![],
        vec![],
        fake,
    );

    let p = PackageRef::parse("org.vibevm/wal").unwrap();
    let m = r.resolve(&p).unwrap();
    assert_eq!(m.registry_name.as_deref(), Some("b"));
    assert_eq!(m.resolved.version.to_string(), "0.5.0");
}

#[test]
fn resolve_aggregates_walk_attempts_when_no_registry_has_it() {
    let cache = tempdir().unwrap();
    let fake = Arc::new(FakeBackend::default());
    // No seed for any URL — both registries return UnknownPackage
    // for `flow:ghost`. The resolver collects both into the
    // aggregate `PackageNotFoundEverywhere` report so the
    // operator sees per-registry status.

    let r = build_resolver(
        cache.path(),
        vec![
            registry_section("a", "git@host:org-a"),
            registry_section("b", "git@host:org-b"),
        ],
        vec![],
        vec![],
        fake,
    );

    let p = PackageRef::parse("org.vibevm/ghost").unwrap();
    let err = r.resolve(&p).unwrap_err();
    match err {
        RegistryError::PackageNotFoundEverywhere {
            group,
            name,
            summary,
            attempts,
        } => {
            assert_eq!(attempts.len(), 2, "expected 2 walk attempts: {attempts:?}");
            assert_eq!(group, org());
            assert_eq!(name, "ghost");
            assert!(
                summary.contains("a") && summary.contains("b"),
                "summary must list both walked registries: {summary}"
            );
            assert!(
                summary.contains("not found"),
                "expected `not found` status label: {summary}"
            );
        }
        other => panic!("expected PackageNotFoundEverywhere with attempts, got: {other:?}"),
    }
}

#[test]
fn resolve_unknown_when_no_registries_and_no_override() {
    let cache = tempdir().unwrap();
    let fake = Arc::new(FakeBackend::default());
    let r = build_resolver(cache.path(), vec![], vec![], vec![], fake);
    let p = PackageRef::parse("org.vibevm/wal").unwrap();
    let err = r.resolve(&p).unwrap_err();
    assert!(matches!(err, RegistryError::UnknownPackage { .. }));
}

/// PROP-002 §2.3.1 strict-auth corollary: when
/// `with_strict_auth(true)` is set, a 401 on an `auth = "none"`
/// public registry halts instead of walking past. Useful for
/// CI / cron where the operator wants to gate "must come from
/// the private registry; if its 401 leaks to a public fallback,
/// fail loudly". Default behaviour (without strict_auth) is
/// covered by `resolve_walks_past_auth_failed_when_registry_is_public`
/// below.
#[test]
fn resolve_strict_auth_halts_on_public_401_instead_of_walking() {
    let cache = tempdir().unwrap();
    let fake = Arc::new(FakeBackend::default());
    // Primary public registry returns AuthFailed; secondary has
    // the package. With strict_auth on, the resolver must NOT
    // walk to the secondary.
    fake.seed_auth_failure("git@host:org-a/org.vibevm.wal.git");
    fake.seed_tags("git@host:org-b/org.vibevm.wal.git", vec!["v0.5.0".into()]);

    let r = build_resolver(
        cache.path(),
        vec![
            registry_section("public-a", "git@host:org-a"),
            registry_section("public-b", "git@host:org-b"),
        ],
        vec![],
        vec![],
        fake,
    )
    .with_strict_auth(true);
    assert!(r.strict_auth());

    let p = PackageRef::parse("org.vibevm/wal").unwrap();
    let err = r.resolve(&p).unwrap_err();
    match err {
        RegistryError::Git(GitError::AuthFailed { url }) => {
            assert!(
                url.contains("org-a"),
                "halt error must surface the failing registry's URL: {url}"
            );
        }
        other => {
            panic!("strict-auth: expected halt with AuthFailed on first registry, got: {other:?}")
        }
    }
}

/// PROP-002 §2.3.1: 401 / 403 on an `auth = "none"` registry is
/// reclassified as "no public answer here", and the resolver
/// walks to the next registry. Closes the original opencode
/// regression where GitVerse's 401 (its policy on missing
/// public repos) halted resolution before GitHub got a chance.
#[test]
fn resolve_walks_past_auth_failed_when_registry_is_public() {
    let cache = tempdir().unwrap();
    let fake = Arc::new(FakeBackend::default());
    // First registry: returns AuthFailed (think GitVerse-style 401
    // for a missing public repo). Second registry: serves the
    // package.
    fake.seed_auth_failure("git@host:org-a/org.vibevm.wal.git");
    fake.seed_tags("git@host:org-b/org.vibevm.wal.git", vec!["v0.5.0".into()]);

    let r = build_resolver(
        cache.path(),
        vec![
            registry_section("public-a", "git@host:org-a"),
            registry_section("public-b", "git@host:org-b"),
        ],
        vec![],
        vec![],
        fake,
    );

    let p = PackageRef::parse("org.vibevm/wal").unwrap();
    let m = r
        .resolve(&p)
        .expect("public-a's AuthFailed must walk to public-b, not halt");
    assert_eq!(m.registry_name.as_deref(), Some("public-b"));
    assert_eq!(m.resolved.version.to_string(), "0.5.0");
}

/// PROP-002 §2.3.1: 401 / 403 on an authenticated registry
/// (`auth = "token-env"` in this test) is a real `AuthFailed`
/// halt — the operator declared this registry expects creds and
/// the creds presented were rejected (or absent / expired).
/// Walking past would mask the configuration error.
///
/// We use `open_with_explicit_token` indirectly through the
/// resolver's `from_manifest` path by pre-loading the env-var.
/// Skipping the env layer in this test would require a
/// resolver-level test-only constructor; instead we set the
/// env via a helper that doesn't need `unsafe` (read-only,
/// because the value is already there from the caller).
///
/// In this test we don't actually need a token *value* — the
/// walk-vs-halt decision is gated on `auth_kind`, not on
/// whether the token resolved. We mark the registry
/// `auth = "token-env"` with no env-var set; the resolver's
/// `MissingToken` precheck does NOT fire because the
/// `MissingToken` path only triggers when a git invocation is
/// attempted, and AuthFailed is already on the wire from
/// `list_tags`. So this test exercises the AuthFailed-on-
/// authenticated-registry branch directly.
#[test]
fn resolve_halts_on_auth_failed_against_authenticated_registry() {
    let cache = tempdir().unwrap();
    let fake = Arc::new(FakeBackend::default());
    // The authenticated registry returns AuthFailed.
    fake.seed_auth_failure("https://internal.example.com/vibespecs/org.vibevm.wal.git");
    // A second registry has the package — but the resolver must
    // NOT walk to it (the operator declared the first registry
    // as authenticated; AuthFailed is information they need).
    fake.seed_tags(
        "git@host:org-public/org.vibevm.wal.git",
        vec!["v0.5.0".into()],
    );

    // Stash the token in an env-var so `from_manifest` can find
    // one. We can't `set_var` from this test (`forbid(unsafe_code)`),
    // so we use a name that's already in the test process env or
    // leverage a side door. Simplest: declare `auth = token-env`
    // with NO `token_env` field — `resolve_token_env_name` will
    // derive a name from the host that almost certainly isn't
    // set, so the registry opens with `effective_token = None`.
    // The MissingToken precheck would normally fire, but our
    // FakeBackend's `list_tags` returns AuthFailed first, before
    // any token-aware code path runs. (The AuthFailed comes from
    // the seeded backend, simulating a real 401 from the host;
    // we bypass the precheck by virtue of how the fake works.)
    //
    // Actually simpler still: just set `auth = "credential-helper"`,
    // which never triggers MissingToken (the precheck only fires
    // for `TokenEnv`). The walk-vs-halt rule applies the same:
    // any `auth != None` halts on AuthFailed.
    let auth_section = RegistrySection {
        name: "internal".to_string(),
        url: "https://internal.example.com/vibespecs".to_string(),
        r#ref: "main".to_string(),
        naming: NamingConvention::Fqdn,
        auth: vibe_core::manifest::AuthKind::CredentialHelper,
        token_env: None,
    };
    let r = build_resolver(
        cache.path(),
        vec![
            auth_section,
            registry_section("public-fallback", "git@host:org-public"),
        ],
        vec![],
        vec![],
        fake,
    );

    let p = PackageRef::parse("org.vibevm/wal").unwrap();
    let err = r.resolve(&p).unwrap_err();
    match err {
        RegistryError::Git(GitError::AuthFailed { url }) => {
            assert!(
                url.contains("internal.example.com"),
                "halt error must surface the authenticated registry's URL, got: {url}"
            );
        }
        other => {
            panic!("expected halt with AuthFailed against authenticated registry, got: {other:?}")
        }
    }
}

/// PROP-002 §2.2.1 + §2.3.1 corollary: when a registry is
/// declared `auth = "token-env"` but the env-var is absent, the
/// resolver must surface `MissingToken` immediately on that
/// registry — it must NOT silently walk past, because doing so
/// would mask the operator's configuration error (the
/// authenticated registry was supposed to answer; a missing
/// token is a setup mistake, not a "package not here" signal).
#[test]
fn resolve_halts_on_missing_token_for_authenticated_registry() {
    let cache = tempdir().unwrap();
    let fake = Arc::new(FakeBackend::default());
    // Public fallback also has the package — must NOT be walked
    // past the missing-token registry.
    fake.seed_tags(
        "git@host:org-public/org.vibevm.wal.git",
        vec!["v0.5.0".into()],
    );

    // `auth = token-env` with an env-var that resolves to nothing
    // (deliberately exotic name unlikely to be set anywhere).
    let env_name = "VIBEVM_REGISTRY_TOKEN_DEFINITELY_NOT_SET_ABCXYZ";
    let r = build_resolver(
        cache.path(),
        vec![
            registry_section_token_env("internal", "https://internal.example/vibespecs", env_name),
            registry_section("public-fallback", "git@host:org-public"),
        ],
        vec![],
        vec![],
        fake,
    );

    let p = PackageRef::parse("org.vibevm/wal").unwrap();
    let err = r.resolve(&p).unwrap_err();
    match err {
        RegistryError::MissingToken { registry, env_var } => {
            assert_eq!(registry, "internal");
            assert_eq!(env_var, env_name);
        }
        other => panic!(
            "expected MissingToken halt, got: {other:?}; resolver must NOT walk past missing-token registries"
        ),
    }
}

#[test]
fn override_short_circuits_registry_resolution() {
    let cache = tempdir().unwrap();
    let fake = Arc::new(FakeBackend::default());
    // Registry has flow:wal at 0.2.0, but override pins to a fork.
    fake.seed_tags("git@host:org-a/org.vibevm.wal.git", vec!["v0.2.0".into()]);
    // Override URL: serve a manifest pinned at "my-fix" branch.
    fake.seed_file(
        "git@my-fork:vibevm/wal-fork.git",
        "my-fix",
        "vibe.toml",
        manifest_text("wal", "flow", "0.2.0").into_bytes(),
    );

    let ovr = OverrideSection {
        pkgref: "org.vibevm/wal".to_string(),
        source_url: "git@my-fork:vibevm/wal-fork.git".to_string(),
        r#ref: Some("my-fix".to_string()),
        reason: Some("waiting on upstream PR".to_string()),
    };

    let r = build_resolver(
        cache.path(),
        vec![registry_section("a", "git@host:org-a")],
        vec![],
        vec![ovr],
        fake,
    );

    let p = PackageRef::parse("org.vibevm/wal").unwrap();
    let m = r.resolve(&p).unwrap();
    assert!(m.overridden);
    assert!(m.registry_name.is_none());
    assert_eq!(m.source_url, "git@my-fork:vibevm/wal-fork.git");
    assert_eq!(m.source_ref.as_deref(), Some("my-fix"));
    assert_eq!(m.resolved.version.to_string(), "0.2.0");
}

#[test]
fn override_uses_default_ref_when_unspecified() {
    let cache = tempdir().unwrap();
    let fake = Arc::new(FakeBackend::default());
    fake.seed_file(
        "git@my-fork:vibevm/wal-fork.git",
        DEFAULT_OVERRIDE_REF,
        "vibe.toml",
        manifest_text("wal", "flow", "1.0.0").into_bytes(),
    );

    let ovr = OverrideSection {
        pkgref: "org.vibevm/wal".to_string(),
        source_url: "git@my-fork:vibevm/wal-fork.git".to_string(),
        r#ref: None,
        reason: None,
    };

    let r = build_resolver(cache.path(), vec![], vec![], vec![ovr], fake);
    let p = PackageRef::parse("org.vibevm/wal").unwrap();
    let m = r.resolve(&p).unwrap();
    assert_eq!(m.source_ref.as_deref(), Some(DEFAULT_OVERRIDE_REF));
    assert_eq!(m.resolved.version.to_string(), "1.0.0");
}

#[test]
fn override_refuses_when_manifest_identity_mismatches() {
    let cache = tempdir().unwrap();
    let fake = Arc::new(FakeBackend::default());
    // The manifest at the pinned ref claims to be `flow:atomic-commits`,
    // but the override is for `flow:wal`. Refuse loudly — silently
    // installing as `flow:wal` would corrupt the lockfile.
    fake.seed_file(
        "git@my-fork:vibevm/wal-fork.git",
        "main",
        "vibe.toml",
        manifest_text("atomic-commits", "flow", "0.1.0").into_bytes(),
    );

    let ovr = OverrideSection {
        pkgref: "org.vibevm/wal".to_string(),
        source_url: "git@my-fork:vibevm/wal-fork.git".to_string(),
        r#ref: None,
        reason: None,
    };
    let r = build_resolver(cache.path(), vec![], vec![], vec![ovr], fake);
    let p = PackageRef::parse("org.vibevm/wal").unwrap();
    let err = r.resolve(&p).unwrap_err();
    match err {
        RegistryError::MalformedMeta { reason, .. } => {
            assert!(reason.contains("refusing to install"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn fetch_dispatches_to_registry_that_resolved() {
    let cache = tempdir().unwrap();
    let pkg_cache = tempdir().unwrap();
    let upstream = tempdir().unwrap();

    // Build an upstream tree at the second registry's URL.
    let pkg_root = upstream.path().join("pkg");
    fs::create_dir_all(&pkg_root).unwrap();
    fs::write(
        pkg_root.join("vibe.toml"),
        manifest_text("wal", "flow", "0.5.0"),
    )
    .unwrap();

    let fake = Arc::new(FakeBackend::default());
    fake.seed_tags("git@host:org-b/org.vibevm.wal.git", vec!["v0.5.0".into()]);
    fake.seed_bootstrap("git@host:org-b/org.vibevm.wal.git", pkg_root.clone());

    let r = build_resolver(
        cache.path(),
        vec![
            registry_section("a", "git@host:org-a"), // empty (no seed)
            registry_section("b", "git@host:org-b"),
        ],
        vec![],
        vec![],
        fake.clone(),
    );

    let p = PackageRef::parse("org.vibevm/wal").unwrap();
    let resolution = r.resolve(&p).unwrap();
    let cached = r.fetch(&resolution, pkg_cache.path()).unwrap();

    assert_eq!(cached.registry_name.as_deref(), Some("b"));
    assert!(!cached.overridden);
    assert_eq!(cached.source_uri, "git@host:org-b/org.vibevm.wal.git");
    assert_eq!(cached.source_ref.as_deref(), Some("v0.5.0"));
    assert_eq!(cached.package_meta().version.to_string(), "0.5.0");
    assert!(cached.cache_dir.join("vibe.toml").exists());
    assert!(!cached.cache_dir.join(".git").exists());
    // Bootstrap exactly once — only against registry "b".
    assert_eq!(fake.bootstrap_count(), 1);
}

#[test]
fn fetch_override_clones_into_overrides_subtree_and_marks_overridden() {
    let cache = tempdir().unwrap();
    let pkg_cache = tempdir().unwrap();
    let upstream = tempdir().unwrap();

    let pkg_root = upstream.path().join("pkg");
    fs::create_dir_all(&pkg_root).unwrap();
    fs::write(
        pkg_root.join("vibe.toml"),
        manifest_text("wal", "flow", "0.9.0"),
    )
    .unwrap();

    let fake = Arc::new(FakeBackend::default());
    // For override: backend serves manifest via `fetch_file_at_ref`
    // (resolve), then clones via `bootstrap` (fetch).
    fake.seed_file(
        "git@my-fork:vibevm/wal-fork.git",
        "my-fix",
        "vibe.toml",
        manifest_text("wal", "flow", "0.9.0").into_bytes(),
    );
    fake.seed_bootstrap("git@my-fork:vibevm/wal-fork.git", pkg_root.clone());

    let ovr = OverrideSection {
        pkgref: "org.vibevm/wal".to_string(),
        source_url: "git@my-fork:vibevm/wal-fork.git".to_string(),
        r#ref: Some("my-fix".to_string()),
        reason: Some("PR pending".to_string()),
    };

    let r = build_resolver(cache.path(), vec![], vec![], vec![ovr], fake.clone());

    let p = PackageRef::parse("org.vibevm/wal").unwrap();
    let resolution = r.resolve(&p).unwrap();
    let cached = r.fetch(&resolution, pkg_cache.path()).unwrap();

    assert!(cached.overridden);
    assert!(cached.registry_name.is_none());
    assert_eq!(cached.source_uri, "git@my-fork:vibevm/wal-fork.git");
    assert_eq!(cached.source_ref.as_deref(), Some("my-fix"));
    assert_eq!(cached.package_meta().version.to_string(), "0.9.0");
    // Override clone lives under
    // `cache_root/__overrides__/<group>.<name>/clone/` — keyed by
    // `(group, name)` identity (PROP-008).
    let overrides_root = cache
        .path()
        .join("__overrides__")
        .join("org.vibevm.wal")
        .join("clone");
    assert!(overrides_root.join(".git").exists());
    // Materialised cache holds payload only.
    assert!(cached.cache_dir.join("vibe.toml").exists());
    assert!(!cached.cache_dir.join(".git").exists());
}
