use specmark::verifies;
use vibe_core::manifest::{ExtensionKey, ExtensionUse, ExtensionsControl};

use crate::registry::{
    ContributionTier, DependencyExtensionSource, ExtensionProvider, SelectorSubject,
    collect_extensions,
};

use super::support::{
    declaration, dependency, dependency_with_kind, host, package_key, provider_id, world,
};

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ORDER-LAW")]
fn reverse_lexical_lock_order_and_manifest_declaration_order_are_preserved() {
    let registry = collect_extensions(world(
        vec![
            dependency(
                "org.zed",
                "z-tool",
                vec![
                    declaration("zeta", "phase:build"),
                    declaration("alpha", "phase:build"),
                ],
            ),
            dependency(
                "org.aaa",
                "a-tool",
                vec![declaration("middle", "phase:build")],
            ),
        ],
        host(Vec::new(), ExtensionsControl::default()),
        None,
    ))
    .unwrap();

    let plan = registry.plan("phase:build".parse().unwrap(), SelectorSubject::unscoped());
    let keys = plan
        .iter()
        .map(|row| row.key().as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        keys,
        [
            "org.zed/z-tool#zeta",
            "org.zed/z-tool#alpha",
            "org.aaa/a-tool#middle",
        ]
    );
    assert_eq!(plan[0].provider_ordinal(), Some(0));
    assert_eq!(plan[0].declaration_ordinal(), 0);
    assert_eq!(plan[1].declaration_ordinal(), 1);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ORDER-LAW")]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#AUTO-BY-FAMILY")]
fn selected_stack_phase_is_preset_while_nonselected_stack_stays_active_dependency() {
    let stack = provider_id("org.stack", "rust-stack");
    let registry = collect_extensions(world(
        vec![
            dependency_with_kind(
                "org.other",
                "other-stack",
                vibe_core::PackageKind::Stack,
                vec![
                    declaration("other-phase", "phase:build"),
                    declaration("other-slot", "slot:pre-install"),
                ],
            ),
            dependency_with_kind(
                "org.stack",
                "rust-stack",
                vibe_core::PackageKind::Stack,
                vec![
                    declaration("stack-phase", "phase:build"),
                    declaration("stack-slot", "slot:pre-install"),
                    declaration("stack-compile", "compile:source"),
                ],
            ),
        ],
        host(Vec::new(), ExtensionsControl::default()),
        Some(stack),
    ))
    .unwrap();

    let phase = registry.plan("phase:build".parse().unwrap(), SelectorSubject::unscoped());
    assert_eq!(phase[0].key().as_str(), "org.stack/rust-stack#stack-phase");
    assert_eq!(phase[0].natural_tier(), ContributionTier::Preset);
    assert!(phase[0].active_by_default());
    assert_eq!(phase[1].key().as_str(), "org.other/other-stack#other-phase");
    assert_eq!(phase[1].natural_tier(), ContributionTier::Dependency);
    assert!(phase[1].active_by_default());
    let ExtensionProvider::Dependency(non_selected) = phase[1].provider() else {
        panic!("non-selected stack remains an installed dependency provider")
    };
    assert_eq!(non_selected.kind, vibe_core::PackageKind::Stack);

    let slot = registry.plan(
        "slot:pre-install".parse().unwrap(),
        SelectorSubject::unscoped(),
    );
    assert_eq!(
        slot.iter()
            .map(|row| row.key().as_str())
            .collect::<Vec<_>>(),
        [
            "org.other/other-stack#other-slot",
            "org.stack/rust-stack#stack-slot",
        ]
    );
    let compile = registry
        .rows()
        .iter()
        .find(|row| row.key().as_str().ends_with("#stack-compile"))
        .unwrap();
    assert_eq!(compile.natural_tier(), ContributionTier::Dependency);
    assert!(!compile.active_by_default());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#HOST-ACTIVATION")]
fn host_declaration_precedes_activation_and_activation_never_duplicates() {
    let activated_key = package_key("org.stack", "rust-stack", "auto-phase");
    let controls = ExtensionsControl {
        uses: vec![ExtensionUse {
            reference: activated_key,
            config: None,
        }],
        disable: Vec::new(),
    };
    let registry = collect_extensions(world(
        vec![
            dependency(
                "org.other",
                "other-tool",
                vec![declaration("dependency", "phase:build")],
            ),
            dependency(
                "org.stack",
                "rust-stack",
                vec![declaration("auto-phase", "phase:build")],
            ),
        ],
        host(vec![declaration("host", "phase:build")], controls),
        Some(provider_id("org.stack", "rust-stack")),
    ))
    .unwrap();

    let plan = registry.plan("phase:build".parse().unwrap(), SelectorSubject::unscoped());
    assert_eq!(
        plan.iter()
            .map(|row| row.key().as_str())
            .collect::<Vec<_>>(),
        [
            "org.other/other-tool#dependency",
            "__host__/demo#host",
            "org.stack/rust-stack#auto-phase",
        ]
    );
    let activated = plan[2];
    assert_eq!(activated.natural_tier(), ContributionTier::Preset);
    assert_eq!(activated.effective_tier(), ContributionTier::HostActivation);
    assert!(activated.active_by_default());
    assert!(activated.is_activated());
    assert_eq!(activated.activation_ordinal(), Some(0));
    assert_eq!(
        plan.iter()
            .filter(|row| row.key().as_str().ends_with("#auto-phase"))
            .count(),
        1
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-REGISTRY")]
fn provider_metadata_is_retained_separately_from_the_opaque_key() {
    let registry = collect_extensions(world(
        vec![dependency(
            "org.demo",
            "tools",
            vec![declaration("announce", "phase:test")],
        )],
        host(Vec::new(), ExtensionsControl::default()),
        None,
    ))
    .unwrap();
    let row = &registry.rows()[0];
    assert_eq!(
        row.key(),
        &ExtensionKey::authored("org.demo/tools#announce")
    );
    let ExtensionProvider::Dependency(provider) = row.provider() else {
        panic!("installed row must retain dependency metadata");
    };
    assert_eq!(provider.id, provider_id("org.demo", "tools"));
    assert_eq!(provider.root.to_string_lossy(), "vibedeps/tools");
    assert_eq!(provider.version, "1.2.3");
    assert_eq!(provider.content_hash.as_str(), "sha256:aa");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#HOST-ACTIVATION")]
fn installed_input_shape_has_no_consumer_controls() {
    let source = dependency("org.demo", "tools", Vec::new());
    let DependencyExtensionSource {
        provider,
        declarations,
    } = source;
    assert_eq!(provider.id, provider_id("org.demo", "tools"));
    assert!(declarations.is_empty());
}
