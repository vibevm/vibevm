//! `build_install_resolver` flag-clause and posture tests — split out of
//! `builder.rs` to keep the production module under the 600-line file budget
//! (guide#surface-form). Each guard test flips one `PackageSourceOptions`
//! field at a time and asserts the guard fires before any registry is
//! opened; the posture tests drive the DEFAULT options (the hosted
//! disposition) against real on-disk registries.

// This whole file is test code (referenced via `#[cfg(test)] #[path]` in
// builder.rs). The `#[spec(deviates)]` on `empty_manifest` is the
// conform-recognised testimony for the test-fixture `.unwrap()` — the
// `#[path]` indirection hides the enclosing-module gate from the per-file
// fact extractor, so the deviation annotation carries the boundary here.

use std::path::Path;

use specmark::verifies;
use vibe_core::manifest::Manifest;
use vibe_core::{GlobalRegistryConfig, PackageRef};
use vibe_install::InstallSource;
// Linking this isolates this test binary's per-user settings home
// before the first `#[test]` body runs. Load-bearing since R1-RESOLVER:
// the offline bail consults the machine store (`store::list_all`), and
// without isolation that read would hit the operator's real
// `~/.vibe/cache` — a warm real store would break the bail tests.
use vibe_test_support as _;

use super::*;

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
    let options = PackageSourceOptions {
        embedded_short_circuit: true,
        no_prefer_embedded: true,
        ..PackageSourceOptions::default()
    };
    // A project root with no `packages/` so the project-local discovery
    // (PROP-030 §3.3) does not activate and the test stays focused on the
    // embedded-short-circuit × no-prefer-embedded guard.
    let project_root = tempfile::tempdir().unwrap();
    // `.map(|_| ())` so the `Ok` payload is `()` (Debug) — `InstallResolver`
    // deliberately isn't Debug (it holds live registry handles).
    let err = build_install_resolver(
        &options,
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
    // PROP-030 §3.1 + PROP-002 §2.2.2.1: the offline posture (resolved by
    // the SURFACE from its flags / `VIBE_OFFLINE` / `[net].offline`,
    // PROP-010 §2.5) with no embedded registry, no explicit registry path
    // (and no local registry in the merged effective set) has nothing local
    // to resolve from. It must fail with an actionable message rather than
    // fall through to the declared network walk (whose construction is
    // what a plain install does).
    // A project root with no `packages/` so project-local does not rescue
    // the bail (this test asserts the bail fires).
    let project_root = tempfile::tempdir().unwrap();
    let err = build_install_resolver(
        &PackageSourceOptions::default(),
        &empty_manifest(),
        None,
        project_root.path(),
        &GlobalRegistryConfig::default(),
        // the resolved posture, handed down by the surface.
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

/// PROP-030 §3.3: `--prefer-local` and `--no-prefer-local` are mutually
/// exclusive — same guard shape as the embedded pair.
#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-030#project-local",
    r = 1
)]
fn prefer_local_conflicts_with_no_prefer_local() {
    let options = PackageSourceOptions {
        prefer_local: true,
        no_prefer_local: true,
        ..PackageSourceOptions::default()
    };
    let project_root = tempfile::tempdir().unwrap();
    let err = build_install_resolver(
        &options,
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
/// options would bail with "no registry configured".
#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-030#project-local",
    r = 1
)]
fn project_local_packages_activate_resolver_without_vibe_embedded() {
    let project_root = temp_project_with_packages("org.vibevm", "wal", "the project-local copy");
    // embedded_root = None: this is the load-bearing case. Without
    // project-local, build_install_resolver would bail with "no registry
    // configured"; with project-local, it returns an Embedded resolver
    // whose local family is the single project-local registry.
    let resolver = build_install_resolver(
        &PackageSourceOptions::default(),
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

// ---- the DEFAULT-options posture (the later hosted disposition) ------------

/// Write one package into a local-registry-shaped tree rooted at `root`:
/// `<root>/<group>/<name>/v<version>/vibe.toml`, distinguishable by
/// `description`.
fn write_registry_package(root: &Path, group: &str, name: &str, version: &str, description: &str) {
    let dir = root.join(group).join(name).join(format!("v{version}"));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("vibe.toml"),
        format!(
            "[package]\ngroup = \"{group}\"\nname = \"{name}\"\nkind = \"tool\"\n\
             version = \"{version}\"\ndescription = \"{description}\"\n"
        ),
    )
    .unwrap();
}

/// A project whose `<root>/packages/` tree carries `group/name @ 0.1.0`.
fn temp_project_with_packages(group: &str, name: &str, description: &str) -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    write_registry_package(
        &project
            .path()
            .join(vibe_core::layout::current_packages_root()),
        group,
        name,
        "0.1.0",
        description,
    );
    project
}

