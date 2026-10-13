//! Exact scaffold preparation and finite saved-state creation authority.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-SCAFFOLD-NO-OVERWRITE");
use crate::document_analysis::{
    AnalysisCapture, DocumentAnalysisHost, ResourceDisposition, ResourceId,
    edit::{AbsentDocumentBarrier, DocumentEditAuthority, EditError},
};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use vibe_core::{
    PackageRef,
    manifest::change::{ChangeContext, ChangeId},
};
use vibe_requirements::{change::ScaffoldOutput, complete::CapturedFactSet};
use vibe_workspace::source_query::SourceQueryService;
mod apply;
mod journal;
pub use apply::{ScaffoldOutcome, ScaffoldState};

#[derive(Debug, thiserror::Error)]
pub enum ScaffoldRefusal {
    #[error("document coordination refused: {0}")]
    Edit(#[from] EditError),
    #[error("scaffold destination already exists")]
    Occupied,
    #[error("scaffold origin is unavailable or changed")]
    Origin,
    #[error("invalid scaffold declaration or finite grant")]
    Invalid,
    #[error("scaffold infrastructure unavailable: {0}")]
    Infrastructure(String),
    #[error("scaffold recovery preserves an unrelated third state")]
    ThirdState,
    #[error("scaffold journal unavailable or invalid")]
    Journal,
}
fn infra(e: impl std::fmt::Display) -> ScaffoldRefusal {
    ScaffoldRefusal::Infrastructure(e.to_string())
}
#[derive(Debug)]
pub struct ScaffoldAuthority {
    host: Arc<DocumentAnalysisHost>,
    directory: PathBuf,
    node: ResourceId,
    resources: BTreeMap<String, ResourceId>,
    edits: Arc<DocumentEditAuthority>,
    root_binding: String,
}
impl ScaffoldAuthority {
    /// Trusted host grants exactly one immutable directory and four files.
    /// A CLI declaration/capture is not this grant.
    pub fn new(
        host: Arc<DocumentAnalysisHost>,
        node: ResourceId,
        directory: PathBuf,
    ) -> Result<Arc<Self>, ScaffoldRefusal> {
        let text = directory.to_str().ok_or(ScaffoldRefusal::Invalid)?;
        vibe_safefs::split_relative(text).map_err(infra)?;
        let mut resources = BTreeMap::new();
        for name in ["change.toml", "intent.md", "refinement.md", "acceptance.md"] {
            let path = format!("{text}/{name}");
            resources.insert(
                name.into(),
                host.resource(&path).ok_or(ScaffoldRefusal::Invalid)?,
            );
        }
        let ids = resources.values().copied().chain([node]).collect();
        let edits = DocumentEditAuthority::new(host.clone(), ids)?;
        let root_binding = vibe_safefs::Project::open(host.root())
            .map_err(infra)?
            .root_correlation_token()
            .map_err(infra)?;
        Ok(Arc::new(Self {
            host,
            directory,
            node,
            resources,
            edits,
            root_binding,
        }))
    }
}
pub struct PreparedScaffold {
    authority: Arc<ScaffoldAuthority>,
    output: ScaffoldOutput,
    source: SourceQueryService,
    pin: PackageRef,
    node_before: vibe_safefs::StableFileSnapshot,
    node_path: String,
    editor_capture: String,
    barrier: AbsentDocumentBarrier,
    id: String,
}
impl std::fmt::Debug for PreparedScaffold {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedScaffold")
            .field("id", &self.id)
            .field("directory", &self.authority.directory)
            .finish()
    }
}
impl PreparedScaffold {
    pub fn output(&self) -> &ScaffoldOutput {
        &self.output
    }
    pub fn barrier(&self) -> &AbsentDocumentBarrier {
        &self.barrier
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    fn current(&self, project: &vibe_safefs::Project) -> Result<(), ScaffoldRefusal> {
        self.barrier.check_participants()?;
        if project.root_correlation_token().map_err(infra)? != self.authority.root_binding {
            return Err(ScaffoldRefusal::Invalid);
        }
        let current = project
            .read_file_snapshot_bounded(&self.node_path, self.node_before.bytes.len())
            .map_err(|_| ScaffoldRefusal::Edit(EditError::Stale))?;
        if current.as_ref() != Some(&self.node_before) {
            return Err(EditError::Stale.into());
        }
        let held = self
            .source
            .retain_source_inventory(
                self.pin.group.as_ref().unwrap().as_str(),
                self.pin.name.as_str(),
            )
            .map_err(|_| ScaffoldRefusal::Origin)?;
        let origin = &self.output.analysis.manifest.origins()[0];
        if held.pin() != &self.pin
            || Some(held.content_hash().as_str()) != origin.selected_content_hash.as_deref()
            || format!("sha256:{}", held.manifest().sha256) != origin.manifest_sha256
        {
            return Err(ScaffoldRefusal::Origin);
        }
        Ok(())
    }
}
fn absence(project: &vibe_safefs::Project, path: &Path) -> Result<(), ScaffoldRefusal> {
    let text = path.to_str().ok_or(ScaffoldRefusal::Invalid)?;
    let (parents, name) = vibe_safefs::split_relative(text).map_err(infra)?;
    let parents: Vec<_> = parents.iter().map(String::as_str).collect();
    if let Some(parent) = project.dir_if_present(&parents).map_err(infra)? {
        if parent.inspect_child_state(&name).map_err(infra)?.is_some() {
            return Err(ScaffoldRefusal::Occupied);
        }
    }
    Ok(())
}
fn build_new_change(
    host: &Arc<DocumentAnalysisHost>,
    capture: &AnalysisCapture,
    node_id: ResourceId,
    from: &str,
    id: &str,
    slug: &str,
    title: &str,
    spec_root: Option<&Path>,
    addresses: &[String],
) -> Result<(SourceQueryService, PackageRef, ScaffoldOutput), ScaffoldRefusal> {
    let node = capture.resource(node_id).ok_or(ScaffoldRefusal::Invalid)?;
    let saved = node.disk_observation().ok_or(EditError::SaveRequired)?;
    if node.raw() != Some(saved.bytes.as_slice())
        || !matches!(
            node.disposition(),
            ResourceDisposition::Disk | ResourceDisposition::Replaced
        )
    {
        return Err(EditError::SaveRequired.into());
    }
    let manifest = capture
        .manifest(node_id)
        .map_err(|_| ScaffoldRefusal::Invalid)?;
    let parent = Path::new(node.path()).parent().unwrap();
    let selected = if parent.as_os_str().is_empty() {
        "."
    } else {
        parent.to_str().ok_or(ScaffoldRefusal::Invalid)?
    };
    let context = ChangeContext::for_node(&manifest, selected, spec_root)
        .map_err(|_| ScaffoldRefusal::Invalid)?;
    let id = ChangeId::parse(id).map_err(|_| ScaffoldRefusal::Invalid)?;
    let pin = PackageRef::parse(from).map_err(|_| ScaffoldRefusal::Invalid)?;
    if !pin.version.is_exact_pin() || pin.kind.is_none() || pin.group.is_none() {
        return Err(ScaffoldRefusal::Invalid);
    }
    let source =
        SourceQueryService::prepare_host(host.root()).map_err(|_| ScaffoldRefusal::Origin)?;
    let inventory = Arc::new(
        source
            .retain_source_inventory(pin.group.as_ref().unwrap().as_str(), pin.name.as_str())
            .map_err(|_| ScaffoldRefusal::Origin)?,
    );
    if inventory.pin() != &pin {
        return Err(ScaffoldRefusal::Origin);
    }
    let facts = CapturedFactSet::observe(inventory).map_err(|_| ScaffoldRefusal::Origin)?;
    let selection = if addresses.is_empty() {
        facts.select_all()
    } else {
        facts.select_exact(addresses)
    };
    let mut output =
        vibe_requirements::change::prepare_scaffold(&context, &id, slug, title, &selection)
            .map_err(|_| ScaffoldRefusal::Origin)?;
    output.directory = parent.join(&output.directory);
    if output.files.values().map(Vec::len).sum::<usize>() > 4 * 1024 * 1024 {
        return Err(ScaffoldRefusal::Invalid);
    }

    if host.resource(node.path()) != Some(node_id) {
        return Err(ScaffoldRefusal::Invalid);
    }
    let project = vibe_safefs::Project::open(host.root()).map_err(infra)?;
    if project
        .read_file_snapshot_bounded(node.path(), saved.bytes.len())
        .map_err(|_| ScaffoldRefusal::Edit(EditError::Stale))?
        .as_ref()
        != Some(saved)
    {
        return Err(EditError::Stale.into());
    }
    absence(&project, &output.directory)?;
    Ok((source, pin, output))
}
/// Read-only preparation. No edit grant, participant barrier, or journal.
pub fn preview_new_change(
    host: &Arc<DocumentAnalysisHost>,
    capture: &AnalysisCapture,
    node_id: ResourceId,
    from: &str,
    id: &str,
    slug: &str,
    title: &str,
    spec_root: Option<&Path>,
    addresses: &[String],
) -> Result<ScaffoldOutput, ScaffoldRefusal> {
    build_new_change(
        host, capture, node_id, from, id, slug, title, spec_root, addresses,
    )
    .map(|(_, _, output)| output)
}
pub fn prepare_new_change(
    authority: &Arc<ScaffoldAuthority>,
    capture: &AnalysisCapture,
    from: &str,
    id: &str,
    slug: &str,
    title: &str,
    spec_root: Option<&Path>,
    addresses: &[String],
) -> Result<PreparedScaffold, ScaffoldRefusal> {
    let (source, pin, output) = build_new_change(
        &authority.host,
        capture,
        authority.node,
        from,
        id,
        slug,
        title,
        spec_root,
        addresses,
    )?;
    if output.directory != authority.directory {
        return Err(ScaffoldRefusal::Invalid);
    }
    let node = capture
        .resource(authority.node)
        .ok_or(ScaffoldRefusal::Invalid)?;
    let saved = node.disk_observation().ok_or(EditError::SaveRequired)?;
    let project = vibe_safefs::Project::open(authority.host.root()).map_err(infra)?;
    absence(&project, &authority.directory)?;
    let plan = authority.edits.prepare_absent_with_saved(
        capture,
        &authority.resources.values().copied().collect::<Vec<_>>(),
        &[authority.node],
    )?;
    let barrier = plan.barrier(Duration::from_secs(60))?;
    let mut prepared = PreparedScaffold {
        authority: authority.clone(),
        output,
        source,
        pin,
        node_before: saved.clone(),
        node_path: node.path().into(),
        editor_capture: capture.id().into(),
        barrier,
        id: String::new(),
    };
    prepared.id = journal::from_plan(&prepared).id;
    prepared.current(&project)?;
    Ok(prepared)
}
#[cfg(test)]
mod m_02_d_1_cases;
