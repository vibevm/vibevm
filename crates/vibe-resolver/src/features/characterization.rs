//! Literal grammar and legacy definition/request behavior before lowering.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#LEGACY-PACKAGE-COMPATIBILITY");

use super::*;

fn table(text: &str) -> FeaturesTable {
    toml::from_str(text).unwrap()
}
fn explicit(names: &[&str]) -> FeatureRequest {
    FeatureRequest {
        explicit: names.iter().map(|s| (*s).into()).collect(),
        no_defaults: true,
        all: false,
    }
}

#[test]
fn legacy_quoted_feature_and_group_ids_are_literal() {
    let manifest = vibe_core::manifest::Manifest::parse_str(
        r#"
[package]
group = "org.example"
name = "literal"
kind = "flow"
version = "1.0.0"
[features]
default = ["lint level"]
"lint level" = ["_internal helper", "_internal helper", "dep:compiler"]
"_internal helper" = []
[features.exclusive]
"build mode" = ["lint level"]
"#,
    )
    .unwrap();
    assert!(legacy::validate_features_table(&manifest.features).is_empty());
    let selected = legacy::expand_features(&manifest.features, &FeatureRequest::default()).unwrap();
    assert_eq!(
        selected.active_features,
        BTreeSet::from([
            "default".into(),
            "lint level".into(),
            "_internal helper".into()
        ])
    );
    assert_eq!(
        FeatureValue::parse("lint level").unwrap(),
        FeatureValue::Feature("lint level".into())
    );
}

#[test]
fn legacy_repeated_edges_are_redundant_but_repeated_exclusive_members_false_conflict() {
    let t = table("a=['b','b']\nb=[]");
    assert_eq!(
        legacy::expand_features(&t, &explicit(&["a"]))
            .unwrap()
            .active_features,
        BTreeSet::from(["a".into(), "b".into()])
    );
    let t = table("a=[]\n[exclusive]\ng=['a','a']");
    assert_eq!(
        legacy::expand_features(&t, &explicit(&["a"])).unwrap_err(),
        FeatureError::ExclusiveViolation {
            group: "g".into(),
            active: vec!["a".into(), "a".into()]
        }
    );
}

#[test]
fn legacy_empty_and_unknown_group_members_are_not_checked() {
    let t = table("a=[]\n[exclusive]\nempty=[]\nunknown=['not-declared']");
    assert!(legacy::validate_features_table(&t).is_empty());
    assert_eq!(
        legacy::expand_features(&t, &explicit(&["a"]))
            .unwrap()
            .active_features,
        BTreeSet::from(["a".into()])
    );
}

#[test]
fn legacy_dormant_private_unknown_reference_is_missed_but_syntax_is_reported() {
    let missing = table("public=[]\n_private=['missing']");
    assert!(legacy::validate_features_table(&missing).is_empty());
    assert!(legacy::expand_features(&missing, &explicit(&["public"])).is_ok());
    let malformed = table("public=[]\n_private=['dep:']");
    assert!(
        legacy::validate_features_table(&malformed)
            .iter()
            .any(|s| s.contains("malformed"))
    );
    assert!(legacy::expand_features(&malformed, &explicit(&["public"])).is_ok());
}

#[test]
fn legacy_all_takes_precedence_over_the_explicit_list() {
    let t = table("public=[]\n_private=[]");
    let request = FeatureRequest {
        explicit: vec!["_private".into(), "unknown".into()],
        no_defaults: false,
        all: true,
    };
    assert_eq!(
        legacy::expand_features(&t, &request)
            .unwrap()
            .active_features,
        BTreeSet::from(["public".into()])
    );
    assert!(matches!(
        legacy::expand_features(&t, &explicit(&["_private"])),
        Err(FeatureError::PrivateFeature(_))
    ));
}

#[test]
fn legacy_validator_confuses_a_valid_definition_with_all_features_request() {
    let t = table("default=['a']\na=[]\nb=[]\n[exclusive]\ng=['a','b']");
    assert!(legacy::expand_features(&t, &FeatureRequest::default()).is_ok());
    assert!(
        legacy::validate_features_table(&t)
            .iter()
            .any(|s| s.contains("exclusive group"))
    );
    assert!(matches!(
        legacy::expand_features(
            &t,
            &FeatureRequest {
                all: true,
                ..Default::default()
            }
        ),
        Err(FeatureError::ExclusiveViolation { .. })
    ));
}
