use super::*;
use vibe_core::manifest::Requires;
fn id(s: &str) -> DependencyDeclarationId {
    DependencyDeclarationId::new(s).unwrap()
}
fn local(s: &str) -> LocalFeatureRef {
    LocalFeatureRef::new(s).unwrap()
}
fn declarations() -> FeatureDeclarations {
    let r: Requires = toml::from_str("[packages]\n\"org.test/backend\" = '^1'").unwrap();
    let mut d =
        FeatureDeclarations::from_requires(&r, &BTreeSet::from([id("org.test/backend")])).unwrap();
    d.aliases.insert(local("backend"), id("org.test/backend"));
    d.exported_subskills.insert("stack/rust".into());
    d.config_values
        .insert(local("verbose"), ConfigValue::Bool(true));
    d
}
fn table(raw: &str) -> FeaturesTable {
    toml::from_str(raw).unwrap()
}
fn expand(raw: &str) -> FeatureExpansion {
    expand_declared_features(
        &table(raw),
        &declarations(),
        &FeatureRequest::default(),
        &BTreeSet::new(),
    )
    .unwrap()
}

#[test]
fn optional_activation_and_strong_forwarding_share_effect_set() {
    let d = declarations();
    let t = table("default = ['dep:backend', 'backend/fast', 'subskill:stack/rust']");
    let mut p = FeatureProgram::lower_legacy(&t, &d).unwrap();
    p.features
        .get_mut(&local("default"))
        .unwrap()
        .insert(FeatureEffect::ConfigValue(local("verbose")));
    let out = expand_feature_effects(&p, &d, &FeatureRequest::default(), &BTreeSet::new()).unwrap();
    assert_eq!(out.active_deps, BTreeSet::from(["org.test/backend".into()]));
    assert_eq!(
        out.dep_features["org.test/backend"],
        BTreeSet::from(["fast".into()])
    );
    assert!(out.active_subskills.contains("stack/rust"));
    assert_eq!(out.config_values["verbose"], ConfigValue::Bool(true));
    assert_eq!(d.dependencies.len(), 1);
}

#[test]
fn weak_only_stays_dormant_and_strong_alone_activates() {
    let weak = expand("default = ['backend?/trace']");
    assert!(weak.active_deps.is_empty());
    assert!(weak.dep_features.is_empty());
    assert!(weak.weak_dep_features["org.test/backend"].contains("trace"));
    let strong = expand("default = ['backend/fast']");
    assert!(strong.active_deps.contains("org.test/backend"));
    assert!(strong.dep_features["org.test/backend"].contains("fast"));
}

#[test]
fn weak_forwarding_is_order_independent_and_survives_merge() {
    let a = expand("default = ['a', 'z']\na = ['backend?/trace']\nz = ['dep:backend']");
    let b = expand("default = ['z', 'a']\nz = ['backend?/trace']\na = ['dep:backend']");
    assert_eq!(a.dep_features, b.dep_features);
    assert!(a.dep_features["org.test/backend"].contains("trace"));
    let weak = expand("default = ['backend?/trace']");
    let strong = expand("default = ['dep:backend']");
    let mut left = weak.clone();
    left.merge(&strong).unwrap();
    let mut right = strong;
    right.merge(&weak).unwrap();
    assert_eq!(left, right);
    assert_eq!(left.dep_features, a.dep_features);
}

#[test]
fn required_and_already_active_dependencies_enable_weak_requests() {
    let t = table("default = ['backend?/trace']");
    let mut d = declarations();
    let active = BTreeSet::from([id("org.test/backend")]);
    let supplied = expand_declared_features(&t, &d, &FeatureRequest::default(), &active).unwrap();
    d.dependencies
        .get_mut(&id("org.test/backend"))
        .unwrap()
        .optional = false;
    let required =
        expand_declared_features(&t, &d, &FeatureRequest::default(), &BTreeSet::new()).unwrap();
    assert_eq!(required, supplied);
    assert!(required.dep_features["org.test/backend"].contains("trace"));
}

#[test]
fn typed_and_legacy_expansion_are_identical() {
    let d = declarations();
    let t = table(
        "default = ['helper', 'backend?/trace']\nhelper = ['backend/fast', 'subskill:stack/rust']",
    );
    let p = FeatureProgram::lower_legacy(&t, &d).unwrap();
    let req = FeatureRequest::default();
    assert_eq!(
        expand_declared_features(&t, &d, &req, &BTreeSet::new()).unwrap(),
        expand_feature_effects(&p, &d, &req, &BTreeSet::new()).unwrap()
    );
    let direct = table("default = ['org.test/backend/fast', 'org.test/backend?/trace']");
    assert_eq!(
        expand_features(&direct, &req).unwrap(),
        expand_declared_features(&direct, &d, &req, &BTreeSet::new()).unwrap()
    );
}

