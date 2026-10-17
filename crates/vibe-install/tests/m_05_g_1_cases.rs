//! Production builder → transaction → workspace planner parity (M05G1-ORACLE).
mod g1_support;
use g1_support::{Fixture, p};
use vibe_install::{InstallRequest, InstallSource, NullObserver, Plan};
use vibe_package_source as package_source;
use vibe_resolver::{SolveError, resolvo_engine::request::NormalizedRequest};

fn request(roots: Vec<vibe_core::PackageRef>) -> InstallRequest {
    InstallRequest {
        roots,
        features: Default::default(),
        language: None,
        exact: false,
        generated_by: "G1 parity".into(),
        offline: true,
    }
}
fn ready(
    source: &impl InstallSource,
    fixture: &Fixture,
    roots: Vec<vibe_core::PackageRef>,
) -> Box<vibe_install::PlannedInstall> {
    let Plan::Ready(plan) =
        vibe_install::plan(source, &fixture.root, request(roots), &NullObserver).unwrap()
    else {
        panic!("expected plan");
    };
    plan
}

#[test]
fn workspace_root_union_and_selected_providers() {
    let _isolation = vibe_test_support::isolated_home().unwrap();
    let fixture = Fixture::new();
    fixture.legacy();
    // Bare workspace roots are coordinate ordered. Give the already-reachable
    // capability provider the earlier coordinate required by naive/SAT.
    fixture.package(
        "a-provider",
        1,
        "[provides]\ncapabilities=['capability:engine@1.0.0']\n",
    );
    fixture.package("app", 1, "[requires.packages]\n'org.g1/dep'='^1'\n[requires]\ncapabilities=['capability:engine@1.0.0']\n[[requires_any]]\none_of=['org.g1/a-provider','org.g1/other']\n");
    std::fs::write(fixture.root.join("vibe.toml"), "[project]\nname='g1'\nversion='1.0.0'\n[workspace]\nmembers=['member']\n[requires.packages]\n'org.g1/a-provider'='*'\n'org.g1/app'='^1'\n").unwrap();
    std::fs::create_dir(fixture.root.join("member")).unwrap();
    std::fs::write(fixture.root.join("member/vibe.toml"), "[project]\nname='member'\nversion='1.0.0'\n[requires.packages]\n'org.g1/app'='=1.0.0'\n'org.g1/other'='*'\n").unwrap();
    let mut cells = 0;
    for mode in ["local", "multi", "embedded"] {
        for backend in ["resolvo", "naive", "sat"] {
            let source = fixture.source(Some(backend), mode);
            let plan = ready(&source, &fixture, vec![]);
            assert_eq!(plan.resolution.len(), 4);
            let normalized = plan.normalized_request.as_ref().unwrap();
            let app_origins: Vec<_> = normalized
                .root_origins
                .iter()
                .filter(|o| o.constraint.name == "app")
                .collect();
            assert_eq!(app_origins.len(), 2);
            assert_ne!(app_origins[0].consumer, app_origins[1].consumer);
            assert!(app_origins.iter().any(|o| o.constraint == p("app@^1")));
            assert!(app_origins.iter().any(|o| o.constraint == p("app@=1.0.0")));
            assert_eq!(plan.selected_provenance.len(), 3);
            for proof in &plan.selected_provenance {
                assert_eq!(proof.selected.parent, p("app@=1.0.0"));
                assert!(!proof.provider_identity.content.is_empty());
                assert!(!proof.declaration_identity.definition.is_empty());
                let target = &proof.selected.provider;
                assert!(
                    plan.resolution
                        .iter()
                        .any(|n| target.name == n.name && target.version.matches(&n.version))
                );
            }
            assert!(plan.selected_provenance.iter().any(|e| {
                e.selected.declaration.starts_with("requires.capabilities")
                    && e.selected.provider == p("a-provider@=1.0.0")
            }));
            assert!(
                plan.selected_provenance
                    .iter()
                    .any(|e| e.selected.declaration == "requires_any[0]")
            );
            cells += 1;
        }
    }
    assert_eq!(cells, 9);
}

