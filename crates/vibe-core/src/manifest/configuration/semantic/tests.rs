//! Frozen recipe and semantic-versus-presentation discrimination.
use crate::manifest::ConfigurationDecl;

#[test]
fn presentation_and_input_order_do_not_change_the_semantic_basis() {
    let text = "[[features]]\nid='a'\nname='A'\nrequires=['b']\n[[features]]\nid='b'\nname='B'\n[[profiles]]\nid='p'\nname='P'\nfeatures=['a']\n";
    let mut d: ConfigurationDecl = toml::from_str(text).unwrap();
    let first = d.semantic_fingerprint().unwrap();
    d.features.reverse();
    d.features[0].name = "Renamed".into();
    d.profiles[0].default = true;
    d.profiles[0].description = Some("Presentation only".into());
    assert_eq!(d.semantic_fingerprint().unwrap(), first);
    d.features[0].default = true;
    assert_ne!(d.semantic_fingerprint().unwrap(), first);
}

#[test]
fn absent_and_empty_defaults_are_semantic() {
    let mut d:ConfigurationDecl=toml::from_str("[[features]]\nid='a'\nname='A'\n[[groups]]\nmode='choice'\nid='g'\nname='G'\ncardinality='zero-or-more'\nmembers=['a']\n").unwrap();
    let absent = d.semantic_fingerprint().unwrap();
    if let crate::manifest::ConfigurationGroupDecl::Choice { defaults, .. } = &mut d.groups[0] {
        *defaults = Some(vec![]);
    }
    assert_ne!(d.semantic_fingerprint().unwrap(), absent);
}

#[test]
fn empty_recipe_has_a_frozen_digest() {
    assert_eq!(
        ConfigurationDecl::default().semantic_fingerprint().unwrap(),
        "configuration-semantic/1:sha256:fdad2195df9b77d1fd565e12617971acaac54cf29d1bd2059f19c471e204e7e8"
    );
}

fn nonempty() -> ConfigurationDecl {
    toml::from_str(
        r#"
[[features]]
id='a'
name='A'
effects=[{kind='config-value',name='mode'}]
[[features]]
id='b'
name='B'
[[features]]
id='gate'
name='Gate'
[[groups]]
mode='choice'
id='mode'
name='Mode'
cardinality='zero-or-more'
members=['a','b']
defaults=[]
guards=[{member='gate'}]
[[groups]]
mode='choice'
id='activation'
name='Activation'
cardinality='zero-or-one'
members=['gate']
defaults=[]
[[parameters]]
id='n'
name='Count'
type='integer'
min=-10
max=10
default=-7
[[parameters]]
id='token'
name='Token'
type='string'
secret=true
default='public-authored-literal'
[[profiles]]
id='preset'
name='Preset'
selections.mode={intent='exact',members=['a']}
parameters.n=-3
[config_values]
mode=-1
"#,
    )
    .unwrap()
}

#[test]
fn nonempty_profile_typed_values_rules_guards_visibility_and_effects_change_meaning() {
    use crate::configuration::MergeIntent;
    use crate::manifest::{
        ConfigurationFeatureVisibility as Visibility, ConfigurationGroupDecl, ParameterValue,
    };
    let d = nonempty();
    let basis = d.semantic_fingerprint().unwrap();
    assert_eq!(
        basis,
        "configuration-semantic/1:sha256:951baac2a06b5669709cdcefff8c9420253e949007980056fbde7c614937b36d"
    );
    let mut additive = d.clone();
    additive.profiles[0]
        .selections
        .get_mut("mode")
        .unwrap()
        .intent = MergeIntent::Additive;
    let mut omitted = d.clone();
    omitted.profiles[0]
        .selections
        .get_mut("mode")
        .unwrap()
        .members = None;
    let mut empty = d.clone();
    empty.profiles[0]
        .selections
        .get_mut("mode")
        .unwrap()
        .members = Some(vec![]);
    let mut missing = d.clone();
    missing.profiles[0].selections.clear();
    let mut preset = d.clone();
    preset.profiles[0]
        .parameters
        .insert("n".into(), ParameterValue::Integer(-2));
    let variants = [additive, omitted, empty, missing, preset];
    let hashes: std::collections::BTreeSet<_> = std::iter::once(basis.clone())
        .chain(variants.iter().map(|v| v.semantic_fingerprint().unwrap()))
        .collect();
    assert_eq!(
        hashes.len(),
        6,
        "profile exact/additive/omitted/empty/missing and typed presets are distinct"
    );
    for mut change in [
        d.clone(),
        d.clone(),
        d.clone(),
        d.clone(),
        d.clone(),
        d.clone(),
        d.clone(),
        d.clone(),
    ]
    .into_iter()
    .enumerate()
    {
        match change.0 {
            0 => change.1.parameters[0].min = Some(-9),
            1 => change.1.parameters[1].secret = false,
            2 => change.1.features[2].visibility = Visibility::Internal,
            3 => {
                if let ConfigurationGroupDecl::Choice { guards, .. } = &mut change.1.groups[0] {
                    guards.clear();
                }
            }
            4 => change.1.features[0].effects.clear(),
            5 => {
                change
                    .1
                    .config_values
                    .insert("mode".into(), ParameterValue::Bool(true));
            }
            6 => {
                change
                    .1
                    .config_values
                    .insert("mode".into(), ParameterValue::Text("-1".into()));
            }
            7 => change.1.parameters[0].default = Some(ParameterValue::Integer(7)),
            _ => unreachable!(),
        }
        assert_ne!(
            change.1.semantic_fingerprint().unwrap(),
            basis,
            "semantic discriminator {}",
            change.0
        );
    }
    let mut text_true = d.clone();
    text_true
        .config_values
        .insert("mode".into(), ParameterValue::Text("true".into()));
    let mut bool_true = d.clone();
    bool_true
        .config_values
        .insert("mode".into(), ParameterValue::Bool(true));
    assert_ne!(
        text_true.semantic_fingerprint().unwrap(),
        bool_true.semantic_fingerprint().unwrap()
    );
}
