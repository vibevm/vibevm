use super::*;
use crate::configuration::vocabulary::source_input;
fn input(text: &str) -> w::ConfigurationInput {
    w::ConfigurationInput {
        intent: source_input(&toml::from_str::<m::ConfigurationIntentDecl>(text).unwrap()),
        disabled_features: vec![],
    }
}
fn run(
    d: &m::ConfigurationDecl,
    i: &w::ConfigurationInput,
    previous: Option<&w::ConfigurationRecord>,
    replacement: bool,
    root: &std::path::Path,
) -> EvaluatedConfiguration {
    let path = |s: &str| root.join(s);
    evaluate(EvaluationRequest {
        history_use: crate::configuration::HistoryUse::Local,
        definition: d,
        input: i,
        previous: previous.map(crate::configuration::ConfigurationHistory::from),
        replacement,
        reviewed: false,
        expected_definition: None,
        fresh_parameters: &BTreeSet::new(),
        ephemeral: &[],
        facts: HostFacts {
            resolve_path: &path,
            ..Default::default()
        },
        prompt: None,
    })
    .unwrap()
}
#[test]
fn retained_path_revalidates_old_realization_after_cwd_and_validation_change() {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    std::fs::create_dir(b.path().join("data")).unwrap();
    let mut d: m::ConfigurationDecl =
        toml::from_str("[[parameters]]\nid='root'\nname='Root'\ntype='path'\ndefault='data'")
            .unwrap();
    let i = input("policy='require-selection'\ndefault_features=false");
    let first = run(&d, &i, None, true, a.path());
    assert!(first.record.evaluation.resolved.is_some());
    d.parameters[0].must_exist = true;
    let second = run(&d, &i, Some(&first.record), false, b.path());
    assert!(second.record.evaluation.resolved.is_none());
    assert!(
        second
            .record
            .evaluation
            .problems
            .iter()
            .any(|p| matches!(p, w::ConfigurationProblem::Parameter(_)))
    );
    assert!(
        second
            .record
            .evaluation
            .questions
            .iter()
            .any(|q| matches!(q, w::ConfigurationQuestion::Review(_)))
    );
    // A deliberately fresh full replacement may select B/data.
    let fresh = run(&d, &i, Some(&first.record), true, b.path());
    assert!(fresh.record.evaluation.resolved.is_some());
}
#[test]
fn explicit_answers_survive_changed_defaults_automatic_answers_request_review() {
    let dir = tempfile::tempdir().unwrap();
    let mut d: m::ConfigurationDecl =
        toml::from_str("[[parameters]]\nid='n'\nname='Number'\ntype='integer'\ndefault=1").unwrap();
    let automatic = input("policy='require-selection'\ndefault_features=false");
    let explicit = input("policy='require-selection'\ndefault_features=false\nparameters.n=7");
    let old_auto = run(&d, &automatic, None, true, dir.path());
    let old_explicit = run(&d, &explicit, None, true, dir.path());
    d.parameters[0].default = Some(m::ParameterValue::Integer(2));
    assert!(
        run(&d, &automatic, Some(&old_auto.record), false, dir.path())
            .record
            .evaluation
            .resolved
            .is_none()
    );
    assert!(
        run(&d, &explicit, Some(&old_explicit.record), false, dir.path())
            .record
            .evaluation
            .resolved
            .is_some()
    );
}
#[test]
fn typed_snapshot_retains_profile_selection_and_signed_data() {
    let d: m::ConfigurationDecl = toml::from_str(
        r#"
[[features]]
id='a'
name='A'
effects=[{kind='config-value',name='n'}]
[[features]]
id='b'
name='B'
[[groups]]
mode='choice'
id='mode'
name='Mode'
cardinality='zero-or-more'
members=['a','b']
defaults=[]
[[profiles]]
id='preset'
name='Preset'
selections.mode={intent='exact',members=[]}
[config_values]
n=-9223372036854775808
"#,
    )
    .unwrap();
    let wire = definition::definition(&d);
    let bytes = serde_json::to_vec(&wire).unwrap();
    let restored: w::ConfigurationDefinition = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(definition::read_definition(&restored).unwrap(), d);
}
#[test]
fn secret_answers_are_rejected_before_public_echo_and_resupply_is_not_equality() {
    let dir = tempfile::tempdir().unwrap();
    let d: m::ConfigurationDecl =
        toml::from_str("[[parameters]]\nid='token'\nname='Token'\ntype='string'\nsecret=true")
            .unwrap();
    let i = input("policy='require-selection'\ndefault_features=false");
    let provided = vec![("token".into(), "CANARY_SECRET".into())];
    let first = evaluate(EvaluationRequest {
        history_use: crate::configuration::HistoryUse::Local,
        definition: &d,
        input: &i,
        previous: None,
        replacement: true,
        reviewed: false,
        expected_definition: None,
        fresh_parameters: &BTreeSet::new(),
        ephemeral: &provided,
        facts: Default::default(),
        prompt: None,
    })
    .unwrap();
    assert!(
        !serde_json::to_string(&first.record)
            .unwrap()
            .contains("CANARY_SECRET")
    );
    assert_eq!(first.parameters[0].value, "CANARY_SECRET");
    let next = run(&d, &i, Some(&first.record), false, dir.path());
    assert!(next.record.evaluation.resolved.is_none());
    assert!(matches!(
        &next.record.evaluation.questions[0],
        w::ConfigurationQuestion::SecretResupply(_)
    ));
}

