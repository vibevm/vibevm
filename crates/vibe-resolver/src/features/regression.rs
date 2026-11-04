//! Definition validity is independent of an invalid all-alternatives request.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#LEGACY-PACKAGE-COMPATIBILITY");
use super::*;

#[test]
fn valid_exclusive_definition_does_not_activate_all_alternatives() {
    let table: FeaturesTable =
        toml::from_str("default=['a']\na=[]\nb=[]\n[exclusive]\ng=['a','b']").unwrap();
    let findings = validate_features_table(&table);
    assert!(
        findings.is_empty(),
        "valid definition/defaults: {findings:?}"
    );
    assert!(matches!(
        expand_features(
            &table,
            &FeatureRequest {
                all: true,
                ..Default::default()
            }
        ),
        Err(FeatureError::ExclusiveViolation { .. })
    ));
}

#[test]
fn repeated_exclusive_members_are_one_set_member() {
    let table: FeaturesTable =
        toml::from_str("default=['a']\na=[]\n[exclusive]\ng=['a','a']").unwrap();
    assert!(validate_features_table(&table).is_empty());
    let result = expand_features(&table, &FeatureRequest::default()).unwrap();
    assert_eq!(
        result.active_features,
        BTreeSet::from(["a".into(), "default".into()])
    );
}

#[test]
fn dormant_private_references_and_syntax_are_both_checked() {
    let table: FeaturesTable =
        toml::from_str("public=[]\n_first=['missing']\n_second=['dep:']").unwrap();
    let findings = validate_features_table(&table);
    assert!(
        findings
            .iter()
            .any(|s| s.contains("unknown feature") && s.contains("_first")),
        "{findings:?}"
    );
    assert!(
        findings
            .iter()
            .any(|s| s.contains("malformed") && s.contains("_second")),
        "{findings:?}"
    );
}

#[test]
fn exclusive_members_must_be_declared_even_when_no_features_are_requested() {
    let table: FeaturesTable = toml::from_str("public=[]\n[exclusive]\ng=['missing']").unwrap();
    let findings = validate_features_table(&table);
    assert!(
        findings
            .iter()
            .any(|s| s.contains("unknown feature") && s.contains("g")),
        "{findings:?}"
    );
}

#[test]
fn an_unselected_private_bad_reference_prevents_expansion() {
    let table: FeaturesTable = toml::from_str("public=[]\n_private=['missing']").unwrap();
    assert!(matches!(
        expand_features(
            &table,
            &FeatureRequest {
                explicit: vec!["public".into()],
                no_defaults: true,
                all: false
            }
        ),
        Err(FeatureError::UnknownFeature(_))
    ));
}
