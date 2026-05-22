//! Live end-to-end tests against the public internet.
//!
//! These tests reach `github.com` and `gitverse.ru` — but NOT the
//! canonical `vibespecs` orgs. Smoke fixtures live in dedicated
//! test-orgs so the canonical orgs stay populated with only real,
//! installable packages:
//!
//! - GitHub: `https://github.com/vibespecstest1` (registry-side) +
//!   `https://github.com/vibespecstest2` (external-target side).
//! - GitVerse: `https://gitverse.ru/vibespecstest3`.
//!
//! Marked `#[ignore]` so `cargo test --workspace` stays hermetic;
//! run them explicitly with:
//!
//! ```
//! cargo test --test cli_live_e2e -- --ignored
//! ```
//!
//! What they prove
//! ===============
//!
//! 1. `cross_registry_resolution_routes_each_package_to_correct_host`
//!    — given a two-registry layout (GitHub `vibespecstest1` primary +
//!    GitVerse `vibespecstest3` secondary), `vibe install` resolves a
//!    GitHub-only package against GitHub and a GitVerse-only package
//!    against GitVerse, in a single invocation. The lockfile records
//!    the correct `registry` per package, proving the fall-through
//!    walk on `UnknownPackage` works against real hosts.
//! 2. `install_github_smoke_alone` / `install_gitverse_smoke_alone` —
//!    split-half coverage so that a failure in one host doesn't
//!    obscure the other in the cross-registry combined case.
//!
//! Test fixtures published live
//! ============================
//!
//! - GitHub: `https://github.com/vibespecstest1/flow-vibevm-github-smoke`
//!   (created via `vibe registry publish` API path; see
//!   `fixtures/manual-test-packages/flow-vibevm-github-smoke/`).
//! - GitVerse: `https://gitverse.ru/vibespecstest3/vibevm-direct-push-smoke`
//!   (created via `vibe registry publish --repo-url …` direct-push;
//!   see `fixtures/manual-test-packages/flow-vibevm-direct-push-smoke/`).
//!
//! Both carry `v0.0.1` and a single eager file plus a boot snippet —
//! enough to exercise the resolver, fetcher, integrity check, and
//! materialisation paths without burning a real package name in the
//! canonical `vibespecs` orgs.

use std::fs;
use std::path::Path;

use assert_cmd::Command;

const TEST_REGISTRY_GITHUB_NAME: &str = "vibespecstest1";
const TEST_REGISTRY_GITHUB_URL: &str = "https://github.com/vibespecstest1";
const TEST_REGISTRY_GITVERSE_NAME: &str = "vibespecstest3";
/// SSH form for GitVerse: the live test reaches it via the local
/// `gitverse.ru` SSH key (`spec/boot/90-user.md` covers the
/// preflight). HTTPS against GitVerse repos demands credentials
/// even for public reads — the canonical `vibespecs` org happens to
/// be publicly readable over HTTPS too, but new test orgs cannot
/// be assumed public, and SSH is the documented operator path.
const TEST_REGISTRY_GITVERSE_URL: &str = "git@gitverse.ru:vibespecstest3";

fn vibe() -> Command {
    Command::cargo_bin("vibe").expect("vibe binary built")
}

/// Initialise a project and overwrite the default `[[registry]]`
/// blocks with the live-test orgs. The `vibespecstest1` org hosts
/// GitHub-side fixtures (default `kind-name` naming, matching
/// canonical `vibespecs`); `vibespecstest3` hosts GitVerse-side
/// fixtures (default `name` naming for that registry, matching the
/// canonical `vibespecs-gitverse` shape).
fn init_project(dir: &Path) {
    vibe().arg("init").arg("--path").arg(dir).assert().success();
    let manifest = format!(
        r#"[project]
name = "live-e2e"
version = "0.0.1"

[[registry]]
name = "{TEST_REGISTRY_GITHUB_NAME}"
url = "{TEST_REGISTRY_GITHUB_URL}"

[[registry]]
name = "{TEST_REGISTRY_GITVERSE_NAME}"
url = "{TEST_REGISTRY_GITVERSE_URL}"
naming = "name"
"#
    );
    fs::write(dir.join("vibe.toml"), manifest).expect("vibe.toml writeable");
}

