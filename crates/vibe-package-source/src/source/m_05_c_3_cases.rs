use super::*;
// Production planning uses the process-local test store; fixtures reuse coordinates.
static STORE: std::sync::Mutex<()> = std::sync::Mutex::new(());
use vibe_core::manifest::{feature_constraints::*, feature_effects::*};
use vibe_resolver::features::configuration::ConfigurationDefinition;
use vibe_resolver::resolvo_engine::{configuration::CandidateConfiguration, request::*};
fn p(n: &str) -> PackageRef {
    PackageRef::parse(&format!("org.example/{n}")).unwrap()
}
fn fixture(
    solver: Option<&'static str>,
) -> (tempfile::TempDir, InstallResolver, NormalizedRequest) {
    let dir = tempfile::tempdir().unwrap();
    for (name, body) in [
        (
            "app",
            "[requires.packages]\n'org.example/dep'='*'\n[[requires_any]]\none_of=['org.example/missing','org.example/provider']\n[requires]\ncapabilities=['capability:engine@1.0.0']\n",
        ),
        ("dep", ""),
        (
            "provider",
            "[provides]\ncapabilities=['capability:engine@1.0.0']\n",
        ),
    ] {
        let path = dir.path().join(format!("org.example/{name}/v1.0.0"));
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(path.join("vibe.toml"),format!("[package]\ngroup='org.example'\nname='{name}'\nkind='flow'\nversion='1.0.0'\n{body}")).unwrap();
    }
    let source = InstallResolver::Local(
        crate::cells::local_registry(dir.path().into()).unwrap(),
        solver,
    );
    let metadata = vibe_registry::CandidateMetadata::read_local_registry(
        &dir.path().join("org.example/app/v1.0.0"),
    )
    .unwrap();
    let mut wire = consumer_codec::decode_request(include_bytes!(
        "../../../../formats/corpora/consumer-configuration-request/e1/valid/omitted.json"
    ))
    .unwrap();
    wire.edges[0].package = consumer_wire::Package::Vibe(Box::new(consumer_wire::PackageVibe {
        reference: "org.example/app".into(),
    }));
    wire.edges[0].features = vec!["off".into()];
    wire.edges[0].selections.clear();
    let declarations = FeatureDeclarations::from_requires(
        &metadata.manifest.requires,
        &[DependencyDeclarationId::new("org.example/dep").unwrap()].into(),
    )
    .unwrap();
    let definition = CandidateConfiguration {
        package: p("app"),
        version: metadata.manifest.require_package().unwrap().version.clone(),
        identity: metadata.identity,
        definition: ConfigurationDefinition {
            instance: consumer_codec::admit_request(&wire).unwrap()[0]
                .instance
                .clone(),
            program: FeatureProgram {
                features: [
                    (LocalFeatureRef::new("off").unwrap(), Default::default()),
                    (
                        LocalFeatureRef::new("on").unwrap(),
                        [FeatureEffect::ActivateDependency(
                            DependencyDeclarationId::new("org.example/dep").unwrap(),
                        )]
                        .into(),
                    ),
                ]
                .into(),
                exclusive: Default::default(),
            },
            declarations,
            constraints: Constraints::default(),
        },
    };
    let mut request = NormalizedRequest::legacy(&[p("app")], &Default::default());
    request.configuration = Some(ConfigurationInput {
        request: wire,
        definitions: vec![definition],
        defaults: vec![],
    });
    (dir, source, request)
}
#[test]
fn same_request_catalog_across_solve_metadata_and_masked_routes() {
    let _store = STORE.lock().unwrap_or_else(|e| e.into_inner());
    let (_dir, source, request) = fixture(None);
    let frozen = source.bind_request(request.clone()).unwrap();
    let before = frozen.catalog_inspection().unwrap().clone();
    let graph = frozen.solve(&request.roots).unwrap();
    assert_eq!(graph.packages.len(), 2);
    assert!(
        frozen
            .manifest_of(&p("app@=1.0.0"))
            .unwrap()
            .requires
            .packages
            .is_empty()
    );
    assert_eq!(
        frozen
            .catalog_manifest_of(&p("app@=1.0.0"))
            .unwrap()
            .requires
            .packages
            .len(),
        1
    );
    let first = frozen.route_resolution().unwrap();
    assert_eq!(first.provenance.len(), 2);
    assert!(
        first
            .provenance
            .iter()
            .all(|e| e.selected.provider == p("provider@=1.0.0")
                && e.configured_definition.is_some())
    );
    let retried = frozen
        .solve_masked(&request.roots, &Default::default())
        .unwrap();
    assert_eq!(format!("{graph:?}"), format!("{retried:?}"));
    let second = frozen.route_resolution().unwrap();
    assert_eq!(first.provenance, second.provenance);
    assert_eq!(first.effects, second.effects);
    assert_eq!(&before, frozen.catalog_inspection().unwrap());
    assert_eq!(
        request.configuration.as_ref().unwrap().request,
        frozen
            .normalized_request()
            .unwrap()
            .configuration
            .as_ref()
            .unwrap()
            .request
    );
    // Exercise the public production planner with this exact configured source.
    // The returned plan retains both normalized intent and selected provenance.
    let project = tempfile::tempdir().unwrap();
    std::fs::write(
        project.path().join("vibe.toml"),
        "[project]\nname='configured'\nversion='1.0.0'\n",
    )
    .unwrap();
    let planned = vibe_install::plan(
        &frozen,
        project.path(),
        vibe_install::InstallRequest {
            roots: request.roots.clone(),
            features: Default::default(),
            language: None,
            exact: false,
            generated_by: "C3 fixture".into(),
            offline: true,
        },
        &vibe_install::NullObserver,
    )
    .unwrap();
    let vibe_install::Plan::Ready(mut planned) = planned else {
        panic!("configured request must plan")
    };
    assert_eq!(planned.selected_provenance, first.provenance);
    // Empty legacy roots cannot fast-path away explicitly admitted intent.
    let bare = vibe_install::plan(
        &frozen,
        project.path(),
        vibe_install::InstallRequest {
            roots: vec![],
            features: Default::default(),
            language: None,
            exact: false,
            generated_by: "C3 bare".into(),
            offline: true,
        },
        &vibe_install::NullObserver,
    )
    .unwrap();
    let vibe_install::Plan::Ready(bare) = bare else {
        panic!("admitted intent was discarded");
    };
    assert_eq!(bare.selected_provenance, first.provenance);
    assert_eq!(bare.resolution.len(), planned.resolution.len());

    assert_eq!(
        planned
            .normalized_request
            .as_ref()
            .unwrap()
            .configuration
            .as_ref()
            .unwrap()
            .request,
        request.configuration.as_ref().unwrap().request
    );
    // A graph changed after confirmation must refuse before any project write.
    planned.resolution[0].requires.clear();
    let before_manifest = std::fs::read(project.path().join("vibe.toml")).unwrap();
    assert!(matches!(
        vibe_install::apply(&frozen, *planned, Default::default(), &Default::default()),
        Err(vibe_install::Error::Solve(
            vibe_resolver::SolveError::UnsupportedRoute(_)
        ))
    ));
    assert_eq!(
        std::fs::read(project.path().join("vibe.toml")).unwrap(),
        before_manifest
    );
    assert!(!project.path().join("vibe.lock").exists());
    let mut on = request.clone();
    on.configuration.as_mut().unwrap().request.edges[0].features = vec!["on".into()];
    let activated = source.bind_request(on.clone()).unwrap();
    let active = activated.solve(&on.roots).unwrap();
    assert_eq!(active.packages.len(), 3);
    assert!(activated.route_resolution().unwrap().provenance.iter().any(
        |e| e.selected.declaration == "configuration.dependencies[org.example/dep]"
            && e.selected.provider == p("dep@=1.0.0")
    ));
    let mask = [("org.example/app".into(), "org.example/dep".into())].into();
    let masked = activated.solve_masked(&on.roots, &mask).unwrap();
    assert_eq!(masked.packages.len(), 2);
    assert_eq!(
        activated
            .manifest_of(&p("app@=1.0.0"))
            .unwrap()
            .requires
            .packages,
        vec![p("dep@*")]
    );
    assert_eq!(
        activated
            .route_resolution()
            .unwrap()
            .provenance
            .iter()
            .filter(|e| e.selected.declaration.starts_with("requires_any")
                || e.selected.declaration.starts_with("requires.capabilities"))
            .cloned()
            .collect::<Vec<_>>(),
        first.provenance
    );
    let mut duplicate = on.clone();
    let definition = &mut duplicate.configuration.as_mut().unwrap().definitions[0].definition;
    let alias = DependencyDeclarationId::new("alias").unwrap();
    definition.declarations.dependencies.insert(
        alias.clone(),
        definition.declarations.dependencies
            [&DependencyDeclarationId::new("org.example/dep").unwrap()]
            .clone(),
    );
    definition
        .program
        .features
        .get_mut(&LocalFeatureRef::new("on").unwrap())
        .unwrap()
        .insert(FeatureEffect::ActivateDependency(alias));
    let duplicate = source.bind_request(duplicate).unwrap();
    duplicate.solve(&request.roots).unwrap();
    let declared: std::collections::BTreeSet<_> = duplicate
        .route_resolution()
        .unwrap()
        .provenance
        .into_iter()
        .filter(|e| e.selected.provider.name == "dep")
        .map(|e| e.selected.declaration)
        .collect();
    assert_eq!(
        declared,
        [
            "configuration.dependencies[alias]".into(),
            "configuration.dependencies[org.example/dep]".into()
        ]
        .into()
    );
    let conflict = [p("app@=2.0.0")];
    assert!(matches!(
        frozen.solve(&conflict),
        Err(vibe_resolver::SolveError::Configuration(_))
    ));
    assert!(matches!(
        frozen.solve_masked(&conflict, &Default::default()),
        Err(vibe_resolver::SolveError::Configuration(_))
    ));
}
#[test]
fn unsupported_backend_refuses_without_fallback() {
    let _store = STORE.lock().unwrap_or_else(|e| e.into_inner());
    for backend in ["naive", "sat"] {
        let (_dir, source, request) = fixture(Some(backend));
        let frozen = source.bind_request(request.clone()).unwrap();
        for result in [
            frozen.solve(&request.roots),
            frozen.solve_masked(&request.roots, &Default::default()),
        ] {
            assert!(
                matches!(result,Err(vibe_resolver::SolveError::BackendCapability{backend:ref selected,..}) if selected==backend)
            );
        }
        assert!(
            matches!(frozen.manifest_of(&p("app")),Err(vibe_resolver::SolveError::BackendCapability{backend:ref selected,..}) if selected==backend)
        );
        assert!(frozen.route_resolution().is_none());
    }
    for backend in ["resolvo", "naive", "sat"] {
        let (dir, source, mut request) = fixture(Some(backend));
        let newer = dir.path().join("org.example/dep/v2.0.0");
        std::fs::create_dir_all(&newer).unwrap();
        std::fs::write(
            newer.join("vibe.toml"),
            "[package]\ngroup='org.example'\nname='dep'\nkind='flow'\nversion='2.0.0'\n",
        )
        .unwrap();
        let provider = dir.path().join("org.example/provider/v1.0.0/vibe.toml");
        let mut text = std::fs::read_to_string(&provider).unwrap();
        text.push_str("[requires.packages]\n'org.example/dep'='=1.0.0'\n");
        std::fs::write(provider, text).unwrap();
        request.configuration = None;
        request.roots = vec![p("provider"), p("app")];
        let frozen = source.bind_request(request.clone()).unwrap();
        frozen.solve(&request.roots).unwrap();
        let before = frozen.route_resolution().unwrap();
        let blocked = [("org.example/provider".into(), "org.example/dep".into())].into();
        let graph = frozen.solve_masked(&request.roots, &blocked).unwrap();
        let after = frozen.route_resolution().unwrap();
        assert_eq!(
            graph
                .find(p("dep").group.as_ref().unwrap(), "dep")
                .unwrap()
                .version
                .to_string(),
            "1.0.0"
        );
        assert_eq!(
            after.provenance,
            before
                .provenance
                .iter()
                .filter(|e| !(e.selected.parent.name == "provider"
                    && e.selected.provider.name == "dep"))
                .cloned()
                .collect::<Vec<_>>()
        );
        assert_eq!(
            graph,
            frozen.solve_masked(&request.roots, &blocked).unwrap()
        );
        let direct = source.bind_request(request.clone()).unwrap();
        assert_eq!(
            graph,
            direct.solve_masked(&request.roots, &blocked).unwrap()
        );
    }
}

