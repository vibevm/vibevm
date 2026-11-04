//! Finite old/new comparisons for valid package definitions and their requests.
//! Deliberately corrected invalid-definition behavior is in regression.rs.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#LEGACY-PACKAGE-COMPATIBILITY");

use super::*;

fn request(explicit: &[&str], no_defaults: bool, all: bool) -> FeatureRequest {
    FeatureRequest {
        explicit: explicit.iter().map(|s| (*s).into()).collect(),
        no_defaults,
        all,
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#LEGACY-PACKAGE-COMPATIBILITY"
)]
fn named_valid_package_matrix_matches_the_original_expansion() {
    let graph = "default=['a']\na=['b','b','dep:compiler','compiler/strong','other?/weak','subskill:guide/path']\nb=['_helper']\n_helper=['b']\nextra=[]";
    let overlapping = "a=[]\nb=[]\nc=[]\n[exclusive]\nfirst=['a','b']\nsecond=['b','c']";
    let exclusive_default = "default=['a']\na=[]\nb=[]\n[exclusive]\nmode=['b','a']";
    let literal = "default=['lint level']\n'lint level'=['_internal helper','dep:compiler']\n'_internal helper'=[]\n' padded '=[]\n[exclusive]\n'build mode'=['lint level']";
    let rows = [
        (
            "default closes private cycle and effects",
            graph,
            request(&[], false, false),
        ),
        ("no defaults", graph, request(&[], true, false)),
        ("explicit union", graph, request(&["extra"], false, false)),
        ("explicit only", graph, request(&["extra"], true, false)),
        (
            "all includes default and reaches private",
            graph,
            request(&[], true, true),
        ),
        (
            "all ignores explicit list",
            graph,
            request(&["unknown", "_helper"], true, true),
        ),
        (
            "private direct refusal",
            graph,
            request(&["_helper"], false, false),
        ),
        (
            "unknown direct refusal",
            graph,
            request(&["unknown"], false, false),
        ),
        (
            "unknown underscore is still private",
            graph,
            request(&["_missing"], true, false),
        ),
        (
            "explicit named default overrides no-default flag",
            graph,
            request(&["default"], true, false),
        ),
        (
            "repeated explicit IDs are a set",
            graph,
            request(&["extra", "extra"], true, false),
        ),
        (
            "overlap permits disjoint ends",
            overlapping,
            request(&["a", "c"], true, false),
        ),
        (
            "overlap permits shared member",
            overlapping,
            request(&["b"], true, false),
        ),
        (
            "overlap refuses selected conflicting pair",
            overlapping,
            request(&["a", "b"], true, false),
        ),
        (
            "package explicit does not replace default",
            exclusive_default,
            request(&["b"], false, false),
        ),
        (
            "no default permits other alternative",
            exclusive_default,
            request(&["b"], true, false),
        ),
        (
            "empty legacy set is a no-op",
            "a=[]\n[exclusive]\nempty=[]",
            request(&["a"], true, false),
        ),
        ("empty table", "", request(&[], false, false)),
        (
            "literal internal spaces and private reference",
            literal,
            request(&[], false, false),
        ),
        (
            "literal spelling is not trimmed",
            literal,
            request(&[" padded "], true, false),
        ),
        (
            "trimmed spelling is not an alias",
            literal,
            request(&["padded"], true, false),
        ),
    ];
    for (name, text, request) in rows {
        let table: FeaturesTable = toml::from_str(text).unwrap();
        assert_eq!(
            expand_features(&table, &request),
            legacy::expand_features(&table, &request),
            "{name}"
        );
    }
}

#[test]
fn parsed_literal_table_roundtrip_and_effect_values_preserve_their_exact_subjects() {
    let raw = "default=['entry mode']\n'entry mode'=['dep:compiler','compiler/optimise','optional?/trace','subskill:guide/a b','_internal helper']\n'_internal helper'=[]\n[exclusive]\n'build mode'=['entry mode']";
    let table: FeaturesTable = toml::from_str(raw).unwrap();
    let written = toml::to_string(&table).unwrap();
    let roundtrip: FeaturesTable = toml::from_str(&written).unwrap();
    assert_eq!(table, roundtrip);
    let result = expand_features(&roundtrip, &FeatureRequest::default()).unwrap();
    assert_eq!(
        result.active_features,
        BTreeSet::from([
            "default".into(),
            "entry mode".into(),
            "_internal helper".into()
        ])
    );
    assert_eq!(result.active_deps, BTreeSet::from(["compiler".into()]));
    assert_eq!(
        result.dep_features,
        BTreeMap::from([("compiler".into(), BTreeSet::from(["optimise".into()]))])
    );
    assert_eq!(
        result.weak_dep_features,
        BTreeMap::from([("optional".into(), BTreeSet::from(["trace".into()]))])
    );
    assert_eq!(
        result.active_subskills,
        BTreeSet::from(["guide/a b".into()])
    );
    assert!(validate_features_table(&roundtrip).is_empty());
}
