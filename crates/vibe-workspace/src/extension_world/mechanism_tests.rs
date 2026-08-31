//! The durable adapter's mechanism carriage, proven through REAL manifests.
//!
//! The kernel's own cells prove the collection and selection laws over
//! hand-built worlds. What only this cell can prove is the carriage itself:
//! that `[[mechanism]]` reaches the world from the SAME parse `[[extension]]`
//! does, for both source kinds, and that the host's `[mechanisms]` routes —
//! which the world deliberately does not carry — drive selection over the
//! registry the adapter collected.

use specmark::verifies;
use tempfile::TempDir;
use vibe_core::manifest::{Manifest, MechanismKey, ProviderPin};
use vibe_extension_registry::{MechanismProvider, SelectionStep, resolve_mechanism};

use super::test_support::{id, lock, locked, node, resolved, slot};
use super::{
    DurableExtensionWorld, ExtensionWorldEpoch, collect_owner_mechanisms, collect_owner_view,
};

const PROVIDER_SLOT: &str = r#"
[package]
group = "org.example"
name = "build-tools"
kind = "tool"
version = "1.0.0"

[[extension]]
id = "announce"
point = "phase:build"
handler = { kind = "builtin", name = "log" }

[[mechanism]]
id = "cargo-v2"
role = "build"
name = "cargo"
handler = { kind = "native", crate_dir = "crates/cargo-provider" }
protocol = 1
config_schema = "schemas/cargo-build-v1.jtd.json"
freshness = "provider"

[mechanisms]
"build:cargo" = "org.example/build-tools#cargo-v2"
"#;

const ROUTED_NODE: &str = r#"
[project]
name = "demo"
version = "0.1.0"

[requires.packages]
"org.example/build-tools" = "=1.0.0"

[[mechanism]]
id = "house-zip"
role = "package"
name = "zip"
handler = { kind = "script", base = "scripts/zip" }
protocol = 1
config_schema = "schemas/house-zip.jtd.json"
freshness = "engine"

[mechanisms]
"build:cargo" = "org.example/build-tools#cargo-v2"
"#;

/// Write the shared fixture tree and snapshot the node's durable world.
fn fixture(node_body: &str) -> (TempDir, Manifest, DurableExtensionWorld) {
    let workspace = TempDir::new().unwrap();
    let root = workspace.path();
    slot(root, "org.example", "build-tools", PROVIDER_SLOT);
    let manifest = node(root, node_body);
    let lockfile = lock(vec![locked("org.example", "build-tools", &[])]);
    let world = DurableExtensionWorld::from_lock(root, root, &manifest, &lockfile).unwrap();
    (workspace, manifest, world)
}

fn key(spelling: &str) -> MechanismKey {
    spelling.parse().unwrap()
}

fn pin(spelling: &str) -> ProviderPin {
    spelling.parse().unwrap()
}

/// Both world source kinds carry their `[[mechanism]]` declarations out of the
/// one parse that already produced their `[[extension]]` declarations — the
/// carriage §3.0 freezes. An unchanged manifest keeps an empty vector, which
/// is the historical world exactly.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
fn the_durable_world_carries_mechanisms_from_the_same_parse_as_extensions() {
    let (_workspace, _manifest, world) = fixture(ROUTED_NODE);

    let installed = world
        .installed()
        .find(|source| source.provider.id == id("org.example", "build-tools"))
        .expect("the lock installs the provider package");
    assert_eq!(
        installed.declarations.len(),
        1,
        "its extension still arrives"
    );
    assert_eq!(installed.mechanisms.len(), 1);
    assert_eq!(installed.mechanisms[0].id, "cargo-v2");
    assert_eq!(installed.mechanisms[0].name, "cargo");
    assert_eq!(installed.mechanisms[0].handler.kind(), "native");

    assert_eq!(world.host().mechanisms.len(), 1);
    assert_eq!(world.host().mechanisms[0].id, "house-zip");

    // A package that declares no provider carries an empty vector, not an
    // absence the collector has to reason about.
    let (_bare_workspace, _bare_manifest, bare) =
        fixture("[project]\nname = \"demo\"\nversion = \"0.1.0\"\n");
    assert!(bare.host().mechanisms.is_empty());
}

