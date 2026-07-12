//! End-to-end tests for the M1.16 redirect-stub installs (PROP-002 §2.4.2)
//! and the `vibe registry redirect-update` args-level guard rails.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::{
    git_available, init_project, make_redirect_stub_bare_repo, run_git, vibe,
    write_project_with_per_package_registry,
};
use predicates::prelude::*;
use specmark::verifies;

// ---------------------------------------------------------------------------
// M1.16 — install via registry redirect stub (PROP-002 §2.4.2)
// ---------------------------------------------------------------------------

/// Build a target bare repo carrying `vibe.toml` + a single
/// content file. Used as the redirect target — `vibe install` reads
/// the manifest from this repo after following the stub's marker.
///
/// The `version` parameter must match the `--tag <vN.M.P>` the stub
/// surfaces (or the pinned ref) so the post-fetch identity check in
/// the resolver passes.
fn make_redirect_target_bare_repo(
    root: &Path,
    repo_name: &str,
    pkg_kind: &str,
    pkg_name: &str,
    version: &str,
    tag: &str,
) -> PathBuf {
    let src = root.join(format!("src-target-{repo_name}"));
    fs::create_dir_all(&src).unwrap();
    run_git(&src, &["init", "--initial-branch=main"]);
    run_git(&src, &["config", "user.email", "target@example.com"]);
    run_git(&src, &["config", "user.name", "Target"]);
    fs::write(src.join(".gitattributes"), "* text=auto eol=lf\n").unwrap();

    // Minimal valid package manifest. The package ships one content file
    // (`MANIFEST.md`), materialised verbatim into its vibedeps/ slot.
    let manifest = format!(
        r#"[package]
group = "org.vibevm"
name = "{pkg_name}"
kind = "{pkg_kind}"
version = "{version}"
"#
    );
    fs::write(src.join("vibe.toml"), manifest).unwrap();
    fs::create_dir_all(src.join(format!("spec/{pkg_kind}s/{pkg_name}"))).unwrap();
    fs::write(
        src.join(format!("spec/{pkg_kind}s/{pkg_name}/MANIFEST.md")),
        format!("# {pkg_kind}:{pkg_name}@{version}\nReached via redirect.\n"),
    )
    .unwrap();
    run_git(&src, &["add", "-A"]);
    run_git(
        &src,
        &["commit", "-m", &format!("{pkg_kind}:{pkg_name}@{version}")],
    );
    run_git(&src, &["tag", tag]);

    let bare = root.join(format!("{repo_name}.git"));
    run_git(
        root,
        &[
            "clone",
            "--bare",
            src.to_str().unwrap(),
            bare.to_str().unwrap(),
        ],
    );
    run_git(&bare, &["symbolic-ref", "HEAD", "refs/heads/main"]);
    bare
}

