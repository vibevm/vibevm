//! Ordinary parser roundtrips and real normalization, not a parallel source parser.
use super::*;
use crate::configuration::{
    ChoiceAnswer, ChoiceOutcome, ChoicePolicy, ConsumerChoices, FeatureBase, FeatureInput,
    MergeIntent, Selection,
};
use crate::manifest::Manifest;

fn header(kind: &str) -> String {
    format!("[package]\ngroup='org.example'\nname='test'\nversion='1.0.0'\nkind='{kind}'\n")
}
fn rich() -> &'static str {
    r#"
[configuration.config_values]
verbose = true
[[configuration.features]]
id = "mup"
name = "Planning"
[[configuration.features]]
id = "wal"
name = "Log"
[[configuration.features]]
id = "audit"
name = "Audit"
guards = [{member="wal"}]
effects = [{kind="config-value",name="verbose"}]
[[configuration.groups]]
mode="choice"
id="planning"
name="Planning method"
cardinality="exactly-one"
members=["mup","wal"]
defaults=["mup"]
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
[[configuration.profiles]]
id="audit profile"
name="Audited"
default=true
selections.planning={intent="exact",members=["wal"]}
selections.extras={intent="additive",members=[]}
parameters.limit=40
"#
}
fn input() -> FeatureInput {
    FeatureInput {
        base: FeatureBase::Selected(vec![]),
        additions: vec![],
        disabled: vec![],
    }
}
fn consumer(planning: &str) -> ConsumerChoices {
    ConsumerChoices {
        key: "project/dep".into(),
        policy: ChoicePolicy::RequireSelection,
        answers: BTreeMap::from([
            (
                "planning".into(),
                ChoiceAnswer {
                    intent: MergeIntent::Exact,
                    selection: Selection::Explicit(vec![planning.into()]),
                },
            ),
            (
                "extras".into(),
                ChoiceAnswer {
                    intent: MergeIntent::Additive,
                    selection: Selection::Explicit(vec!["audit".into()]),
                },
            ),
        ]),
    }
}

