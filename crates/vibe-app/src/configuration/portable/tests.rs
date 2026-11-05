//! Real evaluated source history passes both portable codecs without source-host I/O.
#![cfg(test)]
use super::*;
use std::collections::BTreeSet;
use vibe_core::manifest::{ConfigurationDecl, ConfigurationIntentDecl};
use vibe_wire::generated::application::e2::apps as full;

fn fixture() -> (tempfile::TempDir, w::ConfigurationRecord) {
    let root = tempfile::tempdir().unwrap();
    for path in ["data", "profile-data", "chosen"] {
        std::fs::create_dir(root.path().join(path)).unwrap();
    }
    let d: ConfigurationDecl = toml::from_str(
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
[[features]]
id='z'
name='Z'
[[groups]]
mode='choice'
id='ui'
name='UI'
cardinality='zero-or-one'
members=['gate']
defaults=[]
[[groups]]
mode='choice'
id='g.quoted'
name='Dormant profile override'
cardinality='zero-or-more'
members=['a','b']
guards=[{member='gate'}]
defaults=[]
[[groups]]
mode='choice'
id='additive'
name='Dormant empty contribution'
cardinality='zero-or-more'
members=['z']
guards=[{member='gate'}]
[[groups]]
mode='closure'
id='empty-fallback'
name='Declared empty fallback'
cardinality='zero-or-more'
members=['a']
fallback=[]
[[groups]]
mode='closure'
id='no-fallback'
name='No fallback'
cardinality='zero-or-more'
members=['b']
[[parameters]]
id='flag'
name='False'
type='bool'
default=false
[[parameters]]
id='zero'
name='Zero'
type='integer'
default=0
min=-9223372036854775808
max=9223372036854775807
[[parameters]]
id='low'
name='Minimum'
type='integer'
default=-9223372036854775808
[[parameters]]
id='high'
name='Maximum'
type='integer'
default=9223372036854775807
[[parameters]]
id='empty'
name='Empty text'
type='string'
default=''
[[parameters]]
id='dir'
name='Profile path'
type='path'
must_exist=true
default='unused'
[[parameters]]
id='default-dir'
name='Default path'
type='path'
must_exist=true
default='data'
[[parameters]]
id='explicit-dir'
name='Explicit path'
type='path'
must_exist=true
[[parameters]]
id='token'
name='Optional secret'
type='string'
secret=true
default='public-authored-default'
[[profiles]]
id='p'
name='Profile'
default=true
parameters={dir='profile-data'}
selections={'g.quoted'={intent='exact',members=['a']}}
"#,
    )
    .unwrap();
    let original:ConfigurationIntentDecl=toml::from_str(r#"policy='use-declared-defaults'
profile='p'
default_features=false
selections={ui={intent='exact',members=[]},'g.quoted'={intent='exact'},additive={intent='additive',members=[]}}
parameters={explicit-dir='chosen'}
"#).unwrap();
    let input = w::ConfigurationInput {
        intent: super::super::vocabulary::source_input(&original),
        disabled_features: vec![],
    };
    let resolve = |raw: &str| root.path().join(raw);
    let result = crate::configuration::evaluate(crate::configuration::EvaluationRequest {
        history_use: crate::configuration::HistoryUse::Local,
        definition: &d,
        input: &input,
        previous: None,
        replacement: true,
        reviewed: false,
        expected_definition: None,
        fresh_parameters: &BTreeSet::new(),
        ephemeral: &[("token".into(), "SYN_PORTABLE_ANSWER".into())],
        facts: crate::HostFacts {
            resolve_path: &resolve,
            ..Default::default()
        },
        prompt: None,
    })
    .unwrap();
    assert!(
        result.record.evaluation.resolved.is_some(),
        "{:?}",
        result.record.evaluation.questions
    );
    (root, result.record)
}
fn document(history: w::PortableConfigurationHistory) -> full::AppsManifest {
    full::AppsManifest {
        protocol: "vibe-apps/2".into(),
        roots: vec!["org.example/demo@1.0.0".into()],
        exported: "2026-11-05T00:00:00Z".into(),
        home: full::AppsHome::User,
        platform: "linux".into(),
        arch: "x86_64".into(),
        application: vec![w::PortableApplication {
            key: "org.example/demo@1.0.0".into(),
            installer: "main".into(),
            platform: "linux".into(),
            arch: "x86_64".into(),
            mark: w::PortableApplicationMark::Manual,
            held: false,
            hidden: false,
            license_accepted: false,
            configuration: w::PortableConfigurationState::Configured(Box::new(
                w::PortableConfigurationStateConfigured { history },
            )),
            requires: vec![],
        }],
        collection: vec![],
        pin: vec![],
        launcher: vec![],
    }
}
#[test]
fn json_and_toml_keep_real_original_history_after_source_paths_disappear() {
    let (root, record) = fixture();
    let original = encode(&record);
    let doc = document(original.clone());
    let json = serde_json::to_string_pretty(&doc).unwrap();
    let toml = toml::to_string(&doc).unwrap();
    assert!(!json.contains("SYN_PORTABLE_ANSWER") && !toml.contains("SYN_PORTABLE_ANSWER"));
    assert!(json.contains("public-authored-default") && toml.contains("public-authored-default"));
    assert!(json.contains("-9223372036854775808") && toml.contains("9223372036854775807"));
    drop(root);
    for text in [&json, &toml] {
        let crate::PortableDocument::Full(doc) =
            crate::parse_portable_document("fixture", text).unwrap()
        else {
            panic!("full version dispatch")
        };
        assert_eq!(
            crate::PortableDocument::Full(doc.clone()).application_count(),
            1
        );

        let w::PortableConfigurationState::Configured(row) = &doc.application[0].configuration
        else {
            panic!("configured row")
        };
        assert_eq!(row.history, original);
        let history = decode(&row.history).unwrap();
        assert_eq!(history.definition, record.definition);
        assert_eq!(history.input, record.evaluation.input);
        assert!(history.input.intent.selections.contains_key("g.quoted"));
        assert!(
            history.input.intent.selections["g.quoted"]
                .members
                .is_none()
        );
        assert_eq!(
            history.input.intent.selections["additive"].members,
            Some(vec![])
        );
        for parameter in history.view().parameters {
            if let HistoricalParameter::Public {
                reusable_local_realization,
                ..
            } = parameter
            {
                assert!(reusable_local_realization.is_none());
            }
        }
    }
}
#[test]
fn source_history_references_and_reliance_have_honest_admission() {
    let (_root, record) = fixture();
    let original = encode(&record);
    for reason in ["future:group", "choice:", "parameter: "] {
        let mut h = original.clone();
        h.reliance.push(reason.into());
        assert!(decode(&h).is_err());
    }
    let mut retained = original.clone();
    retained.reliance.push("closure:previously-retained".into());
    assert!(
        decode(&retained).is_ok(),
        "known retained IDs are not target membership checks"
    );
    let mut h = original.clone();
    let p = h
        .parameters
        .iter_mut()
        .find_map(|p| match p {
            w::PortableParameterHistory::Public(p) if p.id == "dir" => Some(p),
            _ => None,
        })
        .unwrap();
    p.origin = w::ConfigurationParameterOrigin::Profile(Box::new(
        w::ConfigurationParameterOriginProfile {
            profile: "missing".into(),
        },
    ));
    assert!(decode(&h).is_err());
    let mut h = original;
    let p = h
        .parameters
        .iter_mut()
        .find_map(|p| match p {
            w::PortableParameterHistory::Public(p) if p.id == "dir" => Some(p),
            _ => None,
        })
        .unwrap();
    p.source = Some(w::ConfigurationValue::Text(Box::new(
        w::ConfigurationValueText {
            value: "other lexical source".into(),
        },
    )));
    assert!(decode(&h).is_err());
}
#[test]
fn portable_omission_does_not_relax_current_canonical_input() {
    let omitted: w::PortableConfigurationSelection =
        serde_json::from_str(r#"{"intent":"exact"}"#).unwrap();
    assert!(omitted.members.is_none());
    let alias: w::PortableConfigurationSelection =
        serde_json::from_str(r#"{"intent":"exact","members":null}"#).unwrap();
    assert_eq!(omitted, alias);
    assert!(serde_json::from_str::<w::ConfigurationSelection>(r#"{"intent":"exact"}"#).is_err());
    assert!(
        serde_json::from_str::<w::PortableConfigurationSelection>(r#"{"members":[]}"#).is_err()
    );
    assert!(
        serde_json::from_str::<w::PortableConfigurationSelection>(
            r#"{"intent":"exact","unknown":true}"#
        )
        .is_err()
    );
}
