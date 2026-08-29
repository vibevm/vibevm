//! The durable world adapter's REDs. Out-of-line so the production cell
//! keeps its own file-length budget, and so the `unwrap`s a fixture needs are
//! scoped as test code by the file-grain conform frontend. The fixture
//! scaffolding itself lives in [`super::test_support`], shared with the
//! sibling assertion cells along the `transform/plan_test_support.rs` seam.

use specmark::verifies;
use tempfile::TempDir;
use vibe_core::manifest::{Manifest, ProjectSection};
use vibe_extension_registry::{ContributionTier, ExtensionProvider};

use super::test_support::{
    fixture, found, group, id, key, keys, lock, locked, node, row, slot, world,
};
use super::{DurableExtensionWorld, ExtensionWorldError, collect_owner_view};

// --- the REDs -----------------------------------------------------------

/// Root lock order is the only dependency order the world knows. The fixture
/// lock is deliberately the reverse of alphabetical, so a name sort anywhere
/// — in the snapshot or in an owner view — reverses this sequence.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ORDER-LAW")]
fn lock_order_is_the_only_dependency_order_and_never_the_alphabetical_one() {
    let (workspace, manifest, lockfile) = fixture();
    let world = world(&workspace, &manifest, &lockfile);

    let snapshot = keys(
        world
            .installed()
            .map(|source| source.provider.id.to_string()),
    );
    assert_eq!(
        snapshot,
        ["org.zed/z-tools", "org.mid/m-tools", "org.aaa/a-tools"],
        "the snapshot is the lock, row for row"
    );
    let mut alphabetical = snapshot.clone();
    alphabetical.sort();
    assert_ne!(
        snapshot, alphabetical,
        "the fixture must be able to tell lock order from name order"
    );

    let view = world.node_owner_view().unwrap();
    assert_eq!(
        keys(
            view.installed
                .iter()
                .map(|source| source.provider.id.to_string())
        ),
        snapshot,
        "the node's owner view keeps the lock's order for its whole closure"
    );

    // And the collected registry's dependency rows follow it too.
    let registry = collect_owner_view(view, Vec::new()).unwrap();
    assert_eq!(
        keys(
            registry
                .rows()
                .iter()
                .filter(|row| row.provider().is_dependency())
                .map(|row| row.key().as_str().to_owned())
        ),
        [
            "org.zed/z-tools#z-src",
            "org.mid/m-tools#m-src",
            "org.aaa/a-tools#a-src",
            "org.aaa/a-tools#a-loud",
        ],
    );
}

/// A dependency's own `[[extensions.use]]` survives into the world — the
/// carriage §5.1 of the R4 architecture names as lost by today's
/// `DependencyExtensionSource` construction — and stays inert there: it is
/// data on the row, never a live control in the node owner's view.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#COMPILE-ACTIVATION")]
fn a_dependency_carries_its_own_controls_and_they_stay_inert_in_the_node_view() {
    let (workspace, manifest, lockfile) = fixture();
    let world = world(&workspace, &manifest, &lockfile);

    let carried = world
        .installed()
        .find(|source| source.provider.id == id("org.zed", "z-tools"))
        .expect("the lock installs z-tools");
    assert_eq!(
        carried.controls.uses.len(),
        1,
        "the package's own activation is retained verbatim on its source row"
    );
    assert_eq!(
        carried.controls.uses[0].reference,
        key("org.aaa/a-tools#a-src")
    );

    let registry = collect_owner_view(world.node_owner_view().unwrap(), Vec::new()).unwrap();
    let activated = row(&registry, "#a-src");
    assert!(
        !activated.is_activated(),
        "a dependency's activation cannot act in another owner's view"
    );
    assert!(!activated.is_enabled());
    assert_eq!(activated.activation_ordinal(), None);
    assert!(
        registry
            .enabled_compile_rows()
            .iter()
            .all(|row| row.key().as_str() != "org.aaa/a-tools#a-src"),
        "and it therefore never reaches the node lane's compile view"
    );
}

