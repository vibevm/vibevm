//! The mechanism plane's collection REDs: the engine's own source, the
//! provider-qualified key law, impersonation of the reserved owner, and the
//! disable controls one host list drives across both planes.

use specmark::verifies;
use vibe_core::manifest::{ExtensionKey, ExtensionsControl, MechanismDecl};
use vibe_core::{PackageKind, PackageName};

use crate::{
    CollectionError, HostExtensionSource, HostIdentity, HostProvider, MechanismProvider,
    builtin_mechanism_source, collect_extensions, collect_mechanisms,
};

use super::support::{
    dependency_source, host, host_source, mechanism, mechanism_key, provider_id, provider_package,
    provider_pin, world,
};

fn disable(spelling: &str) -> ExtensionsControl {
    ExtensionsControl {
        uses: Vec::new(),
        disable: vec![ExtensionKey::authored(spelling)],
    }
}

fn pins(registry: &crate::MechanismRegistry) -> Vec<String> {
    registry
        .rows()
        .iter()
        .map(|row| row.pin().to_string())
        .collect()
}

/// Collection order is builtins, then the installed world in LOCK order, then
/// the host — and a package's providers reach the plane from the same world
/// row its extensions do.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ORDER-LAW")]
fn collection_order_is_builtins_then_lock_order_then_host() {
    let registry = collect_mechanisms(&world(
        vec![
            provider_package(
                "org.zed",
                "z-tools",
                vec![mechanism("z-build", "build:zig")],
            ),
            provider_package(
                "org.aaa",
                "a-tools",
                vec![mechanism("a-build", "build:ant")],
            ),
        ],
        host_source(
            Vec::new(),
            ExtensionsControl::default(),
            vec![mechanism("node-build", "build:make")],
        ),
        None,
    ))
    .expect("a world of ordinary providers collects");

    assert_eq!(
        pins(&registry)[builtin_mechanism_source().declarations().len()..],
        [
            "org.zed/z-tools#z-build",
            "org.aaa/a-tools#a-build",
            "__host__/demo#node-build",
        ],
        "lock order, not name order, and the host last"
    );
    let plugin = registry
        .find(&provider_pin("org.zed/z-tools#z-build"))
        .expect("the installed row is keyed by its exact identity");
    assert_eq!(plugin.provider_ordinal(), Some(0));
    assert!(matches!(
        plugin.provider(),
        MechanismProvider::Dependency(_)
    ));
    let node = registry
        .find(&provider_pin("__host__/demo#node-build"))
        .expect("the host's own row is keyed through the host-owner codec");
    assert!(matches!(node.provider(), MechanismProvider::Host(_)));
    assert!(node.provider_ordinal().is_none());
}

/// Mutation 4's RED. A collected manifest that claims `org.vibevm/vibe`
/// refuses collection by name; merely BEING that coordinate without declaring
/// a provider takes nothing and is legal.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_collected_manifest_claiming_the_reserved_owner_refuses_collection() {
    let error = collect_mechanisms(&world(
        vec![provider_package(
            "org.vibevm",
            "vibe",
            vec![mechanism("cargo", "build:cargo")],
        )],
        host(Vec::new(), ExtensionsControl::default()),
        None,
    ))
    .expect_err("a stranger may not mint rows under the engine's identity");

    let CollectionError::ReservedMechanismOwner { owner, id } = &error else {
        panic!("expected an impersonation refusal, got: {error}");
    };
    assert_eq!(owner, "org.vibevm/vibe");
    assert_eq!(id, "cargo");
    assert!(error.to_string().contains("reserved engine provider"));

    // The same coordinate, declaring nothing, is an ordinary installed package.
    let registry = collect_mechanisms(&world(
        vec![provider_package("org.vibevm", "vibe", Vec::new())],
        host(Vec::new(), ExtensionsControl::default()),
        None,
    ))
    .expect("a package that declares no provider impersonates nothing");
    assert_eq!(
        registry.rows().len(),
        builtin_mechanism_source().declarations().len(),
    );
}

/// Mutation 5's RED. One provider identity, one row: a second declaration of
/// the same id inside one manifest is a collision, not a silent overwrite.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn two_declarations_of_one_provider_identity_refuse_collection() {
    let error = collect_mechanisms(&world(
        vec![provider_package(
            "org.example",
            "build-tools",
            vec![
                mechanism("cargo-v2", "build:cargo"),
                mechanism("cargo-v2", "package:zip"),
            ],
        )],
        host(Vec::new(), ExtensionsControl::default()),
        None,
    ))
    .expect_err("two sites cannot claim one exact identity");

    let CollectionError::DuplicateMechanismKey {
        pin, first, second, ..
    } = &error
    else {
        panic!("expected a key-law refusal, got: {error}");
    };
    assert_eq!(pin.to_string(), "org.example/build-tools#cargo-v2");
    assert!(first.contains("declaration index 0"), "{first}");
    assert!(second.contains("declaration index 1"), "{second}");
}