#[test]
fn conflicting_workspace_roots_keep_both_origins() {
    let _isolation = vibe_test_support::isolated_home().unwrap();
    let fixture = Fixture::new();
    fixture.package("app", 1, "");
    fixture.package("app", 2, "");
    std::fs::write(fixture.root.join("vibe.toml"), "[project]\nname='g1'\nversion='1.0.0'\n[workspace]\nmembers=['member']\n[requires.packages]\n'org.g1/app'='=1.0.0'\n").unwrap();
    std::fs::create_dir(fixture.root.join("member")).unwrap();
    std::fs::write(
        fixture.root.join("member/vibe.toml"),
        "[project]\nname='member'\nversion='1.0.0'\n[requires.packages]\n'org.g1/app'='=2.0.0'\n",
    )
    .unwrap();
    for mode in ["local", "multi", "embedded"] {
        for backend in ["resolvo", "naive", "sat"] {
            let source = fixture.source(Some(backend), mode);
            let Err(vibe_install::Error::Solve(SolveError::RootConstraints { origins, .. })) =
                vibe_install::plan(&source, &fixture.root, request(vec![]), &NullObserver)
            else {
                panic!("expected equivalent typed root conflict: {mode}/{backend}");
            };
            assert_eq!(origins.len(), 2);
            assert_ne!(origins[0].consumer, origins[1].consumer);
            assert_eq!(origins[0].constraint, p("app@=1.0.0"));
            assert_eq!(origins[1].constraint, p("app@=2.0.0"));
            assert!(!fixture.root.join("vibe.lock").exists());
        }
    }
}

#[test]
fn visibility_retry_preserves_choices() {
    let _isolation = vibe_test_support::isolated_home().unwrap();
    let fixture = Fixture::new();
    let configured = fixture.configured();
    fixture.package("provider", 1, "[provides]\ncapabilities=['capability:engine@1.0.0']\n[requires.packages]\n'org.g1/dep'={version='*',access='private'}\n");
    // Provider must be transitive for its private edge to be hidden.
    let mut configured = configured;
    configured.roots.remove(0);
    configured.root_origins.remove(0);
    for mode in ["local", "multi", "embedded"] {
        let source = fixture.source(None, mode);
        let frozen = source.bind_request(configured.clone()).unwrap();
        frozen.solve(&configured.roots).unwrap();
        let before = frozen.route_resolution().unwrap();
        assert!(
            before
                .recorded
                .graph
                .packages
                .iter()
                .any(|n| n.name == "dep")
        );
        let plan = ready(&frozen, &fixture, configured.roots.clone());
        assert!(!plan.resolution.iter().any(|n| n.name == "dep"));
        assert_eq!(
            plan.selected_provenance,
            before
                .provenance
                .into_iter()
                .filter(|e| e.selected.provider.name != "dep")
                .collect::<Vec<_>>()
        );
        let after = frozen.route_resolution().unwrap();
        assert_eq!(
            after.effects["org.g1/app@1.0.0"],
            before.effects["org.g1/app@1.0.0"]
        );
        assert_eq!(after.rejected, before.rejected);
        assert_eq!(after.rejected.len(), 1);
        assert_eq!(
            plan.normalized_request
                .unwrap()
                .configuration
                .unwrap()
                .request,
            configured.configuration.as_ref().unwrap().request
        );
    }
}

#[test]
fn invalid_version_and_no_leakage() {
    let _isolation = vibe_test_support::isolated_home().unwrap();
    for backend in ["resolvo", "naive", "sat"] {
        let fixture = Fixture::new();
        fixture.package("app", 1, "[provides]\ncapabilities=['stack:rust']\n[requires.packages]\n'org.g1/dep'='=1.0.0'\n[target.'context(stack:rust)'.dependencies.packages]\n'org.g1/dep'='=2.0.0'\n");
        fixture.package("dep", 1, "");
        fixture.package("dep", 2, "");
        let source = fixture.source(Some(backend), "local");
        // Direct technical solve deliberately omits conditional activation.
        let frozen = source
            .bind_request(NormalizedRequest::legacy(&[p("app")], &Default::default()))
            .unwrap();
        let graph = frozen.solve(&[p("app")]).unwrap();
        assert_eq!(
            graph
                .find(p("dep").group.as_ref().unwrap(), "dep")
                .unwrap()
                .version
                .to_string(),
            "1.0.0"
        );
        let outcome = vibe_install::plan(
            &source,
            &fixture.root,
            request(vec![p("app")]),
            &NullObserver,
        );
        assert!(
            matches!(
                &outcome,
                Err(vibe_install::Error::Solve(
                    SolveError::RootConstraints { .. }
                ))
            ),
            "{backend}: {:?}",
            outcome.err()
        );
        assert!(!fixture.root.join("vibe.lock").exists());
    }
}