#[test]
fn ordinary_roundtrip_retains_all_data_and_evaluates_dormant_answers() {
    let text = header("flow")
        + rich()
        + "\n[hooks]\npost-install='hooks/done'\n[compatibility]\nmin_vibe_version='1.0.0'\n";
    let manifest = Manifest::parse_str(&text).unwrap();
    let rendered = toml::to_string(&manifest).unwrap();
    let back = Manifest::parse_str(&rendered).unwrap();
    assert_eq!(manifest, back);
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("vibe.toml");
    manifest.write(&path).unwrap();
    assert_eq!(Manifest::read(path).unwrap(), manifest);
    let d = manifest.configuration.as_ref().unwrap();
    assert_eq!(d.parameters[0].id, "limit");
    assert_eq!(
        d.profiles[0].parameters["limit"],
        ParameterValue::Integer(40)
    );
    let model = d.compile_choices().unwrap();
    for (planning, expected) in [("mup", vec!["mup"]), ("wal", vec!["wal", "audit"])] {
        let c = consumer(planning);
        let result = model.evaluate(&input(), std::slice::from_ref(&c));
        assert_eq!(result.original, [c]);
        let ChoiceOutcome::Resolved { features } = result.outcome else {
            panic!("{result:?}")
        };
        assert_eq!(features.selected, expected);
    }
}
#[test]
fn scopes_and_legacy_are_distinct_and_never_merged() {
    let legacy =
        Manifest::parse_str(&(header("flow") + "[features]\ndefault=['a']\na=[]\n")).unwrap();
    assert!(legacy.configuration.is_none());
    assert!(
        Manifest::parse_str(&(header("flow") + rich() + "\n[features]\na=[]\n"))
            .unwrap_err()
            .to_string()
            .contains("conflicts")
    );
    assert!(
        Manifest::parse_str("[project]\nname='x'\nversion='1.0.0'\n[configuration]\n").is_err()
    );
    let app = header("app")
        + rich()
        + r#"
[application]
id='test'
[[installer_options]]
id='main'
name='Main'
platform='linux'
platform_name='Linux'
arch='x86_64'
result={external='/tmp/test'}
[[installer_options.configuration.features]]
id='mup'
name='Different installer meaning'
[[installer_options.configuration.parameters]]
id='flag'
name='Flag'
type='bool'
[[installer_options.steps]]
id='setup'
title='Setup'
kind='shell'
command='true'
when=[{feature='mup'},{param='flag'}]
"#;
    let m = Manifest::parse_str(&app).unwrap();
    assert_ne!(
        m.configuration.as_ref().unwrap().features[0].name,
        m.installer_options[0]
            .configuration
            .as_ref()
            .unwrap()
            .features[0]
            .name
    );
    assert_eq!(
        Manifest::parse_str(&toml::to_string(&m).unwrap()).unwrap(),
        m
    );
    assert!(
        Manifest::parse_str(&(app + "\n[[installer_options.features]]\nid='old'\nname='Old'\n"))
            .unwrap_err()
            .to_string()
            .contains("conflicts")
    );
}
#[test]
fn all_nine_kinds_share_data_without_acquiring_effect_authority() {
    for kind in [
        "flow", "feat", "stack", "tool", "mcp", "lang", "app", "doc", "remap",
    ] {
        let extra = match kind {
            "mcp" => {
                "\n[[mcp_server]]\nname='server'\nurl='https://example.com/mcp'\ntransport='streamable-http'\n"
            }
            "doc" => "\n[[documents]]\npackage='org.example/subject'\nversion='*'\n",
            "remap" => "\n[[remap]]\nregistry='vibespecs'\nurl='https://example.com/registry'\n",
            _ => "",
        };
        let card = if kind == "doc" {
            "title='Test manual'\nabstract='A manual explaining the example workflow and its supported options.'\n"
        } else {
            ""
        };
        let text = header(kind) + card + rich() + extra;
        let m = Manifest::parse_str(&text).unwrap_or_else(|e| panic!("{kind}: {e}"));
        assert!(m.configuration.unwrap().compile_choices().is_ok());
        if matches!(kind, "doc" | "remap") {
            let bad = text.replace(
                "kind=\"config-value\",name=\"verbose\"",
                "kind=\"subskill\",path=\"audit\"",
            );
            assert!(
                Manifest::parse_str(&bad)
                    .unwrap_err()
                    .to_string()
                    .contains("data effects only")
            );
        }
    }
}
#[test]
fn closed_vocabulary_and_full_dormant_validation() {
    for replace in [
        rich().replace("{member=\"wal\"}", "{member=\"absent\"}"),
        rich().replace("cardinality=\"zero-or-more\"", "cardinality=\"all\""),
        rich().replace("mode=\"choice\"", "mode=\"closure\""),
        rich().replace("name=\"verbose\"", "name=\"missing\""),
        rich().replace("type=\"integer\"", "type=\"string\""),
        rich().replace("parameters.limit=40", "parameters.limit='40'"),
        rich().replace("parameters.limit=40", "parameters.unknown=40"),
        rich().replace("default=20", "default=20\nsecret=true"),
        rich().replace("members=[]", "members=['missing']"),
        rich().replace("name = \"Audit\"", "name = \"Audit\"\nunknown=true"),
    ] {
        assert!(
            Manifest::parse_str(&(header("flow") + &replace)).is_err(),
            "{replace}"
        );
    }
    let omitted = rich().replace("members=[]", "");
    let m = Manifest::parse_str(&(header("flow") + &omitted)).unwrap();
    assert!(
        m.configuration.unwrap().profiles[0].selections["extras"]
            .members
            .is_none()
    );
}
#[test]
fn effect_targets_bind_real_requires_across_sources_and_paths_are_checked() {
    for declaration in [
        "version='^1'",
        "git='https://example.com/repo',rev='abc'",
        "path='../dep'",
        "version.var='dep-version'",
    ] {
        let text=header("flow")+&format!("[requires.packages]\n'flow:org.example/dep'={{{declaration}}}\n")+&rich().replace("kind=\"config-value\",name=\"verbose\"","kind=\"forward\",target=\"org.example/dep\",feature=\"target feature\",strength=\"weak\"");
        let m = Manifest::parse_str(&text).unwrap();
        let back = Manifest::parse_str(&toml::to_string(&m).unwrap()).unwrap();
        assert_eq!(m, back);
    }
    for effect in [
        "kind='dependency',target='unknown'",
        "kind='dependency',target='org.example/missing'",
        "kind='subskill',path='../escape'",
        "kind='subskill',path='/escape'",
        "kind='forward',target='org.example/missing',feature='a',strength='maybe'",
        "kind='config-value',name='missing'",
    ] {
        let text = rich().replace("kind=\"config-value\",name=\"verbose\"", effect);
        assert!(
            Manifest::parse_str(&(header("flow") + &text)).is_err(),
            "{effect}"
        );
    }
}
#[test]
fn cardinalities_literal_ids_and_partial_profiles_use_shared_model() {
    for (bound, empty_ok) in [
        ("zero-or-more", true),
        ("one-or-more", false),
        ("zero-or-one", true),
        ("exactly-one", false),
    ] {
        let text = header("flow")
            + &format!(
                "[[configuration.features]]\nid='with space'\nname='Feature'\n[[configuration.groups]]\nmode='choice'\nid='group name'\nname='Group'\ncardinality='{bound}'\nmembers=['with space']\n[[configuration.profiles]]\nid='incomplete'\nname='Partial'\n"
            );
        let m = Manifest::parse_str(&text).unwrap();
        let model = m.configuration.unwrap().compile_choices().unwrap();
        assert_eq!(
            matches!(
                model.evaluate(&input(), &[]).outcome,
                ChoiceOutcome::Resolved { .. }
            ),
            empty_ok
        );
    }
}

