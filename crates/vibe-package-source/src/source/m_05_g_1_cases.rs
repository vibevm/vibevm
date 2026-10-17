use super::*;
use crate as package_source;
use vibe_resolver::resolvo_engine::request::NormalizedRequest;
#[path = "../../../vibe-install/tests/g1_support/mod.rs"]
mod fixture;
use fixture::{Fixture, p};

#[test]
fn supported_legacy_matrix() {
    let fixture = Fixture::new();
    fixture.legacy();
    // Provider is an explicit earlier root: within naive/SAT capability limits.
    let roots = [p("provider"), p("app"), p("other")];
    let request = NormalizedRequest::legacy(&roots, &Default::default());
    let mut reference = None;
    let mut cells = 0;
    for mode in ["local", "multi", "embedded"] {
        for backend in [None, Some("resolvo"), Some("naive"), Some("sat")] {
            let source = fixture.source(backend, mode);
            let frozen = source.bind_request(request.clone()).unwrap();
            let graph = frozen.solve(&roots).unwrap();
            let first = frozen.route_resolution().unwrap();
            assert_eq!(first.provenance.len(), 3);
            assert_eq!(
                graph
                    .packages
                    .iter()
                    .map(|n| (n.name.as_str(), n.version.to_string(), n.is_root))
                    .collect::<Vec<_>>(),
                vec![
                    ("provider", "1.0.0".into(), true),
                    ("app", "1.0.0".into(), true),
                    ("other", "1.0.0".into(), true),
                    ("dep", "1.0.0".into(), false),
                ]
            );
            assert!(
                first
                    .recorded
                    .choices
                    .iter()
                    .any(|e| e.declaration == "requires.packages[0]"
                        && e.provider == p("dep@=1.0.0"))
            );
            assert!(
                first
                    .recorded
                    .choices
                    .iter()
                    .any(|e| e.declaration.starts_with("requires.capabilities[0]:")
                        && e.provider == p("provider@=1.0.0"))
            );
            for edge in &first.provenance {
                assert_eq!(edge.selected.parent, p("app@=1.0.0"));
                assert!(
                    graph
                        .packages
                        .iter()
                        .any(|n| edge.selected.provider.group.as_ref() == Some(&n.group)
                            && edge.selected.provider.name == n.name
                            && edge.selected.provider.version.matches(&n.version))
                );
            }
            assert_eq!(
                first.effects["org.g1/app@1.0.0"].active_features,
                ["default".into(), "helper".into()].into()
            );
            assert_eq!(
                first.effects["org.g1/app@1.0.0"].dep_features["org.g1/dep"],
                ["trace".into()].into()
            );
            // Validate every graph edge against the actual declaration witness.
            // Graph multiplicity differs when two declarations select one provider.
            for node in &graph.packages {
                let actual: BTreeSet<_> =
                    node.dependencies.iter().map(ToString::to_string).collect();
                let expected: BTreeSet<_> = first
                    .recorded
                    .choices
                    .iter()
                    .filter(|e| {
                        e.parent.group.as_ref() == Some(&node.group) && e.parent.name == node.name
                    })
                    .map(|e| e.provider.to_string())
                    .collect();
                assert_eq!(actual, expected);
            }
            let mut class = graph.clone();
            for node in &mut class.packages {
                node.dependencies.clear();
            }
            if let Some((g, effects)) = &reference {
                assert_eq!(&class, g);
                assert_eq!(&first.effects, effects);
            } else {
                reference = Some((class, first.effects.clone()));
            }
            // OR may have multiple valid witnesses because both alternatives are roots.
            let or = first
                .recorded
                .choices
                .iter()
                .find(|e| e.declaration == "requires_any[0]")
                .unwrap();
            assert_eq!(
                or.provider,
                if or.alternative == 0 {
                    p("provider@=1.0.0")
                } else {
                    assert_eq!(or.alternative, 1);
                    p("other@=1.0.0")
                }
            );
            for node in &graph.packages {
                let reference = p(&format!("{}@={}", node.name, node.version));
                assert_eq!(
                    frozen.manifest_of(&reference).unwrap(),
                    first.manifests[&format!("org.g1/{}@{}", node.name, node.version)]
                );
            }
            assert_eq!(
                graph,
                frozen.solve_masked(&roots, &Default::default()).unwrap()
            );
            let retried = frozen.route_resolution().unwrap();
            assert_eq!(first.provenance, retried.provenance);
            assert_eq!(first.effects, retried.effects);
            assert_eq!(
                frozen.normalized_request().unwrap().root_origins,
                request.root_origins
            );
            cells += 1;
        }
    }
    assert_eq!(cells, 12);
}

