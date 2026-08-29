//! Owner scoping at the boot seam: a node lane and a package's unit lane are
//! scoped by DIFFERENT manifests, and the two entries here prove it on one
//! world rather than by inspection.
//!
//! **What the assertions read.** The production behavior catalog ships
//! exactly `xml-minify`, so any OTHER `compile:*` builtin a manifest declares
//! lowers to the bounded `UnknownBuiltin` refusal — and that refusal names its
//! lane OWNER in full plus a bounded preview of the offending row. That makes
//! it a precise probe of which manifest scoped which lane: if the node's lane
//! ever saw the package's declaration, or the package's lane the node's, the
//! preview would move. It reads the property a byte assertion would, without
//! needing a lane to exist.
//!
//! The previews are capped at eight characters BY DESIGN (a declaration key
//! can be attacker-sized), so this fixture picks builtin names whose first
//! eight characters differ — the assertions read what the refusal law
//! actually shows, rather than asking it to show more.
//!
//! **Two installed packages, not one.** The second package exists so that a
//! tree with SEVERAL refusing owners is expressible. `unit_owner_plans`
//! promises a refusal that names the same owner every run; the walk order is
//! what delivers that, and a `HashMap` iteration order is not stable between
//! instances — so the promise is pinned behaviourally, over freshly built
//! tables, rather than by asserting the source contains a `sort` call.

use super::*;

use std::fs;

use specmark::verifies;
use tempfile::TempDir;
use vibe_core::manifest::{LockedPackage, Materialization};
use vibe_core::{ContentHash, PackageKind};

use crate::vibedeps::slot_abs_path;

/// The three builtin names, distinct within the bounded preview window.
const NODE_BEHAVIOR: &str = "nodeonly";
const PACKAGE_BEHAVIOR: &str = "pkgonly";
const SECOND_PACKAGE_BEHAVIOR: &str = "alphaonly";

/// The second installed package's name — lexicographically BEFORE `tools`, so
/// the canonical walk's answer is decidable and is not the insertion order of
/// either fixture.
const SECOND_PACKAGE: &str = "alpha";

/// Materialise one installed package's slot, declaring one `compile:document`
/// builtin of its own.
fn write_package_slot(root: &Path, name: &str, behavior: &str) {
    let slot = slot_abs_path(
        root,
        &Group::parse("org.pkgs").expect("a valid group"),
        name,
        &semver::Version::parse("1.0.0").expect("a valid version"),
    );
    fs::create_dir_all(&slot).expect("the slot directory");
    fs::write(
        slot.join(Manifest::FILENAME),
        format!(
            r#"
[package]
group = "org.pkgs"
name = "{name}"
kind = "tool"
version = "1.0.0"

[[extension]]
id = "package-only-transform"
point = "compile:document"
handler = {{ kind = "builtin", name = "{behavior}" }}
"#
        ),
    )
    .expect("the slot manifest");
}

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

    write_package_slot(root, "tools", PACKAGE_BEHAVIOR);

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
    lockfile.packages = vec![locked("tools")];

    let world = durable_world(root, root, &manifest, Some(&lockfile))
        .expect("the fixture lock and tree agree, so a world is observable");
    (workspace, world)
}

/// [`world`], plus a SECOND installed package whose own declaration also
/// refuses — the two-bad-owners tree.
///
/// The second package is deliberately NOT in the node's requires: it is
/// installed and owns its own unit lane, which is all a lane owner needs, and
/// leaving it outside the node's closure keeps every node-lane assertion in
/// this cell reading exactly what it read before.
fn world_with_two_refusing_owners() -> (TempDir, DurableExtensionWorld) {
    let (workspace, _) = world();
    let root = workspace.path();
    write_package_slot(root, SECOND_PACKAGE, SECOND_PACKAGE_BEHAVIOR);
    let manifest = Manifest::read(root.join(Manifest::FILENAME)).expect("the node manifest parses");
    let mut lockfile = Lockfile::empty("fixture", "1970-01-01T00:00:00Z");
    // Lock order deliberately puts the second package LAST, so a walk that
    // followed lock order rather than canonical unit order would answer
    // `tools` and the pin would be red.
    lockfile.packages = vec![locked("tools"), locked(SECOND_PACKAGE)];
    let world = durable_world(root, root, &manifest, Some(&lockfile))
        .expect("the fixture lock and tree agree, so a world is observable");
    (workspace, world)
}

