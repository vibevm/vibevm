use super::*;
use vibe_wire::generated::consumer_configuration::e1::request as wire;
fn f(s: &str) -> LocalFeatureRef {
    LocalFeatureRef::new(s).unwrap()
}
fn request() -> Request {
    codec::decode_request(include_bytes!(
        "../../../../../formats/corpora/consumer-configuration-request/e1/valid/omitted.json"
    ))
    .unwrap()
}
fn definition(request: &Request, cardinality: Cardinality) -> ConfigurationDefinition {
    let program = FeatureProgram {
        features: [
            (f("a"), BTreeSet::new()),
            (f("b"), BTreeSet::new()),
            (f("c"), BTreeSet::new()),
            (
                f("ordinary"),
                [FeatureEffect::ConfigValue(f("flag"))].into(),
            ),
        ]
        .into(),
        exclusive: BTreeMap::new(),
    };
    ConfigurationDefinition {
        instance: codec::admit_request(request).unwrap()[0].instance.clone(),
        program,
        declarations: FeatureDeclarations {
            config_values: [(f("flag"), ConfigValue::Bool(true))].into(),
            ..Default::default()
        },
        constraints: Constraints {
            groups: [(
                f("choice"),
                ConstrainedGroup {
                    cardinality,
                    members: [f("a"), f("b")].into(),
                    guards: vec![],
                    defaults: Some([f("a")].into()),
                },
            )]
            .into(),
            member_guards: BTreeMap::new(),
        },
    }
}
fn choose(request: &mut Request, index: usize, members: &[&str], exact: bool) {
    let choice = request.edges[index].selections.get_mut("choice").unwrap();
    choice.members = Some(members.iter().map(|s| (*s).into()).collect());
    choice.intent = if exact {
        wire::SelectionIntent::Exact
    } else {
        wire::SelectionIntent::Additive
    };
}
#[test]
fn generated_request_needs_selection_without_effects() {
    let request = request();
    let definition = definition(&request, Cardinality::ExactlyOne);
    let before = definition.program.clone();
    let request_before = request.clone();
    let definitions = [definition];
    let resolution = resolve_consumer_configuration(&request, &definitions).unwrap();
    assert_eq!(
        resolution.result.outcomes[0].status,
        OutcomeStatus::NeedsSelection
    );
    assert_eq!(resolution.result.outcomes[0].needs, vec!["choice"]);
    assert!(resolution.expansions[0].is_none());
    assert_eq!(before, definitions[0].program);
    assert_eq!(request, request_before);
    assert_eq!(
        resolution.result.edges[0].selections["choice"].members,
        None
    );
    assert!(resolution.result.edges[0].domain.lock.is_none());
}
#[test]
fn explicit_defaults_and_empty_policy() {
    let mut request = request();
    request.edges[0].policy = wire::EdgePolicy::UseDeclaredDefaults;
    let definition = definition(&request, Cardinality::ExactlyOne);
    let resolution = resolve_consumer_configuration(&request, &[definition]).unwrap();
    assert_eq!(resolution.result.outcomes[0].active_features, vec!["a"]);
    assert_eq!(
        resolution.result.outcomes[0].defaults_used[0].provenance,
        "manifest:consumer-a"
    );
    choose(&mut request, 0, &[], false);
    let resolution = resolve_consumer_configuration(
        &request,
        &[self::definition(&request, Cardinality::ExactlyOne)],
    )
    .unwrap();
    assert_eq!(
        resolution.result.outcomes[0].status,
        OutcomeStatus::NeedsSelection
    );
    assert!(resolution.result.outcomes[0].defaults_used.is_empty());
}
#[test]
fn shared_additive_union_and_actual_evaluator() {
    let mut request = request();
    choose(&mut request, 0, &["a"], false);
    let mut second = request.edges[0].clone();
    second.consumer = "consumer-b".into();
    request.edges.push(second);
    choose(&mut request, 1, &["b"], false);
    request.edges[0].features.push("ordinary".into());
    let resolution =
        resolve_consumer_configuration(&request, &[definition(&request, Cardinality::ZeroOrMore)])
            .unwrap();
    assert_eq!(resolution.result.outcomes.len(), 1);
    assert_eq!(
        resolution.result.outcomes[0].selected["choice"],
        vec!["a", "b"]
    );
    assert_eq!(
        resolution.expansions[0].as_ref().unwrap().config_values["flag"],
        ConfigValue::Bool(true)
    );
}
#[test]
fn separate_environment_domains_keep_different_choices() {
    let mut request = request();
    choose(&mut request, 0, &["a"], true);
    let first = definition(&request, Cardinality::ExactlyOne);
    let mut second = request.edges[0].clone();
    second.consumer = "consumer-b".into();
    second.domain.definition.reference = "env-b".into();
    request.edges.push(second);
    choose(&mut request, 1, &["b"], true);
    let second_request = Request {
        epoch: wire::RequestEpoch::DefaultName,
        edges: vec![request.edges[1].clone()],
    };
    let resolution = resolve_consumer_configuration(
        &request,
        &[first, definition(&second_request, Cardinality::ExactlyOne)],
    )
    .unwrap();
    assert_eq!(resolution.result.outcomes.len(), 2);
    assert_eq!(resolution.result.outcomes[0].active_features, vec!["a"]);
    assert_eq!(resolution.result.outcomes[1].active_features, vec!["b"]);
}
#[test]
fn exact_conflicts_name_both_edges_and_additive_must_fit() {
    let mut request = request();
    choose(&mut request, 0, &["a"], true);
    let mut second = request.edges[0].clone();
    second.consumer = "consumer-b".into();
    request.edges.push(second);
    for exact in [true, false] {
        choose(&mut request, 1, &["b"], exact);
        let resolution = resolve_consumer_configuration(
            &request,
            &[definition(&request, Cardinality::ExactlyOne)],
        )
        .unwrap();
        assert_eq!(
            resolution.result.outcomes[0].status,
            OutcomeStatus::Conflict
        );
        assert_eq!(resolution.result.outcomes[0].conflicting_edges, vec![0, 1]);
        assert_eq!(resolution.result.edges[1].consumer, "consumer-b");
        assert!(resolution.expansions[0].is_none());
    }
    choose(&mut request, 1, &["a"], false);
    assert_eq!(
        resolve_consumer_configuration(&request, &[definition(&request, Cardinality::ExactlyOne)])
            .unwrap()
            .result
            .outcomes[0]
            .status,
        OutcomeStatus::Resolved
    );
}
#[test]
fn dormant_intent_survives_until_positive_guard_is_active() {
    let mut request=codec::decode_request(include_bytes!("../../../../../formats/corpora/consumer-configuration-request/e1/valid/dormant-native-version.json")).unwrap();
    let mut definition = definition(&request, Cardinality::ExactlyOne);
    definition.constraints.groups.insert(
        f("branch"),
        ConstrainedGroup {
            cardinality: Cardinality::ExactlyOne,
            members: [f("c")].into(),
            guards: vec![Guard::Member(f("b"))],
            defaults: None,
        },
    );
    let resolution = resolve_consumer_configuration(
        &request,
        &[ConfigurationDefinition {
            instance: definition.instance.clone(),
            program: definition.program.clone(),
            declarations: definition.declarations.clone(),
            constraints: definition.constraints.clone(),
        }],
    )
    .unwrap();
    assert_eq!(resolution.result.outcomes[0].active_features, vec!["a"]);
    assert_eq!(
        resolution.result.edges[0].selections["branch"].members,
        Some(vec!["c".into()])
    );
    choose(&mut request, 0, &["b"], false);
    let resolution = resolve_consumer_configuration(&request, &[definition]).unwrap();
    assert_eq!(
        resolution.result.outcomes[0].active_features,
        vec!["b", "c"]
    );
}
#[test]
fn cardinality_all_features_and_implicit_closure_refuse() {
    for cardinality in [
        Cardinality::ZeroOrMore,
        Cardinality::OneOrMore,
        Cardinality::ZeroOrOne,
        Cardinality::ExactlyOne,
    ] {
        let mut request = request();
        choose(&mut request, 0, &[], false);
        let resolution =
            resolve_consumer_configuration(&request, &[definition(&request, cardinality)]).unwrap();
        assert_eq!(
            resolution.result.outcomes[0].status,
            if cardinality.accepts(0) {
                OutcomeStatus::Resolved
            } else {
                OutcomeStatus::NeedsSelection
            }
        );
    }
    let mut request = request();
    choose(&mut request, 0, &["a"], false);
    request.edges[0].all_features = true;
    assert!(
        resolve_consumer_configuration(&request, &[definition(&request, Cardinality::ExactlyOne)])
            .is_err()
    );
    request.edges[0]
        .selections
        .get_mut("choice")
        .unwrap()
        .members = None;
    assert!(
        resolve_consumer_configuration(&request, &[definition(&request, Cardinality::ExactlyOne)])
            .is_err()
    );
    choose(&mut request, 0, &["a"], false);
    request.edges[0].all_features = false;
    let mut definition = definition(&request, Cardinality::ExactlyOne);
    definition
        .program
        .features
        .get_mut(&f("a"))
        .unwrap()
        .insert(FeatureEffect::Local(f("b")));
    assert!(resolve_consumer_configuration(&request, &[definition]).is_err());
}
#[test]
fn owner_target_roles_and_configuration_are_identity() {
    let mut request = request();
    choose(&mut request, 0, &["a"], false);
    let first = definition(&request, Cardinality::ExactlyOne);
    request.edges[0].domain.target.producer = "another-owner".into();
    assert!(resolve_consumer_configuration(&request, &[first]).is_err());
    let first = definition(&request, Cardinality::ExactlyOne);
    let mut second = request.edges[0].clone();
    second.consumer = "b".into();
    second.configuration = "other-configuration".into();
    request.edges.push(second);
    assert!(resolve_consumer_configuration(&request, &[first]).is_err());
}

