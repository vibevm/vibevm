use super::{FeaturesTable, effects::*};
use crate::manifest::Requires;
use std::collections::{BTreeMap, BTreeSet};

fn id(s: &str) -> DependencyDeclarationId {
    DependencyDeclarationId::new(s).unwrap()
}
fn local(s: &str) -> LocalFeatureRef {
    LocalFeatureRef::new(s).unwrap()
}
fn declarations() -> FeatureDeclarations {
    let requires: Requires = toml::from_str("[packages]\n\"org.test/backend\" = { version = '^1', link = 'dynamic', access = 'private', friend = false, exclude = ['secret'] }").unwrap();
    let mut d =
        FeatureDeclarations::from_requires(&requires, &BTreeSet::from([id("org.test/backend")]))
            .unwrap();
    d.aliases.insert(local("backend"), id("org.test/backend"));
    d.exported_subskills.insert("stack/rust".into());
    d.config_values
        .insert(local("verbose"), ConfigValue::Bool(true));
    d
}

#[test]
fn stable_ids_ignore_order_version_and_aliases() {
    let mut a = declarations();
    let r: Requires =
        toml::from_str("[packages]\n\"org.test/z\" = '*'\n\"org.test/backend\" = '^2'").unwrap();
    let b = FeatureDeclarations::from_requires(&r, &BTreeSet::new()).unwrap();
    a.aliases.insert(local("another"), id("org.test/backend"));
    assert!(a.dependencies.contains_key(&id("org.test/backend")));
    assert!(b.dependencies.contains_key(&id("org.test/backend")));
    assert_eq!(a.dependencies.len(), 1);
}