#[test]
#[ignore = "live: hits github.com — run with `cargo test --test cli_live_e2e -- --ignored`"]
fn install_github_smoke_alone() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());

    vibe()
        .arg("install")
        .arg("flow:vibevm-github-smoke")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();

    // Lockfile must record the GitHub registry as the source.
    let lock_text = fs::read_to_string(project.path().join("vibe.lock")).expect("lockfile present");
    let lock: vibe_core::manifest::Lockfile = toml::from_str(&lock_text).expect("lockfile parses");
    let pkg = lock
        .packages
        .iter()
        .find(|p| p.name == "vibevm-github-smoke")
        .expect("flow:vibevm-github-smoke must land in the lockfile");
    assert_eq!(
        pkg.registry.as_deref(),
        Some(TEST_REGISTRY_GITHUB_NAME),
        "GitHub package must attribute to `vibespecstest1` registry; lockfile entry: {pkg:?}"
    );
    assert!(
        pkg.source_url.contains("github.com"),
        "source_url must point at github.com; got `{}`",
        pkg.source_url
    );
    assert_eq!(pkg.version.to_string(), "0.0.1");

    // The package's eager file lands at the conventional path.
    assert!(
        project
            .path()
            .join("spec/flows/vibevm-github-smoke/PROTOCOL.md")
            .is_file(),
        "PROTOCOL.md must be materialised"
    );
}

#[test]
#[ignore = "live: hits gitverse.ru — run with `cargo test --test cli_live_e2e -- --ignored`"]
fn install_gitverse_smoke_alone() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());

    vibe()
        .arg("install")
        .arg("flow:vibevm-direct-push-smoke")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();

    // Lockfile must record the GitVerse registry as the source after
    // the GitHub `[[registry]]` returned `UnknownPackage` (the package
    // does not exist on GitHub by design).
    let lock_text = fs::read_to_string(project.path().join("vibe.lock")).expect("lockfile present");
    let lock: vibe_core::manifest::Lockfile = toml::from_str(&lock_text).expect("lockfile parses");
    let pkg = lock
        .packages
        .iter()
        .find(|p| p.name == "vibevm-direct-push-smoke")
        .expect("flow:vibevm-direct-push-smoke must land in the lockfile");
    assert_eq!(
        pkg.registry.as_deref(),
        Some(TEST_REGISTRY_GITVERSE_NAME),
        "GitVerse-only package must attribute to `vibespecstest3`; lockfile entry: {pkg:?}"
    );
    assert!(
        pkg.source_url.contains("gitverse.ru"),
        "source_url must point at gitverse.ru; got `{}`",
        pkg.source_url
    );
    assert_eq!(pkg.version.to_string(), "0.0.1");

    assert!(
        project
            .path()
            .join("spec/flows/vibevm-direct-push-smoke/PROTOCOL.md")
            .is_file(),
        "PROTOCOL.md must be materialised"
    );
}

#[test]
#[ignore = "live: hits github.com + gitverse.ru — run with `cargo test --test cli_live_e2e -- --ignored`"]
fn cross_registry_resolution_routes_each_package_to_correct_host() {
    // The headline test: prove that with both default registries
    // configured, two packages requested in the same `vibe install`
    // invocation route to the correct host based on which registry
    // carries them. Each is a name-only request (`flow:<name>`) — no
    // operator hint about which registry to use. The resolver walks
    // GitHub first (primary), falls through on `UnknownPackage`, and
    // lands on GitVerse for the package that only exists there.
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());

    vibe()
        .arg("install")
        .arg("flow:vibevm-github-smoke")
        .arg("flow:vibevm-direct-push-smoke")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();

    let lock_text = fs::read_to_string(project.path().join("vibe.lock")).expect("lockfile present");
    let lock: vibe_core::manifest::Lockfile = toml::from_str(&lock_text).expect("lockfile parses");

    let github_pkg = lock
        .packages
        .iter()
        .find(|p| p.name == "vibevm-github-smoke")
        .expect("github fixture installed");
    assert_eq!(
        github_pkg.registry.as_deref(),
        Some(TEST_REGISTRY_GITHUB_NAME),
        "github fixture must attribute to `vibespecstest1`; got: {github_pkg:?}"
    );
    assert!(
        github_pkg.source_url.contains("github.com"),
        "github fixture source_url must be on github.com; got `{}`",
        github_pkg.source_url
    );

    let gitverse_pkg = lock
        .packages
        .iter()
        .find(|p| p.name == "vibevm-direct-push-smoke")
        .expect("gitverse fixture installed");
    assert_eq!(
        gitverse_pkg.registry.as_deref(),
        Some(TEST_REGISTRY_GITVERSE_NAME),
        "gitverse fixture must attribute to `vibespecstest3`; got: {gitverse_pkg:?}"
    );
    assert!(
        gitverse_pkg.source_url.contains("gitverse.ru"),
        "gitverse fixture source_url must be on gitverse.ru; got `{}`",
        gitverse_pkg.source_url
    );

    // Sanity: integrity hashes are present + distinct between the two.
    assert!(github_pkg.content_hash.starts_with("sha256:"));
    assert!(gitverse_pkg.content_hash.starts_with("sha256:"));
    assert_ne!(
        github_pkg.content_hash, gitverse_pkg.content_hash,
        "different packages must produce different content hashes"
    );
}
