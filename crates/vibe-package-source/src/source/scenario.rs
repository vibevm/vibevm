//! Scenario normalization stays with the incumbent package-source composition.
//! This is explicit finite source capture, not retained inspection or install.
use super::InstallResolver;
use vibe_orchestrator::machine_operation::boot_debugger::{
    hypothetical::{HostCapture, PreparedScenario},
    scenario::{Draft, Selections},
};
use vibe_resolver::{
    FeatureRequest,
    resolvo_engine::request::{ConfigurationInput, NormalizedRequest},
};

impl InstallResolver {
    /// Capture two complete selected worlds from an immutable draft. Both
    /// worlds use the real normalized resolver and captured builtin compiler
    /// authority; this method never calls the effectful install planner.
    /// The caller must separately retain/bind the result before wire preview.
    /// Source capture failures are not successful empty worlds.
    pub fn capture_boot_scenario(
        &self,
        baseline: HostCapture,
        draft: &Draft,
        target: String,
    ) -> Result<PreparedScenario, CaptureFailure> {
        self.capture_boot_scenario_with_derivatives(baseline, draft, target, &Default::default())
    }
    pub fn capture_boot_scenario_with_derivatives(
        &self,
        baseline: HostCapture,
        draft: &Draft,
        target: String,
        derivatives: &vibe_orchestrator::machine_operation::boot_debugger::DerivativeCatalog,
    ) -> Result<PreparedScenario, CaptureFailure> {
        let candidate = draft
            .candidate(&baseline, &target)
            .map_err(CaptureFailure::Incompatible)?;
        let baseline_request = request(&baseline, &draft.baseline_selections, "baseline");
        let candidate_request = request(
            &candidate.host,
            &candidate.selections,
            &candidate.draft_identity,
        );
        let baseline =
            self.capture_boot_world_with_derivatives(baseline_request, baseline, derivatives)?;
        let candidate = self.capture_boot_world_with_derivatives(
            candidate_request,
            candidate.host,
            derivatives,
        )?;
        PreparedScenario::new(baseline, candidate, target).map_err(CaptureFailure::Incompatible)
    }
}

fn request(host: &HostCapture, selections: &Selections, identity: &str) -> NormalizedRequest {
    let mut request = NormalizedRequest::legacy(
        &host.manifest.requires.packages,
        &FeatureRequest {
            explicit: selections.features.clone(),
            no_defaults: selections.no_defaults,
            all: selections.all_features,
        },
    );
    for root in &mut request.root_origins {
        root.consumer = format!("boot-scenario:{}:{identity}", host.id);
    }
    request.configuration = selections
        .configuration
        .clone()
        .map(|request| ConfigurationInput {
            request,
            // bind_request admits actual definitions from its frozen catalog.
            definitions: vec![],
            defaults: vec![],
        });
    request
}

pub use vibe_orchestrator::machine_operation::boot_debugger::scenarios::ScenarioFailure as CaptureFailure;
pub(super) fn solve_failure(e: vibe_resolver::SolveError) -> CaptureFailure {
    use vibe_resolver::SolveError;
    match e {
        SolveError::Provider(_)
        | SolveError::BackendCapability { .. }
        | SolveError::UnsupportedRoute(_) => CaptureFailure::Incomplete(e.to_string()),
        SolveError::RootConstraints { conflict, .. } => solve_failure(*conflict),
        SolveError::Configuration(error) => {
            use vibe_resolver::resolvo_engine::configuration::ConfigurationSolveError as C;
            match *error {
                C::NeedsSelection { .. } | C::Conflict { .. } => {
                    CaptureFailure::Incompatible(error.to_string())
                }
                _ => CaptureFailure::Incomplete(error.to_string()),
            }
        }
        _ => CaptureFailure::Incompatible(e.to_string()),
    }
}
pub(super) fn install_failure(e: vibe_install::Error) -> CaptureFailure {
    match e {
        vibe_install::Error::Solve(e) => solve_failure(e),
        vibe_install::Error::Feature(_) => CaptureFailure::Incompatible(e.to_string()),
        _ => CaptureFailure::Incomplete(e.to_string()),
    }
}

