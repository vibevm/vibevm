//! Named, finite compatibility cases against the pre-extraction app evaluator.

use super::*;

#[test]
fn legacy_group_replacement_loses_a_direct_child_selection() {
    let option = super::tests::validated_parent_group();
    let request = FeatureRequest {
        enable: vec!["parent-b".into(), "child-a".into()],
        disable: vec![],
        profile: None,
    };
    let old = legacy_oracle::resolve_features(&option, None, &request, None, None).unwrap();
    assert_eq!(
        old.selected,
        ["parent-b"],
        "the old algorithm silently loses child-a"
    );
}

#[test]
fn validated_legacy_duplicate_requires_keeps_first_occurrence_semantics() {
    let mut manifest = vibe_core::manifest::Manifest::parse_str(
        "[package]\ngroup='org.example'\nname='duplicates'\nversion='1.0.0'\nkind='app'\n[application]\nid='duplicates'",
    ).unwrap();
    let mut option = super::tests::validated_parent_group();
    option.features.push(node("peer", None, &[]));
    option
        .features
        .iter_mut()
        .find(|f| f.id == "child-a")
        .unwrap()
        .requires = vec![
        "parent-a".into(),
        "peer".into(),
        "parent-a".into(),
        "peer".into(),
    ];
    manifest.installer_options.push(option);
    manifest.validate().unwrap();
    let option = &manifest.installer_options[0];
    let request = FeatureRequest {
        enable: vec!["child-a".into()],
        disable: vec![],
        profile: None,
    };
    let old = legacy_oracle::resolve_features(option, None, &request, None, Some(&[])).unwrap();
    let new = resolve_features(option, None, &request, None, Some(&[])).unwrap();
    assert_eq!(old, new);
    assert_eq!(new.selected, ["parent-a", "child-a", "peer"]);
    assert_eq!(
        new.implied
            .iter()
            .map(|r| (r.feature.as_str(), r.rule))
            .collect::<Vec<_>>(),
        [
            ("parent-a", ImpliedRule::Parent),
            ("peer", ImpliedRule::Requires)
        ]
    );
}

fn node(id: &str, parent: Option<&str>, requires: &[&str]) -> FeatureDecl {
    FeatureDecl {
        id: id.into(),
        name: id.into(),
        parent: parent.map(str::to_owned),
        requires: requires.iter().map(|id| (*id).into()).collect(),
        ..Default::default()
    }
}

fn option(features: Vec<FeatureDecl>) -> InstallerOptionDecl {
    let mut option: InstallerOptionDecl = toml::from_str(
        "id='main'\nname='Main'\nplatform='linux'\nplatform_name='Linux'\narch='x86_64'",
    )
    .unwrap();
    option.features = features;
    option
}

/// Characterizes the old bug before extraction: fallback runs before an
/// ordinary requires chain reaches its fixed point. Moving an unchanged node
/// changes whether the same explicit request is refused.
#[test]
fn legacy_requires_chain_exposes_early_group_fallback() {
    let mut fallback = node("fallback", None, &[]);
    fallback.default = true;
    fallback.exclusive_group = Some("choice".into());
    let mut selected = node("selected", None, &[]);
    selected.exclusive_group = Some("choice".into());
    let relay = node("relay", None, &["selected"]);
    let root = node("root", None, &["relay"]);
    let late = option(vec![
        relay.clone(),
        fallback.clone(),
        selected.clone(),
        root.clone(),
    ]);
    let early = option(vec![root, relay, fallback, selected]);
    let chosen = vec!["root".to_owned()];
    let request = FeatureRequest::default();
    let old_late = legacy_oracle::resolve_features(&late, None, &request, None, Some(&chosen));
    let old_early = legacy_oracle::resolve_features(&early, None, &request, None, Some(&chosen));
    assert!(matches!(old_late, Err(StepError::FeatureConflict { .. })));
    assert_eq!(old_early.unwrap().selected, ["root", "relay", "selected"]);
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#LEGACY-APP-COMPATIBILITY"
)]
fn named_precedence_matrix_matches_legacy() {
    let mut base = node("base", None, &[]);
    base.default = true;
    let mut required = node("required", None, &[]);
    required.required = true;
    let mut child = node("child", Some("parent"), &["peer"]);
    child.exclusive_group = Some("pick".into());
    let mut sibling = node("sibling", Some("parent"), &[]);
    sibling.exclusive_group = Some("pick".into());
    sibling.default = true;
    let model = option(vec![
        base,
        required,
        node("parent", None, &[]),
        child,
        sibling,
        node("peer", None, &[]),
    ]);
    let profile = ProfileDecl {
        id: "small".into(),
        name: "Small".into(),
        features: vec!["base".into()],
        ..Default::default()
    };
    let previous = vec!["parent".into(), "child".into(), "removed".into()];
    let prompt = vec!["child".into()];
    let cases = [
        ("defaults", None, FeatureRequest::default(), None, None),
        (
            "profile",
            Some(&profile),
            FeatureRequest {
                profile: Some("small".into()),
                ..Default::default()
            },
            None,
            None,
        ),
        (
            "recorded filters removed",
            None,
            FeatureRequest::default(),
            Some(previous.as_slice()),
            None,
        ),
        (
            "flags restart from defaults",
            None,
            FeatureRequest {
                enable: vec!["peer".into()],
                ..Default::default()
            },
            Some(previous.as_slice()),
            None,
        ),
        (
            "prompt overrides base",
            None,
            FeatureRequest::default(),
            None,
            Some(prompt.as_slice()),
        ),
        (
            "explicit group replacement",
            None,
            FeatureRequest {
                enable: vec!["child".into()],
                ..Default::default()
            },
            None,
            None,
        ),
        (
            "disabled required refuses",
            None,
            FeatureRequest {
                disable: vec!["required".into()],
                ..Default::default()
            },
            None,
            None,
        ),
        (
            "disabled ancestor refuses",
            None,
            FeatureRequest {
                enable: vec!["child".into()],
                disable: vec!["parent".into()],
                profile: None,
            },
            None,
            None,
        ),
        (
            "contradictory flags refuse",
            None,
            FeatureRequest {
                enable: vec!["base".into()],
                disable: vec!["base".into()],
                profile: None,
            },
            None,
            None,
        ),
        (
            "unknown flag",
            None,
            FeatureRequest {
                enable: vec!["missing".into()],
                ..Default::default()
            },
            None,
            None,
        ),
        (
            "explicit group conflict",
            None,
            FeatureRequest {
                enable: vec!["child".into(), "sibling".into()],
                ..Default::default()
            },
            None,
            None,
        ),
    ];
    for (name, profile, request, previous, prompt) in cases {
        let old = legacy_oracle::resolve_features(&model, profile, &request, previous, prompt);
        let new = resolve_features(&model, profile, &request, previous, prompt);
        match (old, new) {
            (Ok(old), Ok(new)) => assert_eq!(old, new, "{name}"),
            (Err(old), Err(new)) => assert_eq!(old.to_string(), new.to_string(), "{name}"),
            pair => panic!("{name}: {pair:?}"),
        }
    }
}

