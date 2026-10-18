// Shared adapter fixture. Production entrypoint claims must still name their
// host constructor: this fixture explicitly configures retained history.
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/next/derivatives/service_fixture.rs"
));

struct InspectionFixture {
    source: Fixture,
    route: std::sync::Arc<vibe_orchestrator::machine_operation::boot_debugger::BootDebuggerRoute>,
    snapshot: String,
    provider_calls: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    catalog: vibe_workspace::source_query::derivatives::DerivativeCatalog,
}
impl InspectionFixture {
    fn new(family: &str) -> Self {
        use vibe_orchestrator::{document_analysis::*, machine_operation::boot_debugger::*};
        let source = Fixture::new(family);
        let result = vibe_workspace::derivatives::MaterializedDerivative::derive(
            &source.text,
            &source.base,
            &source.local,
            &source.output,
            "result",
        )
        .unwrap();
        let catalog = vibe_workspace::source_query::derivatives::DerivativeCatalog::new(vec![
            result.result().record().clone(),
        ])
        .unwrap();
        let root = source.output.root_path();
        fs::write(
            root.join("vibe.toml"),
            "[project]\nname='inspection'\nversion='1.0.0'\n",
        )
        .unwrap();
        let artifact = "<!-- vibe:static org.example/local — inert-data -->\nCaptured boot summary has no semantic resource join.\n";
        fs::write(root.join("STATIC.md"), artifact).unwrap();
        let host = DocumentAnalysisHost::new(
            root,
            "vibe.toml",
            vec![
                ResourceDeclaration {
                    path: "vibe.toml".into(),
                    role: ResourceRole::Manifest,
                },
                ResourceDeclaration {
                    path: "STATIC.md".into(),
                    role: ResourceRole::Opaque,
                },
            ],
            AnalysisLimits {
                resource_bytes: 8192,
                capture_bytes: 16384,
                open_buffers: 2,
            },
        )
        .unwrap();
        let artifact = host.resource("STATIC.md").unwrap();
        let capture = host.client().unwrap().freeze().unwrap();
        let provider_calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let route = std::sync::Arc::new(
            BootDebuggerRoute::new(host, artifact, CapturedKind::Static)
                .unwrap()
                .with_scenario_producer(std::sync::Arc::new(RejectingInspectionProducer(
                    provider_calls.clone(),
                ))),
        );
        let snapshot = route.retain_derivative_capture(capture, &catalog).unwrap();
        let (route, snapshot) = if family == "vibe-spec" {
            Self::compiled_route(&source, &catalog)
        } else {
            (route, snapshot)
        };
        Self {
            source,
            route,
            snapshot,
            provider_calls,
            catalog,
        }
    }
    fn compiled_route(
        source: &Fixture,
        catalog: &vibe_workspace::source_query::derivatives::DerivativeCatalog,
    ) -> (
        std::sync::Arc<vibe_orchestrator::machine_operation::boot_debugger::BootDebuggerRoute>,
        String,
    ) {
        use vibe_orchestrator::machine_operation::boot_debugger::{
            CapturedKind, debugger_request as q, debugger_result as r,
        };
        let root = source.output.root_path();
        fs::write(root.join("vibe.toml"), "[project]\nname='inspection'\nversion='1.0.0'\n[requires.packages]\n'org.example/app'={version='=1.0.0',link='static'}\n'org.example/local'={version='=1.0.0',link='dynamic'}\n").unwrap();
        let registry = root.join("vibevm/vibepacks");
        let local = registry.join("org.example/local/v1.0.0");
        fs::create_dir_all(&local).unwrap();
        fs::write(local.join("vibe.toml"), format!("[package]\ngroup='org.example'\nname='local'\nkind='flow'\nversion='1.0.0'\nformat='normal'\n[[package.sources]]\n{}", toml::to_string(&source.binding.local().descriptor).unwrap())).unwrap();
        fs::write(
            source.local.root_path().join("vibe.toml"),
            fs::read(local.join("vibe.toml")).unwrap(),
        )
        .unwrap();
        for file in &source.binding.local().files {
            fs::create_dir_all(local.join(&file.path).parent().unwrap()).unwrap();
            fs::write(local.join(&file.path), &file.bytes).unwrap();
        }
        let app = registry.join("org.example/app/v1.0.0");
        fs::create_dir_all(app.join("vibevm/vibespecs/boot")).unwrap();
        fs::write(app.join("vibe.toml"), "[package]\ngroup='org.example'\nname='app'\nkind='flow'\nversion='1.0.0'\nformat='normal'\n[boot_snippet]\nsource='vibevm/vibespecs/boot/entry.md'\n[[boot_snippet.fragment]]\nsource='vibevm/vibespecs/boot/repeated.md'\n[requires.packages]\n'org.example/local'='=1.0.0'\n").unwrap();
        fs::write(
            app.join("vibevm/vibespecs/boot/entry.md"),
            "# Root {#root}\n\n#use spec://org.example/local/common/POLICY#RETRY\n",
        )
        .unwrap();
        fs::write(
            app.join("vibevm/vibespecs/boot/repeated.md"),
            "# Repeated {#repeated}\n\n#use spec://org.example/local/common/POLICY#RETRY\n",
        )
        .unwrap();
        let route = vibe_package_source::InstallResolver::open_boot_debugger(
            root,
            Some(&registry),
            "STATIC.md",
            CapturedKind::Static,
        )
        .unwrap();
        route.configure_derivatives(catalog.clone()).unwrap();
        let captured = route
            .request(q::Request {
                schema: 1,
                context_id: route.context_id().into(),
                command: q::Command::Capture(Box::new(q::CommandCapture {})),
            })
            .unwrap();
        let snapshot = captured.snapshot_id.unwrap();
        let produced = route
            .request(q::Request {
                schema: 1,
                context_id: route.context_id().into(),
                command: q::Command::Scenario(Box::new(q::CommandScenario {
                    snapshot_id: snapshot,
                    target: "os:linux".into(),
                    draft_json: serde_json::to_string(
                        &vibe_workspace::boot_debugger::scenario::Draft {
                            baseline_selections: Default::default(),
                            edits: vec![],
                        },
                    )
                    .unwrap(),
                })),
            })
            .unwrap();
        let r::Body::Preview(body) = produced.body else {
            panic!("expected prepared builtin preview")
        };
        assert_eq!(
            body.status,
            r::BodyPreviewStatus::Complete,
            "{:?}",
            body.missing
        );
        assert!(body.artifact.as_ref().unwrap().contains("RETRY"));
        (
            route,
            body.produced_snapshot_id
                .expect("same-invocation retained capture"),
        )
    }
    fn inspect(&self) -> vibe_wire::generated::boot_debugger::e1::request::Request {
        use vibe_wire::generated::boot_debugger::e1::request as q;
        q::Request {
            schema: 1,
            context_id: self.route.context_id().into(),
            command: q::Command::Inspect(Box::new(q::CommandInspect {
                snapshot_id: self.snapshot.clone(),
            })),
        }
    }
}

