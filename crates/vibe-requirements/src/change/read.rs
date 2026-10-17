//! Saved bounded observations, no scaffold/publication or root from a manifest.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-AUTHORED-GRAMMAR");
use std::path::{Path, PathBuf};
use vibe_core::manifest::{
    Manifest,
    change::{
        CHANGE_MANIFEST, ChangeContext, ChangeError, ChangeId, ChangeManifest, ChangeMigrationPlan,
        ChangeMigrationSelection, plan_change_migration,
    },
};
use vibe_safefs::{Project, StableFileSnapshot};
#[derive(Debug, thiserror::Error)]
pub enum SavedChangeError {
    #[error("release evidence refused: {0}")]
    Evidence(#[from] vibe_wire::behaviour::release_impact::ReleaseImpactError),
    #[error("change declaration refused: {0}")]
    Declaration(#[from] ChangeError),
    #[error("saved change unavailable at {0}")]
    Unavailable(&'static str),
    #[error("required change document is absent: {0}")]
    MissingDocument(String),
}
#[derive(Debug)]
pub struct SavedChange {
    pub manifest: ChangeManifest,
    pub manifest_observation: StableFileSnapshot,
    pub input_observations: Vec<(PathBuf, StableFileSnapshot)>,
}
fn observed(
    project: &Project,
    path: &Path,
    cap: usize,
    boundary: &'static str,
) -> Result<StableFileSnapshot, SavedChangeError> {
    let relative = path
        .to_str()
        .ok_or(SavedChangeError::Unavailable(boundary))?;
    project
        .read_file_snapshot_bounded(relative, cap)
        .map_err(|_| SavedChangeError::Unavailable(boundary))?
        .ok_or(SavedChangeError::Unavailable(boundary))
}
fn context(
    project: &Project,
    selected_node: &str,
    spec_root: Option<&Path>,
) -> Result<ChangeContext, SavedChangeError> {
    let snapshot = observed(
        project,
        Path::new("vibe.toml"),
        1024 * 1024,
        "selected node manifest",
    )?;
    let text = std::str::from_utf8(&snapshot.bytes)
        .map_err(|_| SavedChangeError::Unavailable("node UTF8"))?;
    let manifest = Manifest::parse_str(text)
        .map_err(|_| SavedChangeError::Unavailable("node manifest grammar"))?;
    Ok(ChangeContext::for_node(
        &manifest,
        selected_node,
        spec_root,
    )?)
}
fn inputs(
    project: &Project,
    change: &ChangeManifest,
) -> Result<Vec<(PathBuf, StableFileSnapshot)>, SavedChangeError> {
    let mut result = vec![];
    let mut remaining = 64 * 1024 * 1024;
    for document in change.documents().iter().filter(|d| d.input) {
        let path = change.directory().join(&document.path);
        let relative = path
            .to_str()
            .ok_or(SavedChangeError::Unavailable("role path"))?;
        let snapshot = project
            .read_file_snapshot_bounded(relative, (16 * 1024 * 1024).min(remaining))
            .map_err(|_| SavedChangeError::Unavailable("held role input"))?
            .ok_or_else(|| SavedChangeError::MissingDocument(document.path.clone()))?;
        remaining = remaining
            .checked_sub(snapshot.bytes.len())
            .ok_or(SavedChangeError::Unavailable("role byte bound"))?;
        result.push((path, snapshot));
    }
    let retained: Vec<_> = result
        .iter()
        .map(|(path, snapshot)| (path.as_path(), snapshot.bytes.as_slice()))
        .collect();
    super::evidence::from_observations(change, &retained)?;
    Ok(result)
}
pub fn read_saved_change(
    node_root: &Path,
    selected_node: &str,
    spec_root: Option<&Path>,
    id: &str,
) -> Result<SavedChange, SavedChangeError> {
    let project = Project::open(node_root)
        .map_err(|_| SavedChangeError::Unavailable("held selected node"))?;
    let context = context(&project, selected_node, spec_root)?;
    let id = ChangeId::parse(id)?;
    let directory = context.directory(&id);
    let snapshot = observed(
        &project,
        &directory.join(CHANGE_MANIFEST),
        64 * 1024,
        "change manifest",
    )?;
    let text = std::str::from_utf8(&snapshot.bytes)
        .map_err(|_| SavedChangeError::Unavailable("change UTF8"))?;
    let manifest = ChangeManifest::parse_str(text, &context, &directory)?;
    let input_observations = inputs(&project, &manifest)?;
    let saved = SavedChange {
        manifest,
        manifest_observation: snapshot,
        input_observations,
    };
    Ok(saved)
}
pub fn plan_saved_change_migration(
    node_root: &Path,
    selected_node: &str,
    spec_root: Option<&Path>,
    id: &str,
    selection: ChangeMigrationSelection<'_>,
) -> Result<(ChangeMigrationPlan, StableFileSnapshot), SavedChangeError> {
    let project = Project::open(node_root)
        .map_err(|_| SavedChangeError::Unavailable("held selected node"))?;
    let context = context(&project, selected_node, spec_root)?;
    let id = ChangeId::parse(id)?;
    let directory = context.directory(&id);
    let snapshot = observed(
        &project,
        &directory.join(CHANGE_MANIFEST),
        64 * 1024,
        "migration manifest",
    )?;
    let text = std::str::from_utf8(&snapshot.bytes)
        .map_err(|_| SavedChangeError::Unavailable("migration UTF8"))?;
    let plan = plan_change_migration(text, &context, &directory, selection)?;
    inputs(&project, &plan.checked)?;
    Ok((plan, snapshot))
}