#[test]
fn production_visibility_retry_retains_configured_choices() {
    let _store = STORE.lock().unwrap_or_else(|e| e.into_inner());
    let (dir, source, request) = fixture(None);
    let path = dir.path().join("org.example/provider/v1.0.0/vibe.toml");
    let mut text = std::fs::read_to_string(&path).unwrap();
    text.push_str("[requires.packages]\n'org.example/dep'={version='*',access='private'}\n");
    std::fs::write(path, text).unwrap();
    let frozen = source.bind_request(request.clone()).unwrap();
    frozen.solve(&request.roots).unwrap();
    let before = frozen.route_resolution().unwrap();
    assert_eq!(before.recorded.graph.packages.len(), 3);
    let project = tempfile::tempdir().unwrap();
    std::fs::write(
        project.path().join("vibe.toml"),
        "[project]\nname='retry'\nversion='1.0.0'\n",
    )
    .unwrap();
    let vibe_install::Plan::Ready(plan) = vibe_install::plan(
        &frozen,
        project.path(),
        vibe_install::InstallRequest {
            roots: request.roots.clone(),
            features: Default::default(),
            language: None,
            exact: false,
            generated_by: "C3".into(),
            offline: true,
        },
        &vibe_install::NullObserver,
    )
    .unwrap() else {
        panic!("expected selected plan");
    };
    assert_eq!(plan.resolution.len(), 2);
    assert_eq!(
        plan.selected_provenance,
        before
            .provenance
            .into_iter()
            .filter(|p| p.selected.provider.name != "dep")
            .collect::<Vec<_>>()
    );
    assert_eq!(
        plan.normalized_request
            .unwrap()
            .configuration
            .unwrap()
            .request,
        request.configuration.unwrap().request
    );
    assert_eq!(
        frozen
            .route_resolution()
            .unwrap()
            .recorded
            .graph
            .packages
            .len(),
        2
    );
}