#[test]
fn all_source_kinds_preserve_explicit_metadata_once() {
    let r: Requires = toml::from_str(r#"[packages]
"org.test/a" = { version = '^1', link = 'static', access = 'private', friend = false, exclude = ['a'] }
"org.test/b" = { git = 'https://example.test/repo', tag = 'v1', version = '^1', link = 'static', access = 'private', friend = false, exclude = ['a'] }
"org.test/c" = { path = '../c', version = '^1', link = 'static', access = 'private', friend = false, exclude = ['a'] }
"org.test/d" = { version.var = 'shared', link = 'static', access = 'private', friend = false, exclude = ['a'] }
"#).unwrap();
    let d = FeatureDeclarations::from_requires(&r, &BTreeSet::new()).unwrap();
    assert_eq!(d.dependencies.len(), 4);
    for dep in d.dependencies.values() {
        assert_eq!(dep.link, Some(crate::manifest::LinkType::Static));
        assert_eq!(dep.access, Some(crate::manifest::AccessLevel::Private));
        assert_eq!(dep.friend, Some(false));
        assert_eq!(dep.exclude, Some(vec!["a".into()]));
        assert!(!dep.optional);
    }
    assert_eq!(
        d.dependencies[&id("org.test/a")].source,
        DependencySource::Registry(r.packages[0].clone())
    );
    assert_eq!(
        d.dependencies[&id("org.test/b")].source,
        DependencySource::Git(r.git_packages[0].clone())
    );
    assert_eq!(
        d.dependencies[&id("org.test/c")].source,
        DependencySource::Path(r.path_packages[0].clone())
    );
    assert_eq!(
        d.dependencies[&id("org.test/d")].source,
        DependencySource::Workspace(r.var_packages[0].clone())
    );
}

#[test]
fn legacy_and_typed_effect_sets_are_identical() {
    let d = declarations();
    let t: FeaturesTable = toml::from_str("default = ['helper', 'dep:backend', 'backend/fast', 'org.test/backend?/trace', 'subskill:stack/rust']\nhelper = []").unwrap();
    let p = FeatureProgram::lower_legacy(&t, &d).unwrap();
    assert_eq!(
        p.features[&local("default")],
        BTreeSet::from([
            FeatureEffect::Local(local("helper")),
            FeatureEffect::ActivateDependency(id("org.test/backend")),
            FeatureEffect::TargetFeature {
                dependency: id("org.test/backend"),
                feature: local("fast"),
                strength: FeatureStrength::Strong
            },
            FeatureEffect::TargetFeature {
                dependency: id("org.test/backend"),
                feature: local("trace"),
                strength: FeatureStrength::Weak
            },
            FeatureEffect::ExportedSubskill("stack/rust".into()),
        ])
    );
}

#[test]
fn typed_config_and_exports_require_declarations() {
    let mut d = declarations();
    let p = FeatureProgram {
        features: BTreeMap::from([(
            local("default"),
            BTreeSet::from([
                FeatureEffect::ConfigValue(local("verbose")),
                FeatureEffect::ExportedSubskill("stack/rust".into()),
            ]),
        )]),
        ..Default::default()
    };
    p.validate(&d).unwrap();
    d.config_values.clear();
    assert!(matches!(p.validate(&d), Err(EffectError::UnknownConfig(_))));
    d.config_values
        .insert(local("verbose"), ConfigValue::Integer(2));
    d.exported_subskills.clear();
    assert!(matches!(
        p.validate(&d),
        Err(EffectError::UnknownSubskill(_))
    ));
}

#[test]
fn undeclared_targets_and_private_dormant_refs_refuse() {
    for raw in [
        "dep:missing",
        "missing?/trace",
        "missing/fast",
        "absent",
        "subskill:missing",
    ] {
        let t = FeaturesTable {
            features: BTreeMap::from([("_dormant".into(), vec![raw.into()])]),
            ..Default::default()
        };
        assert!(
            FeatureProgram::lower_legacy(&t, &declarations()).is_err(),
            "{raw}"
        );
    }
}

#[test]
fn malformed_qualified_refs_and_paths_refuse() {
    for raw in [
        "",
        "dep:",
        "dep:org.test//x",
        "org.test/x/y/z",
        "org.test/x?/?f",
        "x??/f",
        "dep:x@1",
        "x/",
        "/x",
        "subskill:../x",
        "subskill:/root",
        "subskill:a//b",
        "a b",
        "x/f?",
        "dep:org.test/x?",
    ] {
        assert!(FeatureEffect::parse_legacy(raw).is_err(), "{raw}");
    }
    assert!(matches!(
        FeatureEffect::parse_legacy("org.test/backend?/trace").unwrap(),
        FeatureEffect::TargetFeature {
            strength: FeatureStrength::Weak,
            ..
        }
    ));
}

#[test]
fn aliases_are_local_and_cannot_shadow_ids() {
    let a = declarations();
    let t: FeaturesTable = toml::from_str("default = ['dep:backend']").unwrap();
    FeatureProgram::lower_legacy(&t, &a).unwrap();
    let mut b = a.clone();
    b.aliases.clear();
    assert!(matches!(
        FeatureProgram::lower_legacy(&t, &b),
        Err(EffectError::UnknownDependency(_))
    ));
    b.dependencies.insert(
        id("backend"),
        a.dependencies.values().next().unwrap().clone(),
    );
    b.aliases.insert(local("backend"), id("org.test/backend"));
    assert!(matches!(b.validate(), Err(EffectError::Duplicate(_))));
    b.aliases = BTreeMap::from([(local("other"), id("missing"))]);
    assert!(matches!(
        b.validate(),
        Err(EffectError::UnknownDependency(_))
    ));
}

#[test]
fn duplicate_declarations_and_dangling_metadata_refuse() {
    let mut r: Requires = toml::from_str("[packages]\n\"org.test/a\" = '*'").unwrap();
    r.packages.push(r.packages[0].clone());
    assert!(matches!(
        FeatureDeclarations::from_requires(&r, &BTreeSet::new()),
        Err(EffectError::Duplicate(_))
    ));
    r.packages.pop();
    r.friend_flags.insert("org.test/missing".into(), false);
    assert!(matches!(
        FeatureDeclarations::from_requires(&r, &BTreeSet::new()),
        Err(EffectError::DanglingMetadata(_))
    ));
    r.friend_flags.clear();
    assert!(matches!(
        FeatureDeclarations::from_requires(&r, &BTreeSet::from([id("missing")])),
        Err(EffectError::UnknownDependency(_))
    ));
}

#[test]
fn new_wire_grammar_remains_unpublished() {
    assert!(toml::from_str::<FeaturesTable>("default = [{ activate = 'backend' }]").is_err());
    assert!(
        toml::from_str::<Requires>(
            "[packages]\n\"org.test/a\" = { version = '*', optional = true }"
        )
        .is_err()
    );
}

#[test]
fn unavailable_export_catalogue_is_not_known_empty() {
    let d = FeatureDeclarations::default();
    let table: FeaturesTable =
        toml::from_str("default = ['a']\na = ['subskill:stack/rust']").unwrap();
    let partial = FeatureProgram::lower_legacy_dependencies(&table, &d).unwrap();
    assert_eq!(partial, FeatureProgram::lower_table(&table).unwrap());
    assert!(matches!(
        partial.validate(&d),
        Err(EffectError::UnknownSubskill(_))
    ));
    assert!(matches!(
        FeatureProgram::lower_legacy(&table, &d),
        Err(EffectError::UnknownSubskill(_))
    ));
    for body in [
        "_private = ['missing?/fast']",
        "default = ['dep:missing']",
        "_private = ['absent']",
        "default = ['subskill:../bad']",
    ] {
        let table: FeaturesTable = toml::from_str(body).unwrap();
        assert!(FeatureProgram::lower_legacy_dependencies(&table, &d).is_err());
    }
    assert!(d.exported_subskills.is_empty());
}