/// Default finite local producer. Only the admitted local registry can be read;
/// no global registry, network, install planner or preparation writer is used.
struct LocalProducer {
    root: std::path::PathBuf,
    registry: std::path::PathBuf,
}
impl vibe_orchestrator::machine_operation::boot_debugger::scenarios::ScenarioProducer
    for LocalProducer
{
    fn capture(
        &self,
        host: HostCapture,
        draft: &Draft,
        target: &str,
    ) -> Result<PreparedScenario, CaptureFailure> {
        self.capture_with_derivatives(host, draft, target, &Default::default())
    }
    fn capture_with_derivatives(
        &self,
        host: HostCapture,
        draft: &Draft,
        target: &str,
        derivatives: &vibe_orchestrator::machine_operation::boot_debugger::DerivativeCatalog,
    ) -> Result<PreparedScenario, CaptureFailure> {
        if !host.manifest.requires.git_packages.is_empty() {
            return Err(CaptureFailure::Incomplete(
                "git source preparation unsupported".into(),
            ));
        }
        let resolver = crate::build_install_resolver(
            &crate::PackageSourceOptions {
                registry: Some(self.registry.clone()),
                ..Default::default()
            },
            &host.manifest,
            None,
            &self.root,
            &Default::default(),
            true,
            &[],
        )
        .map_err(|e| CaptureFailure::Incomplete(e.to_string()))?;
        resolver.capture_boot_scenario_with_derivatives(host, draft, target.into(), derivatives)
    }
}
impl InstallResolver {
    /// Ordinary fresh-project debugger composition. Missing boot artifacts are
    /// represented by the document owner; no dummy files are created.
    pub fn open_boot_debugger(
        root: &std::path::Path,
        registry: Option<&std::path::Path>,
        artifact: &str,
        kind: vibe_orchestrator::machine_operation::boot_debugger::CapturedKind,
    ) -> Result<
        std::sync::Arc<vibe_orchestrator::machine_operation::boot_debugger::BootDebuggerRoute>,
        String,
    > {
        use vibe_orchestrator::{
            document_analysis::*, machine_operation::boot_debugger::BootDebuggerRoute,
        };
        let root = root.canonicalize().map_err(|e| e.to_string())?;
        let project = vibe_safefs::Project::open(&root).map_err(|e| e.to_string())?;
        let mut files = std::collections::BTreeMap::new();
        super::preview::capture_spec_tree(
            &project,
            &root,
            &vibe_core::layout::current_specs_root(),
            &mut files,
        )?;
        let mut declarations = vec![ResourceDeclaration {
            path: "vibe.toml".into(),
            role: ResourceRole::Manifest,
        }];
        if artifact != "vibe.toml" {
            declarations.push(ResourceDeclaration {
                path: artifact.into(),
                role: ResourceRole::Opaque,
            });
        }
        for path in files.keys() {
            if !declarations.iter().any(|d| d.path == *path) {
                declarations.push(ResourceDeclaration {
                    path: path.clone(),
                    role: ResourceRole::Opaque,
                });
            }
        }
        let host = DocumentAnalysisHost::new(
            &root,
            "vibe.toml",
            declarations,
            AnalysisLimits {
                resource_bytes: 64 * 1024 * 1024,
                capture_bytes: 128 * 1024 * 1024,
                open_buffers: 16,
            },
        )
        .map_err(|e| e.to_string())?;
        let resource = host
            .resource(artifact)
            .ok_or("artifact declaration unavailable")?;
        let producer = LocalProducer {
            registry: registry
                .map(|p| p.to_owned())
                .unwrap_or_else(|| root.join("vibevm/vibepacks")),
            root,
        };
        let route = BootDebuggerRoute::new(host, resource, kind)
            .map_err(|e| e.message)?
            .with_scenario_producer(std::sync::Arc::new(producer));
        Ok(std::sync::Arc::new(route))
    }
}