#[test]
fn dormant_member_exact_intent_and_group_guards() {
    let mut request = request();
    choose(&mut request, 0, &["a"], false);
    request.edges[0].selections.insert(
        "branch".into(),
        wire::Selection {
            intent: wire::SelectionIntent::Exact,
            provenance: "manifest:branch".into(),
            members: Some(vec!["c".into()]),
        },
    );
    let mut second = request.edges[0].clone();
    second.consumer = "consumer-b".into();
    second.selections.get_mut("branch").unwrap().members = Some(vec![]);
    request.edges.push(second);
    let mut definition = definition(&request, Cardinality::ExactlyOne);
    definition.constraints.groups.insert(
        f("branch"),
        ConstrainedGroup {
            cardinality: Cardinality::ZeroOrOne,
            members: [f("c")].into(),
            guards: vec![Guard::Group(f("choice"))],
            defaults: None,
        },
    );
    definition
        .constraints
        .member_guards
        .insert(f("c"), vec![Guard::Member(f("b"))]);
    let definitions = [definition];
    let result = resolve_consumer_configuration(&request, &definitions).unwrap();
    assert_eq!(result.result.outcomes[0].status, OutcomeStatus::Resolved);
    assert!(result.result.outcomes[0].selected["branch"].is_empty());
    assert_eq!(
        result.result.edges[0].selections["branch"].members,
        Some(vec!["c".into()])
    );
    choose(&mut request, 0, &["b"], false);
    choose(&mut request, 1, &["b"], false);
    let result = resolve_consumer_configuration(&request, &definitions).unwrap();
    assert_eq!(result.result.outcomes[0].status, OutcomeStatus::Conflict);
    assert_eq!(result.result.outcomes[0].conflicting_edges, vec![0, 1]);
}