#[test]
#[verifies("spec://vibevm/modules/vibe-registry/PROP-002#redirect", r = 1)]
fn install_via_redirect_pass_through_tag() {
    if !git_available() {
        eprintln!("skipping install_via_redirect_pass_through_tag: git not on PATH");
        return;
    }
    let outer = tempfile::tempdir().unwrap();
    let outer_path = outer.path();
    let cache = outer_path.join("cache");
    fs::create_dir_all(&cache).unwrap();

    // Target lives outside the registry org, simulating an external
    // author's repo. URL form `file://...` so the resolver's git
    // archive operations work end to end.
    let target_root = outer_path.join("target-host");
    fs::create_dir_all(&target_root).unwrap();
    let target_bare = make_redirect_target_bare_repo(
        &target_root,
        "external-flow-internal",
        "flow",
        "internal",
        "0.1.0",
        "v0.1.0",
    );
    let target_url = format!(
        "file://{}",
        target_bare.to_string_lossy().replace('\\', "/")
    );

    // Stub lives at `<org_root>/org.vibevm.internal.git` so the resolver's
    // composed per-package URL hits it via the registry's naming
    // convention (fqdn — `<group>.<name>`).
    let org_root = outer_path.join("org-root");
    fs::create_dir_all(&org_root).unwrap();
    let _stub = make_redirect_stub_bare_repo(
        &org_root,
        "org.vibevm.internal",
        &target_url,
        "pass-through-tag",
        None,
        &["v0.1.0"],
    );

    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    let registry_url = format!(
        "git+file://{}",
        org_root.to_string_lossy().replace('\\', "/")
    );
    write_project_with_per_package_registry(project.path(), &registry_url);

    vibe()
        .env("VIBE_REGISTRY_CACHE", &cache)
        .arg("install")
        .arg("org.vibevm/internal@^0.1")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();

    let lock_text = fs::read_to_string(project.path().join("vibe.lock")).unwrap();
    let lock: vibe_core::manifest::Lockfile = toml::from_str(&lock_text).unwrap();
    assert_eq!(lock.packages.len(), 1);
    let entry = &lock.packages[0];
    assert_eq!(entry.kind, vibe_core::PackageKind::Flow);
    assert_eq!(entry.name, "internal");
    assert_eq!(entry.version.to_string(), "0.1.0");
    assert_eq!(entry.source_ref.as_deref(), Some("v0.1.0"));
    // Lockfile records the TARGET URL in source_url (this is where
    // content actually came from) and the STUB URL in via_redirect.
    assert!(
        entry.source_url.contains("external-flow-internal"),
        "expected target URL in source_url, got: {}",
        entry.source_url
    );
    assert!(
        entry.via_redirect.is_some(),
        "via_redirect must be populated for redirect-resolved packages"
    );
    let via = entry.via_redirect.as_deref().unwrap();
    assert!(
        via.contains("org.vibevm.internal") && !via.contains("external-flow-internal"),
        "expected stub URL in via_redirect, got: {via}"
    );
    assert_eq!(entry.registry.as_deref(), Some("default"));
    assert!(!entry.overridden);

    // Content from the TARGET was materialised verbatim into the
    // package's `vibedeps/` slot — keyed by the resolved `(kind, name,
    // version)`, regardless of the redirect indirection.
    let materialised = project
        .path()
        .join("vibedeps/flow-internal/0.1.0/spec/flows/internal/MANIFEST.md");
    assert!(
        materialised.is_file(),
        "expected target's content at {}",
        materialised.display()
    );
    let body = fs::read_to_string(&materialised).unwrap();
    assert!(
        body.contains("Reached via redirect"),
        "content body unexpected: {body}"
    );
}

#[test]
#[verifies("spec://vibevm/modules/vibe-registry/PROP-002#redirect", r = 1)]
fn install_via_redirect_pinned_policy_uses_pinned_ref() {
    if !git_available() {
        eprintln!("skipping install_via_redirect_pinned_policy: git not on PATH");
        return;
    }
    let outer = tempfile::tempdir().unwrap();
    let outer_path = outer.path();
    let cache = outer_path.join("cache");
    fs::create_dir_all(&cache).unwrap();

    // Target carries a single tag `v1.0.0` — the pinned ref the stub
    // points at. The stub also tags `v1.0.0` so the install
    // pipeline's pinned re-resolve hits the same slot; pinned policy
    // is exercised by the fact that the stub's marker carries
    // `pinned_ref = "v1.0.0"` rather than `pass-through-tag` semantics.
    // (The pure "stub_tag != pinned_ref" case is exercised by the
    // resolver-level hermetic test
    // `resolve_redirect_pinned_uses_pinned_ref` in vibe-registry,
    // which uses a FakeBackend; the install-pipeline shape requires
    // the stub to surface the resolved version as a tag for the
    // depsolver's pinned re-resolve.)
    let target_root = outer_path.join("target-host");
    fs::create_dir_all(&target_root).unwrap();
    let target_bare = make_redirect_target_bare_repo(
        &target_root,
        "external-flow-pinned",
        "flow",
        "pinned",
        "1.0.0",
        "v1.0.0",
    );
    let target_url = format!(
        "file://{}",
        target_bare.to_string_lossy().replace('\\', "/")
    );

    let org_root = outer_path.join("org-root");
    fs::create_dir_all(&org_root).unwrap();
    let _stub = make_redirect_stub_bare_repo(
        &org_root,
        "org.vibevm.pinned",
        &target_url,
        "pinned",
        Some("v1.0.0"),
        &["v1.0.0"],
    );

    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    let registry_url = format!(
        "git+file://{}",
        org_root.to_string_lossy().replace('\\', "/")
    );
    write_project_with_per_package_registry(project.path(), &registry_url);

    vibe()
        .env("VIBE_REGISTRY_CACHE", &cache)
        .arg("install")
        .arg("org.vibevm/pinned@^1")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();

    let lock_text = fs::read_to_string(project.path().join("vibe.lock")).unwrap();
    let lock: vibe_core::manifest::Lockfile = toml::from_str(&lock_text).unwrap();
    let entry = &lock.packages[0];
    assert_eq!(
        entry.version.to_string(),
        "1.0.0",
        "pinned policy resolves to target's pinned-ref version"
    );
    assert_eq!(entry.source_ref.as_deref(), Some("v1.0.0"));
    assert!(entry.via_redirect.is_some());
}

