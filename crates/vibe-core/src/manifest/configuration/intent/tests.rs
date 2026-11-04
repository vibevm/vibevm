//! Source answers normalize into the existing evaluator without replacing originals.
use super::*;
use crate::configuration::{ChoiceOutcome, MergeIntent};
use crate::manifest::Manifest;

fn target() -> ConfigurationDecl {
    Manifest::parse_str(
        r#"
[package]
group="org.example"
name="target"
version="1.0.0"
kind="flow"
[[configuration.features]]
id="base"
name="Base"
default=true
[[configuration.features]]
id="_public"
name="Public despite spelling"
requires=["backing node"]
[[configuration.features]]
id="backing node"
name="Backing"
visibility="internal"
[[configuration.features]]
id="quiet"
name="Quiet"
[[configuration.features]]
id="wal"
name="Log"
visibility="internal"
[[configuration.features]]
id="audit"
name="Audit"
guards=[{member="wal"}]
[[configuration.groups]]
mode="choice"
id="mode"
name="Mode"
cardinality="exactly-one"
members=["quiet","wal"]
defaults=["quiet"]
[[configuration.groups]]
mode="choice"
id="extras"
name="Extras"
cardinality="zero-or-more"
members=["audit"]
[[configuration.parameters]]
id="limit"
name="Limit"
type="integer"
min=1
max=100
default=20
[[configuration.parameters]]
id="path"
name="Path"
type="path"
must_exist=true
[[configuration.parameters]]
id="token"
name="Token"
type="string"
secret=true
[[configuration.profiles]]
id="audited"
name="Audited"
default=true
features=["backing node"]
selections.mode={intent="exact",members=["wal"]}
selections.extras={intent="additive",members=["audit"]}
parameters.limit=40
"#,
    )
    .unwrap()
    .configuration
    .unwrap()
}
fn intent(text: &str) -> ConfigurationIntentDecl {
    toml::from_str(text).unwrap()
}
fn resolved(d: &ConfigurationDecl, n: &NormalizedConfigurationInput) -> Vec<String> {
    let r = d
        .compile_choices()
        .unwrap()
        .evaluate(&n.features, std::slice::from_ref(&n.choices));
    let ChoiceOutcome::Resolved { features } = r.outcome else {
        panic!("{r:?}")
    };
    features.selected
}
#[test]
fn explicitly_named_profile_and_overrides_preserve_dormant_original_input() {
    let d = target();
    let original = intent(
        "policy='require-selection'\nprofile='audited'\ndefault_features=false\nselections.mode={intent='exact',members=['quiet']}\nparameters.limit=30",
    );
    let before = original.clone();
    let n = d.normalize_input(&original, "member-a/target").unwrap();
    assert_eq!(original, before);
    assert_eq!(
        n.features.base,
        FeatureBase::Selected(vec!["backing node".into()])
    );
    assert_eq!(n.parameters["limit"], ParameterValue::Integer(30));
    assert_eq!(resolved(&d, &n), ["backing node", "quiet"]);
    let evaluated = d
        .compile_choices()
        .unwrap()
        .evaluate(&n.features, std::slice::from_ref(&n.choices));
    assert_eq!(evaluated.controls[1].projections[0].dormant, ["audit"]);
    let mut back = original;
    back.selections.remove("mode");
    let n = d.normalize_input(&back, "member-a/target").unwrap();
    assert_eq!(resolved(&d, &n), ["backing node", "wal", "audit"]);
}
#[test]
fn explicit_empty_and_omitted_replace_profile_answers_independently() {
    let d = target();
    let empty = intent(
        "policy='require-selection'\nprofile='audited'\nselections.extras={intent='exact',members=[]}",
    );
    let n = d.normalize_input(&empty, "a").unwrap();
    assert_eq!(
        n.choices.answers["extras"].selection,
        Selection::Explicit(vec![])
    );
    assert_eq!(resolved(&d, &n), ["backing node", "wal"]);
    let omitted =
        intent("policy='require-selection'\nprofile='audited'\nselections.mode={intent='exact'}");
    let n = d.normalize_input(&omitted, "a").unwrap();
    assert_eq!(n.choices.answers["mode"].selection, Selection::Omitted);
    assert_eq!(n.choices.answers["mode"].intent, MergeIntent::Exact);
    let r = d
        .compile_choices()
        .unwrap()
        .evaluate(&n.features, &[n.choices]);
    assert!(matches!(r.outcome, ChoiceOutcome::Unresolved));
    assert_eq!(r.questions[0].group, "mode");
}
#[test]
fn default_profile_is_not_selected_and_parameter_defaults_are_not_filled() {
    let d = target();
    for defaults in [true, false] {
        let i = intent(&format!(
            "policy='require-selection'\ndefault_features={defaults}"
        ));
        let n = d.normalize_input(&i, "a").unwrap();
        assert!(n.parameters.is_empty());
        assert!(n.choices.answers.is_empty());
        assert_eq!(
            n.features.base,
            if defaults {
                FeatureBase::Defaults
            } else {
                FeatureBase::Selected(vec![])
            }
        );
        assert!(matches!(
            d.compile_choices()
                .unwrap()
                .evaluate(&n.features, &[n.choices])
                .outcome,
            ChoiceOutcome::Unresolved
        ));
    }
    let n = d
        .normalize_input(&intent("policy='use-declared-defaults'"), "a")
        .unwrap();
    assert_eq!(resolved(&d, &n), ["base", "quiet"]);
}
#[test]
fn visibility_is_explicit_and_only_direct_additions_are_public_only() {
    let d = target();
    let n = d
        .normalize_input(
            &intent("policy='use-declared-defaults'\nfeatures=['_public']"),
            "a",
        )
        .unwrap();
    assert_eq!(
        resolved(&d, &n),
        ["base", "_public", "backing node", "quiet"]
    );
    let e = d
        .normalize_input(
            &intent("policy='use-declared-defaults'\nfeatures=['backing node']"),
            "a",
        )
        .unwrap_err();
    assert!(e.message.contains("internal feature"));
    let n = d
        .normalize_input(
            &intent("policy='require-selection'\nselections.mode={intent='exact',members=['wal']}"),
            "a",
        )
        .unwrap();
    assert_eq!(resolved(&d, &n), ["base", "wal"]);
    let back: ConfigurationDecl = toml::from_str(&toml::to_string(&d).unwrap()).unwrap();
    assert_eq!(back, d);
}
#[test]
fn unknown_dormant_inputs_and_typed_parameter_errors_point_to_actual_input() {
    let d = target();
    for (tail, field) in [
        ("profile='missing'", "configuration.profile"),
        ("features=['missing']", "configuration.features"),
        (
            "selections.unknown={intent='exact'}",
            "configuration.selections.unknown",
        ),
        (
            "selections.extras={intent='exact',members=['gone']}",
            "configuration.selections.extras.members",
        ),
        ("parameters.missing=1", "configuration.parameters.missing"),
        ("parameters.limit='forty'", "configuration.parameters.limit"),
        ("parameters.limit=101", "configuration.parameters.limit"),
    ] {
        let i = intent(&("policy='require-selection'\n".to_owned() + tail));
        let e = d.normalize_input(&i, "a").unwrap_err();
        assert_eq!(e.field.to_string(), field, "{e}");
    }
    let i = intent("policy='require-selection'\nparameters.token='SYN_SOURCE_SECRET'");
    let e = d.normalize_input(&i, "a").unwrap_err();
    assert_eq!(e.field.to_string(), "configuration.parameters.token");
    assert!(!format!("{e:?}").contains("SYN_SOURCE_SECRET"));
    let i = intent("policy='require-selection'\nparameters.path='/a/nonexistent/host/path'");
    assert_eq!(
        d.normalize_input(&i, "a").unwrap().parameters["path"],
        ParameterValue::Text("/a/nonexistent/host/path".into())
    );
}
#[test]
fn required_policy_and_source_shape_remain_distinct_from_target_admission() {
    assert!(toml::from_str::<ConfigurationIntentDecl>("features=[]").is_err());
    assert!(toml::from_str::<ConfigurationIntentDecl>("policy='yes'").is_err());
    assert!(
        toml::from_str::<ConfigurationIntentDecl>("policy='require-selection'\ninstance='fake'")
            .is_err()
    );
    let d = target();
    for text in [
        "policy='require-selection'\nfeatures=['base','base']",
        "policy='require-selection'\nselections.extras={intent='exact',members=['audit','audit']}",
        "policy='require-selection'\nprofile=' '",
    ] {
        assert!(d.normalize_input(&intent(text), "a").is_err());
    }
}