struct RejectingInspectionProducer(std::sync::Arc<std::sync::atomic::AtomicUsize>);
impl vibe_orchestrator::machine_operation::boot_debugger::scenarios::ScenarioProducer
    for RejectingInspectionProducer
{
    fn capture(
        &self,
        _: vibe_workspace::boot_debugger::hypothetical::HostCapture,
        _: &vibe_workspace::boot_debugger::scenario::Draft,
        _: &str,
    ) -> Result<
        vibe_workspace::boot_debugger::hypothetical::PreparedScenario,
        vibe_orchestrator::machine_operation::boot_debugger::scenarios::ScenarioFailure,
    > {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Err(vibe_orchestrator::machine_operation::boot_debugger::scenarios::ScenarioFailure::Incomplete("inspection invoked a producer".into()))
    }
}
fn inspection_files(root: &Path) -> Vec<(String, Option<Vec<u8>>)> {
    fn walk(root: &Path, at: &Path, rows: &mut Vec<(String, Option<Vec<u8>>)>) {
        let metadata = fs::symlink_metadata(at).unwrap();
        assert!(!metadata.file_type().is_symlink());
        rows.push((
            at.strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            metadata.is_file().then(|| fs::read(at).unwrap()),
        ));
        if metadata.is_dir() {
            for entry in fs::read_dir(at).unwrap() {
                walk(root, &entry.unwrap().path(), rows);
            }
        }
    }
    let mut rows = Vec::new();
    walk(root, root, &mut rows);
    rows.sort();
    rows
}
