//! Owner scoping at the boot seam: a node lane and a package's unit lane are
//! scoped by DIFFERENT manifests, and the two entries here prove it on one
//! world rather than by inspection.
//!
//! **What the assertions read.** T5 ships an EMPTY production behavior
//! catalog, so any `compile:*` builtin a manifest declares today lowers to
//! the bounded `UnknownBuiltin` refusal — and that refusal names its lane
//! OWNER in full plus a bounded preview of the offending row. That makes it a
//! precise probe of which manifest scoped which lane: if the node's lane ever
//! saw the package's declaration, or the package's lane the node's, the
//! preview would move. It is the sharpest instrument available before R4.2
//! registers a real behavior, and it reads the property a byte assertion
//! would.
//!
//! The previews are capped at eight characters BY DESIGN (a declaration key
//! can be attacker-sized), so this fixture picks two builtin names whose
//! first eight characters differ — the assertion reads what the refusal law
//! actually shows, rather than asking it to show more.

use super::*;

use std::fs;

use specmark::verifies;
use tempfile::TempDir;
use vibe_core::manifest::{LockedPackage, Materialization};
use vibe_core::{ContentHash, PackageKind};

use crate::vibedeps::slot_abs_path;

/// The two builtin names, distinct within the bounded preview window.
const NODE_BEHAVIOR: &str = "nodeonly";
const PACKAGE_BEHAVIOR: &str = "pkgonly";

/// One workspace whose node and whose single installed package EACH declare
/// their own `compile:document` extension, and nothing else.
fn world() -> (TempDir, DurableExtensionWorld) {
    world_with_host_controls("")
}

/// [`world`], with extra host-manifest `[extensions]` controls appended —
/// the seam the collection-refusal pin drives.
fn world_with_host_controls(controls: &str) -> (TempDir, DurableExtensionWorld) {
    let workspace = TempDir::new().expect("a temp workspace");
    let root = workspace.path();

    let slot = slot_abs_path(
        root,
        &Group::parse("org.pkgs").expect("a valid group"),
        "tools",
        &semver::Version::parse("1.0.0").expect("a valid version"),
    );
    fs::create_dir_all(&slot).expect("the slot directory");
    fs::write(
        slot.join(Manifest::FILENAME),
        format!(
            r#"
[package]
group = "org.pkgs"
name = "tools"
kind = "tool"
version = "1.0.0"

[[extension]]
id = "package-only-transform"
point = "compile:document"
handler = {{ kind = "builtin", name = "{PACKAGE_BEHAVIOR}" }}
"#
        ),
    )
    .expect("the slot manifest");

    fs::write(
        root.join(Manifest::FILENAME),
        format!(
            r#"
[project]
group = "org.demo"
name = "host"
version = "0.1.0"

[requires.packages]
"org.pkgs/tools" = "=1.0.0"

[[extension]]
id = "node-only-transform"
point = "compile:document"
handler = {{ kind = "builtin", name = "{NODE_BEHAVIOR}" }}
{controls}"#
        ),
    )
    .expect("the node manifest");
    let manifest = Manifest::read(root.join(Manifest::FILENAME)).expect("the node manifest parses");

    let mut lockfile = Lockfile::empty("fixture", "1970-01-01T00:00:00Z");
    lockfile.packages = vec![LockedPackage {
        kind: PackageKind::Tool,
        name: PackageName::parse("tools").expect("a valid name"),
        group: Group::parse("org.pkgs").expect("a valid group"),
        version: semver::Version::parse("1.0.0").expect("a valid version"),
        registry: None,
        source_url: "file:///fixture".into(),
        source_ref: None,
        resolved_commit: None,
        content_hash: ContentHash::parse("sha256:aa").expect("a valid hash"),
        boot_snippet: None,
        files_written: Vec::new(),
        dependencies: Vec::new(),
        admitted_by: None,
        via_override: None,
        overridden: false,
        source_kind: None,
        via_redirect: None,
        features: Vec::new(),
        subskills_active: Vec::new(),
        describes: None,
        language: None,
        materialization: Materialization::Copy,
    }];

    let world = durable_world(root, root, &manifest, Some(&lockfile))
        .expect("the fixture lock and tree agree, so a world is observable");
    (workspace, world)
}

/// The one installed unit's identity, in the shape the unit table keys by.
fn unit() -> UnitId {
    (
        Group::parse("org.pkgs").expect("a valid group"),
        "tools".to_string(),
    )
}

/// PROP-054 `##COMPILE-ACTIVATION`, at this seam: the node lane sees the
/// NODE's declaration and not the package's; the package's unit lane sees the
/// package's and not the node's.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#COMPILE-ACTIVATION")]
fn the_node_lane_and_the_unit_lane_are_scoped_by_different_manifests() {
    let (_workspace, world) = world();

    let node = node_owner_plan(Some(&world), ".").expect_err("the empty T5 catalog refuses");
    let node = node.to_string();
    assert!(
        node.contains("`.`"),
        "the refusal names the lane owner it was scoped for: {node}"
    );
    assert!(
        node.contains(NODE_BEHAVIOR) && node.contains("org.demo"),
        "the node lane is scoped by the NODE's own manifest: {node}"
    );
    assert!(
        !node.contains("org.pkgs") && !node.contains(PACKAGE_BEHAVIOR),
        "a dependency's compile declaration is INERT in the node's view until \
         the node activates it: {node}"
    );

    let package = unit_owner_plan(Some(&world), &unit()).expect_err("the empty T5 catalog refuses");
    let package = package.to_string();
    assert!(
        package.contains("`org.pkgs/tools`"),
        "the refusal names the lane owner it was scoped for: {package}"
    );
    assert!(
        package.contains(PACKAGE_BEHAVIOR) && package.contains("org.pkgs"),
        "the unit lane is scoped by THAT package's own manifest: {package}"
    );
    assert!(
        !package.contains("org.demo") && !package.contains(NODE_BEHAVIOR),
        "nothing of the node enters a package's own lane: {package}"
    );
}