#[test]
fn secret_default_is_usable_but_never_substitutes_for_previously_supplied_secret() {
    let dir = tempfile::tempdir().unwrap();
    let d:m::ConfigurationDecl=toml::from_str("[[parameters]]\nid='token'\nname='Token'\ntype='string'\nsecret=true\nrequired=true\ndefault='authored-default'").unwrap();
    let i = input("policy='require-selection'\ndefault_features=false");
    let first = run(&d, &i, None, true, dir.path());
    assert!(first.record.evaluation.resolved.is_some());
    assert_eq!(first.parameters[0].value, "authored-default");
    assert!(
        matches!(&first.record.evaluation.parameters[0],w::ConfigurationParameterResult::Secret(p) if !p.provided&&matches!(p.origin,w::ConfigurationParameterOrigin::Default(_)))
    );
    let again = run(&d, &i, Some(&first.record), false, dir.path());
    assert!(again.record.evaluation.resolved.is_some());
    let supplied = evaluate(EvaluationRequest {
        history_use: crate::configuration::HistoryUse::Local,
        definition: &d,
        input: &i,
        previous: None,
        replacement: true,
        reviewed: false,
        expected_definition: None,
        fresh_parameters: &BTreeSet::new(),
        ephemeral: &[("token".into(), "CANARY_ANSWER".into())],
        facts: Default::default(),
        prompt: None,
    })
    .unwrap();
    let absent = run(&d, &i, Some(&supplied.record), false, dir.path());
    assert!(absent.record.evaluation.resolved.is_none());
    assert!(absent.parameters.is_empty());
    assert!(matches!(
        &absent.record.evaluation.questions[0],
        w::ConfigurationQuestion::SecretResupply(_)
    ));
    assert!(
        !serde_json::to_string(&supplied.record)
            .unwrap()
            .contains("CANARY_ANSWER")
    );
}
#[test]
fn closure_fallback_basis_survives_deactivation_and_explicit_override_releases_it() {
    let dir = tempfile::tempdir().unwrap();
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
id='mode'
name='Mode'
cardinality='exactly-one'
members=['a','b']
fallback=['a']
"#,
    )
    .unwrap();
    let on = input("policy='require-selection'\ndefault_features=false\nfeatures=['parent']");
    let first = run(&d, &on, None, true, dir.path());
    assert!(first.record.evaluation.resolved.is_some());
    assert_eq!(first.record.evaluation.reliance, vec!["closure:mode"]);
    let off = input("policy='require-selection'\ndefault_features=false");
    let dormant = run(&d, &off, Some(&first.record), false, dir.path());
    assert!(dormant.record.evaluation.resolved.is_some());
    assert_eq!(dormant.record.evaluation.reliance, vec!["closure:mode"]);
    let m::ConfigurationGroupDecl::Closure { fallback, .. } = &mut d.groups[0] else {
        unreachable!()
    };
    *fallback = Some(vec!["b".into()]);
    assert!(
        run(&d, &off, Some(&dormant.record), false, dir.path())
            .record
            .evaluation
            .resolved
            .is_none()
    );
    assert!(
        run(&d, &on, Some(&dormant.record), false, dir.path())
            .record
            .evaluation
            .resolved
            .is_none()
    );
    let explicit = input("policy='require-selection'\ndefault_features=false\nfeatures=['b']");
    let resolved = run(&d, &explicit, Some(&dormant.record), false, dir.path());
    assert!(resolved.record.evaluation.resolved.is_some());
    assert!(resolved.record.evaluation.reliance.is_empty());
}
#[test]
fn explicit_parameter_override_releases_its_previous_automatic_basis() {
    let dir = tempfile::tempdir().unwrap();
    let mut d: m::ConfigurationDecl =
        toml::from_str("[[parameters]]\nid='n'\nname='Number'\ntype='integer'\ndefault=1").unwrap();
    let original = input("policy='require-selection'\ndefault_features=false");
    let first = run(&d, &original, None, true, dir.path());
    d.parameters[0].default = Some(m::ParameterValue::Integer(2));
    let explicit = input("policy='require-selection'\ndefault_features=false\nparameters.n=7");
    let result = run(&d, &explicit, Some(&first.record), false, dir.path());
    assert!(result.record.evaluation.resolved.is_some());
    assert!(result.record.evaluation.reliance.is_empty());
}