/// Owner scoping is exact in both directions, and a lane sees only its own
/// closure.
///
/// The node's disable acts in the node's view and is absent from z-tools';
/// z-tools' activation acts in z-tools' view and is absent from the node's.
/// In z-tools' lane the sibling `org.aaa/a-tools` appears only through the
/// dependency tier — never as host — while `org.mid/m-tools`, which z-tools
/// does not require, is not in that world at all.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#COMPILE-ACTIVATION")]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#HOST-ACTIVATION")]
fn owner_scoping_is_exact_in_both_directions_and_a_lane_sees_only_its_closure() {
    let (workspace, manifest, lockfile) = fixture();
    let world = world(&workspace, &manifest, &lockfile);

    let node_lane = collect_owner_view(world.node_owner_view().unwrap(), Vec::new()).unwrap();
    let owner = id("org.zed", "z-tools");
    let package_lane =
        collect_owner_view(world.package_owner_view(&owner).unwrap(), Vec::new()).unwrap();

    // The node's own control is live in the node's view…
    assert!(row(&node_lane, "#a-loud").is_disabled());
    assert!(!row(&node_lane, "#a-src").is_activated());
    // …and absent from the package's.
    assert!(
        !row(&package_lane, "#a-loud").is_disabled(),
        "the node's disable cannot reach a package's lane"
    );
    // The package's own control is live in its own view.
    let activated = row(&package_lane, "#a-src");
    assert!(activated.is_activated());
    assert_eq!(activated.activation_ordinal(), Some(0));
    assert_eq!(activated.effective_tier(), ContributionTier::HostActivation);

    // The host seat of each lane is that lane's owner, and only that owner.
    assert!(matches!(
        row(&node_lane, "#node-doc").provider(),
        ExtensionProvider::Host(_)
    ));
    assert!(matches!(
        row(&package_lane, "#z-src").provider(),
        ExtensionProvider::Host(_)
    ));
    assert!(
        !found(&package_lane, "#node-doc"),
        "the node's declarations are not in a package's lane at all"
    );
    assert!(
        matches!(
            row(&package_lane, "#a-src").provider(),
            ExtensionProvider::Dependency(_)
        ),
        "a sibling reaches a package's lane only through the dependency tier"
    );

    // Closure scoping: m-tools is the node's dependency, not z-tools'.
    assert!(found(&node_lane, "#m-src"));
    assert!(
        !found(&package_lane, "#m-src"),
        "a package's lane carries only the package's own closure"
    );
    // And the owner itself never occupies both seats.
    assert_eq!(
        world
            .package_owner_view(&owner)
            .unwrap()
            .installed
            .iter()
            .map(|source| source.provider.id.to_string())
            .collect::<Vec<_>>(),
        ["org.aaa/a-tools"],
    );

    // Each lane's compile view is that lane's own ordered list.
    assert_eq!(
        keys(
            node_lane
                .enabled_compile_rows()
                .iter()
                .map(|row| row.key().as_str().to_owned())
        ),
        ["__host__/demo#node-doc"],
    );
    assert_eq!(
        keys(
            package_lane
                .enabled_compile_rows()
                .iter()
                .map(|row| row.key().as_str().to_owned())
        ),
        ["org.zed/z-tools#z-src", "org.aaa/a-tools#a-src"],
    );
}

/// A component the model still carries as a bare string is parsed at this
/// seam through the one existing grammar, and refuses TYPED by naming the
/// component — never a panic, never a silent fallback identity, never a row
/// quietly dropped. The manifest is built structurally on purpose: the
/// adapter must refuse on its own, not lean on an upstream validation that a
/// caller may not have run.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
fn an_untypable_package_name_component_is_a_typed_refusal_naming_it() {
    let workspace = TempDir::new().unwrap();
    let manifest = Manifest {
        project: Some(ProjectSection {
            name: "Not A Package Name".to_owned(),
            group: Some(group("org.demo")),
            version: "0.1.0".to_owned(),
            spec_format: None,
            authors: Vec::new(),
        }),
        ..Manifest::default()
    };

    let error = DurableExtensionWorld::from_lock(
        workspace.path(),
        workspace.path(),
        &manifest,
        &lock(Vec::new()),
    )
    .expect_err("a grouped project's name must be a package name");

    let ExtensionWorldError::UntypedComponent {
        component,
        spelling,
        ..
    } = &error
    else {
        panic!("expected a typed component refusal, got: {error}");
    };
    assert_eq!(*component, "[project].name");
    assert_eq!(spelling, "Not A Package Name");
    let message = error.to_string();
    assert!(message.contains("[project].name"), "{message}");
    assert!(message.contains("fix:"), "{message}");
}

/// A slot whose manifest declares a different identity than the lock row
/// refuses by name: its declarations would otherwise enter the world under
/// the lock's key and be attributed to a package that never wrote them.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
fn a_slot_declaring_another_identity_refuses_by_name() {
    let workspace = TempDir::new().unwrap();
    let root = workspace.path();
    slot(
        root,
        "org.aaa",
        "a-tools",
        r#"
[package]
group = "org.aaa"
name = "a-tools"
kind = "flow"
version = "1.0.0"
"#,
    );
    let manifest = node(root, "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n");

    let error = DurableExtensionWorld::from_lock(
        root,
        root,
        &manifest,
        &lock(vec![locked("org.aaa", "a-tools", &[])]),
    )
    .expect_err("the slot declares kind `flow` where the lock requires `tool`");
    assert!(
        matches!(error, ExtensionWorldError::SlotIdentityMismatch { .. }),
        "{error}"
    );
    let message = error.to_string();
    assert!(message.contains("flow:org.aaa/a-tools@1.0.0"), "{message}");
    assert!(message.contains("tool:org.aaa/a-tools@1.0.0"), "{message}");
}

/// A coordinate the root lock does not install owns no unit lane in this
/// world, and asking for one refuses rather than returning an empty view.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#COMPILE-ACTIVATION")]
fn an_uninstalled_coordinate_owns_no_unit_lane() {
    let (workspace, manifest, lockfile) = fixture();
    let world = world(&workspace, &manifest, &lockfile);

    let error = world
        .package_owner_view(&id("org.ghost", "missing"))
        .expect_err("a coordinate outside the world owns no lane in it");
    assert!(
        matches!(error, ExtensionWorldError::UnknownOwner { .. }),
        "{error}"
    );
    assert!(error.to_string().contains("org.ghost/missing"));
}