#[test]
fn production_conditional_route_retains_selected_declaration() {
    let _store = STORE.lock().unwrap_or_else(|e| e.into_inner());
    let (dir, source, mut request) = fixture(None);
    std::fs::write(dir.path().join("org.example/app/v1.0.0/vibe.toml"),
        "[package]\ngroup='org.example'\nname='app'\nkind='flow'\nversion='1.0.0'\n[provides]\ncapabilities=['stack:rust']\n[target.'context(stack:rust)'.dependencies.packages]\n'org.example/dep'='^1'\n").unwrap();
    request.configuration = None;
    let frozen = source.bind_request(request.clone()).unwrap();
    let project = tempfile::tempdir().unwrap();
    std::fs::write(
        project.path().join("vibe.toml"),
        "[project]\nname='conditional'\nversion='1.0.0'\n",
    )
    .unwrap();
    let vibe_install::Plan::Ready(plan) = vibe_install::plan(
        &frozen,
        project.path(),
        vibe_install::InstallRequest {
            roots: request.roots.clone(),
            features: Default::default(),
            language: None,
            exact: false,
            generated_by: "C3".into(),
            offline: true,
        },
        &vibe_install::NullObserver,
    )
    .unwrap() else {
        panic!("expected selected plan");
    };
    assert_eq!(plan.resolution.len(), 2);
    assert_eq!(plan.selected_provenance.len(), 1);
    let edge = &plan.selected_provenance[0].selected;
    assert_eq!(
        edge.declaration,
        "target[context(stack:rust)].dependencies.packages[0]"
    );
    assert_eq!(edge.parent, p("app@=1.0.0"));
    assert_eq!(edge.provider, p("dep@=1.0.0"));
    assert_eq!(plan.normalized_request.unwrap().roots, request.roots);
    let first = frozen.route_resolution().unwrap();
    assert_eq!(
        first.recorded.graph,
        frozen
            .solve_masked(&request.roots, &Default::default())
            .unwrap()
    );
    assert_eq!(
        first.provenance,
        frozen.route_resolution().unwrap().provenance
    );
}