#[test]
fn feature_closure_remains_inspectable_while_a_required_secret_blocks_the_operation() {
    let dir = tempfile::tempdir().unwrap();
    let d:m::ConfigurationDecl=toml::from_str("[[features]]\nid='frontend'\nname='Frontend'\ndefault=true\nrequires=['core']\n[[features]]\nid='core'\nname='Core'\n[[parameters]]\nid='token'\nname='Token'\ntype='string'\nsecret=true\nrequired=true").unwrap();
    let answer = run(
        &d,
        &input("policy='require-selection'"),
        None,
        true,
        dir.path(),
    );
    assert!(answer.record.evaluation.resolved.is_none());
    let features = answer.record.evaluation.feature_result.unwrap();
    assert_eq!(features.selected, ["frontend", "core"]);
    assert!(features.implied.iter().any(|i| i.feature == "core"
        && matches!(i.reason, w::ConfigurationImplicationReason::Requires(_))));
    assert!(
        answer
            .record
            .evaluation
            .questions
            .iter()
            .any(|q| matches!(q,w::ConfigurationQuestion::Parameter(p) if p.id=="token"))
    );
}

#[test]
fn accepted_basis_review_revalidates_prior_path_for_default_profile_and_explicit_origins() {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    std::fs::create_dir(b.path().join("data")).unwrap();
    for intent in [
        "policy='require-selection'\ndefault_features=false",
        "policy='require-selection'\ndefault_features=false\nprofile='preset'",
        "policy='require-selection'\ndefault_features=false\nparameters.root='data'",
    ] {
        let mut definition:m::ConfigurationDecl=toml::from_str("[[parameters]]\nid='root'\nname='Root'\ntype='path'\ndefault='data'\n[[profiles]]\nid='preset'\nname='Preset'\nparameters.root='data'").unwrap();
        let input = input(intent);
        let first = run(&definition, &input, None, true, a.path());
        assert!(first.record.evaluation.resolved.is_some());
        definition.parameters[0].must_exist = true;
        let path = |value: &str| b.path().join(value);
        let checked = evaluate(EvaluationRequest {
            history_use: crate::configuration::HistoryUse::Local,
            definition: &definition,
            input: &input,
            previous: Some((&first.record).into()),
            replacement: false,
            reviewed: true,
            expected_definition: None,
            fresh_parameters: &BTreeSet::new(),
            ephemeral: &[],
            facts: HostFacts {
                resolve_path: &path,
                ..Default::default()
            },
            prompt: None,
        })
        .unwrap();
        assert!(
            checked.record.evaluation.resolved.is_none(),
            "review cannot reanchor {intent}"
        );
        assert!(
            checked
                .record
                .evaluation
                .problems
                .iter()
                .any(|p| matches!(p, w::ConfigurationProblem::Parameter(_)))
        );
        assert!(
            !checked
                .record
                .evaluation
                .questions
                .iter()
                .any(|q| matches!(q, w::ConfigurationQuestion::Review(_)))
        );
    }
}

