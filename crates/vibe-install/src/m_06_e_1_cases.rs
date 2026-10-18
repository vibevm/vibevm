//! Real admitted configuration transactions and the metadata-only projection.
use vibe_install::InstallSource;
use vibe_resolver as resolver;
#[path = "../../../vibevm/vibepacks/org.vibevm.world/redbook/v1.1.0/tests/f1_fixture.rs"]
mod fixture;

#[test]
fn selected_configuration_and_visibility_are_exact_without_fetch_or_apply() {
    let temp = tempfile::tempdir().unwrap();
    let registry = temp.path().join("registry");
    fixture::registry(&registry);
    let root = temp.path().join("project");
    let host = fixture::project(&root);
    let before = std::fs::read(root.join("vibe.toml")).unwrap();
    let source = vibe_package_source::build_install_resolver(
        &vibe_package_source::PackageSourceOptions {
            registry: Some(registry),
            ..Default::default()
        },
        &host,
        None,
        &root,
        &Default::default(),
        true,
        &[],
    )
    .unwrap();
    for branch in ["mup", "wal"] {
        let request = fixture::intent(Some(branch), &[]);
        let transaction = source.bind_request(request.clone()).unwrap();
        let graph =
            vibe_install::resolve_boot_comparison(&transaction, &request.roots, &host).unwrap();
        let route = transaction.route_resolution().unwrap();
        assert_eq!(route.recorded.graph, graph);
        assert_eq!(
            transaction
                .normalized_request()
                .unwrap()
                .configuration
                .as_ref()
                .unwrap()
                .request,
            request.configuration.as_ref().unwrap().request
        );
        let names = graph
            .iter()
            .map(|n| n.name.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(names, fixture::expected_names(branch, &[]));
        assert!(
            route
                .provenance
                .iter()
                .any(|p| p.configured_definition.is_some())
        );
        assert!(route.provenance.iter().all(|p| {
            graph.iter().any(|n| {
                p.selected.provider.group.as_ref() == Some(&n.group)
                    && p.selected.provider.name == n.name
            })
        }));
        assert_eq!(std::fs::read(root.join("vibe.toml")).unwrap(), before);
        assert!(!root.join("vibe.lock").exists());
    }
}
