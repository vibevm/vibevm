//! M05G1-CORPUS: the shared evaluator is the configured graph/effect oracle.
use super::{configuration::*, m_05_c_2_cases as accepted, *};
use crate::features::configuration::resolve_consumer_configuration;
use vibe_core::manifest::{feature_constraints::*, feature_effects::*};
use vibe_wire::generated::consumer_configuration::e1::{request as wire, result::OutcomeStatus};

fn p(name: &str) -> PackageRef {
    PackageRef::parse(&format!("org.example/{name}")).unwrap()
}
fn f(name: &str) -> LocalFeatureRef {
    LocalFeatureRef::new(name).unwrap()
}

#[test]
fn configured_candidate_effects_and_reasons() {
    let catalog = accepted::catalog();
    let mut request = accepted::request();
    request.edges[0].features.push("a".into());
    let mut second = request.edges[0].clone();
    second.consumer = "g1-second-consumer".into();
    second.features = vec!["b".into()];
    request.edges.push(second);
    let mut definitions = accepted::definitions(&catalog, &request);
    definitions[1].definition.program.features.remove(&f("b"));
    let result = ResolvoDepSolver::new(&catalog)
        .solve_configuration(&[p("app"), p("blocker")], &request, &definitions, &[])
        .unwrap();
    accepted::assert_selected(&result, &["a", "b"]);
    assert_eq!(result.request, request);
    assert_eq!(result.rejected.len(), 1);
    assert_eq!(result.rejected[0].identity, definitions[1].identity);
    assert_eq!(result.rejected[0].edges, vec![0, 1]);
    assert!(!result.rejected[0].reason.is_empty());
    assert!(
        result
            .choices
            .iter()
            .all(|edge| edge.provider == p("good@=1.0.0"))
    );
}

#[test]
fn accepted_group_matrix_matches_complete_effect_or_typed_conflict() {
    let catalog = accepted::catalog();
    let mut exercised = 0;
    for cardinality in [
        Cardinality::ZeroOrMore,
        Cardinality::OneOrMore,
        Cardinality::ZeroOrOne,
        Cardinality::ExactlyOne,
    ] {
        // Omitted, explicit empty, single, multi, additive union and exact conflict.
        for (members, second, exact, defaults) in [
            (None, None, false, false),
            (Some(vec![]), None, false, false),
            (Some(vec!["a"]), None, false, false),
            (Some(vec!["a", "b"]), None, false, false),
            (Some(vec!["a"]), Some(vec!["b"]), false, false),
            (Some(vec!["a"]), Some(vec!["b"]), true, false),
            (None, None, false, true),
        ] {
            let mut request = accepted::request();
            request.edges[0].selections.insert(
                "choice".into(),
                wire::Selection {
                    intent: if exact {
                        wire::SelectionIntent::Exact
                    } else {
                        wire::SelectionIntent::Additive
                    },
                    provenance: "manifest:g1-a".into(),
                    members: members.map(|v| v.into_iter().map(str::to_owned).collect()),
                },
            );
            if defaults {
                request.edges[0].policy = wire::EdgePolicy::UseDeclaredDefaults;
            }
            if let Some(members) = second {
                let mut edge = request.edges[0].clone();
                edge.consumer = "g1-b".into();
                edge.selections.get_mut("choice").unwrap().members =
                    Some(members.into_iter().map(str::to_owned).collect());
                request.edges.push(edge);
            }
            let mut definitions = accepted::definitions(&catalog, &request);
            definitions[0].definition.constraints.groups.insert(
                f("choice"),
                ConstrainedGroup {
                    cardinality,
                    members: [f("a"), f("b")].into(),
                    guards: vec![],
                    defaults: Some([f("a")].into()),
                },
            );
            let oracle =
                resolve_consumer_configuration(&request, &[definitions[0].definition.clone()]);
            let bindings = if defaults {
                vec![DefaultBinding::shown(0, &request, &definitions[0])]
            } else {
                vec![]
            };
            let result = ResolvoDepSolver::new(&catalog).solve_configuration(
                &[p("app@=1.0.0"), p("blocker")],
                &request,
                &definitions,
                &bindings,
            );
            let oracle = match oracle {
                Ok(oracle) => oracle,
                Err(reason) => {
                    let Err(ConfigurationSolveError::Conflict {
                        request: retained,
                        candidates,
                        ..
                    }) = result
                    else {
                        panic!("expected candidate conflict for {reason}");
                    };
                    assert_eq!(retained, request);
                    assert!(
                        candidates
                            .iter()
                            .any(|c| c.coordinate == "org.example/app@1.0.0"
                                && c.reason.contains(&reason))
                    );
                    exercised += 1;
                    continue;
                }
            };
            match oracle.result.outcomes[0].status {
                OutcomeStatus::Resolved => {
                    let result = result.unwrap();
                    assert_eq!(result.request, request);
                    assert_eq!(
                        &result.effects["org.example/app@1.0.0"],
                        oracle.expansions[0].as_ref().unwrap()
                    );
                    let extra: Vec<_> = oracle.result.outcomes[0]
                        .active_features
                        .iter()
                        .filter(|s| s.as_str() != "ordinary")
                        .map(String::as_str)
                        .collect();
                    accepted::assert_selected(&result, &extra);
                }
                OutcomeStatus::NeedsSelection => assert!(matches!(
                    result,
                    Err(ConfigurationSolveError::NeedsSelection { .. })
                )),
                OutcomeStatus::Conflict => {
                    let Err(ConfigurationSolveError::Conflict {
                        request: retained,
                        candidates,
                        ..
                    }) = result
                    else {
                        panic!("expected typed conflict");
                    };
                    assert_eq!(retained, request);
                    assert_eq!(candidates[0].edges, vec![0, 1]);
                    assert!(!candidates[0].reason.is_empty());
                }
            }
            exercised += 1;
        }
    }
    assert_eq!(exercised, 28);
}

