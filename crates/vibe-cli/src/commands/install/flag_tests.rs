//! The CLI adapter's own reds — the env-rung offline ladder resolving into
//! the shared builder's bail, and the `InstallArgs` → `PackageSourceOptions`
//! projection staying faithful. The guard-clause and posture reds moved to
//! `vibe-package-source` with the builder (R7.4 A15a); what must live HERE
//! is the surface wiring: this crate's ladder resolution and this crate's
//! projection.

// This whole file is test code (referenced via `#[cfg(test)] #[path]` in
// resolver.rs). The `#[spec(deviates)]` on `empty_manifest` is the
// conform-recognised testimony for the test-fixture `.unwrap()` — the
// `#[path]` indirection hides the enclosing-module gate from the per-file
// fact extractor, so the deviation annotation carries the boundary here.

use std::path::PathBuf;

use rust_ai_native_env_audit::EnvGuard;
use specmark::verifies;
use vibe_core::GlobalRegistryConfig;
use vibe_core::manifest::Manifest;
// Linking this isolates this test binary's per-user settings home
// before the first `#[test]` body runs. Load-bearing since R1-RESOLVER:
// the offline bail consults the machine store (`store::list_all`), and
// without isolation that read would hit the operator's real
// `~/.vibe/cache` — a warm real store would break the bail tests.
use vibe_test_support as _;

use super::*;

/// A fully-defaulted `InstallArgs` — every flag off — that tests flip
/// one field at a time.
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
        force: false,
        prefer_embedded: false,
        no_prefer_embedded: false,
        no_default_registry: false,
        offline: false,
        embedded_short_circuit: false,
        prefer_local: false,
        no_prefer_local: false,
        trace_compile: false,
    }
}

/// A minimal package manifest — no `[[registry]]`, so the declared walk
/// is empty. Enough for the bail under test, which reads only
/// `manifest.registries`.
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

/// PROP-010 §2.5 — the `VIBE_OFFLINE` rung: env alone (no flag, no
/// config key) resolves the posture, and the resulting run bails with
/// the same actionable message the `--offline` flag gives — the bail
/// happens at resolver construction (now in the shared
/// `vibe-package-source` builder), before any network walk. The ladder
/// half is THIS surface's (`output::resolve_offline`); this red pins the
/// two halves together.
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

/// The projection is a pure field copy: every `InstallArgs` field the
/// shared builder reads lands on its options twin, and nothing else
/// leaks across (no normalisation, no reordering, no defaults invented).
#[test]
#[verifies("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail")]
fn the_projection_copies_exactly_the_ten_builder_fields() {
    use vibe_package_source::PackageSourceOptions;

    // All-off: the hosted default posture, field for field.
    let off = package_source_options(&base_args());
    assert_eq!(off, PackageSourceOptions::default());

    // Every flag on: each field carries its own value across, untouched.
    let mut on = base_args();
    on.registry = Some(PathBuf::from("C:/reg"));
    on.solver = Some("sat".into());
    on.auth_required = true;
    on.prefer_embedded = true;
    on.no_prefer_embedded = false;
    on.no_default_registry = true;
    on.embedded_short_circuit = true;
    on.prefer_local = true;
    on.no_prefer_local = false;
    on.git = Some("https://example.invalid/x.git".into());
    let projected = package_source_options(&on);
    assert_eq!(
        projected,
        PackageSourceOptions {
            registry: Some(PathBuf::from("C:/reg")),
            solver: Some("sat".into()),
            auth_required: true,
            prefer_embedded: true,
            no_prefer_embedded: false,
            no_default_registry: true,
            embedded_short_circuit: true,
            prefer_local: true,
            no_prefer_local: false,
            has_git_source_flag: true,
        },
        "the projection is the pure field copy the shared builder reads"
    );
}
