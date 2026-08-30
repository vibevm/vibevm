//! §3.1's four-step resolution law, step by step, plus the replacement
//! fixture the plan node accepts this atom on.
//!
//! What "the builtin did not run" can honestly mean here is NOT SELECTED:
//! execution does not exist at this atom — no provider protocol, no invocation,
//! nothing to run — so the proof these REDs carry is that routing returns the
//! plugin row, that the builtin row is still collected and still queryable, and
//! that removing the route hands the key straight back to it.

use specmark::verifies;
use vibe_core::manifest::{ExtensionKey, ExtensionsControl, MechanismRoutes};

use crate::{
    MechanismRegistry, MechanismResolutionError, SelectionStep, collect_mechanisms,
    resolve_mechanism,
};

use super::support::{host, mechanism, mechanism_key, provider_package, provider_pin, world};

/// A world where `org.example/build-tools` ships a second Cargo provider and
/// nothing selects it yet.
fn replacement_world(controls: ExtensionsControl) -> MechanismRegistry {
    collect_mechanisms(&world(
        vec![provider_package(
            "org.example",
            "build-tools",
            vec![mechanism("cargo-v2", "build:cargo")],
        )],
        host(Vec::new(), controls),
        None,
    ))
    .unwrap_or_else(|error| panic!("the replacement world collects: {error}"))
}

fn routes(pairs: &[(&str, &str)]) -> MechanismRoutes {
    let mut routes = MechanismRoutes::default();
    for (key, pin) in pairs {
        routes.insert(mechanism_key(key), provider_pin(pin));
    }
    routes
}

/// Step 3, and mutation 2's RED. With no pin and no route, the shipped builtin
/// answers — which is only possible because the collector ALWAYS appends the
/// engine's own source.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn the_shipped_default_answers_an_unrouted_key() {
    let registry = replacement_world(ExtensionsControl::default());
    let selection = resolve_mechanism(
        &registry,
        &mechanism_key("build:cargo"),
        None,
        &MechanismRoutes::default(),
    )
    .expect("the engine ships a default for `build:cargo`");

    assert_eq!(selection.via(), SelectionStep::BuiltinDefault);
    assert_eq!(selection.row().pin().to_string(), "org.vibevm/vibe#cargo");
    assert!(selection.row().is_builtin());
    assert!(
        selection.displaced_default().is_none(),
        "the default displaces nothing by being the default"
    );
}

/// THE replacement fixture. The host routes `build:cargo` to a plugin: the
/// plugin row is selected, the builtin row is still present and queryable, and
/// the selection names the default it displaced.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_host_route_selects_the_plugin_while_the_builtin_stays_collected() {
    let registry = replacement_world(ExtensionsControl::default());
    let key = mechanism_key("build:cargo");
    let selection = resolve_mechanism(
        &registry,
        &key,
        None,
        &routes(&[("build:cargo", "org.example/build-tools#cargo-v2")]),
    )
    .expect("the route names an installed provider of this exact capability");

    assert_eq!(selection.via(), SelectionStep::HostRoute);
    assert_eq!(
        selection.row().pin().to_string(),
        "org.example/build-tools#cargo-v2"
    );
    assert!(!selection.row().is_builtin());
    assert_eq!(
        selection
            .displaced_default()
            .map(|row| row.pin().to_string()),
        Some("org.vibevm/vibe#cargo".to_owned()),
    );

    // The builtin was not removed, hidden, or overwritten — only NOT SELECTED.
    let builtin = registry
        .builtin_default(&key)
        .expect("the shipped row survives being displaced");
    assert!(builtin.is_enabled());
    assert!(
        registry
            .rows()
            .iter()
            .any(|row| row.pin() == builtin.pin() && row.is_builtin()),
        "and it is still an ordinary, queryable row of the registry"
    );
}

