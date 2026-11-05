//! Console callbacks preserve the same original inputs as machine submissions.
use super::*;
use crate::configuration::{ConfigurationRecord, ConfigurationSubmission};
fn configured(dir: &Path, definition: &str) -> ResolvedPackage {
    let mut r = package(dir, BUILD);
    r.options[0].features.clear();
    r.options[0].configuration = Some(toml::from_str(definition).unwrap());
    r
}
fn submission(intent: &str, negative: &str) -> ConfigurationSubmission {
    let mut intent: serde_json::Value = serde_json::from_str(intent).unwrap();
    let fields = intent.as_object_mut().unwrap();
    fields.entry("features").or_insert(serde_json::json!([]));
    fields.entry("selections").or_insert(serde_json::json!({}));
    fields.entry("parameters").or_insert(serde_json::json!({}));
    let negative: serde_json::Value = serde_json::from_str(negative).unwrap();
    crate::configuration::parse_submission(&serde_json::to_vec(&serde_json::json!({"protocol":"vibe-configuration-input/1","input":{"intent":intent,"disabledFeatures":negative}})).unwrap()).unwrap()
}
fn checked(
    r: &ResolvedPackage,
    dir: &Path,
    input: Option<&ConfigurationSubmission>,
    previous: Option<&ConfigurationRecord>,
    prompts: Prompts<'_>,
    attended: bool,
) -> crate::Plan {
    let home = SettingsRoot::new(&dir.join("settings"));
    let parameters = Default::default();
    let origins = Default::default();
    let features = [];
    plan(&PlanInput {
        import_scope: None,
        transfer: None,
        agent_modes: None,
        configuration: input,
        resolved: r,
        option: &r.options[0],
        request: &FeatureRequest::default(),
        parameters: &[],
        accept_license: false,
        previous: previous.map(|c| Previous {
            configuration: Some(c),
            secret_origins: &origins,
            features: &features,
            parameters: &parameters,
            license: None,
            ask: true,
        }),
        settings: &home,
        facts: HostFacts::default(),
        offline: true,
        attended,
        prompts,
        agent: AgentHooks::default(),
        requirements: None,
        required_by: None,
    })
    .unwrap()
}
#[test]
fn enter_preserves_retained_profile_instead_of_new_default_profile() {
    let dir = tempfile::tempdir().unwrap();
    let r = configured(
        dir.path(),
        "[[features]]\nid='chosen'\nname='Chosen'\n[[profiles]]\nid='first'\nname='Declared default'\ndefault=true\n[[profiles]]\nid='retained'\nname='Retained'\nfeatures=['chosen']",
    );
    let i = submission(
        r#"{"policy":"require-selection","defaultFeatures":false,"profile":"retained"}"#,
        "[]",
    );
    let first = checked(&r, dir.path(), Some(&i), None, Prompts::default(), false);
    let called = RefCell::new(false);
    let chooser = |prompt: &str, rows: &[crate::Choice]| {
        assert_eq!(prompt, "Configuration profile");
        assert_eq!(rows[0].id.as_deref(), Some("retained"));
        *called.borrow_mut() = true;
        Ok(Some(0))
    };
    let second = checked(
        &r,
        dir.path(),
        None,
        first.configuration.as_ref(),
        Prompts {
            chooser: Some(&chooser),
            ..Default::default()
        },
        true,
    );
    assert!(*called.borrow());
    assert_eq!(second.features, ["chosen"]);
    assert_eq!(
        second
            .configuration
            .unwrap()
            .evaluation
            .input
            .intent
            .profile
            .as_deref(),
        Some("retained")
    );
}
#[test]
fn positive_checkbox_clears_old_negative_and_preserves_unseen_dormant_group_input() {
    let dir = tempfile::tempdir().unwrap();
    let r = configured(
        dir.path(),
        "[[features]]\nid='foo'\nname='Foo'\n[[features]]\nid='gate'\nname='Gate'\n[[features]]\nid='a'\nname='A'\n[[groups]]\nmode='choice'\nid='activation'\nname='Activation'\ncardinality='zero-or-one'\nmembers=['gate']\n[[groups]]\nmode='choice'\nid='dormant'\nname='Dormant'\ncardinality='zero-or-more'\nmembers=['a']\nguards=[{member='gate'}]",
    );
    let i = submission(
        r#"{"policy":"require-selection","defaultFeatures":false,"selections":{"activation":{"intent":"exact","members":[]},"dormant":{"intent":"exact","members":["a"]}}}"#,
        r#"["foo"]"#,
    );
    let first = checked(&r, dir.path(), Some(&i), None, Prompts::default(), false);
    let chooser = |prompt: &str, rows: &[crate::Choice], selected: &[bool]| {
        if prompt == "Select features" {
            assert_eq!(rows[0].id.as_deref(), Some("foo"));
            assert!(!selected[0]);
            Ok(Some(vec![0]))
        } else {
            Ok(Some(vec![]))
        }
    };
    let second = checked(
        &r,
        dir.path(),
        None,
        first.configuration.as_ref(),
        Prompts {
            features: Some(&chooser),
            ..Default::default()
        },
        true,
    );
    assert_eq!(second.features, ["foo"]);
    let original = second.configuration.unwrap().evaluation.input;
    assert!(original.disabled_features.is_empty());
    assert_eq!(
        original.intent.selections["dormant"].members.as_deref(),
        Some(["a".to_string()].as_slice())
    );
}
#[test]
fn console_review_contains_real_before_after_parameter_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let mut r = configured(
        dir.path(),
        "[[parameters]]\nid='n'\nname='Count'\ntype='integer'\ndefault=1",
    );
    let i = submission(
        r#"{"policy":"require-selection","defaultFeatures":false}"#,
        "[]",
    );
    let first = checked(&r, dir.path(), Some(&i), None, Prompts::default(), false);
    r.options[0].configuration.as_mut().unwrap().parameters[0].default =
        Some(vibe_core::manifest::ParameterValue::Integer(2));
    let called = RefCell::new(false);
    let chooser = |prompt: &str, rows: &[crate::Choice]| {
        assert_eq!(prompt, "The automatic configuration basis changed");
        let description = rows[0].description.as_deref().unwrap();
        assert!(
            description.contains("Before:")
                && description.contains("After:")
                && description.contains("Parameter `n`")
        );
        assert!(
            description.contains("\"value\": \"1\"") && description.contains("\"value\": \"2\"")
        );
        *called.borrow_mut() = true;
        Ok(Some(0))
    };
    let second = checked(
        &r,
        dir.path(),
        None,
        first.configuration.as_ref(),
        Prompts {
            chooser: Some(&chooser),
            ..Default::default()
        },
        true,
    );
    assert!(*called.borrow());
    assert_eq!(second.parameters[0].value, "2");
}