#[test]
fn authored_workspace_consumers_reach_configured_source() {
    use vibe_core::manifest::{ConsumerConfiguration, ConsumerSelection, ConsumerSelectionIntent};
    let _isolation = vibe_test_support::isolated_home().unwrap();
    let fixture = Fixture::new();
    fixture.configured();
    std::fs::create_dir(fixture.root.join("member")).unwrap();
    for (path, choice, body) in [
        (
            fixture.root.join("vibe.toml"),
            "a",
            "[workspace]\nmembers=['member']\n[requires.packages]\n'org.g1/provider'='*'\n'org.g1/app'='^1'\n'org.g1/blocker'='*'\n",
        ),
        (
            fixture.root.join("member/vibe.toml"),
            "b",
            "[requires.packages]\n'org.g1/app'='^1'\n",
        ),
    ] {
        let mut manifest = vibe_core::manifest::Manifest::parse_str(&format!(
            "[project]\nname='g1'\nversion='1.0.0'\n{body}"
        ))
        .unwrap();
        manifest.consumer_configuration.push(ConsumerConfiguration {
            dependency: "org.g1/app".into(),
            domain: "g1-domain".into(),
            definition: "g1-definition".into(),
            definition_owner: "g1-owner".into(),
            target: "g1-target".into(),
            target_owner: "g1-owner".into(),
            instance: "g1-app".into(),
            configuration: "g1-config".into(),
            selections: [(
                "choice".into(),
                ConsumerSelection {
                    intent: ConsumerSelectionIntent::Additive,
                    members: Some(vec![choice.into()]),
                },
            )]
            .into(),
        });
        manifest.write(path).unwrap();
    }
    for mode in ["local", "multi", "embedded"] {
        for backend in ["resolvo", "naive", "sat"] {
            let source = fixture.source(Some(backend), mode);
            let result = vibe_install::plan(&source, &fixture.root, request(vec![]), &NullObserver);
            if backend != "resolvo" {
                assert!(
                    matches!(result, Err(vibe_install::Error::Solve(SolveError::BackendCapability { backend: selected, .. })) if selected == backend)
                );
                continue;
            }
            let Plan::Ready(plan) = result.unwrap() else {
                panic!("expected authored configuration plan");
            };
            let normalized = plan.normalized_request.as_ref().unwrap();
            let input = normalized.configuration.as_ref().unwrap();
            assert_eq!(input.request.edges.len(), 2);
            assert_ne!(
                input.request.edges[0].consumer,
                input.request.edges[1].consumer
            );
            assert_eq!(
                input.request.edges[0].selections["choice"].members,
                Some(vec!["a".into()])
            );
            assert_eq!(
                input.request.edges[1].selections["choice"].members,
                Some(vec!["b".into()])
            );
            assert_eq!(
                normalized
                    .root_origins
                    .iter()
                    .filter(|o| o.constraint.name == "app")
                    .count(),
                2
            );
            assert!(plan.resolution.iter().any(|n| n.name == "good"));
            assert!(!plan.resolution.iter().any(|n| n.name == "bad"));
            assert_eq!(plan.selected_provenance.len(), 3);
            assert!(
                plan.selected_provenance
                    .iter()
                    .any(
                        |e| e.selected.declaration == "configuration.dependencies[org.g1/good]"
                            && e.selected.provider == p("good@=1.0.0")
                    )
            );
        }
    }
}