#[test]
fn guarded_dormant_intent_matches_complete_selected_effects() {
    let catalog = accepted::catalog();
    for choice in ["a", "b"] {
        let mut request = accepted::request();
        for (group, member) in [("choice", choice), ("branch", "c")] {
            request.edges[0].selections.insert(
                group.into(),
                wire::Selection {
                    intent: wire::SelectionIntent::Exact,
                    provenance: format!("manifest:{group}"),
                    members: Some(vec![member.into()]),
                },
            );
        }
        let mut definitions = accepted::definitions(&catalog, &request);
        let definition = &mut definitions[0].definition;
        definition
            .program
            .features
            .insert(f("c"), Default::default());
        definition.constraints.groups.insert(
            f("choice"),
            ConstrainedGroup {
                cardinality: Cardinality::ExactlyOne,
                members: [f("a"), f("b")].into(),
                guards: vec![],
                defaults: None,
            },
        );
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
        let oracle = resolve_consumer_configuration(&request, &[definition.clone()]).unwrap();
        let result = ResolvoDepSolver::new(&catalog)
            .solve_configuration(
                &[p("app@=1.0.0"), p("blocker")],
                &request,
                &definitions,
                &[],
            )
            .unwrap();
        assert_eq!(result.request, request);
        assert_eq!(
            &result.effects["org.example/app@1.0.0"],
            oracle.expansions[0].as_ref().unwrap()
        );
        accepted::assert_selected(&result, if choice == "a" { &["a"] } else { &["b", "c"] });
        assert_eq!(
            result.request.edges[0].selections["branch"].members,
            Some(vec!["c".into()])
        );
    }
}

#[test]
fn configured_forwarding_native_and_multiple_instances_are_explicit_limits() {
    let catalog = accepted::catalog();
    let inspection = catalog.inspect().clone();
    let request = accepted::request();
    for strength in [FeatureStrength::Strong, FeatureStrength::Weak] {
        let mut definitions = accepted::definitions(&catalog, &request);
        definitions[0]
            .definition
            .program
            .features
            .get_mut(&f("ordinary"))
            .unwrap()
            .insert(FeatureEffect::TargetFeature {
                dependency: DependencyDeclarationId::new("org.example/good").unwrap(),
                feature: f("forwarded"),
                strength,
            });
        assert!(
            matches!(ResolvoDepSolver::new(&catalog).solve_configuration(&[p("app")], &request, &definitions, &[]), Err(ConfigurationSolveError::Unsupported(reason)) if reason.contains("forwarding"))
        );
    }
    let mut separate = request.clone();
    let mut second = separate.edges[0].clone();
    second.consumer = "separate-domain".into();
    second.domain.definition.reference = "environment-b".into();
    separate.edges.push(second);
    assert!(
        matches!(ResolvoDepSolver::new(&catalog).solve_configuration(&[p("app")], &separate, &accepted::definitions(&catalog, &request), &[]), Err(ConfigurationSolveError::Unsupported(reason)) if reason.contains("multiple configuration instances"))
    );
    let native = vibe_wire::behaviour::consumer_configuration::decode_request(include_bytes!("../../../../formats/corpora/consumer-configuration-request/e1/valid/dormant-native-version.json")).unwrap();
    assert!(
        matches!(ResolvoDepSolver::new(&catalog).solve_configuration(&[p("app")], &native, &[], &[]), Err(ConfigurationSolveError::Unsupported(reason)) if reason.contains("native"))
    );
    assert_eq!(catalog.inspect(), &inspection);
}