/// One locked package in the shape the durable world adapter reads.
fn locked(name: &str) -> LockedPackage {
    LockedPackage {
        kind: PackageKind::Tool,
        name: PackageName::parse(name).expect("a valid name"),
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
    }
}

/// The one installed unit's identity, in the shape the unit table keys by.
fn unit() -> UnitId {
    (
        Group::parse("org.pkgs").expect("a valid group"),
        "tools".to_string(),
    )
}

/// The second installed unit's identity.
fn second_unit() -> UnitId {
    (
        Group::parse("org.pkgs").expect("a valid group"),
        SECOND_PACKAGE.to_string(),
    )
}

/// The orphan-unit id the tests reuse: a package the world does not install.
fn uninstalled_unit() -> UnitId {
    (
        Group::parse("org.pkgs").expect("a valid group"),
        "not-installed".to_string(),
    )
}

/// PROP-054 `##COMPILE-ACTIVATION`, at this seam: the node lane sees the
/// NODE's declaration and not the package's; the package's unit lane sees the
/// package's and not the node's.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#COMPILE-ACTIVATION")]
fn the_node_lane_and_the_unit_lane_are_scoped_by_different_manifests() {
    let (_workspace, world) = world();

    let node = node_owner_plan(Some(&world), ".").expect_err("an off-catalog builtin name refuses");
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

    let package =
        unit_owner_plan(Some(&world), &unit()).expect_err("an off-catalog builtin name refuses");
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
    let orphan = uninstalled_unit();
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

/// The per-unit emission path uses the PACKAGE's plan, and lowers nothing.
///
/// The behavioural test above proves the two entries scope differently; this
/// proves the emission cell reads the right one. Swapping the source is the
/// one mutation that leaves every byte in this repository unchanged — every
/// owner here declares no compile-point extension, so both plans are empty —
/// and therefore the one a byte assertion cannot see.
///
/// T10C moved the lowering OUT of the emission loop (R4 architecture §7.1: a
/// plan digest is an input to its unit's fingerprint, so plans must exist
/// before fingerprints), so the fence now reads two things: the emission cell
/// reads a per-unit plan off the key it is filed under, and it calls NEITHER
/// lowering entry — one lowering per unit per run, never two.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#COMPILE-ACTIVATION")]
fn the_per_unit_emission_path_asks_for_the_packages_own_plan() {
    let emission = include_str!("hybrid_emit.rs");
    assert!(
        emission.contains("plans.get(id)"),
        "the per-unit path reads THAT package's plan off its own key"
    );
    assert!(
        !emission.contains("unit_owner_plan") && !emission.contains("node_owner_plan"),
        "the emission cell lowers nothing: one lowering per unit per run"
    );
    let composition = include_str!("../bootgen.rs");
    assert!(
        composition.contains("unit_owner_plans(root_world.as_ref(), &table)"),
        "every table unit's plan is lowered once, from the run's ONE world snapshot"
    );
    // TWO occurrences, counted rather than merely found: the generate half
    // AND `verify_boot_graph`'s check half must frame the same digests, or
    // `vibe check` would call every framed unit stale on a tree the generator
    // had just left fresh — pinned behaviourally, now that a real behavior
    // exists, by `install::tests_minify_units`.
    // A `contains` alone was proven blind to exactly that mutation: the
    // generate half satisfied it while the check half framed nothing.
    assert_eq!(
        composition
            .matches("fingerprint::fingerprints(&table, &versions, &plan_digest_frames(")
            .count(),
        2,
        "the fingerprints are computed FROM the lowered plans in BOTH the \
         generate and the verify composition, never beside them"
    );
    assert!(
        composition.contains("unit_owner_plans(world.as_ref(), &table)"),
        "the check half lowers the same per-unit plans from its own observation"
    );
    // The canonical walk USED to be fenced here by spelling, because no
    // two-refusers fixture existed to state it behaviourally. One does now —
    // `a_two_refuser_tree_names_the_same_owner_over_freshly_built_tables` — so
    // the spelling assertion is retired rather than kept beside its twin: a
    // source-substring fence that a behavioural test already covers only adds
    // a second thing to update, and it would stay green under a `sort_by` that
    // ordered by something else.
}

/// Every table unit is lowered, in canonical order, exactly once — and the
/// frame map carries an entry only for a NONEMPTY plan.
///
/// The second half is the historical-identity law made mechanical at the seam
/// that produces the map: every owner in this repository (and in this
/// fixture) activates nothing, so the map is empty, so no fingerprint moves.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#COMPILE-ACTIVATION")]
fn every_table_unit_is_lowered_once_and_only_a_nonempty_plan_frames() {
    let (_workspace, world) = world();
    let table: HashMap<UnitId, UnitInput> = [unit(), uninstalled_unit()]
        .into_iter()
        .map(|id| (id, empty_unit_input()))
        .collect();

    // The installed unit declares a `compile:document` builtin the empty T5
    // catalog cannot resolve, so lowering the whole table refuses — which is
    // itself the proof that EVERY unit is lowered, not just the emitted ones.
    let error =
        unit_owner_plans(Some(&world), &table).expect_err("an off-catalog builtin name refuses");
    assert!(
        error.to_string().contains(PACKAGE_BEHAVIOR),
        "the refusal names the unit's own declaration: {error}"
    );

    // With no world, every unit takes the empty plan — and an empty plan
    // frames NOTHING, so no unit's fingerprint moves.
    let plans = unit_owner_plans(None, &table).expect("no lock, no world");
    assert_eq!(plans.len(), table.len(), "one plan per table unit");
    assert!(plans.values().all(TransformPlan::is_empty));
    assert!(
        plan_digest_frames(&plans).is_empty(),
        "an owner that activates nothing contributes no frame"
    );
}

/// The canonical-walk promise, stated behaviourally: on a tree with SEVERAL
/// refusing owners, the refusal names the lexicographically first unit — and
/// it names the same one over freshly built tables.
///
/// Freshly built, and many times, on purpose. `HashMap`'s iteration order is
/// seeded per instance, so one table walked twice proves nothing about a
/// second table; a mutation that dropped the canonical sort would pass such a
/// test roughly half the time. Thirty-two independent tables make the same
/// mutation red with probability `1 - 2^-32`.
///
/// The lock deliberately records the second package LAST, so the answer this
/// pins is the CANONICAL unit order and not the lock's.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#COMPILE-ACTIVATION")]
fn a_two_refuser_tree_names_the_same_owner_over_freshly_built_tables() {
    let (_workspace, world) = world_with_two_refusing_owners();
    for attempt in 0..32 {
        // A fresh table each round: a new `HashMap`, a new iteration order.
        let table: HashMap<UnitId, UnitInput> = [unit(), second_unit()]
            .into_iter()
            .map(|id| (id, empty_unit_input()))
            .collect();
        let error = unit_owner_plans(Some(&world), &table)
            .expect_err("both owners declare an off-catalog builtin")
            .to_string();
        assert!(
            error.contains("org.pkgs/alpha") && error.contains("alphaonl"),
            "attempt {attempt}: the refusal names the lexicographically FIRST \
             unit and its own declaration: {error}"
        );
        assert!(
            !error.contains("org.pkgs/tools") && !error.contains(PACKAGE_BEHAVIOR),
            "attempt {attempt}: the later owner is never the one reported: {error}"
        );
    }
}

/// A boot-bearing unit input with no edges — the table shape these entries
/// only need to be KEYED by.
fn empty_unit_input() -> UnitInput {
    UnitInput {
        own_boot_path: Some("boot/snippet.md".to_string()),
        fragments: Vec::new(),
        origin: String::new(),
        when: None,
        edges: Vec::new(),
        format: Default::default(),
    }
}