#[test]
fn accepting_group_prefill_preserves_explicit_profile_and_reactivated_dormant_answers() {
    for case in 0..3 {
        let dir = tempfile::tempdir().unwrap();
        let extra = if case == 1 {
            "\n[[profiles]]\nid='preset'\nname='Preset'\nselections.g={intent='exact',members=['b']}"
        } else {
            ""
        };
        let guards = if case == 2 {
            "guards=[{member='gate'}]"
        } else {
            ""
        };
        let mut r = configured(
            dir.path(),
            &format!(
                "[[features]]\nid='a'\nname='A'\n[[features]]\nid='b'\nname='B'\n[[features]]\nid='gate'\nname='Gate'\n[[groups]]\nmode='choice'\nid='g'\nname='Group'\ncardinality='exactly-one'\nmembers=['a','b']\ndefaults=['a']\n{guards}\n[[groups]]\nmode='choice'\nid='activation'\nname='Activation'\ncardinality='zero-or-one'\nmembers=['gate']\ndefaults=[]\n{extra}"
            ),
        );
        let i = submission(
            if case == 1 {
                r#"{"policy":"require-selection","defaultFeatures":false,"profile":"preset","selections":{"activation":{"intent":"exact","members":[]}}}"#
            } else {
                r#"{"policy":"require-selection","defaultFeatures":false,"selections":{"activation":{"intent":"exact","members":[]},"g":{"intent":"exact","members":["b"]}}}"#
            },
            "[]",
        );
        let first = checked(&r, dir.path(), Some(&i), None, Prompts::default(), false);
        if case == 2
            && let vibe_core::manifest::ConfigurationGroupDecl::Choice { guards, .. } =
                &mut r.options[0].configuration.as_mut().unwrap().groups[0]
        {
            guards.clear();
        }
        let called = RefCell::new(false);
        let choose = |_: &str, _: &[crate::Choice]| Ok(Some(0));
        let features = |prompt: &str, rows: &[crate::Choice], selected: &[bool]| {
            if prompt == "g" {
                assert_eq!(
                    rows.iter()
                        .map(|r| r.id.as_deref().unwrap())
                        .collect::<Vec<_>>(),
                    ["a", "b"]
                );
                assert_eq!(selected, [false, true], "case {case}");
                *called.borrow_mut() = true;
            }
            Ok(Some(
                selected
                    .iter()
                    .enumerate()
                    .filter_map(|(i, s)| s.then_some(i))
                    .collect(),
            ))
        };
        let second = checked(
            &r,
            dir.path(),
            None,
            first.configuration.as_ref(),
            Prompts {
                chooser: Some(&choose),
                features: Some(&features),
                ..Default::default()
            },
            true,
        );
        assert!(*called.borrow());
        assert_eq!(second.features, ["b"]);
    }
}