/// A pure virtual workspace may route and select but owns no coordinate to
/// declare under — the `[[extension]]` precedent, restated on this plane.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_pure_virtual_workspace_cannot_declare_a_provider() {
    let virtual_host = HostExtensionSource {
        provider: HostProvider {
            identity: HostIdentity::virtual_workspace(),
            root: std::path::PathBuf::from("."),
            version: String::new(),
            kind: None,
            content_hash: None,
        },
        declarations: Vec::new(),
        controls: ExtensionsControl::default(),
        mechanisms: vec![mechanism("coordinated", "build:make")],
    };
    let error = collect_mechanisms(&world(Vec::new(), virtual_host, None))
        .expect_err("a coordinator declares nothing");
    assert!(
        matches!(error, CollectionError::VirtualHostMechanism { ref id } if id == "coordinated"),
        "{error}"
    );
}

/// A caller-constructed declaration is validated by the same grammar the
/// manifest parser applies, and refuses naming the offending provider.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn an_invalid_declaration_refuses_by_provider_and_reason() {
    let mut broken = mechanism("cargo-v2", "build:cargo");
    broken.protocol = 0;
    let error = collect_mechanisms(&world(
        vec![provider_package("org.example", "build-tools", vec![broken])],
        host(Vec::new(), ExtensionsControl::default()),
        None,
    ))
    .expect_err("protocol 0 is not a provider protocol");

    let CollectionError::InvalidMechanism { owner, id, reason } = &error else {
        panic!("expected a grammar refusal, got: {error}");
    };
    assert_eq!(owner, "org.example/build-tools");
    assert_eq!(id, "cargo-v2");
    assert!(reason.contains("protocol"), "{reason}");
}

/// An arbitrary `[project].name` still keys its own providers, because the row
/// is spelled through the ONE reversible host-owner codec rather than an
/// interpolation. This is the machinery behind funnelling an identity that
/// cannot round-trip into the ordinary invalid-declaration refusal: for a
/// typed owner there is no such name.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#HOST-ACTIVATION")]
fn an_awkward_project_name_still_keys_its_providers_through_the_one_codec() {
    for (project, segment) in [
        ("demo", "demo"),
        ("my app", "my%20app"),
        ("odd/# project", "odd%2F%23%20project"),
        ("", ""),
    ] {
        let awkward = HostExtensionSource {
            provider: HostProvider {
                identity: HostIdentity::ungrouped_project(project),
                root: std::path::PathBuf::from("."),
                version: "0.1.0".into(),
                kind: None,
                content_hash: None,
            },
            declarations: Vec::new(),
            controls: ExtensionsControl::default(),
            mechanisms: vec![mechanism("node-build", "build:make")],
        };
        let registry = collect_mechanisms(&world(Vec::new(), awkward, None))
            .unwrap_or_else(|error| panic!("`{project}` keys its own rows: {error}"));
        let expected = format!("__host__/{segment}#node-build");
        assert!(
            registry.find(&provider_pin(&expected)).is_some(),
            "`{project}` must key exactly `{expected}`"
        );
    }
}

/// Disables apply to mechanism rows exactly as they apply to extension rows:
/// the same host list, the same exact-key match, and a disabled row that stays
/// queryable. The one disable list spans both planes, so the extension
/// collector must not call a mechanism key unknown.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#HOST-ACTIVATION")]
fn a_host_disable_reaches_a_mechanism_row_across_one_shared_control_list() {
    let installed = vec![provider_package(
        "org.example",
        "build-tools",
        vec![mechanism("cargo-v2", "build:cargo")],
    )];
    let controls = disable("org.example/build-tools#cargo-v2");
    let snapshot = world(installed.clone(), host(Vec::new(), controls.clone()), None);

    let registry = collect_mechanisms(&snapshot).expect("the disable names a real mechanism row");
    let row = registry
        .find(&provider_pin("org.example/build-tools#cargo-v2"))
        .expect("a disabled row stays retained and queryable");
    assert!(row.is_disabled());
    assert!(!row.is_enabled());

    // And the extension collector, which owns the unknown-key refusal, accepts
    // the very same list rather than calling a legal mechanism disable unknown.
    collect_extensions(snapshot).expect("a mechanism key is a KNOWN disable target");

    // A key that names neither plane is still unknown.
    let error = collect_extensions(world(
        installed,
        host(Vec::new(), disable("org.example/build-tools#ghost")),
        None,
    ))
    .expect_err("an identity in neither plane names nothing");
    assert!(
        matches!(error, CollectionError::UnknownDisable { .. }),
        "{error}"
    );
}

