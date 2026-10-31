//! The feature rule set as a table: base, explicit choices, closure and
//! the refusals that name their reason.

use vibe_core::manifest::{FeatureDecl, InstallerOptionDecl, ProfileDecl};

use super::{FeatureRequest, ImpliedRule, resolve_features, tree_order};
use crate::error::StepError;

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
