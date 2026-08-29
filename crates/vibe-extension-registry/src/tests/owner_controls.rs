use specmark::verifies;
use vibe_core::PackageKind;
use vibe_core::manifest::{ExtensionKey, ExtensionUse, ExtensionsControl};

use crate::{
    CollectionError, ContributionTier, HostIdentity, SelectorSubject, collect_extensions,
    lane_owner_host,
};

use super::support::{
    config, declaration, dependency, dependency_with_controls, host, package_key, provider_id,
    world,
};

fn row<'registry>(
    registry: &'registry crate::ExtensionRegistry,
    suffix: &str,
) -> &'registry crate::ExtensionRegistryRow {
    registry
        .rows()
        .iter()
        .find(|row| row.key().as_str().ends_with(suffix))
        .unwrap_or_else(|| panic!("test row with suffix `{suffix}` exists"))
}

/// An installed source retains its package's parsed controls exactly, and the
/// dependency-seat → owner-seat projection preserves provider identity,
/// root, version, kind, content hash, declarations and those controls.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#COMPILE-ACTIVATION")]
fn controls_survive_adapter_construction_and_projection_exactly() {
    let controls = ExtensionsControl {
        uses: vec![ExtensionUse {
            reference: package_key("org.sib", "s-tools", "s-compile"),
            config: Some(config(&[("level", "owner")])),
        }],
        disable: vec![package_key("org.sib", "s-tools", "s-loud")],
    };
    let source = dependency_with_controls(
        "org.demo",
        "tools",
        PackageKind::Tool,
        vec![declaration("quiet", "compile:source")],
        controls.clone(),
    );
    assert_eq!(source.controls, controls);

    let owner = lane_owner_host(&source);
    assert_eq!(owner.controls, controls);
    assert_eq!(owner.declarations, source.declarations);
    assert_eq!(
        owner.provider.identity,
        HostIdentity::coordinate(provider_id("org.demo", "tools"))
    );
    assert_eq!(owner.provider.root, source.provider.root);
    assert_eq!(owner.provider.version, source.provider.version);
    assert_eq!(owner.provider.kind, Some(source.provider.kind));
    assert_eq!(
        owner.provider.content_hash,
        Some(source.provider.content_hash.clone())
    );
}

/// A dependency's own controls are inert in a selected host world: no
/// activation, no disable, and no validation — even references no world could
/// resolve must not error while the row sits in the installed vector.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#HOST-ACTIVATION")]
fn dependency_controls_are_inert_in_a_selected_host_world() {
    let controls = ExtensionsControl {
        uses: vec![
            ExtensionUse {
                reference: package_key("org.demo", "tools", "rewrite"),
                config: None,
            },
            ExtensionUse {
                reference: ExtensionKey::authored("org.ghost/missing#nope"),
                config: None,
            },
        ],
        disable: vec![package_key("org.demo", "tools", "announce")],
    };
    let registry = collect_extensions(world(
        vec![dependency_with_controls(
            "org.demo",
            "tools",
            PackageKind::Tool,
            vec![
                declaration("announce", "phase:test"),
                declaration("rewrite", "compile:source"),
            ],
            controls,
        )],
        host(Vec::new(), ExtensionsControl::default()),
        None,
    ))
    .unwrap();

    let announce = row(&registry, "#announce");
    assert!(!announce.is_disabled());
    assert!(
        registry
            .plan("phase:test".parse().unwrap(), SelectorSubject::unscoped())
            .len()
            == 1
    );
    let rewrite = row(&registry, "#rewrite");
    assert!(!rewrite.is_activated());
    assert!(!rewrite.is_enabled());
    assert!(
        registry
            .plan(
                "compile:source".parse().unwrap(),
                SelectorSubject::unscoped()
            )
            .is_empty()
    );
}

