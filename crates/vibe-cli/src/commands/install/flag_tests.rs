//! `build_install_resolver` flag-clause tests — split out of `resolver.rs`
//! to keep the production module under the 600-line file budget
//! (guide#surface-form). Each test flips one `InstallArgs` field at a time
//! and asserts the guard fires before any registry is opened.

// This whole file is test code (referenced via `#[cfg(test)] #[path]` in
// resolver.rs). The `#[spec(deviates)]` on `empty_manifest` is the
// conform-recognised testimony for the test-fixture `.unwrap()` — the
// `#[path]` indirection hides the enclosing-module gate from the per-file
// fact extractor, so the deviation annotation carries the boundary here.

use std::path::PathBuf;

use rust_ai_native_env_audit::EnvGuard;
use specmark::verifies;
// Linking this isolates this test binary's per-user settings home
// before the first `#[test]` body runs. Load-bearing since R1-RESOLVER:
// the offline bail consults the machine store (`store::list_all`), and
// without isolation that read would hit the operator's real
// `~/.vibe/cache` — a warm real store would break the bail tests.
use vibe_test_support as _;

use super::*;

/// A fully-defaulted `InstallArgs` — every flag off — that tests flip
/// one field at a time to exercise a single guard clause.
fn base_args() -> InstallArgs {
    InstallArgs {
        packages: Vec::new(),
        path: PathBuf::from("."),
        registry: None,
        assume_yes: false,
        language: None,
        features: Vec::new(),
        no_default_features: false,
        all_features: false,
        exact: false,
        auth_required: false,
        solver: None,
        git: None,
        tag: None,
        branch: None,
        rev: None,
        git_auth: None,
        git_token_env: None,
        allow_hooks: false,
        force: false,
        prefer_embedded: false,
        no_prefer_embedded: false,
        no_default_registry: false,
        offline: false,
        embedded_short_circuit: false,
        prefer_local: false,
        no_prefer_local: false,
    }
}

/// A minimal package manifest — no `[[registry]]`, so the declared walk
/// is empty. Enough for the guard clauses under test, which read only
/// `manifest.registries` (and only after the guards they exercise).
#[specmark::spec(
    deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "no-unwrap-gate: a test fixture over a static valid-manifest literal — \
              parse_str cannot fail on this input; the .unwrap() is a one-off assertion \
              at the test-fixture boundary, not domain logic."
)]
fn empty_manifest() -> Manifest {
    Manifest::parse_str(
        "[package]\ngroup = \"org.vibevm\"\nname = \"x\"\nkind = \"flow\"\nversion = \"0.1.0\"\n",
    )
    .unwrap()
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-030#knob")]
fn short_circuit_conflicts_with_embedded_last() {
    // PROP-030 §3.1: `--embedded-short-circuit` presupposes
    // embedded-first precedence, so pairing it with
    // `--no-prefer-embedded` is a contradiction rejected up front —
    // before any registry is opened or the network is touched.
    let mut args = base_args();
    args.embedded_short_circuit = true;
    args.no_prefer_embedded = true;
    // A project root with no `packages/` so the project-local discovery
    // (PROP-030 §3.3) does not activate and the test stays focused on the
    // embedded-short-circuit × no-prefer-embedded guard.
    let project_root = tempfile::tempdir().unwrap();
    // `.map(|_| ())` so the `Ok` payload is `()` (Debug) — `InstallResolver`
    // deliberately isn't Debug (it holds live registry handles).
    let err = build_install_resolver(
        &args,
        &empty_manifest(),
        None,
        project_root.path(),
        &GlobalRegistryConfig::default(),
        // offline posture (PROP-010 §2.5) — online for this test.
        false,
        &[],
    )
    .map(|_| ())
    .unwrap_err();
    assert!(
        err.to_string().contains("mutually exclusive"),
        "expected a mutual-exclusivity error; got: {err}"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-030#knob")]
fn offline_without_a_local_registry_bails_before_the_network() {
    // PROP-030 §3.1 + PROP-002 §2.2.2.1: the offline posture (today
    // reachable through `vibe install --offline`; PROP-010 §2.5 widened it
    // to the root flag / env / config ladder) with no embedded registry
    // and no `--registry` (and no local registry in the merged effective
    // set) has nothing local to resolve from. It must fail with an
    // actionable message rather than fall through to the declared network
    // walk (whose construction is what a plain install does).
    // A project root with no `packages/` so project-local does not rescue
    // the bail (this test asserts the bail fires).
    let mut args = base_args();
    args.offline = true;
    let project_root = tempfile::tempdir().unwrap();
    let err = build_install_resolver(
        &args,
        &empty_manifest(),
        None,
        project_root.path(),
        &GlobalRegistryConfig::default(),
        // the resolved posture — what `install::run` computes as
        // `resolve_offline(root_offline || args.offline, [net].offline)`.
        true,
        &[],
    )
    .map(|_| ())
    .unwrap_err();
    assert!(
        err.to_string().contains("--offline"),
        "expected the offline bail; got: {err}"
    );
}

/// PROP-010 §2.5 — the `VIBE_OFFLINE` rung: env alone (no flag, no
/// config key) resolves the posture, and the resulting run bails with
/// the same actionable message the `--offline` flag gives — the bail
/// happens at resolver construction, before any network walk. The
/// sibling test above drives the flag rung; this one drives env, so
/// together they pin the ladder's two upper rungs to the same bail.
#[test]
fn env_offline_alone_bails_before_the_network_with_the_same_message() {
    let mut env = EnvGuard::lock();
    env.set("VIBE_OFFLINE", "1");
    // No CLI flag, no config key — the env-var alone carries it.
    let offline = crate::output::resolve_offline(false, false);
    assert!(offline, "VIBE_OFFLINE=1 must resolve the posture");

    let args = base_args();
    let project_root = tempfile::tempdir().unwrap();
    let err = build_install_resolver(
        &args,
        &empty_manifest(),
        None,
        project_root.path(),
        &GlobalRegistryConfig::default(),
        offline,
        &[],
    )
    .map(|_| ())
    .unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("--offline: no local registry available"),
        "expected the offline bail; got: {msg}"
    );
}

/// PROP-030 §3.3: `--prefer-local` and `--no-prefer-local` are mutually
/// exclusive — same guard shape as the embedded pair.
#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-030#project-local",
    r = 1
)]
fn prefer_local_conflicts_with_no_prefer_local() {
    let mut args = base_args();
    args.prefer_local = true;
    args.no_prefer_local = true;
    let project_root = tempfile::tempdir().unwrap();
    let err = build_install_resolver(
        &args,
        &empty_manifest(),
        None,
        project_root.path(),
        &GlobalRegistryConfig::default(),
        false, // online posture — this test exercises the guard, not the bail
        &[],
    )
    .map(|_| ())
    .unwrap_err();
    assert!(
        err.to_string().contains("--prefer-local"),
        "expected a prefer-local mutual-exclusivity error; got: {err}"
    );
}