#[test]
fn installer_effects_cannot_bind_parent_requires_or_escape_their_scope() {
    let text = header("app")
        + r#"
[application]
id="test"
[requires.packages]
"org.example/dep"="^1"
[[installer_options]]
id="main"
name="Main"
platform="linux"
platform_name="Linux"
arch="x86_64"
result={external="/tmp/test"}
[[installer_options.configuration.features]]
id="_detail"
name="Detail"
effects=[{kind="dependency",target="org.example/dep"}]
[[installer_options.steps]]
id="setup"
title="Setup"
kind="shell"
command="true"
"#;
    let failure = Manifest::parse_str(&text).unwrap_err();
    assert_eq!(
        failure.manifest_field().unwrap().to_string(),
        "installer_options[0].configuration.features[0].effects[0]"
    );
    let error = failure.to_string();
    assert!(error.contains("data effects only"), "{error}");
    let data = text.replace(
        "effects=[{kind=\"dependency\",target=\"org.example/dep\"}]",
        "",
    );
    assert!(
        Manifest::parse_str(&data).unwrap().installer_options[0]
            .configuration
            .as_ref()
            .unwrap()
            .compile_choices()
            .is_ok()
    );
}

#[test]
fn closure_groups_preserve_their_separate_fallback_meaning_and_data_is_strict() {
    let text = header("flow")
        + r#"
[[configuration.features]]
id="a"
name="A"
[[configuration.groups]]
mode="closure"
id="g"
name="Group"
cardinality="exactly-one"
members=["a"]
fallback=["a"]
"#;
    let m = Manifest::parse_str(&text).unwrap();
    let c = m.configuration.unwrap();
    let result = c.compile_choices().unwrap().evaluate(&input(), &[]);
    let ChoiceOutcome::Resolved { features } = result.outcome else {
        panic!("{result:?}")
    };
    assert_eq!(features.selected, ["a"]);
    for suffix in ["defaults=['a']", "guards=[]"] {
        assert!(Manifest::parse_str(&(text.clone() + suffix)).is_err());
    }
    assert!(
        Manifest::parse_str(&(header("flow") + "[configuration.config_values]\nx=1.5\n")).is_err()
    );
    let unknown_guard = rich().replace("{member=\"wal\"}", "{member=\"wal\",unknown=true}");
    assert!(Manifest::parse_str(&(header("flow") + &unknown_guard)).is_err());
}

#[test]
fn profile_default_is_retained_but_not_automatically_applied() {
    let m = Manifest::parse_str(&(header("flow") + rich())).unwrap();
    let d = m.configuration.unwrap();
    assert!(d.profiles[0].default);
    let model = d.compile_choices().unwrap();
    let evaluation = model.evaluate(&input(), &[]);
    assert!(matches!(evaluation.outcome, ChoiceOutcome::Unresolved));
    assert_eq!(evaluation.questions[0].group, "planning");
    let extra = "\n[[configuration.profiles]]\nid='another'\nname='Another'\ndefault=true\n";
    assert!(
        Manifest::parse_str(&(header("flow") + rich() + extra))
            .unwrap_err()
            .to_string()
            .contains("at most one profile")
    );
}

#[test]
fn invalid_profile_preset_names_its_real_authored_field() {
    for value in ["'forty'", "101"] {
        let text = header("flow")
            + &rich().replace("parameters.limit=40", &format!("parameters.limit={value}"));
        let error = Manifest::parse_str(&text).unwrap_err();
        assert_eq!(
            error.manifest_field().unwrap().to_string(),
            "configuration.profiles[0].parameters.limit",
            "{error}"
        );
    }
}

#[test]
fn common_definition_retains_the_existing_installation_shorthand() {
    let text = header("app")
        + r#"
[application]
id="test"
[[installer_options]]
id="main"
name="Main"
platform="linux"
platform_name="Linux"
arch="x86_64"
[installer_options.installation]
type="shell"
app_root="/tmp/test"
install="true"
force_install="true"
uninstall="true"
force_uninstall="true"
[[installer_options.configuration.parameters]]
id="flag"
name="Flag"
type="bool"
"#;
    let m = Manifest::parse_str(&text).unwrap();
    assert!(m.installer_options[0].installation.is_some());
    assert_eq!(
        Manifest::parse_str(&toml::to_string(&m).unwrap()).unwrap(),
        m
    );
}