/// A world the adapter does not install owns no lane in it, so it compiles
/// with the empty plan — the same answer a tree with no lock gets, and for
/// the same reason (R4 architecture §3's orphan rule).
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#COMPILE-ACTIVATION")]
fn an_uninstalled_unit_and_a_lockless_tree_both_take_the_empty_plan() {
    let (_workspace, world) = world();
    let orphan = (
        Group::parse("org.pkgs").expect("a valid group"),
        "not-installed".to_string(),
    );
    assert!(
        unit_owner_plan(Some(&world), &orphan)
            .expect("an orphan is outside the extension world, not a fault")
            .is_empty()
    );
    assert!(
        node_owner_plan(None, ".")
            .expect("no lock, no world")
            .is_empty()
    );
    assert!(
        unit_owner_plan(None, &unit())
            .expect("no lock, no world")
            .is_empty()
    );
}

/// A lock that DISAGREES with the tree yields no observable world, and that
/// is not a fault at this seam (module doc, rule 1).
///
/// This is the ordinary mid-`vibe install` state, not a corner case: the
/// boot lane is written before the resolution's lock is published, so the
/// file on disk still lacks the package the node now requires. Refusing
/// there would fail every install that adds a dependency — which is exactly
/// what the first threading of this atom did, and what `cli_clean_and_world`
/// caught.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#COMPILE-ACTIVATION")]
fn a_lock_that_disagrees_with_the_tree_is_unobservable_rather_than_malformed() {
    let (workspace, _) = world();
    let root = workspace.path();
    let manifest = Manifest::read(root.join(Manifest::FILENAME)).expect("the node manifest");

    // The PRE-install epoch: a lock that does not yet know the package the
    // node requires. The world adapter itself refuses this — that strictness
    // is its own, and is asserted here so the tolerance below cannot be
    // mistaken for the adapter having gone soft.
    let stale = Lockfile::empty("fixture", "1970-01-01T00:00:00Z");
    assert!(
        DurableExtensionWorld::from_lock(root, root, &manifest, &stale)
            .expect("the snapshot itself succeeds: an empty lock materialises no slot")
            .node_owner_view()
            .is_err(),
        "the adapter refuses a closure its lock cannot resolve"
    );

    // The seam observes nothing and writes the historical lane.
    let world = durable_world(root, root, &manifest, Some(&stale));
    assert!(
        node_owner_plan(world.as_ref(), ".")
            .expect("a disagreement is not a fault at a seam that owns no epoch")
            .is_empty()
    );
    assert!(
        unit_owner_plan(world.as_ref(), &unit())
            .expect("same for a unit lane")
            .is_empty()
    );
}

/// Rule 2's COLLECTION half: once a world is observed, a refusal of the one
/// kernel collector is a real declaration defect and PROPAGATES — it is
/// never softened into the empty plan the unobservable-world rule returns.
///
/// The two rules in the module doc are different rules, and the lowering
/// half alone cannot keep them apart: the scoping test above already proves
/// a lowering refusal propagates, but a mutation that swallowed only
/// collection refusals would leave every other test green — this fixture's
/// duplicate activation is the one shape that reaches the collector and
/// refuses there. The lock and the tree agree, so rule 1 never fires.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#COMPILE-ACTIVATION")]
fn an_observed_worlds_collection_refusal_propagates_rather_than_emptying() {
    let (_workspace, world) = world_with_host_controls(
        r#"
[[extensions.use]]
ref = "org.pkgs/tools#package-only-transform"

[[extensions.use]]
ref = "org.pkgs/tools#package-only-transform"
"#,
    );
    let error = node_owner_plan(Some(&world), ".")
        .expect_err("a duplicate activation is a declaration defect, not an unobservable world");
    let WorkspaceError::ExtensionWorld { source } = &error else {
        panic!("a collection refusal keeps its own typed arm: {error}")
    };
    assert!(
        source.to_string().contains("duplicate [[extensions.use]]"),
        "the refusal names the duplicate activation: {source}"
    );
}

/// The per-unit emission path asks for the PACKAGE's plan.
///
/// The behavioural test above proves the two entries scope differently; this
/// proves the emission cell calls the right one. Swapping the call is the one
/// mutation that leaves every byte in this repository unchanged — every owner
/// here declares no compile-point extension, so both plans are empty — and
/// therefore the one a byte assertion cannot see.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#COMPILE-ACTIVATION")]
fn the_per_unit_emission_path_asks_for_the_packages_own_plan() {
    let emission = include_str!("hybrid_emit.rs");
    assert!(
        emission.contains("unit_owner_plan(world, id)"),
        "the per-unit path lowers THAT package's own view"
    );
    assert!(
        !emission.contains("node_owner_plan"),
        "the node's plan never reaches a package's unit lane"
    );
}