/// A shipped builtin default is engine-owned, so a host cannot switch it off —
/// the mechanism-plane twin of the reserved `@vibe/` contribution law. The
/// supported move is routing the logical key elsewhere.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#PRESET-LAW")]
fn a_host_cannot_disable_a_shipped_builtin_default() {
    let error = collect_mechanisms(&world(
        Vec::new(),
        host(Vec::new(), disable("org.vibevm/vibe#cargo")),
        None,
    ))
    .expect_err("the engine's own rows are immune to host controls");
    let CollectionError::ReservedMechanismControl { pin } = &error else {
        panic!("expected a reserved-control refusal, got: {error}");
    };
    assert_eq!(pin.to_string(), "org.vibevm/vibe#cargo");
}

/// One coordinate declares one provider set whichever seat it occupies: the
/// declarations a package carries as a dependency are the declarations it
/// carries as its own lane's host.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#COMPILE-ACTIVATION")]
fn a_package_carries_its_providers_into_its_own_lane_seat() {
    let source = dependency_source(
        "org.example",
        "build-tools",
        PackageKind::Tool,
        Vec::new(),
        ExtensionsControl::default(),
        vec![mechanism("cargo-v2", "build:cargo")],
    );
    let owner_seat = crate::lane_owner_host(&source);
    assert_eq!(owner_seat.mechanisms, source.mechanisms);

    let registry = collect_mechanisms(&world(Vec::new(), owner_seat, None))
        .expect("the owner seat collects its own providers");
    let row = registry
        .find(&provider_pin("org.example/build-tools#cargo-v2"))
        .expect("the package's own row, now under the host seat");
    assert!(matches!(row.provider(), MechanismProvider::Host(_)));
    assert_eq!(row.key(), &mechanism_key("build:cargo"));
}

/// The candidate view is membership, never selection: every row servicing one
/// key is listed, builtin and installed alike, in collection order.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-REGISTRY")]
fn candidates_list_every_row_servicing_one_key_in_collection_order() {
    let registry = collect_mechanisms(&world(
        vec![provider_package(
            "org.example",
            "build-tools",
            vec![mechanism("cargo-v2", "build:cargo")],
        )],
        host(Vec::new(), ExtensionsControl::default()),
        None,
    ))
    .expect("a world with one plugin provider collects");

    let key = mechanism_key("build:cargo");
    assert_eq!(
        registry
            .candidates(&key)
            .map(|row| row.pin().to_string())
            .collect::<Vec<_>>(),
        ["org.vibevm/vibe#cargo", "org.example/build-tools#cargo-v2"],
    );
    assert_eq!(
        registry
            .builtin_default(&key)
            .map(|row| row.pin().to_string()),
        Some("org.vibevm/vibe#cargo".to_owned()),
    );
    assert!(
        registry
            .builtin_default(&mechanism_key("build:zig"))
            .is_none()
    );
}

/// The row projects the declaration it was collected from without loss — the
/// registry-display evidence a later CLI atom reads.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-REGISTRY")]
fn a_row_projects_its_declaration_without_loss() {
    let declaration: MechanismDecl = mechanism("cargo-v2", "build:cargo");
    let registry = collect_mechanisms(&world(
        vec![provider_package(
            "org.example",
            "build-tools",
            vec![declaration.clone()],
        )],
        host(Vec::new(), ExtensionsControl::default()),
        None,
    ))
    .expect("the world collects");

    let row = registry
        .find(&provider_pin("org.example/build-tools#cargo-v2"))
        .expect("the plugin row");
    assert_eq!(row.declaration(), &declaration);
    assert_eq!(row.role(), declaration.role);
    assert_eq!(row.logical_name(), "cargo");
    assert_eq!(row.protocol(), declaration.protocol);
    assert_eq!(row.config_schema(), declaration.config_schema);
    assert_eq!(row.handler().kind(), "native");
    assert_eq!(
        row.provider().to_string(),
        provider_id("org.example", "build-tools").to_string(),
    );
    assert_eq!(
        PackageName::parse("build-tools").unwrap().as_str(),
        row.pin().package().unwrap().as_str(),
    );
}
