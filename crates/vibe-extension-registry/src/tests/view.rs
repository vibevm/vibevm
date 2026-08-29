use vibe_core::manifest::{ExtensionAppliesTo, ExtensionKey, ExtensionUse, ExtensionsControl};

use crate::{RegistryState, SelectorSubject, collect_extensions};

use super::support::{declaration, dependency, host, world};

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
