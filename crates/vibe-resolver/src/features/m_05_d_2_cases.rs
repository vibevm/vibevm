use super::configuration::{ConfigurationDefinition, resolve_consumer_configuration};
use std::collections::{BTreeMap, BTreeSet};
use vibe_core::manifest::{feature_constraints::*, feature_effects::*};
use vibe_wire::behaviour::consumer_configuration as codec;
use vibe_wire::generated::consumer_configuration::e1::request::{Request, SelectionIntent};

fn f(name: &str) -> LocalFeatureRef {
    LocalFeatureRef::new(name).unwrap()
}

fn fixture() -> (Request, ConfigurationDefinition) {
    let mut request = codec::decode_request(include_bytes!(
        "../../../../formats/corpora/consumer-configuration-request/e1/valid/omitted.json"
    ))
    .unwrap();
    let choice = request.edges[0].selections.get_mut("choice").unwrap();
    choice.members = Some(vec!["a".into()]);
    let mut branch = choice.clone();
    branch.members = Some(vec!["c".into()]);
    branch.intent = SelectionIntent::Exact;
    branch.provenance = "authored:dormant".into();
    request.edges[0].selections.insert("branch".into(), branch);
    let definition = ConfigurationDefinition {
        instance: codec::admit_request(&request).unwrap()[0].instance.clone(),
        program: FeatureProgram {
            features: [
                (f("a"), BTreeSet::new()),
                (f("b"), BTreeSet::new()),
                (
                    f("c"),
                    [
                        FeatureEffect::ExportedSubskill("stack/rust".into()),
                        FeatureEffect::ConfigValue(f("flag")),
                    ]
                    .into(),
                ),
            ]
            .into(),
            exclusive: BTreeMap::new(),
        },
        declarations: FeatureDeclarations {
            exported_subskills: ["stack/rust".into()].into(),
            config_values: [(f("flag"), ConfigValue::Bool(true))].into(),
            ..Default::default()
        },
        constraints: Constraints {
            groups: [
                (
                    f("choice"),
                    ConstrainedGroup {
                        cardinality: Cardinality::ExactlyOne,
                        members: [f("a"), f("b")].into(),
                        guards: vec![],
                        defaults: None,
                    },
                ),
                (
                    f("branch"),
                    ConstrainedGroup {
                        cardinality: Cardinality::ExactlyOne,
                        members: [f("c")].into(),
                        guards: vec![Guard::Member(f("b"))],
                        defaults: None,
                    },
                ),
            ]
            .into(),
            member_guards: BTreeMap::new(),
        },
    };
    (request, definition)
}

#[test]
fn dormant_selection_is_retained_without_effects() {
    let (mut request, definition) = fixture();
    let authored = request.clone();
    let result =
        resolve_consumer_configuration(&request, std::slice::from_ref(&definition)).unwrap();
    let effects = result.expansions[0].as_ref().unwrap();
    assert_eq!(effects.active_features, ["a".into()].into());
    assert!(effects.active_subskills.is_empty());
    assert!(effects.config_values.is_empty());
    assert!(result.result.outcomes[0].defaults_used.is_empty());
    assert_eq!(
        result.result.edges[0].selections["branch"].members,
        Some(vec!["c".into()])
    );
    assert_eq!(
        result.result.edges[0].selections["branch"].provenance,
        "authored:dormant"
    );
    assert_eq!(request, authored);

    request.edges[0]
        .selections
        .get_mut("choice")
        .unwrap()
        .members = Some(vec!["b".into()]);
    let result = resolve_consumer_configuration(&request, &[definition]).unwrap();
    let effects = result.expansions[0].as_ref().unwrap();
    assert_eq!(effects.active_subskills, ["stack/rust".into()].into());
    assert_eq!(effects.config_values["flag"], ConfigValue::Bool(true));
    assert!(result.result.outcomes[0].defaults_used.is_empty());
}

#[test]
fn removed_dormant_option_reports_drift() {
    let (request, mut definition) = fixture();
    let authored = request.clone();
    // A coherent current definition replaces c with d while its guard stays off.
    let effects = definition.program.features.remove(&f("c")).unwrap();
    definition.program.features.insert(f("d"), effects);
    definition
        .constraints
        .groups
        .get_mut(&f("branch"))
        .unwrap()
        .members = [f("d")].into();
    let error = resolve_consumer_configuration(&request, &[definition])
        .err()
        .unwrap();
    assert_eq!(error, "unknown requested member");
    assert_eq!(request, authored);
}