/// PROP-030 §3.3: a project with `<project_root>/packages/` resolves
/// successfully even when `embedded_root` is `None` (cargo run, test
/// harness, distribution install). Project-local discovery is NOT gated
/// on the running vibe being source-installed, so the local family is
/// non-empty and the resolver is built — without project-local, the same
/// args would bail with "no registry configured".
#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-030#project-local",
    r = 1
)]
fn project_local_packages_activate_resolver_without_vibe_embedded() {
    let project_root = tempfile::tempdir().unwrap();
    // A real packages tree the discovery helper recognises. Needs at
    // least one valid package so opening the LocalRegistry is cheap, but
    // the resolver itself does not read it here — only its presence
    // flips the construction path from the bail to the Embedded variant.
    let pkg_root = project_root
        .path()
        .join(vibe_core::layout::current_packages_root());
    std::fs::create_dir_all(pkg_root.join("org.vibevm").join("wal").join("v0.1.0")).unwrap();
    std::fs::write(
        pkg_root
            .join("org.vibevm")
            .join("wal")
            .join("v0.1.0")
            .join("vibe.toml"),
        "[package]\ngroup=\"org.vibevm\"\nname=\"wal\"\nkind=\"flow\"\nversion=\"0.1.0\"\n",
    )
    .unwrap();

    let args = base_args();
    // embedded_root = None: this is the load-bearing case. Without
    // project-local, build_install_resolver would bail with "no registry
    // configured"; with project-local, it returns an Embedded resolver
    // whose local family is the single project-local registry.
    let resolver = build_install_resolver(
        &args,
        &empty_manifest(),
        None,
        project_root.path(),
        &GlobalRegistryConfig::default(),
        false, // online posture — the resolver must build, not bail
        &[],
    );
    match resolver {
        Ok(_) => { /* the load-bearing assertion: success, not the bail */ }
        Err(e) => panic!(
            "project-local packages/ should activate the resolver even with \
             no vibe-embedded; got: {e}"
        ),
    }
}