#[test]
fn source_route_consistency_and_backend_refusals() {
    let fixture = Fixture::new();
    let request = fixture.configured();
    let mut cells = 0;
    for mode in ["local", "multi", "embedded"] {
        for backend in [None, Some("resolvo"), Some("naive"), Some("sat")] {
            let source = fixture.source(backend, mode);
            let frozen = source.bind_request(request.clone()).unwrap();
            if matches!(backend, Some("naive" | "sat")) {
                for result in [
                    frozen.solve(&request.roots),
                    frozen.solve_masked(&request.roots, &Default::default()),
                ] {
                    assert!(
                        matches!(result, Err(vibe_resolver::SolveError::BackendCapability { backend: selected, .. }) if Some(selected.as_str()) == backend)
                    );
                }
                assert!(
                    matches!(frozen.manifest_of(&p("app")), Err(vibe_resolver::SolveError::BackendCapability { backend: selected, .. }) if Some(selected.as_str()) == backend)
                );
                assert!(frozen.route_resolution().is_none());
            } else {
                let graph = frozen.solve(&request.roots).unwrap();
                assert_eq!(
                    graph
                        .find(p("app").group.as_ref().unwrap(), "app")
                        .unwrap()
                        .version
                        .to_string(),
                    "1.0.0"
                );
                assert!(!graph.packages.iter().any(|n| n.name == "bad"));
                let first = frozen.route_resolution().unwrap();
                assert_eq!(first.rejected.len(), 1);
                assert_eq!(first.rejected[0].coordinate, "org.g1/app@2.0.0");
                assert_eq!(first.rejected[0].edges, vec![0, 1]);
                assert!(!first.rejected[0].reason.is_empty());
                let effects = &first.effects["org.g1/app@1.0.0"];
                assert_eq!(effects.active_features, ["a".into(), "b".into()].into());
                assert_eq!(effects.active_deps, ["org.g1/good".into()].into());
                assert_eq!(first.provenance.len(), 3);
                assert!(first.provenance.iter().any(|e| e.selected.declaration
                    == "configuration.dependencies[org.g1/good]"
                    && e.selected.provider == p("good@=1.0.0")));
                assert_eq!(
                    frozen.manifest_of(&p("app")).unwrap().requires.packages,
                    vec![p("good@*")]
                );
                assert_eq!(
                    graph,
                    frozen
                        .solve_masked(&request.roots, &Default::default())
                        .unwrap()
                );
                assert_eq!(
                    first.provenance,
                    frozen.route_resolution().unwrap().provenance
                );
                assert_eq!(first.effects, frozen.route_resolution().unwrap().effects);
                assert_eq!(
                    format!("{:?}", first.rejected),
                    format!("{:?}", frozen.route_resolution().unwrap().rejected)
                );
                assert_eq!(
                    frozen
                        .normalized_request()
                        .unwrap()
                        .configuration
                        .as_ref()
                        .unwrap()
                        .request,
                    request.configuration.as_ref().unwrap().request
                );
            }
            cells += 1;
        }
    }
    assert_eq!(cells, 12);
}

#[test]
fn source_corruption_is_not_candidate_conflict_or_fallback() {
    for backend in ["resolvo", "naive", "sat"] {
        let fixture = Fixture::new();
        fixture.legacy();
        std::fs::write(
            fixture.registry.join("org.g1/app/v1.0.0/vibe.toml"),
            "invalid TOML [",
        )
        .unwrap();
        let source = fixture.source(Some(backend), "local");
        assert!(matches!(
            source.bind_request(NormalizedRequest::legacy(&[p("app")], &Default::default())),
            Err(vibe_resolver::SolveError::Provider(_))
        ));
    }
}

#[test]
fn legacy_or_search_limits_and_wrong_versions() {
    for mode in ["local", "multi", "embedded"] {
        for backend in ["resolvo", "naive", "sat"] {
            let fixture = Fixture::new();
            fixture.package(
                "app",
                1,
                "[[requires_any]]\none_of=['org.g1/missing','org.g1/provider']\n",
            );
            fixture.package("provider", 1, "");
            let source = fixture.source(Some(backend), mode);
            let bound =
                source.bind_request(NormalizedRequest::legacy(&[p("app")], &Default::default()));
            if mode == "multi" {
                // The offline declared-source owner reports unavailable metadata,
                // not catalog absence. Preserve that source failure for every cell.
                assert!(matches!(bound, Err(vibe_resolver::SolveError::Provider(_))));
            } else {
                let frozen = bound.unwrap();
                let result = frozen.solve(&[p("app")]);
                if backend == "resolvo" {
                    assert_eq!(result.unwrap().packages.len(), 2);
                    let edge = frozen
                        .route_resolution()
                        .unwrap()
                        .recorded
                        .choices
                        .remove(0);
                    assert_eq!(edge.alternative, 1);
                    assert_eq!(edge.provider, p("provider@=1.0.0"));
                } else {
                    // These cells do not search OR alternatives. Preserve this measured
                    // limitation rather than claiming their failure is parity or fallback.
                    assert!(matches!(
                        result,
                        Err(vibe_resolver::SolveError::Provider(_))
                    ));
                    assert!(frozen.route_resolution().is_none());
                }
            }
            fixture.package(
                "app",
                1,
                "[[requires_any]]\none_of=['org.g1/provider@=2.0.0']\n",
            );
            fixture.package("provider", 2, "");
            let request =
                NormalizedRequest::legacy(&[p("provider@=1.0.0"), p("app")], &Default::default());
            let source = fixture.source(Some(backend), mode);
            let frozen = source.bind_request(request.clone()).unwrap();
            assert!(matches!(
                frozen.solve(&request.roots),
                Err(vibe_resolver::SolveError::RootConstraints { .. })
            ));
            assert!(frozen.route_resolution().is_none());
        }
    }
}
