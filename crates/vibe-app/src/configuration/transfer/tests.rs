//! Real A evaluation → portable history → B preparation; no fabricated readiness.
#![cfg(test)]

use super::{TransferRequest, prepare};
use crate::HostFacts;
use crate::configuration::{
    ConfigurationHistory, EvaluatedConfiguration, EvaluationRequest, HistoryUse, evaluate,
    portable, vocabulary,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use vibe_core::manifest::{ConfigurationDecl, ConfigurationIntentDecl, ParameterValue};
use vibe_wire::generated::shared as w;

fn input(text: &str) -> w::ConfigurationInput {
    w::ConfigurationInput {
        intent: vocabulary::source_input(&toml::from_str::<ConfigurationIntentDecl>(text).unwrap()),
        disabled_features: vec![],
    }
}

fn source(
    definition: &ConfigurationDecl,
    input: &w::ConfigurationInput,
    root: &Path,
    ephemeral: &[(String, String)],
) -> w::ConfigurationRecord {
    let resolve = |raw: &str| root.join(raw);
    let result = evaluate(EvaluationRequest {
        definition,
        input,
        previous: None,
        history_use: HistoryUse::Local,
        replacement: true,
        reviewed: false,
        expected_definition: None,
        fresh_parameters: &BTreeSet::new(),
        ephemeral,
        facts: HostFacts {
            resolve_path: &resolve,
            ..Default::default()
        },
        prompt: None,
    })
    .unwrap();
    assert!(
        result.record.evaluation.resolved.is_some(),
        "{:?}",
        result.record.evaluation
    );
    assert!(result.record.evaluation.questions.is_empty());
    assert!(result.record.evaluation.problems.is_empty());
    result.record
}

fn target() -> w::PortableTargetInput {
    w::PortableTargetInput {
        path_bindings: BTreeMap::new(),
        installer: None,
        review_definition: None,
        configuration: None,
    }
}

fn forbidden_resolution(_: &str) -> PathBuf {
    panic!("transfer/local reuse must not resolve source-host or current-cwd paths")
}

fn transfer(
    definition: &ConfigurationDecl,
    history: &w::PortableConfigurationHistory,
    target: &w::PortableTargetInput,
    ephemeral: &[(String, String)],
) -> Result<EvaluatedConfiguration, String> {
    prepare(TransferRequest {
        definition,
        history,
        target,
        ephemeral,
        facts: HostFacts {
            resolve_path: &forbidden_resolution,
            ..Default::default()
        },
        prompt: None,
    })
}

fn public<'a>(
    record: &'a w::ConfigurationRecord,
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

fn text(value: &str) -> w::ConfigurationValue {
    w::ConfigurationValue::Text(Box::new(w::ConfigurationValueText {
        value: value.into(),
    }))
}

fn path_fixture() -> (tempfile::TempDir, ConfigurationDecl, w::ConfigurationRecord) {
    let a = tempfile::tempdir().unwrap();
    for path in ["default-source", "profile-source", "explicit-source"] {
        std::fs::create_dir(a.path().join(path)).unwrap();
    }
    let definition: ConfigurationDecl = toml::from_str(
        r#"
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
default='unused-default'
must_exist=true
[[parameters]]
id='explicit-dir'
name='Explicit directory'
type='path'
must_exist=true
[[parameters]]
id='absent-dir'
name='Absent optional path'
type='path'
[[parameters]]
id='caption'
name='Public nonpath'
type='string'
default='public caption'
[[parameters]]
id='secret-dir'
name='Optional secret path'
type='path'
secret=true
[[profiles]]
id='selected'
name='Selected profile'
parameters={profile-dir='profile-source'}
"#,
    )
    .unwrap();
    let original = input(
        "policy='require-selection'\ndefault_features=false\nprofile='selected'\nparameters.explicit-dir='explicit-source'",
    );
    let record = source(&definition, &original, a.path(), &[]);
    (a, definition, record)
}

fn bound_target(b: &Path) -> w::PortableTargetInput {
    let mut target = target();
    for id in ["default-dir", "profile-dir", "explicit-dir"] {
        let path = b.join(id);
        std::fs::create_dir(&path).unwrap();
        target
            .path_bindings
            .insert(id.into(), path.to_str().unwrap().into());
    }
    target
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-TARGET-INPUT")]
fn absent_source_host_rebinds_all_origins_then_local_maintenance_keeps_b() {
    let (a, definition, original) = path_fixture();
    let history = portable::encode(&original);
    let old_root = a.path().to_owned();
    a.close().unwrap();
    assert!(!old_root.exists());
    let b = tempfile::tempdir().unwrap();
    let target = bound_target(b.path());
    let prepared = transfer(&definition, &history, &target, &[]).unwrap();
    assert!(prepared.record.evaluation.resolved.is_some());
    assert_eq!(prepared.record.evaluation.input, original.evaluation.input);
    for (id, lexical) in [
        ("default-dir", "default-source"),
        ("profile-dir", "profile-source"),
        ("explicit-dir", "explicit-source"),
    ] {
        let prior = public(&original, id);
        let current = public(&prepared.record, id);
        assert_eq!(current.origin, prior.origin);
        assert_eq!(current.source, Some(text(lexical)));
        assert_eq!(
            current.value,
            Some(text(target.path_bindings.get(id).unwrap()))
        );
        assert_ne!(current.value, prior.value);
    }
    assert!(matches!(
        public(&prepared.record, "default-dir").origin,
        w::ConfigurationParameterOrigin::Default(_)
    ));
    assert!(matches!(
        public(&prepared.record, "profile-dir").origin,
        w::ConfigurationParameterOrigin::Profile(_)
    ));
    assert!(matches!(
        public(&prepared.record, "explicit-dir").origin,
        w::ConfigurationParameterOrigin::Explicit(_)
    ));
    let elsewhere = tempfile::tempdir().unwrap();
    for path in ["default-source", "profile-source", "explicit-source"] {
        std::fs::create_dir(elsewhere.path().join(path)).unwrap();
    }
    let resolutions = std::cell::Cell::new(0);
    let different_resolver = |raw: &str| {
        resolutions.set(resolutions.get() + 1);
        elsewhere.path().join(raw)
    };
    let maintained = evaluate(EvaluationRequest {
        definition: &definition,
        input: &prepared.record.evaluation.input,
        previous: Some(ConfigurationHistory::from(&prepared.record)),
        history_use: HistoryUse::Local,
        replacement: false,
        reviewed: false,
        expected_definition: None,
        fresh_parameters: &BTreeSet::new(),
        ephemeral: &[],
        facts: HostFacts {
            resolve_path: &different_resolver,
            ..Default::default()
        },
        prompt: None,
    })
    .unwrap();
    assert!(maintained.record.evaluation.resolved.is_some());
    assert_eq!(resolutions.get(), 0);
    assert_eq!(
        maintained.record.evaluation.parameters,
        prepared.record.evaluation.parameters
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-TARGET-INPUT")]
fn even_optional_nonexistent_path_needs_explicit_target_binding() {
    let a = tempfile::tempdir().unwrap();
    let definition:ConfigurationDecl=toml::from_str("[[parameters]]\nid='dir'\nname='Directory'\ntype='path'\ndefault='not-present'\nmust_exist=false").unwrap();
    let original = input("policy='require-selection'\ndefault_features=false");
    let history = portable::encode(&source(&definition, &original, a.path(), &[]));
    a.close().unwrap();
    let missing = transfer(&definition, &history, &target(), &[]).unwrap();
    assert!(missing.record.evaluation.resolved.is_none());
    assert!(
        missing
            .record
            .evaluation
            .questions
            .iter()
            .any(|q| matches!(q,w::ConfigurationQuestion::Parameter(p)if p.id=="dir"&&!p.secret))
    );
    assert!(public(&missing.record, "dir").value.is_none());
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-TARGET-INPUT")]
fn target_binding_validation_refuses_relative_missing_unknown_nonpath_secret_and_absent() {
    let (a, definition, original) = path_fixture();
    let history = portable::encode(&original);
    a.close().unwrap();
    let b = tempfile::tempdir().unwrap();
    let valid = bound_target(b.path());
    for value in [
        "relative-path".to_string(),
        b.path().join("missing").to_str().unwrap().into(),
    ] {
        let mut invalid = valid.clone();
        invalid.path_bindings.insert("default-dir".into(), value);
        let result = transfer(&definition, &history, &invalid, &[]).unwrap();
        assert!(result.record.evaluation.resolved.is_none());
        assert!(
            result
                .record
                .evaluation
                .problems
                .iter()
                .any(|p| matches!(p,w::ConfigurationProblem::Parameter(p)if p.id=="default-dir"))
        );
        assert!(public(&result.record, "default-dir").value.is_none());
    }
    for id in ["unknown-dir", "caption", "secret-dir"] {
        let mut invalid = valid.clone();
        invalid
            .path_bindings
            .insert(id.into(), "PRIVATE_BINDING_CANARY".into());
        let error = transfer(&definition, &history, &invalid, &[]).unwrap_err();
        assert!(error.contains(id));
        assert!(!error.contains("PRIVATE_BINDING_CANARY"));
    }
    let mut absent = valid;
    absent
        .path_bindings
        .insert("absent-dir".into(), b.path().to_str().unwrap().into());
    let result = transfer(&definition, &history, &absent, &[]).unwrap();
    assert!(result.record.evaluation.resolved.is_none());
    assert!(
        result
            .record
            .evaluation
            .problems
            .iter()
            .any(|p| matches!(p,w::ConfigurationProblem::Parameter(p)if p.id=="absent-dir"))
    );
    let absent = public(&result.record, "absent-dir");
    assert!(absent.source.is_none() && absent.value.is_none());
    assert!(matches!(
        absent.origin,
        w::ConfigurationParameterOrigin::Absent(_)
    ));
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-TARGET-INPUT")]
fn changed_automatic_definition_needs_current_review_or_bound_replacement() {
    let a = tempfile::tempdir().unwrap();
    let mut definition: ConfigurationDecl =
        toml::from_str("[[parameters]]\nid='number'\nname='Number'\ntype='integer'\ndefault=1")
            .unwrap();
    let original = input("policy='require-selection'\ndefault_features=false");
    let history = portable::encode(&source(&definition, &original, a.path(), &[]));
    let old_fingerprint = history.fingerprint.clone();
    definition.parameters[0].default = Some(ParameterValue::Integer(2));
    let current = definition.semantic_fingerprint().unwrap();
    let unreviewed = transfer(&definition, &history, &target(), &[]).unwrap();
    assert!(unreviewed.record.evaluation.resolved.is_none());
    assert!(
        unreviewed
            .record
            .evaluation
            .questions
            .iter()
            .any(|q| matches!(q, w::ConfigurationQuestion::Review(_)))
    );
    let mut reviewed = target();
    reviewed.review_definition = Some(old_fingerprint.clone());
    assert!(
        transfer(&definition, &history, &reviewed, &[])
            .unwrap_err()
            .contains("exact current")
    );
    reviewed.review_definition = Some(current.clone());
    let ready = transfer(&definition, &history, &reviewed, &[]).unwrap();
    assert!(ready.record.evaluation.resolved.is_some());
    assert!(
        matches!(public(&ready.record,"number").value.as_ref(),Some(w::ConfigurationValue::Integer(v))if v.value=="2")
    );
    let mut replaced = target();
    replaced.configuration = Some(w::PortableTargetReplacement {
        protocol: "vibe-configuration-input/1".into(),
        expected_definition: old_fingerprint,
        input: input("policy='require-selection'\ndefault_features=false\nparameters.number=7"),
    });
    assert!(
        transfer(&definition, &history, &replaced, &[])
            .unwrap_err()
            .contains("exact current")
    );
    replaced.configuration.as_mut().unwrap().expected_definition = current;
    let chosen = transfer(&definition, &history, &replaced, &[]).unwrap();
    assert!(chosen.record.evaluation.resolved.is_some());
    assert!(
        matches!(public(&chosen.record,"number").value.as_ref(),Some(w::ConfigurationValue::Integer(v))if v.value=="7")
    );
    assert!(matches!(
        public(&chosen.record, "number").origin,
        w::ConfigurationParameterOrigin::Explicit(_)
    ));
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-TARGET-INPUT")]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-SECRET-INPUT")]
fn optional_explicit_secret_requires_resupply_even_with_public_target_replacement() {
    const A: &str = "SECRET_RUNTIME_A_7351";
    const B: &str = "SECRET_RUNTIME_B_9824";
    let a = tempfile::tempdir().unwrap();
    let definition:ConfigurationDecl=toml::from_str("[[parameters]]\nid='token'\nname='Optional token'\ntype='string'\nsecret=true\ndefault='public-authored-fallback'\n[[parameters]]\nid='caption'\nname='Caption'\ntype='string'\ndefault='A public'").unwrap();
    assert!(!definition.parameters[0].required);
    let original = input("policy='require-selection'\ndefault_features=false");
    let source = source(
        &definition,
        &original,
        a.path(),
        &[("token".into(), A.into())],
    );
    let history = portable::encode(&source);
    let mut target = target();
    target.configuration = Some(w::PortableTargetReplacement {
        protocol: "vibe-configuration-input/1".into(),
        expected_definition: definition.semantic_fingerprint().unwrap(),
        input: input(
            "policy='require-selection'\ndefault_features=false\nparameters.caption='B public'",
        ),
    });
    let waiting = transfer(&definition, &history, &target, &[]).unwrap();
    assert!(waiting.record.evaluation.resolved.is_none());
    assert!(
        waiting
            .record
            .evaluation
            .questions
            .iter()
            .any(|q| matches!(q,w::ConfigurationQuestion::SecretResupply(p)if p.id=="token"))
    );
    assert!(
        !waiting.parameters.iter().any(|p| p.id == "token"),
        "public default cannot satisfy prior explicit provision"
    );
    let supplied = transfer(
        &definition,
        &history,
        &target,
        &[("token".into(), B.into())],
    )
    .unwrap();
    assert!(supplied.record.evaluation.resolved.is_some());
    assert!(
        supplied
            .parameters
            .iter()
            .any(|p| p.id == "token" && p.secret && p.value == B)
    );
    assert!(
        matches!(public(&supplied.record,"caption").source.as_ref(),Some(w::ConfigurationValue::Text(t))if t.value=="B public")
    );
    for value in [
        serde_json::to_string(&source).unwrap(),
        serde_json::to_string(&history).unwrap(),
        serde_json::to_string(&waiting.record).unwrap(),
        serde_json::to_string(&supplied.record).unwrap(),
        format!("{waiting:?}"),
        format!("{supplied:?}"),
    ] {
        assert!(!value.contains(A) && !value.contains(B));
    }
    assert!(
        serde_json::to_string(&history)
            .unwrap()
            .contains("public-authored-fallback")
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-TARGET-INPUT")]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-SECRET-INPUT")]
fn public_to_secret_reclassification_and_ephemeral_public_inputs_refuse_without_values() {
    const PUBLIC: &str = "HISTORICAL_PUBLIC_CANARY_4027";
    const EPHEMERAL: &str = "EPHEMERAL_WRONG_SCOPE_7853";
    let a = tempfile::tempdir().unwrap();
    let mut definition: ConfigurationDecl =
        toml::from_str("[[parameters]]\nid='caption'\nname='Caption'\ntype='string'").unwrap();
    let original = input(&format!(
        "policy='require-selection'\ndefault_features=false\nparameters.caption='{PUBLIC}'"
    ));
    let history = portable::encode(&source(&definition, &original, a.path(), &[]));
    for id in ["caption", "unknown"] {
        let error = transfer(
            &definition,
            &history,
            &target(),
            &[(id.into(), EPHEMERAL.into())],
        )
        .unwrap_err();
        assert!(!error.contains(PUBLIC) && !error.contains(EPHEMERAL));
        assert!(error.contains("not a declared secret"));
    }
    definition.parameters[0].secret = true;
    let mut reviewed = target();
    reviewed.review_definition = Some(definition.semantic_fingerprint().unwrap());
    let error = transfer(
        &definition,
        &history,
        &reviewed,
        &[("caption".into(), EPHEMERAL.into())],
    )
    .unwrap_err();
    assert!(error.contains("historically public"));
    assert!(!error.contains(PUBLIC) && !error.contains(EPHEMERAL));
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-TARGET-INPUT")]
fn target_preparation_retains_dormant_omitted_and_explicit_empty_answers() {
    let a = tempfile::tempdir().unwrap();
    let definition: ConfigurationDecl = toml::from_str(
        r#"
[[features]]
id='gate'
name='Gate'
[[features]]
id='a'
name='A'
[[features]]
id='b'
name='B'
[[groups]]
mode='choice'
id='switch'
name='Switch'
cardinality='zero-or-one'
members=['gate']
defaults=[]
[[groups]]
mode='choice'
id='dormant'
name='Dormant omitted answer'
cardinality='zero-or-more'
members=['a']
guards=[{member='gate'}]
defaults=[]
[[groups]]
mode='choice'
id='extra'
name='Dormant additive empty answer'
cardinality='zero-or-more'
members=['b']
guards=[{member='gate'}]
defaults=[]
"#,
    )
    .unwrap();
    let original = input(
        "policy='use-declared-defaults'\ndefault_features=false\nselections={switch={intent='exact',members=[]},dormant={intent='exact'},extra={intent='additive',members=[]}}",
    );
    let record = source(&definition, &original, a.path(), &[]);
    let history = portable::encode(&record);
    a.close().unwrap();
    let prepared = transfer(&definition, &history, &target(), &[]).unwrap();
    assert!(prepared.record.evaluation.resolved.is_some());
    assert_eq!(prepared.record.evaluation.input, original);
    let selections = &prepared.record.evaluation.input.intent.selections;
    assert_eq!(selections.len(), 3);
    assert!(selections["dormant"].members.is_none());
    assert_eq!(selections["extra"].members, Some(vec![]));
    assert_eq!(selections["extra"].intent, w::ConfigurationIntent::Additive);
    assert_eq!(selections["switch"].members, Some(vec![]));
}
