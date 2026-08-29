use specmark::verifies;
use vibe_core::manifest::{ExtensionAppliesTo, ExtensionKey, ExtensionUse, ExtensionsControl};

use crate::{RegistryState, SelectorSubject, collect_extensions};

use super::support::{declaration, dependency, host, package_key, selected_declaration, world};

#[test]
fn exhaustive_view_is_lossless_effective_order_with_closed_state_precedence() {
    let mut inactive = declaration("inactive", "compile:source");
    inactive.auto = Some(false);
    inactive.applies_to = Some(ExtensionAppliesTo {
        packages: Some(vec!["org.allowed/*".into()]),
        paths: None,
    });
    let mut mismatch = declaration("mismatch", "compile:source");
    mismatch.applies_to = Some(ExtensionAppliesTo {
        packages: Some(vec!["org.allowed/*".into()]),
        paths: None,
    });
    let mut disabled = declaration("disabled", "compile:source");
    disabled.auto = Some(false);
    disabled.applies_to = Some(ExtensionAppliesTo {
        packages: Some(vec!["org.allowed/*".into()]),
        paths: None,
    });
    let disabled_key = ExtensionKey::authored("org.zed/tools#disabled");
    let registry = collect_extensions(world(
        vec![
            dependency("org.zed", "tools", vec![disabled, inactive, mismatch]),
            dependency(
                "org.aaa",
                "other",
                vec![declaration("effective", "phase:test")],
            ),
        ],
        host(
            vec![declaration("host", "phase:test")],
            ExtensionsControl {
                uses: vec![ExtensionUse {
                    reference: ExtensionKey::authored("org.zed/tools#mismatch"),
                    config: None,
                }],
                disable: vec![disabled_key],
            },
        ),
        None,
    ))
    .unwrap();

    let rows = registry.exhaustive(SelectorSubject::unscoped());
    assert_eq!(rows.len(), registry.rows().len());
    assert_eq!(
        rows.iter()
            .map(|view| view.row.key().as_str())
            .collect::<Vec<_>>(),
        [
            "org.zed/tools#disabled",
            "org.zed/tools#inactive",
            "org.aaa/other#effective",
            "__host__/demo#host",
            "org.zed/tools#mismatch",
        ]
    );
    assert_eq!(
        rows.iter().map(|view| view.state()).collect::<Vec<_>>(),
        [
            RegistryState::Disabled,
            RegistryState::Inactive,
            RegistryState::Effective,
            RegistryState::Effective,
            RegistryState::SelectorMismatch,
        ]
    );
    assert_eq!(rows.iter().filter(|view| view.is_effective()).count(), 2);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR")]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn enabled_at_keeps_selector_bearing_rows_that_an_unscoped_plan_drops() {
    let registry = collect_extensions(world(
        vec![dependency(
            "org.zed",
            "tools",
            vec![
                selected_declaration("inactive", Some(vec!["org.allowed/*"]), None),
                selected_declaration("gated", Some(vec!["org.allowed/*"]), None),
                declaration("off", "phase:test"),
            ],
        )],
        host(
            vec![selected_declaration("plain", None, None)],
            ExtensionsControl {
                uses: vec![ExtensionUse {
                    reference: package_key("org.zed", "tools", "gated"),
                    config: None,
                }],
                disable: vec![package_key("org.zed", "tools", "off")],
            },
        ),
        None,
    ))
    .unwrap();

    let compile = "compile:source".parse().unwrap();
    // The unscoped execution plan drops the selector-bearing activated row
    // before any document subject exists…
    assert_eq!(
        registry
            .plan(compile, SelectorSubject::unscoped())
            .iter()
            .map(|row| row.key().as_str())
            .collect::<Vec<_>>(),
        ["__host__/demo#plain"]
    );
    // …while the enabled view retains it, in exact effective order.
    assert_eq!(
        registry
            .enabled_at(compile)
            .iter()
            .map(|row| row.key().as_str())
            .collect::<Vec<_>>(),
        ["__host__/demo#plain", "org.zed/tools#gated"]
    );
    // Disabled rows are excluded at their point exactly as plans exclude
    // them; the disabled `phase:test` row never leaks into the view.
    assert!(
        registry
            .enabled_at("phase:test".parse().unwrap())
            .is_empty()
    );
    // The exhaustive all-view still retains every declaration once.
    assert_eq!(
        registry.exhaustive(SelectorSubject::unscoped()).len(),
        registry.rows().len()
    );
}