/// The ordered-resolution epoch retains the already-parsed package manifest,
/// including routes that become live when this package owns its unit lane.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
fn the_resolution_epoch_retains_package_routes_and_materialised_root() {
    let (workspace, _manifest, _world) = fixture(ROUTED_NODE);
    let root = workspace.path();
    let owner = id("org.example", "build-tools");
    let epoch = ExtensionWorldEpoch::from_resolution(
        root,
        &[resolved(root, "org.example", "build-tools", &[])],
    )
    .unwrap();

    assert_eq!(
        epoch
            .package_manifest(&owner)
            .unwrap()
            .mechanism_routes
            .get("build:cargo"),
        Some(&pin("org.example/build-tools#cargo-v2"))
    );
    assert_eq!(
        epoch.package_root(&owner).unwrap(),
        crate::vibedeps::slot_abs_path(
            root,
            &vibe_core::Group::parse("org.example").unwrap(),
            "build-tools",
            &"1.0.0".parse().unwrap(),
        )
    );
}

/// The plan node's acceptance, end to end through the adapter: the host routes
/// `build:cargo` to a plugin, resolution returns the PLUGIN row, and the
/// builtin row is still collected and queryable — NOT SELECTED, which is the
/// honest level of proof at an atom with no execution.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_routed_plugin_displaces_the_builtin_while_the_builtin_stays_queryable() {
    let (_workspace, manifest, world) = fixture(ROUTED_NODE);
    let view = world.node_owner_view().unwrap();
    let registry = collect_owner_mechanisms(&view).unwrap();

    let build_cargo = key("build:cargo");
    let selection = resolve_mechanism(&registry, &build_cargo, None, &manifest.mechanism_routes)
        .expect("the authored route names an installed provider of this capability");

    assert_eq!(selection.via(), SelectionStep::HostRoute);
    assert_eq!(
        selection.row().pin(),
        &pin("org.example/build-tools#cargo-v2")
    );
    assert!(matches!(
        selection.row().provider(),
        MechanismProvider::Dependency(_)
    ));
    assert_eq!(
        selection
            .displaced_default()
            .map(|row| row.pin().to_string()),
        Some("org.vibevm/vibe#cargo".to_owned()),
    );

    let builtin = registry
        .builtin_default(&build_cargo)
        .expect("the shipped row is still collected");
    assert!(builtin.is_builtin());
    assert!(builtin.is_enabled());

    // The node's own provider is collected under the host-owner codec, and the
    // package's extension plane is untouched by any of this.
    assert!(registry.find(&pin("__host__/demo#house-zip")).is_some());
    let extensions = collect_owner_view(view, Vec::new()).unwrap();
    assert!(
        extensions
            .rows()
            .iter()
            .any(|row| row.key().as_str() == "org.example/build-tools#announce"),
    );
}

/// Take the route out of the very same tree and the shipped default answers
/// again — the restoration half of the fixture, at the adapter level.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn removing_the_route_from_the_manifest_restores_the_builtin() {
    let unrouted = ROUTED_NODE
        .split("[mechanisms]")
        .next()
        .expect("the fixture body has a routes table to remove");
    let (_workspace, manifest, world) = fixture(unrouted);
    assert!(manifest.mechanism_routes.is_empty());

    let view = world.node_owner_view().unwrap();
    let registry = collect_owner_mechanisms(&view).unwrap();
    let selection = resolve_mechanism(
        &registry,
        &key("build:cargo"),
        None,
        &manifest.mechanism_routes,
    )
    .expect("the shipped default answers an unrouted key");

    assert_eq!(selection.via(), SelectionStep::BuiltinDefault);
    assert_eq!(selection.row().pin(), &pin("org.vibevm/vibe#cargo"));
    // …and the plugin is still installed, still a candidate, still inert.
    assert!(
        registry
            .find(&pin("org.example/build-tools#cargo-v2"))
            .is_some(),
    );
}

/// A package's unit lane collects the package's OWN providers from its own
/// host seat, and the node's providers are not in that lane at all.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#COMPILE-ACTIVATION")]
fn a_package_lane_collects_the_package_s_own_providers_and_no_others() {
    let (_workspace, _manifest, world) = fixture(ROUTED_NODE);
    let owner = id("org.example", "build-tools");
    let view = world.package_owner_view(&owner).unwrap();
    let registry = collect_owner_mechanisms(&view).unwrap();

    let row = registry
        .find(&pin("org.example/build-tools#cargo-v2"))
        .expect("the package declares its own provider in its own lane");
    assert!(matches!(row.provider(), MechanismProvider::Host(_)));
    assert!(
        registry.find(&pin("__host__/demo#house-zip")).is_none(),
        "the node's providers are not in a package's lane"
    );
}
