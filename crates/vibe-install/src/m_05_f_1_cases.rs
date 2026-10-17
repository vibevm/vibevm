//! Exercise the external production crate: this safe integration harness is
//! also compiled from lib.rs, so its own local trait is a distinct type.
use vibe_install::{InstallRequest, InstallSource, NullObserver, Plan};
use vibe_resolver as resolver;
use vibe_resolver::resolvo_engine::configuration::ConfigurationSolveError;

#[path = "../../../vibevm/vibepacks/org.vibevm.world/redbook/v1.1.0/tests/f1_fixture.rs"]
mod fixture;

#[test]
fn normal_local_source_and_planner_keep_original_redbook_intent() {
    let isolated = vibe_test_support::isolated_home().expect("test settings isolation linked");
    let temp = tempfile::tempdir().unwrap();
    let registry = temp.path().join("registry");
    fixture::registry(&registry);
    fixture::unused_alternative(&registry);
    let root = temp.path().join("project");
    let manifest = fixture::project(&root);
    let original = std::fs::read(root.join("vibe.toml")).unwrap();
    let source = vibe_package_source::build_install_resolver(
        &vibe_package_source::PackageSourceOptions {
            registry: Some(registry.clone()),
            ..Default::default()
        },
        &manifest,
        None,
        &root,
        &Default::default(),
        true,
        &[],
    )
    .unwrap();
    for branch in ["mup", "wal"] {
        let additions: &[(&str, &[&str])] = &[
            ("additions", &["health-audit", "manual-tests"]),
            ("mup-additions", &["discovery-prompt"]),
            ("wal-additions", &["wal-specspaces"]),
        ];
        let intent = fixture::intent(Some(branch), additions);
        let authored = intent.configuration.as_ref().unwrap().request.clone();
        let bound = source.bind_request(intent).unwrap();
        let graph = bound.solve(&[fixture::p("redbook")]).unwrap();
        let active = if branch == "mup" {
            vec!["health-audit", "manual-tests", "discovery-prompt"]
        } else {
            vec!["health-audit", "manual-tests", "wal-specspaces"]
        };
        assert_eq!(
            graph
                .iter()
                .map(|n| n.name.to_string())
                .collect::<std::collections::BTreeSet<_>>(),
            fixture::expected_names(branch, &active)
        );
        let Plan::Ready(plan) = vibe_install::plan(
            &bound,
            &root,
            InstallRequest {
                roots: vec![],
                features: Default::default(),
                language: None,
                exact: false,
                generated_by: "F1 isolated fixture".into(),
                offline: true,
            },
            &NullObserver,
        )
        .unwrap() else {
            panic!("expected prepared install")
        };
        let admitted = plan.normalized_request.as_ref().unwrap();
        assert_eq!(admitted.configuration.as_ref().unwrap().request, authored);
        assert!(!plan.selected_provenance.is_empty());
        assert!(
            plan.selected_provenance
                .iter()
                .filter(|e| e.selected.parent.name == "redbook")
                .all(|e| e.configured_definition.is_some()
                    && e.selected
                        .declaration
                        .starts_with("configuration.dependencies["))
        );
        let record = vibe_install::requested_record(admitted).unwrap();
        assert_eq!(
            record.configuration,
            Some(serde_json::to_string(&authored).unwrap())
        );
        assert_eq!(record.definitions.len(), 1);
        assert!(record.defaults.is_empty());
        let selected = bound.route_resolution().unwrap();
        assert_eq!(
            selected
                .recorded
                .graph
                .iter()
                .map(|n| n.name.to_string())
                .collect::<std::collections::BTreeSet<_>>(),
            fixture::expected_names(branch, &active)
        );
        let effects = &selected.effects["org.vibevm.world/redbook@1.1.0"];
        assert!(effects.active_features.contains(branch));
        assert!(!effects.active_features.contains(if branch == "mup" {
            "wal-specspaces"
        } else {
            "discovery-prompt"
        }));
        assert_eq!(std::fs::read(root.join("vibe.toml")).unwrap(), original);
        assert!(!root.join("vibe.lock").exists());
    }
    let settings_before = snapshot(isolated);
    let project_before = snapshot(&root);
    let omitted = source.bind_request(fixture::intent(None, &[])).unwrap();
    assert!(
        matches!(omitted.solve(&[fixture::p("redbook")]), Err(vibe_resolver::SolveError::Configuration(e)) if matches!(*e, ConfigurationSolveError::NeedsSelection { .. }))
    );
    assert!(
        vibe_install::plan(
            &omitted,
            &root,
            InstallRequest {
                roots: vec![],
                features: Default::default(),
                language: None,
                exact: false,
                generated_by: "F1 no input".into(),
                offline: true,
            },
            &NullObserver
        )
        .is_err()
    );
    let mut both = fixture::intent(Some("mup"), &[]);
    both.configuration.as_mut().unwrap().request.edges[0]
        .selections
        .get_mut("continuity")
        .unwrap()
        .members = Some(vec!["mup".into(), "wal".into()]);
    let both = source.bind_request(both).unwrap();
    assert!(
        matches!(both.solve(&[fixture::p("redbook")]), Err(vibe_resolver::SolveError::Configuration(e)) if matches!(*e, ConfigurationSolveError::Conflict { .. }))
    );
    assert!(
        vibe_install::plan(
            &both,
            &root,
            InstallRequest {
                roots: vec![],
                features: Default::default(),
                language: None,
                exact: false,
                generated_by: "F1 conflicting input".into(),
                offline: true,
            },
            &NullObserver
        )
        .is_err()
    );
    assert_eq!(std::fs::read(root.join("vibe.toml")).unwrap(), original);
    assert!(!root.join("vibe.lock").exists());
    let unbound = source
        .bind_request(
            vibe_resolver::resolvo_engine::request::NormalizedRequest::legacy(
                &[fixture::p("redbook")],
                &Default::default(),
            ),
        )
        .unwrap();
    assert!(matches!(unbound.solve(&[fixture::p("redbook")]),
        Err(vibe_resolver::SolveError::UnsupportedRoute(message)) if message.contains("current consumer/domain input")));
    let roots = vec![fixture::p("f1-alternative")];
    let ordinary = source
        .bind_request(
            resolver::resolvo_engine::request::NormalizedRequest::legacy(
                &roots,
                &Default::default(),
            ),
        )
        .unwrap();
    let graph = ordinary.solve(&roots).unwrap();
    assert_eq!(
        graph
            .iter()
            .map(|n| n.name.to_string())
            .collect::<std::collections::BTreeSet<_>>(),
        ["f1-alternative".to_owned(), "f1-plain".to_owned()].into()
    );
    let configured = fixture::alternative_intent(&registry);
    let configured = source.bind_request(configured).unwrap();
    let graph = configured.solve(&roots).unwrap();
    assert_eq!(
        graph
            .iter()
            .map(|n| n.name.to_string())
            .collect::<std::collections::BTreeSet<_>>(),
        ["f1-alternative".to_owned(), "f1-plain".to_owned()].into()
    );
    assert_eq!(snapshot(isolated), settings_before);
    assert_eq!(snapshot(&root), project_before);
}

fn snapshot(
    root: &std::path::Path,
) -> std::collections::BTreeMap<std::path::PathBuf, Option<Vec<u8>>> {
    fn visit(
        root: &std::path::Path,
        current: &std::path::Path,
        out: &mut std::collections::BTreeMap<std::path::PathBuf, Option<Vec<u8>>>,
    ) {
        for entry in std::fs::read_dir(current).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                out.insert(path.strip_prefix(root).unwrap().to_owned(), None);
                visit(root, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(root).unwrap().to_owned(),
                    Some(std::fs::read(&path).unwrap()),
                );
            }
        }
    }
    let mut out = Default::default();
    visit(root, root, &mut out);
    out
}