/// Projected into the host seat of its own lane, a package's retained
/// controls become that lane's live controls: they activate and disable the
/// intended dependency rows, with whole-value config replacement.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#COMPILE-ACTIVATION")]
fn projected_owner_controls_activate_and_disable_dependency_rows() {
    let owner = dependency_with_controls(
        "org.demo",
        "tools",
        PackageKind::Tool,
        vec![declaration("own", "compile:source")],
        ExtensionsControl {
            uses: vec![ExtensionUse {
                reference: package_key("org.sib", "s-tools", "s-compile"),
                config: Some(config(&[("level", "owner")])),
            }],
            disable: vec![package_key("org.sib", "s-tools", "s-loud")],
        },
    );
    let sibling = dependency(
        "org.sib",
        "s-tools",
        vec![
            declaration("s-compile", "compile:source"),
            declaration("s-loud", "phase:test"),
        ],
    );
    let registry = collect_extensions(world(vec![sibling], lane_owner_host(&owner), None)).unwrap();

    let activated = row(&registry, "#s-compile");
    assert!(activated.is_activated());
    assert!(activated.is_enabled());
    assert_eq!(activated.activation_ordinal(), Some(0));
    assert_eq!(
        activated.effective_config().unwrap().as_table()["level"].as_str(),
        Some("owner")
    );
    let disabled = row(&registry, "#s-loud");
    assert!(disabled.is_disabled());
    assert!(!disabled.is_enabled());
    let own = row(&registry, "#own");
    assert_eq!(own.effective_tier(), ContributionTier::HostDeclaration);
    assert!(own.is_enabled());
    assert_eq!(
        registry
            .plan(
                "compile:source".parse().unwrap(),
                SelectorSubject::unscoped()
            )
            .len(),
        2
    );
}

/// Host, package and sibling controls each govern exactly their own world:
/// the host's disable is absent from the package's lane, the package's
/// activation is absent from the host's world, and a sibling's controls are
/// inert everywhere except the sibling's own lane — where their unresolved
/// reference finally becomes the loud error it always was.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#HOST-ACTIVATION")]
fn host_package_and_sibling_controls_do_not_leak_across_worlds() {
    let tools = dependency_with_controls(
        "org.demo",
        "tools",
        PackageKind::Tool,
        vec![declaration("quiet", "compile:source")],
        ExtensionsControl {
            uses: vec![ExtensionUse {
                reference: package_key("org.sib", "s-tools", "s-compile"),
                config: None,
            }],
            disable: Vec::new(),
        },
    );
    let sibling = dependency_with_controls(
        "org.sib",
        "s-tools",
        PackageKind::Tool,
        vec![
            declaration("s-compile", "compile:source"),
            declaration("s-loud", "phase:test"),
        ],
        ExtensionsControl {
            uses: vec![ExtensionUse {
                reference: ExtensionKey::authored("org.ghost/missing#nope"),
                config: None,
            }],
            disable: vec![package_key("org.sib", "s-tools", "s-loud")],
        },
    );
    let demo_host = host(
        Vec::new(),
        ExtensionsControl {
            uses: Vec::new(),
            disable: vec![package_key("org.sib", "s-tools", "s-loud")],
        },
    );

    // The selected host world: only the host's disable applied; the
    // package's activation and the sibling's controls stayed inert.
    let host_world =
        collect_extensions(world(vec![tools.clone(), sibling.clone()], demo_host, None)).unwrap();
    assert!(row(&host_world, "#s-loud").is_disabled());
    assert!(!row(&host_world, "#s-compile").is_activated());
    assert!(!row(&host_world, "#quiet").is_activated());
    assert!(!row(&host_world, "#quiet").is_enabled());

    // The tools package's own lane: its activation applied, the host's
    // disable and the sibling's disable did not leak in.
    let tools_lane =
        collect_extensions(world(vec![sibling.clone()], lane_owner_host(&tools), None)).unwrap();
    assert!(row(&tools_lane, "#s-compile").is_activated());
    assert!(!row(&tools_lane, "#s-loud").is_disabled());

    // The sibling's own lane: its retained controls are finally live, so
    // their unresolved reference is the hard error it always was.
    let sibling_lane =
        collect_extensions(world(vec![tools], lane_owner_host(&sibling), None)).unwrap_err();
    assert_eq!(
        sibling_lane,
        CollectionError::UnresolvedUse {
            key: ExtensionKey::authored("org.ghost/missing#nope"),
        }
    );
}

/// A package projected into its own host seat still cannot activate its own
/// declarations: `[[extensions.use]]` targets dependencies only, and the
/// projected seat keeps that boundary loud.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#HOST-ACTIVATION")]
fn projected_package_cannot_activate_its_own_host_seat_declaration() {
    let owner = dependency_with_controls(
        "org.demo",
        "tools",
        PackageKind::Tool,
        vec![declaration("own", "phase:test")],
        ExtensionsControl {
            uses: vec![ExtensionUse {
                reference: package_key("org.demo", "tools", "own"),
                config: None,
            }],
            disable: Vec::new(),
        },
    );
    let error = collect_extensions(world(Vec::new(), lane_owner_host(&owner), None)).unwrap_err();
    assert_eq!(
        error,
        CollectionError::UseTargetsHost {
            key: package_key("org.demo", "tools", "own"),
        }
    );
}