#[test]
#[verifies("spec://vibevm/modules/vibe-registry/PROP-002#redirect", r = 1)]
fn install_via_redirect_identity_mismatch_rejected() {
    if !git_available() {
        eprintln!("skipping install_via_redirect_identity_mismatch: git not on PATH");
        return;
    }
    let outer = tempfile::tempdir().unwrap();
    let outer_path = outer.path();
    let cache = outer_path.join("cache");
    fs::create_dir_all(&cache).unwrap();

    // Stub is hosted at `flow-internal` slot but redirects to a target
    // whose manifest declares `org.vibevm/something-else`. Per PROP-002
    // §2.4.2 identity check, the resolver refuses the install loud.
    let target_root = outer_path.join("target-host");
    fs::create_dir_all(&target_root).unwrap();
    let target_bare = make_redirect_target_bare_repo(
        &target_root,
        "wrong-identity",
        "feat",
        "something-else",
        "0.1.0",
        "v0.1.0",
    );
    let target_url = format!(
        "file://{}",
        target_bare.to_string_lossy().replace('\\', "/")
    );

    let org_root = outer_path.join("org-root");
    fs::create_dir_all(&org_root).unwrap();
    let _stub = make_redirect_stub_bare_repo(
        &org_root,
        "org.vibevm.internal",
        &target_url,
        "pass-through-tag",
        None,
        &["v0.1.0"],
    );

    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    let registry_url = format!(
        "git+file://{}",
        org_root.to_string_lossy().replace('\\', "/")
    );
    write_project_with_per_package_registry(project.path(), &registry_url);

    vibe()
        .env("VIBE_REGISTRY_CACHE", &cache)
        .arg("install")
        .arg("org.vibevm/internal@^0.1")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .failure()
        .stderr(predicate::str::contains("refusing to install"));

    // No file landed; no lockfile entry created.
    assert!(
        !project.path().join("vibe.lock").exists() || {
            let lock_text = fs::read_to_string(project.path().join("vibe.lock")).unwrap();
            let lock: vibe_core::manifest::Lockfile = toml::from_str(&lock_text).unwrap();
            lock.packages.is_empty()
        },
        "lockfile must not record a mismatched-identity package"
    );
}

