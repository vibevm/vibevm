//! The feature rule set as a table: base, explicit choices, closure and
//! the refusals that name their reason.

use vibe_core::manifest::{FeatureDecl, InstallerOptionDecl, ProfileDecl};

use super::{FeatureRequest, ImpliedRule, resolve_features, tree_order};
use crate::error::StepError;

fn validated_preset() -> InstallerOptionDecl {
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
    manifest.installer_options.into_iter().next().unwrap()
}

#[test]
fn direct_flags_override_a_valid_profile_base() {
    let option = validated_preset();
    let profile = &option.profiles[0];
    let selected =
        resolve_features(&option, Some(profile), &request(&["qt"], &[]), None, None).unwrap();
    assert_eq!(selected.selected, ["gui", "qt"]);
    let removed =
        resolve_features(&option, Some(profile), &request(&[], &["gui"]), None, None).unwrap();
    assert!(removed.selected.is_empty());
}

#[test]
fn direct_flags_override_prompt_base_but_direct_contradictions_still_refuse() {
    let option = validated_preset();
    let prompted = ["gtk".to_owned()];
    let selected =
        resolve_features(&option, None, &request(&["qt"], &[]), None, Some(&prompted)).unwrap();
    assert_eq!(selected.selected, ["gui", "qt"]);
    assert!(
        resolve_features(
            &option,
            None,
            &request(&[], &["gui"]),
            None,
            Some(&prompted)
        )
        .unwrap()
        .selected
        .is_empty()
    );
    for conflicting in [request(&["gtk"], &["gui"]), request(&["qt"], &["qt"])] {
        assert!(matches!(
            resolve_features(&option, None, &conflicting, None, Some(&prompted)),
            Err(StepError::FeatureConflict { .. })
        ));
    }
}

#[test]
fn requires_chain_finishes_before_group_fallback_under_declaration_permutations() {
    let root = feature("root", |f| f.requires = vec!["relay".into()]);
    let relay = feature("relay", |f| f.requires = vec!["chosen".into()]);
    let fallback = feature("fallback", |f| {
        f.default = true;
        f.exclusive_group = Some("g".into());
    });
    let chosen = feature("chosen", |f| f.exclusive_group = Some("g".into()));
    for declarations in [
        vec![
            relay.clone(),
            fallback.clone(),
            chosen.clone(),
            root.clone(),
        ],
        vec![root, relay, fallback, chosen],
    ] {
        let mut option = option();
        option.features = declarations;
        option.profiles.clear();
        let result = resolve_features(
            &option,
            None,
            &request(&[], &[]),
            None,
            Some(&["root".into()]),
        )
        .unwrap();
        let mut selected = result.selected;
        selected.sort();
        assert_eq!(selected, ["chosen", "relay", "root"]);
        assert!(
            !result
                .implied
                .iter()
                .any(|r| r.rule == ImpliedRule::ExclusiveDefault)
        );
    }
}

#[test]
fn a_fallback_does_not_activate_another_group_before_its_requires_close() {
    for (first, second) in [("a", "z"), ("z", "a")] {
        let mut option = option();
        option.profiles.clear();
        option.features = vec![
            feature("trigger", |f| {
                f.default = true;
                f.exclusive_group = Some(first.into());
                f.requires = vec!["chosen".into()];
            }),
            feature("fallback", |f| {
                f.default = true;
                f.parent = Some("trigger".into());
                f.exclusive_group = Some(second.into());
            }),
            feature("chosen", |f| {
                f.parent = Some("trigger".into());
                f.exclusive_group = Some(second.into());
            }),
        ];
        let result = resolve_features(&option, None, &request(&[], &[]), None, Some(&[])).unwrap();
        assert_eq!(result.selected, ["trigger", "chosen"]);
        assert_eq!(
            result
                .implied
                .iter()
                .filter(|r| r.rule == ImpliedRule::ExclusiveDefault)
                .map(|r| r.feature.as_str())
                .collect::<Vec<_>>(),
            ["trigger"]
        );
    }
}

fn feature(id: &str, edit: impl FnOnce(&mut FeatureDecl)) -> FeatureDecl {
    let mut decl = FeatureDecl {
        id: id.into(),
        name: id.to_uppercase(),
        ..Default::default()
    };
    edit(&mut decl);
    decl
}