/// The other half of the fixture, and mutation 2's second RED: take the route
/// away and the shipped default answers again, from the same collected world.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn removing_the_route_restores_the_builtin() {
    let registry = replacement_world(ExtensionsControl::default());
    let key = mechanism_key("build:cargo");

    let routed = resolve_mechanism(
        &registry,
        &key,
        None,
        &routes(&[("build:cargo", "org.example/build-tools#cargo-v2")]),
    )
    .expect("routed");
    let unrouted =
        resolve_mechanism(&registry, &key, None, &MechanismRoutes::default()).expect("unrouted");

    assert_ne!(routed.row().pin(), unrouted.row().pin());
    assert_eq!(unrouted.via(), SelectionStep::BuiltinDefault);
    assert_eq!(unrouted.row().pin().to_string(), "org.vibevm/vibe#cargo");
}

/// A selection cannot displace itself: an exact pin (or a route) naming the
/// BUILTIN answers with no displaced default, because nothing was displaced.
///
/// The registry display and the narration show the displaced default as the
/// evidence that a replacement replaced something (§3.1); a builtin carried
/// as its own displacement would be that evidence fabricated. The filter is
/// one clause and every other resolution test stays green without it — this
/// pin is what makes it a law rather than a nicety.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_builtin_selected_by_pin_or_route_displaces_nothing() {
    let registry = replacement_world(ExtensionsControl::default());
    let key = mechanism_key("build:cargo");
    let builtin = provider_pin("org.vibevm/vibe#cargo");

    let pinned = resolve_mechanism(&registry, &key, Some(&builtin), &MechanismRoutes::default())
        .expect("the builtin answers its own pin");
    assert_eq!(pinned.via(), SelectionStep::TargetPin);
    assert_eq!(pinned.row().pin(), &builtin);
    assert!(
        pinned.displaced_default().is_none(),
        "selecting the default displaces nothing"
    );

    let routed = resolve_mechanism(
        &registry,
        &key,
        None,
        &routes(&[("build:cargo", "org.vibevm/vibe#cargo")]),
    )
    .expect("the builtin answers its own route");
    assert!(routed.displaced_default().is_none());
}

/// Step 1 over step 2, and mutation 1's RED. A target pin is the recovery path
/// when a host override is wrong, so it must outrank the route it recovers
/// from.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn an_exact_pin_outranks_a_host_route() {
    let registry = collect_mechanisms(&world(
        vec![
            provider_package(
                "org.example",
                "build-tools",
                vec![mechanism("cargo-v2", "build:cargo")],
            ),
            provider_package(
                "org.other",
                "rescue",
                vec![mechanism("cargo-v3", "build:cargo")],
            ),
        ],
        host(Vec::new(), ExtensionsControl::default()),
        None,
    ))
    .expect("two plugin providers of one capability collect");

    let pin = provider_pin("org.other/rescue#cargo-v3");
    let selection = resolve_mechanism(
        &registry,
        &mechanism_key("build:cargo"),
        Some(&pin),
        &routes(&[("build:cargo", "org.example/build-tools#cargo-v2")]),
    )
    .expect("the pin names an installed provider of this capability");

    assert_eq!(selection.via(), SelectionStep::TargetPin);
    assert_eq!(selection.row().pin(), &pin);
}

/// Mutation 3's RED — the law's whole reason for existing. An installed
/// provider whose logical name IS `cargo`, with no pin and no route, is inert:
/// installing a dependency never lets it seize a logical key.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn an_installed_provider_that_merely_names_the_key_is_inert() {
    let registry = replacement_world(ExtensionsControl::default());
    let key = mechanism_key("build:cargo");

    let selection = resolve_mechanism(&registry, &key, None, &MechanismRoutes::default())
        .expect("the shipped default still answers");
    assert!(
        selection.row().is_builtin(),
        "an unpinned, unrouted foreign row is a candidate and nothing more"
    );

    // It IS installed, and it IS a candidate — membership is not selection.
    assert!(
        registry
            .candidates(&key)
            .any(|row| row.pin().to_string() == "org.example/build-tools#cargo-v2"),
    );
}