#[test]
fn legacy_preset_overrides_are_characterized_from_validated_definitions() {
    let manifest = vibe_core::manifest::Manifest::parse_str(
        r#"
[package]
group = "org.example"
name = "preset"
version = "1.0.0"
kind = "app"
[application]
id = "preset"
[[installer_options]]
id = "main"
name = "Main"
platform = "linux"
platform_name = "Linux"
arch = "x86_64"
result = { external = "/tmp/preset" }
[[installer_options.features]]
id = "gui"
name = "GUI"
[[installer_options.features]]
id = "gtk"
name = "GTK"
parent = "gui"
exclusive_group = "toolkit"
default = true
[[installer_options.features]]
id = "qt"
name = "Qt"
parent = "gui"
exclusive_group = "toolkit"
[[installer_options.profiles]]
id = "typical"
name = "Typical"
features = ["gtk"]
[[installer_options.steps]]
id = "setup"
title = "Set up"
kind = "shell"
command = "true"
"#,
    )
    .unwrap();
    let option = &manifest.installer_options[0];
    let profile = &option.profiles[0];
    let switch = FeatureRequest {
        profile: Some(profile.id.clone()),
        enable: vec!["qt".into()],
        disable: vec![],
    };
    let disable_parent = FeatureRequest {
        profile: Some(profile.id.clone()),
        enable: vec![],
        disable: vec!["gui".into()],
    };
    for (request, expected) in [
        (switch, "more than one explicit selection: gtk, qt"),
        (disable_parent, "selected feature `gtk` is beneath it"),
    ] {
        let error = legacy_oracle::resolve_features(option, Some(profile), &request, None, None)
            .unwrap_err();
        assert!(matches!(error, StepError::FeatureConflict { .. }));
        assert!(error.to_string().contains(expected), "{error}");
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#LEGACY-APP-COMPATIBILITY"
)]
fn named_graph_matrix_matches_legacy_on_unaffected_cases() {
    let cycle = option(vec![node("a", None, &["b"]), node("b", None, &["a"])]);
    let mut required = node("required", Some("root"), &[]);
    required.required = true;
    let tree = option(vec![
        required,
        node("leaf", Some("root"), &["other"]),
        node("root", None, &[]),
        node("other", None, &[]),
    ]);
    let mut fallback = node("fallback", Some("root"), &[]);
    fallback.default = true;
    fallback.exclusive_group = Some("g".into());
    let mut chosen = node("chosen", Some("root"), &[]);
    chosen.exclusive_group = Some("g".into());
    let grouped = option(vec![node("root", None, &[]), fallback, chosen]);
    let rows = [
        ("positive requires cycle", &cycle, vec!["a"], vec![]),
        ("requires disable conflict", &cycle, vec!["a"], vec!["b"]),
        ("parent and required reasons", &tree, vec!["leaf"], vec![]),
        (
            "required child disable",
            &tree,
            vec!["root"],
            vec!["required"],
        ),
        ("inactive group empty", &grouped, vec![], vec![]),
        (
            "active group fallback reason",
            &grouped,
            vec!["root"],
            vec![],
        ),
        ("explicit group choice", &grouped, vec!["chosen"], vec![]),
    ];
    for (name, option, enabled, disabled) in rows {
        let request = FeatureRequest {
            enable: enabled.into_iter().map(str::to_owned).collect(),
            disable: disabled.into_iter().map(str::to_owned).collect(),
            profile: None,
        };
        // An explicit empty prompt base keeps node defaults out of this matrix;
        // the group fallback, when eligible, is what the case must exercise.
        let old = legacy_oracle::resolve_features(option, None, &request, None, Some(&[]));
        let new = resolve_features(option, None, &request, None, Some(&[]));
        match (old, new) {
            (Ok(old), Ok(new)) => assert_eq!(old, new, "{name}"),
            (Err(old), Err(new)) => assert_eq!(old.to_string(), new.to_string(), "{name}"),
            pair => panic!("{name}: {pair:?}"),
        }
    }
}