/// A manifest declaring `group/name = ^0.1` — the "declared dependency"
/// a default-posture run resolves.
fn manifest_declaring(group: &str, name: &str) -> Manifest {
    Manifest::parse_str(&format!(
        "[package]\ngroup = \"org.host\"\nname = \"project\"\nkind = \"flow\"\n\
         version = \"0.1.0\"\n\n[requires]\npackages = {{ \"{group}/{name}\" = \"^0.1\" }}\n"
    ))
    .unwrap()
}

/// The hosted posture on a REAL project-local registry: default options
/// (no flags, public auth walk, every discovery lane on) build the
/// resolver, solve a DECLARED dependency through the selected cells, and
/// fetch it into the machine store tagged `is_local` — the later MCP path
/// is not empty-world-only.
#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-030#project-local",
    r = 1
)]
fn default_options_resolve_fetch_and_solve_a_declared_project_local_dependency() {
    let project = temp_project_with_packages("org.vibevm", "wal", "the default-posture copy");
    let manifest = manifest_declaring("org.vibevm", "wal");
    let resolver = build_install_resolver(
        &PackageSourceOptions::default(),
        &manifest,
        None,
        project.path(),
        &GlobalRegistryConfig::default(),
        false,
        &[],
    )
    .expect("default options build the resolver off a real project-local registry");

    // SOLVE: the declared root resolves through the selected provider +
    // solver cells to exactly itself (the fixture declares no deps).
    let root = PackageRef::parse("org.vibevm/wal@^0.1").unwrap();
    let graph = resolver.solve(std::slice::from_ref(&root)).expect(
        "the default posture solves a declared dependency — the hosted path \
         is not empty-world-only",
    );
    let nodes: Vec<String> = graph
        .iter()
        .map(|node| format!("{}/{}", node.group.as_str(), node.name))
        .collect();
    assert_eq!(nodes, vec!["org.vibevm/wal"], "solved graph: {nodes:?}");

    // FETCH: the resolved node lands in the store, tagged is_local
    // (portable, per-project packages/ — PROP-030 §3.3). The store root is
    // a per-test tempdir — PROP-010 §2.6 threads it in as a builder
    // parameter precisely so tests never share one.
    let node = graph.iter().next().unwrap().clone();
    let store_root = tempfile::tempdir().unwrap();
    let cached = resolver
        .resolve_and_fetch(
            &vibe_install::exact_pinned_pkgref(&node),
            store_root.path(),
            None,
        )
        .expect("the default posture fetches the solved node");
    assert!(cached.is_local, "a project-local fetch tags is_local");
    assert!(!cached.is_embedded, "no embedded registry is in play");
    assert_eq!(
        cached
            .manifest
            .package
            .as_ref()
            .unwrap()
            .description
            .as_deref(),
        Some("the default-posture copy")
    );
}

/// PROP-030 §3.3 ordering: the local family is project-local FIRST, then
/// vibe-embedded — a developer's own in-tree package wins a clash, and the
/// winning fetch tags `is_local`, never `is_embedded`.
#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-030#project-local",
    r = 1
)]
fn project_local_wins_a_clash_with_vibe_embedded_and_tags_is_local() {
    let project = temp_project_with_packages("org.vibevm", "wal", "the project-local copy");
    let embedded_root = tempfile::tempdir().unwrap();
    write_registry_package(
        embedded_root.path(),
        "org.vibevm",
        "wal",
        "0.1.0",
        "the embedded copy",
    );

    let resolver = build_install_resolver(
        &PackageSourceOptions::default(),
        &empty_manifest(),
        Some(embedded_root.path()),
        project.path(),
        &GlobalRegistryConfig::default(),
        false,
        &[],
    )
    .expect("both locals compose into one Embedded resolver");
    let pinned = PackageRef::parse("org.vibevm/wal@=0.1.0").unwrap();
    let store_root = tempfile::tempdir().unwrap();
    let cached = resolver
        .resolve_and_fetch(&pinned, store_root.path(), None)
        .expect("the clash resolves");
    assert!(cached.is_local, "the winner is the project-local copy");
    assert!(!cached.is_embedded, "the embedded copy lost the clash");
    assert_eq!(
        cached
            .manifest
            .package
            .as_ref()
            .unwrap()
            .description
            .as_deref(),
        Some("the project-local copy"),
        "project-local is FIRST in the local family (PROP-030 §3.3)"
    );
}