/// A pin or a route naming a provider this world does not install is its own
/// typed refusal, naming what was asked for and what exists.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn selecting_an_uninstalled_provider_names_what_was_asked_and_what_exists() {
    let registry = replacement_world(ExtensionsControl::default());
    let key = mechanism_key("build:cargo");
    let ghost = provider_pin("org.ghost/missing#cargo-v9");

    for (via, pin, table) in [
        (
            SelectionStep::TargetPin,
            Some(ghost.clone()),
            MechanismRoutes::default(),
        ),
        (
            SelectionStep::HostRoute,
            None,
            routes(&[("build:cargo", "org.ghost/missing#cargo-v9")]),
        ),
    ] {
        let error = resolve_mechanism(&registry, &key, pin.as_ref(), &table)
            .expect_err("an uninstalled identity selects nothing");
        let MechanismResolutionError::UninstalledProvider {
            pin: refused,
            via: refused_via,
            candidates,
            ..
        } = &error
        else {
            panic!("expected an uninstalled refusal, got: {error}");
        };
        assert_eq!(refused.as_ref(), &ghost);
        assert_eq!(*refused_via, via);
        assert_eq!(
            candidates,
            "org.vibevm/vibe#cargo, org.example/build-tools#cargo-v2",
        );
    }
}

/// A provider answers the capability it declares, or it answers nothing:
/// selection is not a rename.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn selecting_a_provider_of_another_capability_refuses() {
    let registry = replacement_world(ExtensionsControl::default());
    let pin = provider_pin("org.vibevm/vibe#vibe-bin");
    let error = resolve_mechanism(
        &registry,
        &mechanism_key("build:cargo"),
        Some(&pin),
        &MechanismRoutes::default(),
    )
    .expect_err("a deploy provider does not service a build key");

    let MechanismResolutionError::CapabilityMismatch { provides, .. } = &error else {
        panic!("expected a capability refusal, got: {error}");
    };
    assert_eq!(provides, &mechanism_key("deploy:vibe-bin"));
    assert!(error.to_string().contains("deploy:vibe-bin"), "{error}");
}

/// A disabled provider is never selected: admitting it would make the host's
/// disable list advisory.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#HOST-ACTIVATION")]
fn a_disabled_provider_is_never_selected() {
    let registry = replacement_world(ExtensionsControl {
        uses: Vec::new(),
        disable: vec![ExtensionKey::authored("org.example/build-tools#cargo-v2")],
    });
    let error = resolve_mechanism(
        &registry,
        &mechanism_key("build:cargo"),
        None,
        &routes(&[("build:cargo", "org.example/build-tools#cargo-v2")]),
    )
    .expect_err("a disabled row cannot be routed to");
    assert!(
        matches!(error, MechanismResolutionError::DisabledProvider { .. }),
        "{error}"
    );
}

/// Step 4: nothing pinned, nothing routed, no shipped default — the refusal
/// lists the installed candidates, bounded.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn an_unselected_key_with_no_default_refuses_and_lists_bounded_candidates() {
    let installed = (0..9)
        .map(|index| {
            provider_package(
                &format!("org.p{index}"),
                "zig-tools",
                vec![mechanism("zig", "build:zig")],
            )
        })
        .collect();
    let registry = collect_mechanisms(&world(
        installed,
        host(Vec::new(), ExtensionsControl::default()),
        None,
    ))
    .expect("nine providers of one uncovered key collect");

    let error = resolve_mechanism(
        &registry,
        &mechanism_key("build:zig"),
        None,
        &MechanismRoutes::default(),
    )
    .expect_err("the engine ships no `build:zig` default");

    let MechanismResolutionError::NoProvider { candidates, .. } = &error else {
        panic!("expected a no-provider refusal, got: {error}");
    };
    assert!(
        candidates.starts_with("org.p0/zig-tools#zig, "),
        "{candidates}"
    );
    assert!(candidates.ends_with(", and 1 more"), "{candidates}");
    assert_eq!(candidates.matches('#').count(), 8, "{candidates}");
    assert!(error.to_string().contains("no target pin"), "{error}");

    // A key nothing installs at all says so plainly.
    let empty = resolve_mechanism(
        &registry,
        &mechanism_key("deploy:nowhere"),
        None,
        &MechanismRoutes::default(),
    )
    .expect_err("nothing services it");
    assert!(empty.to_string().contains("none installed"), "{empty}");
}