#[test]
#[verifies("spec://vibevm/modules/vibe-registry/PROP-002#redirect", r = 1)]
fn install_via_redirect_chain_rejected_at_hop_two() {
    if !git_available() {
        eprintln!("skipping install_via_redirect_chain: git not on PATH");
        return;
    }
    let outer = tempfile::tempdir().unwrap();
    let outer_path = outer.path();
    let cache = outer_path.join("cache");
    fs::create_dir_all(&cache).unwrap();

    // Hop 2 stub — has its own vibe-redirect.toml; the resolver should
    // refuse before reading content.
    let hop2_root = outer_path.join("hop2-host");
    fs::create_dir_all(&hop2_root).unwrap();
    let hop2_target_bare = make_redirect_target_bare_repo(
        &hop2_root,
        "ultimate-target",
        "flow",
        "chain",
        "1.0.0",
        "v1.0.0",
    );
    let hop2_target_url = format!(
        "file://{}",
        hop2_target_bare.to_string_lossy().replace('\\', "/")
    );
    let hop2_stub = make_redirect_stub_bare_repo(
        &hop2_root,
        "org.vibevm.chain-hop2",
        &hop2_target_url,
        "pass-through-tag",
        None,
        &["v1.0.0"],
    );
    let hop2_stub_url = format!("file://{}", hop2_stub.to_string_lossy().replace('\\', "/"));

    // Hop 1 stub in the registry org. It points at hop2_stub_url
    // which is itself a stub. Hop limit = 1 — refuse.
    let org_root = outer_path.join("org-root");
    fs::create_dir_all(&org_root).unwrap();
    let _hop1_stub = make_redirect_stub_bare_repo(
        &org_root,
        "org.vibevm.chain",
        &hop2_stub_url,
        "pass-through-tag",
        None,
        &["v1.0.0"],
    );

    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    let registry_url = format!(
        "git+file://{}",
        org_root.to_string_lossy().replace('\\', "/")
    );
    write_project_with_per_package_registry(project.path(), &registry_url);

    vibe()
        .env("VIBE_REGISTRY_CACHE", &cache)
        .arg("install")
        .arg("org.vibevm/chain@^1")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("redirect chain not allowed")
                .or(predicate::str::contains("hop limit")),
        );
}

// ---------------------------------------------------------------------------
// `vibe registry redirect-update` — args-level guard rails (PROP-002 §2.4.2)
//
// The full apply path requires a real publish host (`creator_for_url` only
// dispatches to GitHub or GitVerse) and is exercised by the production
// smoke walk against `vibespecstest1/feat-helper`. The hermetic tests
// below cover the args parsing + early validation slice: flag-combo
// mutual exclusion, pkgref parsing, missing-manifest, and CLI help. The
// computational core (merging flags into a new RedirectSection, diffing,
// commit-message building) is covered by the unit tests in
// `commands::registry::tests`.
// ---------------------------------------------------------------------------

#[test]
fn redirect_update_help_lists_partial_update_flags() {
    let out = vibe()
        .arg("registry")
        .arg("redirect-update")
        .arg("--help")
        .output()
        .expect("spawn vibe");
    assert!(out.status.success(), "--help should succeed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    for flag in &[
        "--to",
        "--ref-policy",
        "--pinned-ref",
        "--target-auth",
        "--target-token-env",
        "--description",
        "--clear-description",
        "--trust-redirect",
        "--resync",
        "--dry-run",
    ] {
        assert!(
            stdout.contains(flag),
            "expected help to mention `{flag}`, got:\n{stdout}"
        );
    }
}

#[test]
#[verifies("spec://vibevm/modules/vibe-registry/PROP-002#redirect", r = 1)]
fn redirect_update_rejects_description_and_clear_combined() {
    // Mutual-exclusion check fires FIRST in the handler, before any
    // filesystem or network work — so we can test it with an empty
    // working directory (no `vibe.toml` needed).
    let scratch = tempfile::tempdir().unwrap();
    vibe()
        .arg("registry")
        .arg("redirect-update")
        .arg("org.vibevm/wal")
        .arg("--description")
        .arg("new text")
        .arg("--clear-description")
        .arg("--path")
        .arg(scratch.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("mutually exclusive"));
}

#[test]
#[verifies("spec://vibevm/modules/vibe-registry/PROP-002#redirect", r = 1)]
fn redirect_update_rejects_invalid_pkgref() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    write_project_with_per_package_registry(project.path(), "https://github.com/some-org");

    vibe()
        .arg("registry")
        .arg("redirect-update")
        .arg("not-a-pkgref")
        .arg("--description")
        .arg("anything")
        .arg("--path")
        .arg(project.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("not group-qualified"));
}

#[test]
fn redirect_update_rejects_missing_vibe_toml() {
    // Path exists but carries no `vibe.toml` — handler bails after the
    // mutual-exclusion check (passed: no `--clear-description`) with a
    // clear "no vibe.toml" hint pointing at `vibe init`.
    let scratch = tempfile::tempdir().unwrap();
    vibe()
        .arg("registry")
        .arg("redirect-update")
        .arg("org.vibevm/wal")
        .arg("--description")
        .arg("anything")
        .arg("--path")
        .arg(scratch.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("no `vibe.toml`"));
}
