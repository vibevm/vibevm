//! One M16 facade read route over actual capture/selection owners.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-READ-SERVICE");
use crate::document_analysis::*;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
};
use vibe_core::manifest::change::{ChangeContext, ChangeId};
use vibe_requirements::complete::CapturedFactSet;
use vibe_wire::behaviour::change_query as wire;
use vibe_workspace::source_query::SourceQueryService;
use wire::{Request, Response, ResponseRefusedCode as Code};
mod result;
#[derive(Debug, thiserror::Error)]
pub enum ChangeReadError {
    #[error("change read context unavailable: {0}")]
    Context(&'static str),
    #[error("change read refused: {0}")]
    Declaration(#[from] vibe_requirements::change::SavedChangeError),
    #[error("change observed state became stale")]
    Stale,
}
pub struct CapturedChangeQuery {
    root: PathBuf,
    workspace_root: PathBuf,
    host: Arc<DocumentAnalysisHost>,
    client: DocumentClient,
    capture: AnalysisCapture,
    node: ResourceId,
    spec_root: Option<PathBuf>,
    id: String,
    source: Option<SourceQueryService>,
    origins: BTreeMap<String, CapturedFactSet>,
    registry: vibe_facts::RegistrySnapshot,
}
impl CapturedChangeQuery {
    fn revalidate(&self) -> Result<(), ChangeReadError> {
        let current = self.client.freeze().map_err(|_| ChangeReadError::Stale)?;
        for (id, before) in self.capture.resources() {
            let after = current.resource(id).ok_or(ChangeReadError::Stale)?;
            if before.disk_observation() != after.disk_observation()
                || before.raw() != after.raw()
                || before.buffer_version() != after.buffer_version()
                || before.disposition() != after.disposition()
            {
                return Err(ChangeReadError::Stale);
            }
        }
        let registry = vibe_facts::Registry::load_with_witnesses(&self.root)
            .map_err(|_| ChangeReadError::Stale)?;
        if registry.witnesses != self.registry.witnesses {
            return Err(ChangeReadError::Stale);
        }
        if let Some(service) = &self.source {
            for facts in self.origins.values() {
                let pin = facts.inventory().pin();
                let current = service
                    .retain_source_inventory(
                        pin.group.as_ref().unwrap().as_str(),
                        pin.name.as_str(),
                    )
                    .map_err(|_| ChangeReadError::Stale)?;
                if current.pin() != facts.inventory().pin()
                    || current.content_hash() != facts.inventory().content_hash()
                    || current.manifest() != facts.inventory().manifest()
                {
                    return Err(ChangeReadError::Stale);
                }
                if current
                    .documents()
                    .keys()
                    .ne(facts.inventory().documents().keys())
                    || current.documents().iter().any(|(path, doc)| {
                        doc.snapshot() != facts.inventory().documents()[path].snapshot()
                    })
                {
                    return Err(ChangeReadError::Stale);
                }
            }
        }
        Ok(())
    }
    /// Host-selected root/configuration only; no request path can construct it.
    pub fn prepare(
        root: &Path,
        spec_root: Option<&Path>,
        id: &str,
    ) -> Result<Self, ChangeReadError> {
        let saved = vibe_requirements::change::read_saved_change(root, ".", spec_root, id)?;
        let mut declarations = vec![
            ResourceDeclaration {
                path: "vibe.toml".into(),
                role: ResourceRole::Manifest,
            },
            ResourceDeclaration {
                path: "vibe.lock".into(),
                role: ResourceRole::Lockfile,
            },
            ResourceDeclaration {
                path: format!("{}/change.toml", saved.manifest.directory().display()),
                role: ResourceRole::Opaque,
            },
        ];
        for doc in saved.manifest.documents() {
            declarations.push(ResourceDeclaration {
                path: format!("{}/{}", saved.manifest.directory().display(), doc.path),
                role: ResourceRole::Source {
                    manifest: "vibe.toml".into(),
                },
            });
        }
        let host = DocumentAnalysisHost::new(
            root,
            "vibe.toml",
            declarations,
            AnalysisLimits {
                resource_bytes: 16 * 1024 * 1024,
                capture_bytes: 64 * 1024 * 1024,
                open_buffers: 64,
            },
        )
        .map_err(|_| ChangeReadError::Context("read capture"))?;
        let client = host
            .client()
            .map_err(|_| ChangeReadError::Context("read client"))?;
        let capture = client
            .freeze()
            .map_err(|_| ChangeReadError::Context("read capture"))?;
        let node = host.resource("vibe.toml").unwrap();
        let view = super::analyze_change(&capture, node, spec_root, id);
        let parsed = view
            .analysis()
            .map_err(|_| ChangeReadError::Context("captured change grammar/inputs"))?;
        let workspace_root = vibe_workspace::Workspace::discover_selected(root)
            .map_err(|_| ChangeReadError::Context("selected workspace"))?
            .workspace
            .root;
        let source = SourceQueryService::prepare_host(root).ok();
        let mut origins = BTreeMap::new();
        for declared in parsed.manifest.origins() {
            let pin = vibe_core::PackageRef::parse(&declared.package)
                .map_err(|_| ChangeReadError::Context("origin pin"))?;
            if let Some(source) = &source {
                if let Ok(held) = source.retain_source_inventory(
                    pin.group.as_ref().unwrap().as_str(),
                    pin.name.as_str(),
                ) {
                    if let Ok(facts) = CapturedFactSet::observe(Arc::new(held)) {
                        origins.insert(declared.package.clone(), facts);
                    }
                }
            }
        }
        let registry = vibe_facts::Registry::load_with_witnesses(root)
            .map_err(|_| ChangeReadError::Context("adoption registry"))?;
        Ok(Self {
            root: root.into(),
            workspace_root,
            host,
            client,
            capture,
            node,
            spec_root: spec_root.map(Path::to_owned),
            id: id.into(),
            source,
            origins,
            registry,
        })
    }
    /// Read-only selected template from the same captured change inputs.
    /// This is not an admitted machine operation or execution lease.
    pub fn full_pass_plan(&self, work: &str) -> Result<super::ChangeFullPassPlan, anyhow::Error> {
        self.revalidate()?;
        let view = super::analyze_change(
            &self.capture,
            self.node,
            self.spec_root.as_deref(),
            &self.id,
        );
        Ok(super::plan_change_full_pass(&view, work)?)
    }
    pub fn query(&self, request: &Request) -> Response {
        if let Err(error) = wire::validate_request(request) {
            return error;
        }
        if request.id != self.id {
            return wire::refused(
                Code::InvalidId,
                "request is outside captured change context",
                vec![self.id.clone()],
            );
        }
        match result::produce(self, request) {
            Ok(response) => match wire::validate_response(&response) {
                Ok(()) => response,
                Err(reason) => wire::refused(Code::InternalInvariant, reason, vec![]),
            },
            Err(response) => response,
        }
    }
}
impl crate::machine_operation::MachineOperationService {
    /// Read-only domain extension. This does not grant the separately
    /// restricted migration/envelope or external M17 dispatch route.
    pub fn read_change_json(
        root: &Path,
        spec_root: Option<&Path>,
        value: &serde_json::Value,
    ) -> Response {
        let request = match wire::read_request(value) {
            Ok(request) => request,
            Err(response) => return response,
        };
        if let Err(error) = ChangeId::parse(&request.id) {
            return wire::refused(Code::InvalidId, error.to_string(), vec![]);
        }
        match CapturedChangeQuery::prepare(root, spec_root, &request.id) {
            Ok(service) => service.query(&request),
            Err(ChangeReadError::Declaration(error)) => {
                let code = match error {
                    vibe_requirements::change::SavedChangeError::Declaration(_) => {
                        Code::InvalidChange
                    }
                    vibe_requirements::change::SavedChangeError::MissingDocument(_) => {
                        Code::MissingChange
                    }
                    vibe_requirements::change::SavedChangeError::Unavailable(_) => {
                        Code::MissingChange
                    }
                };
                wire::refused(code, error.to_string(), vec![])
            }
            Err(error) => wire::refused(Code::UnavailableContext, error.to_string(), vec![]),
        }
    }
}
#[cfg(test)]
mod m_02_d_2_cases;
