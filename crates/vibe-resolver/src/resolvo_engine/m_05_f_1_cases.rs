use super::configuration::{ConfigurationSolveError, DefaultBinding};
use super::request::consumer_wire as wire;
use super::*;
use crate as resolver;
use crate::LocalRegistryDepProvider;

#[path = "../../../../vibevm/vibepacks/org.vibevm.world/redbook/v1.1.0/tests/f1_fixture.rs"]
mod fixture;

#[test]
fn actual_redbook_closures_defaults_and_local_conflicts() {
    let temp = tempfile::tempdir().unwrap();
    fixture::registry(temp.path());
    fixture::unused_alternative(temp.path());
    let registry = vibe_registry::LocalRegistry::new(temp.path()).unwrap();
    let provider = LocalRegistryDepProvider::new(&registry);
    let catalog = TransactionCatalog::capture(&provider, &[fixture::p("redbook")]).unwrap();
    for (branch, groups, active) in [
        ("mup", vec![], vec![]),
        ("wal", vec![], vec![]),
        (
            "mup",
            vec![
                ("additions", vec!["health-audit", "manual-tests"]),
                ("mup-additions", vec!["discovery-prompt"]),
                ("wal-additions", vec!["wal-specspaces"]),
            ],
            vec!["health-audit", "manual-tests", "discovery-prompt"],
        ),
        (
            "wal",
            vec![
                (
                    "additions",
                    vec!["comparative-research", "dev-runtime-docs"],
                ),
                ("wal-additions", vec!["wal-specspaces"]),
                ("mup-additions", vec!["discovery-prompt"]),
            ],
            vec!["comparative-research", "dev-runtime-docs", "wal-specspaces"],
        ),
    ] {
        let groups: Vec<_> = groups.iter().map(|(g, m)| (*g, m.as_slice())).collect();
        let mut intent = fixture::intent(Some(branch), &groups);
        catalog.admit_source_definitions(&mut intent).unwrap();
        let input = intent.configuration.as_ref().unwrap();
        let result = ResolvoDepSolver::new(&catalog)
            .solve_configuration(&intent.roots, &input.request, &input.definitions, &[])
            .unwrap();
        assert_eq!(result.request, input.request);
        assert_eq!(
            result
                .graph
                .iter()
                .map(|n| n.name.to_string())
                .collect::<std::collections::BTreeSet<_>>(),
            fixture::expected_names(branch, &active)
        );
        let effects = &result.effects["org.vibevm.world/redbook@1.1.0"];
        let expected_features: std::collections::BTreeSet<_> = std::iter::once(branch)
            .chain(active.iter().copied())
            .map(str::to_owned)
            .collect();
        assert_eq!(effects.active_features, expected_features);
        let node = result.graph.iter().find(|n| n.name == "redbook").unwrap();
        assert_eq!(
            node.dependencies
                .iter()
                .map(PackageRef::qualified_name)
                .collect::<std::collections::BTreeSet<_>>(),
            effects.active_deps
        );
        assert!(
            result
                .graph
                .iter()
                .all(|n| n.version == semver::Version::new(1, 1, 0))
        );
    }
    let mut omitted = fixture::intent(None, &[]);
    catalog.admit_source_definitions(&mut omitted).unwrap();
    let input = omitted.configuration.as_mut().unwrap();
    assert!(matches!(
        ResolvoDepSolver::new(&catalog).solve_configuration(
            &omitted.roots,
            &input.request,
            &input.definitions,
            &[]
        ),
        Err(ConfigurationSolveError::NeedsSelection { .. })
    ));
    input.request.edges[0].policy = wire::EdgePolicy::UseDeclaredDefaults;
    assert!(
        ResolvoDepSolver::new(&catalog)
            .solve_configuration(&omitted.roots, &input.request, &input.definitions, &[])
            .is_err()
    );
    let binding = DefaultBinding::shown(0, &input.request, &input.definitions[0]);
    let result = ResolvoDepSolver::new(&catalog)
        .solve_configuration(
            &omitted.roots,
            &input.request,
            &input.definitions,
            &[binding],
        )
        .unwrap();
    assert_eq!(
        result
            .graph
            .iter()
            .map(|n| n.name.to_string())
            .collect::<std::collections::BTreeSet<_>>(),
        fixture::expected_names("mup", &[])
    );

    input.request.edges[0].policy = wire::EdgePolicy::RequireSelection;
    input.request.edges[0].selections.insert(
        "continuity".into(),
        wire::Selection {
            intent: wire::SelectionIntent::Exact,
            provenance: "manifest:consumer-a".into(),
            members: Some(vec!["mup".into(), "wal".into()]),
        },
    );
    assert!(matches!(
        ResolvoDepSolver::new(&catalog).solve_configuration(
            &omitted.roots,
            &input.request,
            &input.definitions,
            &[]
        ),
        Err(ConfigurationSolveError::Conflict { .. })
    ));
    input.request.edges[0]
        .selections
        .get_mut("continuity")
        .unwrap()
        .members = Some(vec!["mup".into()]);
    let roots = vec![fixture::p("redbook"), fixture::p("wal")];
    let result = ResolvoDepSolver::new(&catalog)
        .solve_configuration(&roots, &input.request, &input.definitions, &[])
        .unwrap();
    assert_eq!(
        result
            .graph
            .iter()
            .map(|n| n.name.to_string())
            .collect::<std::collections::BTreeSet<_>>(),
        fixture::expected_names("mup", &["wal"])
    );
    let mut substituted = input.definitions.clone();
    substituted[0].definition.constraints.groups.clear();
    assert!(matches!(
        ResolvoDepSolver::new(&catalog).solve_configuration(
            &roots,
            &input.request,
            &substituted,
            &[]
        ),
        Err(ConfigurationSolveError::Source(_))
    ));

    let roots = vec![fixture::p("f1-alternative")];
    let catalog = TransactionCatalog::capture(&provider, &roots).unwrap();
    assert!(
        catalog
            .inspect()
            .candidates
            .iter()
            .any(|(c, _)| c == "org.vibevm.world/redbook@1.1.0")
    );
    let mut unbound = fixture::alternative_intent(temp.path());
    catalog.admit_source_definitions(&mut unbound).unwrap();
    let input = unbound.configuration.as_ref().unwrap();
    assert_eq!(input.definitions.len(), 1);
    assert_eq!(input.definitions[0].package.name, "f1-plain");
    let result = ResolvoDepSolver::new(&catalog)
        .solve_configuration(&roots, &input.request, &input.definitions, &[])
        .unwrap();
    assert_eq!(
        result
            .graph
            .iter()
            .map(|n| n.name.to_string())
            .collect::<std::collections::BTreeSet<_>>(),
        ["f1-alternative".to_owned(), "f1-plain".to_owned()].into()
    );
    assert!(
        !result
            .effects
            .contains_key("org.vibevm.world/redbook@1.1.0")
    );
    assert!(matches!(
        ResolvoDepSolver::new(&catalog).solve_configuration(
            &[fixture::p("redbook")],
            &input.request,
            &input.definitions,
            &[]
        ),
        Err(ConfigurationSolveError::NeedsSelection { .. })
    ));
}
