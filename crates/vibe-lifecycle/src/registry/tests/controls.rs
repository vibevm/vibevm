use specmark::verifies;
use vibe_core::manifest::{ExtensionKey, ExtensionUse, ExtensionsControl};

use crate::registry::{
    CollectionError, CollectionNotice, ContributionTier, HostIdentity, SelectorSubject,
    collect_extensions,
};

use super::support::{config, declaration, dependency, host, package_key, provider_id, world};

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#HOST-ACTIVATION")]
fn activation_inherits_or_replaces_config_as_one_whole_value() {
    let mut inherit = declaration("inherit", "phase:test");
    inherit.config = Some(config(&[("original", "yes")]));
    let mut replace = declaration("replace", "phase:test");
    replace.config = Some(config(&[("original", "yes")]));
    let mut empty = declaration("empty", "phase:test");
    empty.config = Some(config(&[("original", "yes")]));
    let controls = ExtensionsControl {
        uses: vec![
            ExtensionUse {
                reference: package_key("org.demo", "tools", "inherit"),
                config: None,
            },
            ExtensionUse {
                reference: package_key("org.demo", "tools", "replace"),
                config: Some(config(&[("replacement", "yes")])),
            },
            ExtensionUse {
                reference: package_key("org.demo", "tools", "empty"),
                config: Some(config(&[])),
            },
        ],
        disable: Vec::new(),
    };
    let registry = collect_extensions(world(
        vec![dependency(
            "org.demo",
            "tools",
            vec![inherit, replace, empty],
        )],
        host(Vec::new(), controls),
        None,
    ))
    .unwrap();

    let inherit = row(&registry, "#inherit");
    assert_eq!(inherit.authored_config(), inherit.effective_config());
    let replace = row(&registry, "#replace");
    assert_eq!(
        replace.authored_config().unwrap().as_table()["original"].as_str(),
        Some("yes")
    );
    assert!(
        !replace
            .effective_config()
            .unwrap()
            .as_table()
            .contains_key("original")
    );
    assert_eq!(
        replace.effective_config().unwrap().as_table()["replacement"].as_str(),
        Some("yes")
    );
    let empty = row(&registry, "#empty");
    assert!(empty.effective_config().is_some());
    assert!(empty.effective_config().unwrap().is_empty());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#HOST-ACTIVATION")]
fn disable_is_idempotent_retained_and_wins_after_activation() {
    let key = package_key("org.demo", "tools", "announce");
    let controls = ExtensionsControl {
        uses: vec![ExtensionUse {
            reference: key.clone(),
            config: None,
        }],
        disable: vec![key.clone(), key],
    };
    let registry = collect_extensions(world(
        vec![dependency(
            "org.demo",
            "tools",
            vec![declaration("announce", "phase:test")],
        )],
        host(Vec::new(), controls),
        None,
    ))
    .unwrap();

    assert_eq!(registry.rows().len(), 1);
    let all = registry.all(SelectorSubject::unscoped());
    assert_eq!(all.len(), 1);
    assert!(all[0].selector_matches);
    assert!(!all[0].is_effective());
    assert!(all[0].row.is_activated());
    assert!(all[0].row.is_disabled());
    assert_eq!(
        all[0].row.effective_tier(),
        ContributionTier::HostActivation
    );
    assert!(
        registry
            .plan("phase:test".parse().unwrap(), SelectorSubject::unscoped())
            .is_empty()
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#AUTO-BY-FAMILY")]
fn installed_compile_auto_is_not_active_and_emits_a_notice() {
    let mut compile = declaration("rewrite", "compile:source");
    compile.auto = Some(true);
    let registry = collect_extensions(world(
        vec![dependency("org.demo", "tools", vec![compile.clone()])],
        host(Vec::new(), ExtensionsControl::default()),
        None,
    ))
    .unwrap();
    assert!(!registry.rows()[0].is_enabled());
    assert_eq!(
        registry.notices(),
        [CollectionNotice::InstalledCompileAutoIgnored {
            key: package_key("org.demo", "tools", "rewrite")
        }]
    );
    assert!(
        registry
            .plan(
                "compile:source".parse().unwrap(),
                SelectorSubject::unscoped()
            )
            .is_empty()
    );

    let activated = collect_extensions(world(
        vec![dependency("org.demo", "tools", vec![compile])],
        host(
            Vec::new(),
            ExtensionsControl {
                uses: vec![ExtensionUse {
                    reference: package_key("org.demo", "tools", "rewrite"),
                    config: None,
                }],
                disable: Vec::new(),
            },
        ),
        None,
    ))
    .unwrap();
    assert_eq!(
        activated
            .plan(
                "compile:source".parse().unwrap(),
                SelectorSubject::unscoped()
            )
            .len(),
        1
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#HOST-ACTIVATION")]
fn duplicate_and_unresolved_uses_are_hard_errors() {
    let key = package_key("org.demo", "tools", "announce");
    let duplicate = collect_extensions(world(
        vec![dependency(
            "org.demo",
            "tools",
            vec![declaration("announce", "phase:test")],
        )],
        host(
            Vec::new(),
            ExtensionsControl {
                uses: vec![
                    ExtensionUse {
                        reference: key.clone(),
                        config: None,
                    },
                    ExtensionUse {
                        reference: key.clone(),
                        config: None,
                    },
                ],
                disable: Vec::new(),
            },
        ),
        None,
    ))
    .unwrap_err();
    assert_eq!(
        duplicate,
        CollectionError::DuplicateUse {
            key: key.clone(),
            first: 0,
            duplicate: 1,
        }
    );

    let missing = ExtensionKey::authored("org.missing/nope#unknown");
    let unresolved = collect_extensions(world(
        Vec::new(),
        host(
            Vec::new(),
            ExtensionsControl {
                uses: vec![ExtensionUse {
                    reference: missing.clone(),
                    config: None,
                }],
                disable: Vec::new(),
            },
        ),
        None,
    ))
    .unwrap_err();
    assert_eq!(unresolved, CollectionError::UnresolvedUse { key: missing });
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#HOST-ACTIVATION")]
fn use_must_target_dependency_and_unknown_disable_is_hard() {
    let host_key = ExtensionKey::for_host("demo", "local");
    let host_use = collect_extensions(world(
        Vec::new(),
        host(
            vec![declaration("local", "phase:test")],
            ExtensionsControl {
                uses: vec![ExtensionUse {
                    reference: host_key.clone(),
                    config: None,
                }],
                disable: Vec::new(),
            },
        ),
        None,
    ))
    .unwrap_err();
    assert_eq!(host_use, CollectionError::UseTargetsHost { key: host_key });

    let missing = ExtensionKey::authored("org.missing/nope#unknown");
    let unknown = collect_extensions(world(
        Vec::new(),
        host(
            Vec::new(),
            ExtensionsControl {
                uses: Vec::new(),
                disable: vec![missing.clone(), missing.clone()],
            },
        ),
        None,
    ))
    .unwrap_err();
    assert_eq!(unknown, CollectionError::UnknownDisable { key: missing });
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#HOST-ACTIVATION")]
fn distinct_structural_sites_with_same_delimiter_bearing_key_are_rejected() {
    let coordinate = provider_id("org.demo", "tools");
    let installed = dependency(
        "org.demo",
        "tools",
        vec![declaration("strange#id", "phase:test")],
    );
    let mut host_source = host(
        vec![declaration("strange#id", "phase:build")],
        ExtensionsControl::default(),
    );
    host_source.provider.identity = HostIdentity::coordinate(coordinate);
    let error = collect_extensions(world(vec![installed], host_source, None)).unwrap_err();
    let CollectionError::DuplicateDeclarationKey { key, first, second } = error else {
        panic!("expected rendered-key collision");
    };
    assert_eq!(key.as_str(), "org.demo/tools#strange#id");
    assert!(first.contains("dependency org.demo/tools"));
    assert!(second.contains("host org.demo/tools"));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#HOST-ACTIVATION")]
fn grouped_project_and_package_hosts_use_their_typed_coordinate() {
    let coordinate = provider_id("org.demo", "app");
    let mut grouped = host(
        vec![declaration("local", "phase:test")],
        ExtensionsControl::default(),
    );
    grouped.provider.identity = HostIdentity::coordinate(coordinate.clone());
    let registry = collect_extensions(world(Vec::new(), grouped, None)).unwrap();
    assert_eq!(registry.rows()[0].key().as_str(), "org.demo/app#local");
    assert_eq!(
        registry.rows()[0].provider().to_string(),
        coordinate.to_string()
    );

    let ungrouped = collect_extensions(world(
        Vec::new(),
        host(
            vec![declaration("local", "phase:test")],
            ExtensionsControl::default(),
        ),
        None,
    ))
    .unwrap();
    assert_eq!(ungrouped.rows()[0].key().as_str(), "__host__/demo#local");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-GRAMMAR")]
fn virtual_workspace_can_control_dependencies_but_cannot_declare() {
    let key = package_key("org.demo", "tools", "announce");
    let mut virtual_host = host(
        Vec::new(),
        ExtensionsControl {
            uses: vec![ExtensionUse {
                reference: key,
                config: None,
            }],
            disable: Vec::new(),
        },
    );
    virtual_host.provider.identity = HostIdentity::virtual_workspace();
    let registry = collect_extensions(world(
        vec![dependency(
            "org.demo",
            "tools",
            vec![declaration("announce", "phase:test")],
        )],
        virtual_host,
        None,
    ))
    .unwrap();
    assert_eq!(
        registry
            .plan("phase:test".parse().unwrap(), SelectorSubject::unscoped())
            .len(),
        1
    );

    let mut invalid = host(
        vec![declaration("local", "phase:test")],
        ExtensionsControl::default(),
    );
    invalid.provider.identity = HostIdentity::virtual_workspace();
    assert_eq!(
        collect_extensions(world(Vec::new(), invalid, None)).unwrap_err(),
        CollectionError::VirtualHostDeclaration { id: "local".into() }
    );
}

fn row<'registry>(
    registry: &'registry crate::registry::ExtensionRegistry,
    suffix: &str,
) -> &'registry crate::registry::ExtensionRegistryRow {
    registry
        .rows()
        .iter()
        .find(|row| row.key().as_str().ends_with(suffix))
        .unwrap_or_else(|| panic!("test row with suffix `{suffix}` exists"))
}

/// Reserved engine rows cannot be disabled (or activated) by host controls:
/// the package-skill recovery/reconcile contributions must always run so a
/// stale target never outlives its evidence.
#[test]
fn reserved_engine_rows_refuse_host_disable_and_activation() {
    use crate::registry::{
        ExtensionProvider, SyntheticPresetSource, collect_extensions_with_presets,
    };

    let preset = SyntheticPresetSource {
        key: ExtensionKey::authored("@vibe/package/skill/reconcile"),
        provider: ExtensionProvider::Host(
            super::support::host(
                vec![],
                ExtensionsControl {
                    uses: vec![],
                    disable: vec![],
                },
            )
            .provider,
        ),
        declaration: super::support::declaration("package-skill-reconcile", "phase:package"),
    };
    let host = super::support::host(
        vec![],
        ExtensionsControl {
            uses: vec![],
            disable: vec![ExtensionKey::authored("@vibe/package/skill/reconcile")],
        },
    );
    let error =
        collect_extensions_with_presets(super::support::world(vec![], host, None), vec![preset])
            .unwrap_err();
    assert!(
        error.to_string().contains("reserved engine contribution"),
        "{error}"
    );

    let host = super::support::host(
        vec![],
        ExtensionsControl {
            uses: vec![vibe_core::manifest::ExtensionUse {
                reference: ExtensionKey::authored("@vibe/package/skill/recover"),
                config: None,
            }],
            disable: vec![],
        },
    );
    let error = collect_extensions_with_presets(super::support::world(vec![], host, None), vec![])
        .unwrap_err();
    assert!(
        error.to_string().contains("reserved engine contribution"),
        "{error}"
    );
}
