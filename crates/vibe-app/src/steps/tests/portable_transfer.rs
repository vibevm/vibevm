//! Transported history reaches the ordinary planner without becoming fresh input.
#![cfg(test)]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-TARGET-INPUT");

use super::*;
use crate::configuration::{
    ConfigurationRecord, ConfigurationSubmission, EvaluationRequest, HistoryUse, PortableTransfer,
    evaluate, parse_submission, portable,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use vibe_core::manifest::ConfigurationDecl;
use vibe_wire::generated::shared as w;

fn fixture() -> (
    tempfile::TempDir,
    ResolvedPackage,
    ConfigurationRecord,
    PortableTransfer,
) {
    let a = tempfile::tempdir().unwrap();
    for name in ["default-source", "profile-source", "explicit-source"] {
        fs::create_dir(a.path().join(name)).unwrap();
    }
    let definition: ConfigurationDecl = toml::from_str(
        r#"
[[features]]
id='chosen'
name='Chosen'
[[parameters]]
id='default-dir'
name='Default directory'
type='path'
default='default-source'
must_exist=true
[[parameters]]
id='profile-dir'
name='Profile directory'
type='path'
must_exist=true
[[parameters]]
id='explicit-dir'
name='Explicit directory'
type='path'
must_exist=true
[[profiles]]
id='selected'
name='Selected profile'
features=['chosen']
parameters={profile-dir='profile-source'}
"#,
    )
    .unwrap();
    let input = parse_submission(br#"{"protocol":"vibe-configuration-input/1","input":{"intent":{"policy":"require-selection","features":[],"defaultFeatures":false,"selections":{},"profile":"selected","parameters":{"explicit-dir":{"kind":"text","value":"explicit-source"}}},"disabledFeatures":[]}}"#).unwrap().input;
    let resolve_a = |raw: &str| a.path().join(raw);
    let evaluated = evaluate(EvaluationRequest {
        definition: &definition,
        input: &input,
        previous: None,
        history_use: HistoryUse::Local,
        replacement: true,
        reviewed: false,
        expected_definition: None,
        fresh_parameters: &BTreeSet::new(),
        ephemeral: &[],
        facts: HostFacts {
            resolve_path: &resolve_a,
            ..HostFacts::default()
        },
        prompt: None,
    })
    .unwrap();
    assert!(evaluated.record.evaluation.resolved.is_some());
    assert!(evaluated.record.evaluation.questions.is_empty());
    assert!(evaluated.record.evaluation.problems.is_empty());
    let history = portable::encode(&evaluated.record);
    let old_root = a.path().to_path_buf();
    a.close().unwrap();
    assert!(!old_root.exists());

    let b = tempfile::tempdir().unwrap();
    let mut path_bindings = BTreeMap::new();
    for id in ["default-dir", "profile-dir", "explicit-dir"] {
        let path = b.path().join(id);
        fs::create_dir(&path).unwrap();
        path_bindings.insert(id.into(), path.to_str().unwrap().into());
    }
    let mut resolved = package(b.path(), BUILD);
    assert_eq!(resolved.options[0].id, "main");
    resolved.options[0].features.clear();
    resolved.options[0].configuration = Some(definition);
    let transfer = PortableTransfer {
        history,
        target: w::PortableTargetInput {
            path_bindings,
            installer: Some("main".into()),
            review_definition: None,
            configuration: None,
        },
        required_secrets: vec![],
    };
    (b, resolved, evaluated.record, transfer)
}

fn forbidden_resolution(_: &str) -> PathBuf {
    panic!("portable planner must not resolve A's lexical paths against B's cwd")
}

fn planned(
    root: &Path,
    resolved: &ResolvedPackage,
    transfer: &PortableTransfer,
    configuration: Option<&ConfigurationSubmission>,
    features: &FeatureRequest,
) -> Result<crate::Plan, AppError> {
    let settings = SettingsRoot::new(&root.join("settings"));
    plan(&PlanInput {
        import_scope: None,
        transfer: Some(transfer),
        agent_modes: None,
        configuration,
        resolved,
        option: &resolved.options[0],
        request: features,
        parameters: &[],
        accept_license: false,
        previous: None,
        settings: &settings,
        facts: HostFacts {
            resolve_path: &forbidden_resolution,
            ..HostFacts::default()
        },
        offline: true,
        attended: false,
        prompts: Prompts::default(),
        agent: AgentHooks::default(),
        requirements: None,
        required_by: None,
    })
}

fn public<'a>(
    record: &'a ConfigurationRecord,
    id: &str,
) -> &'a w::ConfigurationParameterResultPublic {
    record
        .evaluation
        .parameters
        .iter()
        .find_map(|p| match p {
            w::ConfigurationParameterResult::Public(p) if p.id == id => Some(p.as_ref()),
            _ => None,
        })
        .unwrap()
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-TARGET-INPUT")]
fn transported_source_history_reaches_target_planner_with_original_input_and_origins() {
    let (b, resolved, source, transfer) = fixture();
    let plan = planned(
        b.path(),
        &resolved,
        &transfer,
        None,
        &FeatureRequest::default(),
    )
    .unwrap();
    assert_eq!(plan.features, ["chosen"]);
    assert_eq!(plan.profile.as_deref(), Some("selected"));
    assert_eq!(plan.steps.len(), 1);
    assert!(plan.steps[0].run);
    assert!(plan.dependencies.is_empty());
    let target = plan.configuration.as_ref().unwrap();
    assert_eq!(target.evaluation.input, source.evaluation.input);
    assert_eq!(target.evaluation.reliance, source.evaluation.reliance);
    for (id, origin) in [
        ("default-dir", "default"),
        ("profile-dir", "profile"),
        ("explicit-dir", "explicit"),
    ] {
        let prior = public(&source, id);
        let after = public(target, id);
        assert_eq!(serde_json::to_value(&prior.origin).unwrap()["kind"], origin);
        assert_eq!(after.origin, prior.origin);
        assert_eq!(after.source, prior.source);
        assert_ne!(after.value, prior.value);
        assert_eq!(
            plan.parameters.iter().find(|p| p.id == id).unwrap().value,
            transfer.target.path_bindings[id]
        );
    }
    assert!(
        !b.path().join("settings").exists(),
        "planning creates no work/home"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-TARGET-INPUT")]
fn portable_planner_refuses_ordinary_configuration_or_feature_inputs() {
    let (b, resolved, source, transfer) = fixture();
    let submission = ConfigurationSubmission {
        protocol: "vibe-configuration-input/1".into(),
        input: source.evaluation.input,
        expected_definition: None,
    };
    let enabled = FeatureRequest {
        enable: vec!["chosen".into()],
        ..Default::default()
    };
    for (configuration, features) in [
        (Some(&submission), FeatureRequest::default()),
        (None, enabled),
    ] {
        let error = planned(b.path(), &resolved, &transfer, configuration, &features).unwrap_err();
        assert!(error.to_string().contains("mutually exclusive"), "{error}");
    }
    assert!(!b.path().join("settings").exists());
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-TARGET-INPUT")]
fn portable_configured_history_cannot_fall_through_a_legacy_installer() {
    let (b, mut resolved, _, transfer) = fixture();
    resolved.options[0].configuration = None;
    let error = planned(
        b.path(),
        &resolved,
        &transfer,
        None,
        &FeatureRequest::default(),
    )
    .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("no common configuration definition"),
        "{error}"
    );
    assert!(!b.path().join("settings").exists());
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-TARGET-INPUT")]
fn portable_target_installer_must_match_the_actual_planned_option() {
    let (b, resolved, _, mut transfer) = fixture();
    transfer.target.installer = Some("another-installer".into());
    let error = planned(
        b.path(),
        &resolved,
        &transfer,
        None,
        &FeatureRequest::default(),
    )
    .unwrap_err();
    assert!(
        error.to_string().contains("names another installer"),
        "{error}"
    );
    assert!(!b.path().join("settings").exists());
}