/// core (required root), gui (default) > { gtk (default) | qt } in group
/// `toolkit`, qt requires extras, docs > api (required child), extras.
fn option() -> InstallerOptionDecl {
    let mut option: InstallerOptionDecl = toml::from_str(
        "id = \"main\"\nname = \"Main\"\nplatform = \"linux\"\nplatform_name = \"Linux\"\narch = \"x86_64\"",
    )
    .unwrap();
    option.features = vec![
        feature("core", |f| f.required = true),
        feature("gui", |f| f.default = true),
        feature("gtk", |f| {
            f.parent = Some("gui".into());
            f.exclusive_group = Some("toolkit".into());
            f.default = true;
        }),
        feature("qt", |f| {
            f.parent = Some("gui".into());
            f.exclusive_group = Some("toolkit".into());
            f.requires = vec!["extras".into()];
        }),
        feature("docs", |_| {}),
        feature("api", |f| {
            f.parent = Some("docs".into());
            f.required = true;
        }),
        feature("extras", |_| {}),
    ];
    option.profiles = vec![ProfileDecl {
        id: "minimal".into(),
        name: "Minimal".into(),
        features: vec!["core".into()],
        ..Default::default()
    }];
    option
}

fn request(enable: &[&str], disable: &[&str]) -> FeatureRequest {
    FeatureRequest {
        enable: enable.iter().map(|id| (*id).to_owned()).collect(),
        disable: disable.iter().map(|id| (*id).to_owned()).collect(),
        profile: None,
    }
}

#[test]
fn the_rule_table() {
    let option = option();
    let profile = &option.profiles[0];
    let previous = vec!["core".to_owned(), "docs".to_owned()];
    let prompt = vec!["qt".to_owned()];
    // (case, profile, request, previous, prompt, selected, implied)
    type Case<'a> = (
        &'a str,
        Option<&'a ProfileDecl>,
        FeatureRequest,
        Option<&'a [String]>,
        Option<&'a [String]>,
        &'a [&'a str],
        &'a [(&'a str, ImpliedRule, Option<&'a str>)],
    );
    let cases: Vec<Case<'_>> = vec![
        (
            "defaults close over required roots",
            None,
            request(&[], &[]),
            None,
            None,
            &["core", "gui", "gtk"],
            &[("core", ImpliedRule::Required, None)],
        ),
        (
            "a child selects its ancestors and its required siblings",
            None,
            request(&["api"], &[]),
            None,
            None,
            &["core", "gui", "gtk", "docs", "api"],
            &[
                ("core", ImpliedRule::Required, None),
                ("docs", ImpliedRule::Parent, Some("api")),
            ],
        ),
        (
            "a parent selects its required child",
            None,
            request(&["docs"], &[]),
            None,
            None,
            &["core", "gui", "gtk", "docs", "api"],
            &[
                ("core", ImpliedRule::Required, None),
                ("api", ImpliedRule::Required, Some("docs")),
            ],
        ),
        (
            "an explicit group member replaces the default and pulls requires",
            None,
            request(&["qt"], &[]),
            None,
            None,
            &["core", "gui", "qt", "extras"],
            &[
                ("core", ImpliedRule::Required, None),
                ("extras", ImpliedRule::Requires, Some("qt")),
            ],
        ),
        (
            "deselecting a parent deselects its descendants",
            None,
            request(&[], &["gui"]),
            None,
            None,
            &["core"],
            &[("core", ImpliedRule::Required, None)],
        ),
        (
            "a profile is the base",
            Some(profile),
            request(&[], &[]),
            None,
            None,
            &["core"],
            &[],
        ),
        (
            "flags apply on top of a profile; the group default follows its parent",
            Some(profile),
            request(&["gui"], &[]),
            None,
            None,
            &["core", "gui", "gtk"],
            &[("gtk", ImpliedRule::ExclusiveDefault, Some("toolkit"))],
        ),
        (
            "an update reuses the recorded set",
            None,
            request(&[], &[]),
            Some(&previous),
            None,
            &["core", "docs", "api"],
            &[("api", ImpliedRule::Required, Some("docs"))],
        ),
        (
            "named flags replace the recorded set with the defaults",
            None,
            request(&["extras"], &[]),
            Some(&previous),
            None,
            &["core", "gui", "gtk", "extras"],
            &[("core", ImpliedRule::Required, None)],
        ),
        (
            "the checkboxes are the base and explicit",
            None,
            request(&[], &[]),
            None,
            Some(&prompt),
            &["core", "gui", "qt", "extras"],
            &[
                ("core", ImpliedRule::Required, None),
                ("gui", ImpliedRule::Parent, Some("qt")),
                ("extras", ImpliedRule::Requires, Some("qt")),
            ],
        ),
    ];
    for (case, profile, request, previous, prompt, selected, implied) in cases {
        let resolved = resolve_features(&option, profile, &request, previous, prompt)
            .unwrap_or_else(|error| panic!("{case}: {error}"));
        assert_eq!(resolved.selected, selected, "{case}");
        let got: Vec<(&str, ImpliedRule, Option<&str>)> = resolved
            .implied
            .iter()
            .map(|row| (row.feature.as_str(), row.rule, row.by.as_deref()))
            .collect();
        assert_eq!(got, implied, "{case}");
    }
}