/// Absence fall-through inside the local family: a coordinate the
/// project-local tree lacks is served by vibe-embedded and tagged
/// `is_embedded` (machine-local) — so the lock records the right
/// source_kind and the reproducibility guard fires only for this half.
#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-030#project-local",
    r = 1
)]
fn embedded_serves_when_project_local_lacks_the_coordinate_and_tags_is_embedded() {
    let project = temp_project_with_packages("org.vibevm", "wal", "the project-local copy");
    let embedded_root = tempfile::tempdir().unwrap();
    // The embedded tree carries a DIFFERENT package: project-local does not
    // serve it, so the walk falls through to vibe-embedded.
    write_registry_package(
        embedded_root.path(),
        "org.vibevm",
        "secret",
        "0.2.0",
        "the embedded copy",
    );

    let resolver = build_install_resolver(
        &PackageSourceOptions::default(),
        &empty_manifest(),
        Some(embedded_root.path()),
        project.path(),
        &GlobalRegistryConfig::default(),
        false,
        &[],
    )
    .expect("both locals compose into one Embedded resolver");
    let pinned = PackageRef::parse("org.vibevm/secret@=0.2.0").unwrap();
    let store_root = tempfile::tempdir().unwrap();
    let cached = resolver
        .resolve_and_fetch(&pinned, store_root.path(), None)
        .expect("the embedded half serves the absent coordinate");
    assert!(cached.is_embedded, "a vibe-embedded fetch tags is_embedded");
    assert!(!cached.is_local, "the project-local tree never served it");
}

/// `--no-prefer-embedded` (EmbeddedLast) with no declared walk: the
/// declared walk's typed absence falls through to the local family, which
/// still serves — precedence selects the ORDER, never disables the locals.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-030#knob")]
fn embedded_last_still_falls_through_to_the_local_family() {
    let project = temp_project_with_packages("org.vibevm", "wal", "the project-local copy");
    let options = PackageSourceOptions {
        no_prefer_embedded: true,
        ..PackageSourceOptions::default()
    };
    let resolver = build_install_resolver(
        &options,
        &empty_manifest(),
        None, // no vibe-embedded; declared walk is empty (no [[registry]])
        project.path(),
        &GlobalRegistryConfig::default(),
        false,
        &[],
    )
    .expect("EmbeddedLast builds with only the local family");
    let pinned = PackageRef::parse("org.vibevm/wal@=0.1.0").unwrap();
    let store_root = tempfile::tempdir().unwrap();
    let cached = resolver
        .resolve_and_fetch(&pinned, store_root.path(), None)
        .expect("the declared absence falls through to the local family");
    assert!(cached.is_local, "the local family still serves");
    assert_eq!(
        cached
            .manifest
            .package
            .as_ref()
            .unwrap()
            .description
            .as_deref(),
        Some("the project-local copy")
    );
}

/// The auth gate rides the options into the declared walk: the strict bit
/// is handed to `MultiRegistryResolver::with_strict_auth` (PROP-002
/// §2.3.1). Proven structurally here — a declared registry must exist for
/// the walk to open, so the test asserts the resolver BUILDS with the bit
/// set rather than driving a 401 (the registry crate's walk tests own the
/// 401 semantics).
#[test]
fn auth_required_option_flows_into_the_declared_walk() {
    let project = temp_project_with_packages("org.vibevm", "wal", "the project-local copy");
    let options = PackageSourceOptions {
        auth_required: true,
        ..PackageSourceOptions::default()
    };
    let resolver = build_install_resolver(
        &options,
        &empty_manifest(),
        None,
        project.path(),
        &GlobalRegistryConfig::default(),
        false,
        &[],
    );
    match resolver {
        Ok(_) => { /* the strict bit is accepted on the composition */ }
        Err(e) => panic!("auth_required must compose, not refuse: {e}"),
    }
}

/// `has_git_source_flag` (the M1.15 `--git` grammar) lifts the
/// "no registry configured" bail for a git-source-only run — the surface
/// projects the flag; the manifest half is checked from the manifest.
#[test]
fn a_git_source_flag_lifts_the_no_registry_bail() {
    let project = tempfile::tempdir().unwrap(); // no packages/, no embedded
    let bail = build_install_resolver(
        &PackageSourceOptions::default(),
        &empty_manifest(),
        None,
        project.path(),
        &GlobalRegistryConfig::default(),
        false,
        &[],
    )
    .map(|_| ())
    .expect_err("no registry, no git source → the actionable bail");
    assert!(
        bail.to_string().contains("no registry configured"),
        "expected the no-registry bail; got: {bail}"
    );

    let options = PackageSourceOptions {
        has_git_source_flag: true,
        ..PackageSourceOptions::default()
    };
    build_install_resolver(
        &options,
        &empty_manifest(),
        None,
        project.path(),
        &GlobalRegistryConfig::default(),
        false,
        &[],
    )
    .expect("a git-source flag alone lifts the bail (git is the resolver)");
}
