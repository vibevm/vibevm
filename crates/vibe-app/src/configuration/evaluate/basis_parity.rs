//! Differential characterization against the immutable pre-extraction evaluator.
#![cfg(test)]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#APP-CONFIGURATION-BASIS");
use super::legacy_basis_oracle::legacy_evaluate;
use super::*;

#[derive(Clone, Copy, Debug)]
enum Mode {
    Reuse,
    Reviewed,
    Replacement,
    ReviewedReplacement,
}
const MODES: [Mode; 4] = [
    Mode::Reuse,
    Mode::Reviewed,
    Mode::Replacement,
    Mode::ReviewedReplacement,
];
fn input(text: &str) -> w::ConfigurationInput {
    w::ConfigurationInput {
        intent: vocabulary::source_input(
            &toml::from_str::<m::ConfigurationIntentDecl>(text).unwrap(),
        ),
        disabled_features: vec![],
    }
}
fn parity(
    definition: &m::ConfigurationDecl,
    input: &w::ConfigurationInput,
    previous: Option<&w::ConfigurationRecord>,
    mode: Mode,
    root: &std::path::Path,
    ephemeral: &[(String, String)],
) -> w::ConfigurationRecord {
    let resolver = |value: &str| root.join(value);
    let fresh = BTreeSet::new();
    let invoke = |old: bool| {
        let request = EvaluationRequest {
            definition,
            input,
            previous: previous.map(crate::configuration::ConfigurationHistory::from),
            history_use: crate::configuration::HistoryUse::Local,
            replacement: matches!(mode, Mode::Replacement | Mode::ReviewedReplacement),
            reviewed: matches!(mode, Mode::Reviewed | Mode::ReviewedReplacement),
            expected_definition: None,
            fresh_parameters: &fresh,
            ephemeral,
            facts: HostFacts {
                resolve_path: &resolver,
                ..Default::default()
            },
            prompt: None,
        };
        if old {
            legacy_evaluate(request)
        } else {
            evaluate(request)
        }
    };
    let old = invoke(true).expect("historical fixture evaluation");
    let current = invoke(false).expect("extracted fixture evaluation");
    assert_eq!(
        serde_json::to_vec(&current.record).unwrap(),
        serde_json::to_vec(&old.record).unwrap(),
        "externally visible wire record differs in {mode:?}"
    );
    assert_eq!(
        current.parameters, old.parameters,
        "ephemeral execution values differ in {mode:?}"
    );
    current.record
}
fn choice_definition() -> m::ConfigurationDecl {
    toml::from_str(
        r#"
[[features]]
id='a'
name='A'
[[features]]
id='b'
name='B'
[[groups]]
mode='choice'
id='g'
name='Group'
cardinality='zero-or-more'
members=['a','b']
defaults=['a']
[[profiles]]
id='p'
name='Profile'
selections.g={intent='exact',members=['b']}
"#,
    )
    .unwrap()
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#APP-CONFIGURATION-BASIS"
)]
fn fresh_reuse_review_replacement_and_profile_overrides_match_old_wire_and_readiness() {
    let root = tempfile::tempdir().unwrap();
    for text in [
        "policy='require-selection'\ndefault_features=false\nselections.g={intent='exact',members=['a']}",
        "policy='use-declared-defaults'\ndefault_features=false",
        "policy='use-declared-defaults'",
        "policy='use-declared-defaults'\nprofile='p'\nselections.g={intent='exact',members=[]}",
        "policy='use-declared-defaults'\nprofile='p'\nselections.g={intent='exact'}",
        "policy='require-selection'\nprofile='p'",
    ] {
        let mut d = choice_definition();
        let i = input(text);
        let prior = parity(&d, &i, None, Mode::Replacement, root.path(), &[]);
        assert!(prior.evaluation.resolved.is_some());
        let m::ConfigurationGroupDecl::Choice { defaults, .. } = &mut d.groups[0] else {
            unreachable!()
        };
        *defaults = Some(vec!["b".into()]);
        for mode in MODES {
            parity(&d, &i, Some(&prior), mode, root.path(), &[]);
        }
    }
    // A new automatic policy is not silently renewed from explicit history.
    let d = choice_definition();
    let explicit = input(
        "policy='require-selection'\ndefault_features=false\nselections.g={intent='exact',members=['a']}",
    );
    let prior = parity(&d, &explicit, None, Mode::Replacement, root.path(), &[]);
    let automatic = input("policy='use-declared-defaults'\ndefault_features=false");
    let changed = parity(&d, &automatic, Some(&prior), Mode::Reuse, root.path(), &[]);
    assert!(changed.evaluation.resolved.is_none());
    assert!(
        changed
            .evaluation
            .questions
            .iter()
            .any(|q| matches!(q, w::ConfigurationQuestion::Review(_)))
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#APP-CONFIGURATION-BASIS"
)]
fn closure_dormancy_removal_and_explicit_takeover_match_the_frozen_evaluator() {
    let root = tempfile::tempdir().unwrap();
    let mut d: m::ConfigurationDecl = toml::from_str(
        r#"
[[features]]
id='parent'
name='Parent'
[[features]]
id='a'
name='A'
parent='parent'
[[features]]
id='b'
name='B'
parent='parent'
[[groups]]
mode='closure'
id='g'
name='Closure'
cardinality='exactly-one'
members=['a','b']
fallback=['a']
"#,
    )
    .unwrap();
    let on = input("policy='require-selection'\ndefault_features=false\nfeatures=['parent']");
    let off = input("policy='require-selection'\ndefault_features=false");
    let first = parity(&d, &on, None, Mode::Replacement, root.path(), &[]);
    assert_eq!(first.evaluation.reliance, ["closure:g"]);
    let dormant = parity(&d, &off, Some(&first), Mode::Reuse, root.path(), &[]);
    assert_eq!(dormant.evaluation.reliance, ["closure:g"]);
    assert!(dormant.evaluation.resolved.is_some());
    let m::ConfigurationGroupDecl::Closure { fallback, .. } = &mut d.groups[0] else {
        unreachable!()
    };
    *fallback = Some(vec!["b".into()]);
    let explicit = input("policy='require-selection'\ndefault_features=false\nfeatures=['b']");
    for mode in MODES {
        parity(&d, &off, Some(&dormant), mode, root.path(), &[]);
        parity(&d, &on, Some(&dormant), mode, root.path(), &[]);
        parity(&d, &explicit, Some(&dormant), mode, root.path(), &[]);
    }
    d.groups.clear();
    for mode in MODES {
        parity(&d, &off, Some(&dormant), mode, root.path(), &[]);
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#APP-CONFIGURATION-BASIS"
)]
fn dormant_absent_and_removed_choice_defaults_keep_original_review_law() {
    let root = tempfile::tempdir().unwrap();
    let mut d: m::ConfigurationDecl = toml::from_str(
        r#"
[[features]]
id='on'
name='On'
[[features]]
id='off'
name='Off'
[[features]]
id='a'
name='A'
[[groups]]
mode='choice'
id='switch'
name='Switch'
cardinality='exactly-one'
members=['on','off']
[[groups]]
mode='choice'
id='g'
name='Dormant'
cardinality='zero-or-more'
members=['a']
guards=[{member='on'}]
"#,
    )
    .unwrap();
    let i = input(
        "policy='use-declared-defaults'\ndefault_features=false\nselections.switch={intent='exact',members=['off']}",
    );
    let prior = parity(&d, &i, None, Mode::Replacement, root.path(), &[]);
    assert!(prior.evaluation.resolved.is_some());
    assert_eq!(prior.evaluation.reliance, ["choice:g"]);
    assert!(
        prior.evaluation.defaults.is_empty(),
        "reliance is wider than active DefaultUse"
    );
    d.groups.pop();
    for mode in MODES {
        parity(&d, &i, Some(&prior), mode, root.path(), &[]);
    }
    let reuse = parity(&d, &i, Some(&prior), Mode::Reuse, root.path(), &[]);
    let reviewed = parity(&d, &i, Some(&prior), Mode::Reviewed, root.path(), &[]);
    assert!(reuse.evaluation.resolved.is_none());
    assert!(reviewed.evaluation.resolved.is_some());
    assert!(reviewed.evaluation.reliance.is_empty());
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#APP-CONFIGURATION-BASIS"
)]
fn parameter_origins_secrets_and_path_realizations_remain_adapter_owned() {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    let mut d: m::ConfigurationDecl = toml::from_str(
        r#"
[[parameters]]
id='n'
name='Count'
type='integer'
default=1
[[parameters]]
id='root'
name='Root'
type='path'
default='data'
[[parameters]]
id='token'
name='Token'
type='string'
secret=true
required=false
default='PUBLIC_AUTHORED_DEFAULT'
"#,
    )
    .unwrap();
    let supplied = vec![("token".into(), "SYNTHETIC_BASIS_EPHEMERAL_901".into())];
    let i = input("policy='require-selection'\ndefault_features=false");
    let prior = parity(&d, &i, None, Mode::Replacement, a.path(), &supplied);
    assert!(prior.evaluation.resolved.is_some());
    d.parameters[0].default = Some(m::ParameterValue::Integer(2));
    for mode in MODES {
        parity(&d, &i, Some(&prior), mode, b.path(), &[]);
        parity(&d, &i, Some(&prior), mode, b.path(), &supplied);
    }
    let explicit = input("policy='require-selection'\ndefault_features=false\nparameters.n=7");
    for mode in MODES {
        parity(&d, &explicit, Some(&prior), mode, b.path(), &supplied);
    }
    d.parameters.clear();
    for mode in MODES {
        parity(&d, &i, Some(&prior), mode, b.path(), &[]);
    }
}