#[test]
fn refusals_name_their_reason() {
    let option = option();
    let cases: Vec<(FeatureRequest, &str)> = vec![
        (request(&["nope"], &[]), "`nope` is not declared"),
        (request(&["gui"], &["gui"]), "both selected and deselected"),
        (
            request(&[], &["core"]),
            "required feature and is always selected",
        ),
        (
            request(&["docs"], &["api"]),
            "required while its parent `docs` is selected",
        ),
        (request(&["qt"], &["extras"]), "`qt` requires it"),
        (request(&["api"], &["docs"]), "`api` is beneath it"),
        (
            request(&[], &["gtk"]),
            "default of exclusive group `toolkit`",
        ),
        (
            request(&["gtk", "qt"], &[]),
            "more than one explicit selection: gtk, qt",
        ),
    ];
    for (request, needle) in cases {
        let error = resolve_features(&option, None, &request, None, None).unwrap_err();
        let text = error.to_string();
        assert!(text.contains(needle), "expected `{needle}` in: {text}");
        let expected = if needle.contains("not declared") {
            "unknown_feature"
        } else {
            "feature_conflict"
        };
        assert_eq!(error.code(), expected, "{text}");
    }
    let mut pulled = option.clone();
    pulled.features[4].requires = vec!["qt".into()];
    let error = resolve_features(&pulled, None, &request(&["docs"], &[]), None, None);
    assert!(
        matches!(&error, Err(StepError::FeatureConflict { reason, .. }) if reason.contains("would select gtk and qt")),
        "{error:?}"
    );
}

#[test]
fn the_tree_order_puts_children_under_their_parents() {
    let option = option();
    let order: Vec<(usize, &str)> = tree_order(&option.features)
        .into_iter()
        .map(|(depth, feature)| (depth, feature.id.as_str()))
        .collect();
    assert_eq!(
        order,
        [
            (0, "core"),
            (0, "gui"),
            (1, "gtk"),
            (1, "qt"),
            (0, "docs"),
            (1, "api"),
            (0, "extras")
        ]
    );
}

pub(super) fn validated_parent_group() -> InstallerOptionDecl {
    vibe_core::manifest::Manifest::parse_str(
        r#"
[package]
group = "org.example"
name = "parent-choice"
version = "1.0.0"
kind = "app"
[application]
id = "parent-choice"
[[installer_options]]
id = "main"
name = "Main"
platform = "linux"
platform_name = "Linux"
arch = "x86_64"
result = { external = "/tmp/parent-choice" }
[[installer_options.features]]
id = "parent-a"
name = "Parent A"
exclusive_group = "parent"
default = true
[[installer_options.features]]
id = "parent-b"
name = "Parent B"
exclusive_group = "parent"
[[installer_options.features]]
id = "child-a"
name = "Child A"
parent = "parent-a"
[[installer_options.steps]]
id = "setup"
title = "Set up"
kind = "shell"
command = "true"
"#,
    )
    .unwrap()
    .installer_options
    .into_iter()
    .next()
    .unwrap()
}

#[test]
fn group_replacement_cannot_discard_a_direct_descendant() {
    let option = validated_parent_group();
    let direct = request(&["parent-b", "child-a"], &[]);
    let error = resolve_features(&option, None, &direct, None, None).unwrap_err();
    assert!(matches!(error, StepError::FeatureConflict { .. }));
    assert!(
        error
            .to_string()
            .contains("discard directly selected feature `child-a`"),
        "{error}"
    );
    // A preset is lower precedence; only direct additions must survive.
    let preset = ["child-a".to_owned()];
    let overridden = resolve_features(
        &option,
        None,
        &request(&["parent-b"], &[]),
        None,
        Some(&preset),
    )
    .unwrap();
    assert_eq!(overridden.selected, ["parent-b"]);
}