#[test]
fn removing_the_last_automatic_choice_or_parameter_requires_review_until_accepted() {
    let dir = tempfile::tempdir().unwrap();
    for parameter in [false, true] {
        let mut definition:m::ConfigurationDecl=toml::from_str(if parameter{"[[parameters]]\nid='n'\nname='Count'\ntype='integer'\ndefault=1"}else{"[[features]]\nid='a'\nname='A'\n[[groups]]\nmode='choice'\nid='g'\nname='Group'\ncardinality='exactly-one'\nmembers=['a']\ndefaults=['a']"}).unwrap();
        let input = input(if parameter {
            "policy='require-selection'\ndefault_features=false"
        } else {
            "policy='use-declared-defaults'\ndefault_features=false"
        });
        let old = run(&definition, &input, None, true, dir.path());
        assert!(old.record.evaluation.resolved.is_some());
        assert_eq!(
            old.record.evaluation.reliance,
            [if parameter { "parameter:n" } else { "choice:g" }]
        );
        if parameter {
            definition.parameters.clear();
        } else {
            definition.groups.clear();
        }
        let reused = run(&definition, &input, Some(&old.record), false, dir.path());
        assert!(
            reused.record.evaluation.resolved.is_none(),
            "deleting the last automatic input cannot erase consent history"
        );
        assert!(
            reused
                .record
                .evaluation
                .questions
                .iter()
                .any(|q| matches!(q, w::ConfigurationQuestion::Review(_)))
        );
        let accepted = evaluate(EvaluationRequest {
            history_use: crate::configuration::HistoryUse::Local,
            definition: &definition,
            input: &input,
            previous: Some((&old.record).into()),
            replacement: false,
            reviewed: true,
            expected_definition: None,
            fresh_parameters: &BTreeSet::new(),
            ephemeral: &[],
            facts: HostFacts::default(),
            prompt: None,
        })
        .unwrap();
        assert!(accepted.record.evaluation.resolved.is_some());
        assert!(accepted.record.evaluation.reliance.is_empty());
        assert!(
            run(&definition, &input, Some(&old.record), true, dir.path())
                .record
                .evaluation
                .resolved
                .is_some()
        );
    }
}

#[test]
fn hidden_required_secret_prompt_can_keep_public_default_but_never_prior_explicit_answer() {
    let definition:m::ConfigurationDecl=toml::from_str("[[parameters]]\nid='token'\nname='Token'\ntype='string'\nsecret=true\nrequired=true\ndefault='public-authored-default'").unwrap();
    let input = input("policy='require-selection'\ndefault_features=false");
    let keep = |_: &m::ParameterDecl, preset: Option<&str>| {
        assert_eq!(preset, Some("public-authored-default"));
        Ok(None)
    };
    let first = evaluate(EvaluationRequest {
        history_use: crate::configuration::HistoryUse::Local,
        definition: &definition,
        input: &input,
        previous: None,
        replacement: true,
        reviewed: false,
        expected_definition: None,
        fresh_parameters: &BTreeSet::new(),
        ephemeral: &[],
        facts: HostFacts::default(),
        prompt: Some(&keep),
    })
    .unwrap();
    assert!(first.record.evaluation.resolved.is_some());
    assert!(
        matches!(&first.record.evaluation.parameters[0],w::ConfigurationParameterResult::Secret(p) if !p.provided&&matches!(p.origin,w::ConfigurationParameterOrigin::Default(_)))
    );
    let supplied = evaluate(EvaluationRequest {
        history_use: crate::configuration::HistoryUse::Local,
        definition: &definition,
        input: &input,
        previous: None,
        replacement: true,
        reviewed: false,
        expected_definition: None,
        fresh_parameters: &BTreeSet::new(),
        ephemeral: &[("token".into(), "ephemeral-answer".into())],
        facts: HostFacts::default(),
        prompt: None,
    })
    .unwrap();
    let no_fallback = |_: &m::ParameterDecl, preset: Option<&str>| {
        assert!(preset.is_none());
        Ok(None)
    };
    let second = evaluate(EvaluationRequest {
        history_use: crate::configuration::HistoryUse::Local,
        definition: &definition,
        input: &input,
        previous: Some((&supplied.record).into()),
        replacement: false,
        reviewed: false,
        expected_definition: None,
        fresh_parameters: &BTreeSet::new(),
        ephemeral: &[],
        facts: HostFacts::default(),
        prompt: Some(&no_fallback),
    })
    .unwrap();
    assert!(second.record.evaluation.resolved.is_none());
    assert!(
        second
            .record
            .evaluation
            .questions
            .iter()
            .any(|q| matches!(q, w::ConfigurationQuestion::SecretResupply(_)))
    );
}