#[test]
fn unknown_typed_legacy_and_already_active_targets_refuse() {
    let d = declarations();
    for raw in [
        "default = ['missing/fast']",
        "_private = ['dep:missing']",
        "default = ['subskill:missing']",
    ] {
        assert!(
            expand_declared_features(
                &table(raw),
                &d,
                &FeatureRequest::default(),
                &BTreeSet::new()
            )
            .is_err()
        );
    }
    let p = FeatureProgram {
        features: BTreeMap::from([(
            local("_private"),
            BTreeSet::from([FeatureEffect::ActivateDependency(id("missing"))]),
        )]),
        ..Default::default()
    };
    assert!(expand_feature_effects(&p, &d, &FeatureRequest::default(), &BTreeSet::new()).is_err());
    assert!(
        expand_feature_effects(
            &FeatureProgram::default(),
            &d,
            &FeatureRequest::default(),
            &BTreeSet::from([id("missing")])
        )
        .is_err()
    );
}

#[test]
fn qualified_weak_refs_and_malformed_refs_use_shared_parser() {
    assert_eq!(
        FeatureValue::parse("org.test/backend?/trace").unwrap(),
        FeatureValue::DepFeature {
            dep_name: "org.test/backend".into(),
            dep_feature: "trace".into(),
            weak: true
        }
    );
    for raw in [
        "org.test/backend/trace/extra",
        "org.test//trace",
        "backend??/trace",
        "dep:org.test/backend@1",
    ] {
        assert!(FeatureValue::parse(raw).is_err());
        let t = FeaturesTable {
            features: BTreeMap::from([("_private".into(), vec![raw.into()])]),
            ..Default::default()
        };
        assert!(expand_features(&t, &FeatureRequest::default()).is_err());
        assert!(!validate_features_table(&t).is_empty());
    }
}

#[test]
fn option_aliases_do_not_escape_declaring_package() {
    let mut other = declarations();
    other.aliases.clear();
    let t = table("default = ['backend/fast']");
    assert!(
        expand_declared_features(&t, &other, &FeatureRequest::default(), &BTreeSet::new()).is_err()
    );
    let out = expand("default = ['backend/fast']");
    assert!(!out.dep_features.contains_key("backend"));
    assert!(out.dep_features.contains_key("org.test/backend"));
}

#[test]
fn validation_checks_definitions_without_selecting_alternatives() {
    let t = table("a = []\nb = []\n[exclusive]\nmode = ['a', 'b']");
    assert!(validate_features_table(&t).is_empty());
    assert!(matches!(
        expand_features(
            &t,
            &FeatureRequest {
                all: true,
                ..Default::default()
            }
        ),
        Err(FeatureError::ExclusiveViolation { .. })
    ));
    let t = table("default = ['a', 'b']\na = []\nb = []\n[exclusive]\nmode = ['a', 'b']");
    assert!(!validate_features_table(&t).is_empty());
    let t = table("_private = ['missing']");
    assert!(!validate_features_table(&t).is_empty());
}

#[test]
fn config_conflict_merge_refuses_atomically() {
    let mut a = FeatureExpansion::default();
    a.config_values
        .insert("mode".into(), ConfigValue::String("a".into()));
    let mut b = FeatureExpansion::default();
    b.config_values
        .insert("mode".into(), ConfigValue::String("b".into()));
    b.active_deps.insert("new".into());
    let before = a.clone();
    assert!(matches!(a.merge(&b), Err(FeatureError::ConfigConflict(_))));
    assert_eq!(a, before);
    a.merge(&before).unwrap();
    assert_eq!(a, before);
}

#[test]
fn full_manifest_refuses_undeclared_default_and_dormant_references() {
    for body in [
        "default = ['dep:org.example/missing']",
        "_private = ['org.example/missing?/fast']",
    ] {
        let manifest = vibe_core::manifest::Manifest::parse_str(&format!("[package]\ngroup='org.example'\nname='app'\nkind='flow'\nversion='1.0.0'\n[features]\n{body}"))
            .unwrap();
        assert!(matches!(
            expand_manifest_features(&manifest, &FeatureRequest::default()),
            Err(FeatureError::Declaration(EffectError::UnknownDependency(name)))
                if name == "org.example/missing"
        ));
    }
}

#[test]
fn full_manifest_required_dependency_enables_weak_forwarding() {
    let manifest = vibe_core::manifest::Manifest::parse_str(
        "[package]\ngroup='org.example'\nname='app'\nkind='flow'\nversion='1.0.0'\n[requires.packages]\n'org.example/dep' = '^1'\n[features]\ndefault = ['org.example/dep?/fast']",
    ).unwrap();
    let out = expand_manifest_features(&manifest, &FeatureRequest::default()).unwrap();
    assert!(out.active_deps.contains("org.example/dep"));
    assert!(out.dep_features["org.example/dep"].contains("fast"));
}