#[test]
fn visible_group_edit_preserves_its_ineligible_member_and_merge_intent() {
    for intent in ["exact", "additive"] {
        let dir = tempfile::tempdir().unwrap();
        let r = configured(
            dir.path(),
            "[[features]]\nid='a'\nname='A'\n[[features]]\nid='b'\nname='B'\nguards=[{member='gate'}]\n[[features]]\nid='gate'\nname='Gate'\n[[groups]]\nmode='choice'\nid='g'\nname='Group'\ncardinality='zero-or-more'\nmembers=['a','b']\n[[groups]]\nmode='choice'\nid='activation'\nname='Activation'\ncardinality='zero-or-one'\nmembers=['gate']",
        );
        let i = submission(
            &format!(
                r#"{{"policy":"require-selection","defaultFeatures":false,"selections":{{"activation":{{"intent":"exact","members":[]}},"g":{{"intent":"{intent}","members":["a","b"]}}}}}}"#
            ),
            "[]",
        );
        let first = checked(&r, dir.path(), Some(&i), None, Prompts::default(), false);
        let chooser = |_: &str, rows: &[crate::Choice], selected: &[bool]| {
            assert!(!rows.iter().any(|r| r.id.as_deref() == Some("b")));
            Ok(Some(
                selected
                    .iter()
                    .enumerate()
                    .filter_map(|(i, v)| v.then_some(i))
                    .collect(),
            ))
        };
        let second = checked(
            &r,
            dir.path(),
            None,
            first.configuration.as_ref(),
            Prompts {
                features: Some(&chooser),
                ..Default::default()
            },
            true,
        );
        let record = second.configuration.unwrap();
        let selection = &record.evaluation.input.intent.selections["g"];
        assert_eq!(
            selection.members.as_ref().unwrap(),
            &vec!["a".to_string(), "b".to_string()]
        );
        assert_eq!(serde_json::to_value(&selection.intent).unwrap(), intent);
        let mut third = ConfigurationSubmission {
            protocol: "vibe-configuration-input/1".into(),
            input: record.evaluation.input,
            expected_definition: None,
        };
        third
            .input
            .intent
            .selections
            .get_mut("activation")
            .unwrap()
            .members = Some(vec!["gate".into()]);
        let third = checked(
            &r,
            dir.path(),
            Some(&third),
            None,
            Prompts::default(),
            false,
        );
        assert_eq!(third.features, ["a", "b", "gate"]);
    }
}

#[test]
fn explicit_omitted_group_does_not_restore_a_suppressed_profile_hidden_member() {
    let dir = tempfile::tempdir().unwrap();
    let r = configured(
        dir.path(),
        "[[features]]\nid='a'\nname='A'\n[[features]]\nid='b'\nname='B'\nguards=[{member='gate'}]\n[[features]]\nid='gate'\nname='Gate'\n[[groups]]\nmode='choice'\nid='g'\nname='Group'\ncardinality='zero-or-more'\nmembers=['a','b']\ndefaults=['a']\n[[groups]]\nmode='choice'\nid='activation'\nname='Activation'\ncardinality='zero-or-one'\nmembers=['gate']\n[[profiles]]\nid='p'\nname='Preset'\nselections.g={intent='exact',members=['b']}",
    );
    let i = submission(
        r#"{"policy":"use-declared-defaults","defaultFeatures":false,"profile":"p","selections":{"activation":{"intent":"exact","members":[]},"g":{"intent":"exact","members":null}}}"#,
        "[]",
    );
    let first = checked(&r, dir.path(), Some(&i), None, Prompts::default(), false);
    let choose = |_: &str, _: &[crate::Choice]| Ok(Some(0));
    let features = |_: &str, _: &[crate::Choice], selected: &[bool]| {
        Ok(Some(
            selected
                .iter()
                .enumerate()
                .filter_map(|(i, v)| v.then_some(i))
                .collect(),
        ))
    };
    let next = checked(
        &r,
        dir.path(),
        None,
        first.configuration.as_ref(),
        Prompts {
            chooser: Some(&choose),
            features: Some(&features),
            ..Default::default()
        },
        true,
    );
    assert_eq!(
        next.configuration
            .unwrap()
            .evaluation
            .input
            .intent
            .selections["g"]
            .members,
        Some(vec!["a".into()])
    );
}
