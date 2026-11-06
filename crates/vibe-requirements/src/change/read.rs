//! Saved byte observations, without publication or roots granted by manifests.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-SAVED-READ");
use std::path::{Path, PathBuf};
use vibe_core::manifest::{
    Manifest,
    change::{
        CHANGE_MANIFEST, ChangeContext, ChangeError, ChangeId, ChangeManifest, ChangeMigrationPlan,
        ChangeMigrationSelection, plan_change_migration,
    },
};
use vibe_safefs::Project;

/// Typed refusal naming the source-read boundary, without raw IO diagnostics.
/// ```
/// use vibe_requirements::change::{read_saved_change,SavedChangeError};
/// let root=tempfile::tempdir().unwrap();
/// assert!(matches!(read_saved_change(root.path(),".",None,"existing-change"),Err(SavedChangeError::Unavailable("selected node manifest"))));
/// ```
#[derive(Debug, thiserror::Error)]
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-SAVED-READ")]
pub enum SavedChangeError {
    #[error(
        "violates REQ spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-AUTHORED-GRAMMAR: declaration refused: {0}; fix surface: authored change declaration"
    )]
    Declaration(#[from] ChangeError),
    #[error(
        "violates REQ spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-SAVED-READ: saved change unavailable at {0}; fix surface: selected node and declared source files"
    )]
    Unavailable(&'static str),
    #[error(
        "violates REQ spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-SAVED-READ: required change document is absent: {0}; fix surface: declared input path"
    )]
    MissingDocument(String),
}

/// Retained bytes from finite per-file observations, not a whole-change transaction.
/// ```
/// # use std::path::Path;
/// # use vibe_requirements::change::*;
/// # let root=tempfile::tempdir().unwrap();
/// # let id="chg-0123456789abcdef0123456789abcdef";
/// # std::fs::write(root.path().join("vibe.toml"),"[project]\ngroup='org.fixture'\nname='alpha'\nversion='1.0.0'\n").unwrap();
/// # let directory=root.path().join("docs/custom-specs/changes").join(id);
/// # std::fs::create_dir_all(&directory).unwrap();
/// # let text=include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../formats/corpora/change-manifest/e1/valid/representative.toml"));
/// # std::fs::write(directory.join("change.toml"),text).unwrap();
/// # std::fs::write(directory.join("intent.md"),"Intent").unwrap();
/// # std::fs::write(directory.join("acceptance.md"),"Accept").unwrap();
/// # let saved=read_saved_change(root.path(),".",Some(Path::new("docs/custom-specs")),id).unwrap();
/// assert_eq!(saved.manifest.id().as_str(),id);
/// assert_eq!(saved.manifest_bytes,text.as_bytes());
/// assert_eq!(saved.input_observations.len(),2);
/// ```
#[derive(Debug)]
pub struct SavedChange {
    pub manifest: ChangeManifest,
    pub manifest_bytes: Vec<u8>,
    pub input_observations: Vec<(PathBuf, Vec<u8>)>,
}

fn observed(
    project: &Project,
    path: &Path,
    boundary: &'static str,
) -> Result<Vec<u8>, SavedChangeError> {
    let relative = path
        .to_str()
        .ok_or(SavedChangeError::Unavailable(boundary))?;
    project
        .read_source_bytes(relative)
        .map_err(|_| SavedChangeError::Unavailable(boundary))?
        .ok_or(SavedChangeError::Unavailable(boundary))
}

fn context(
    project: &Project,
    selected_node: &str,
    spec_root: Option<&Path>,
) -> Result<ChangeContext, SavedChangeError> {
    let bytes = observed(project, Path::new("vibe.toml"), "selected node manifest")?;
    let text =
        std::str::from_utf8(&bytes).map_err(|_| SavedChangeError::Unavailable("node UTF8"))?;
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
) -> Result<Vec<(PathBuf, Vec<u8>)>, SavedChangeError> {
    let mut result = Vec::new();
    for document in change.documents().iter().filter(|d| d.input) {
        let path = change.directory().join(&document.path);
        let relative = path
            .to_str()
            .ok_or(SavedChangeError::Unavailable("role path"))?;
        let bytes = project
            .read_source_bytes(relative)
            .map_err(|_| SavedChangeError::Unavailable("held role input"))?
            .ok_or_else(|| SavedChangeError::MissingDocument(document.path.clone()))?;
        result.push((path, bytes));
    }
    Ok(result)
}

#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-SAVED-READ")]
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
    let bytes = observed(
        &project,
        &directory.join(CHANGE_MANIFEST),
        "change manifest",
    )?;
    let text =
        std::str::from_utf8(&bytes).map_err(|_| SavedChangeError::Unavailable("change UTF8"))?;
    let manifest = ChangeManifest::parse_str(text, &context, &directory)?;
    let input_observations = inputs(&project, &manifest)?;
    Ok(SavedChange {
        manifest,
        manifest_bytes: bytes,
        input_observations,
    })
}

#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-SAVED-READ")]
pub fn plan_saved_change_migration(
    node_root: &Path,
    selected_node: &str,
    spec_root: Option<&Path>,
    id: &str,
    selection: ChangeMigrationSelection<'_>,
) -> Result<(ChangeMigrationPlan, Vec<u8>), SavedChangeError> {
    let project = Project::open(node_root)
        .map_err(|_| SavedChangeError::Unavailable("held selected node"))?;
    let context = context(&project, selected_node, spec_root)?;
    let id = ChangeId::parse(id)?;
    let directory = context.directory(&id);
    let bytes = observed(
        &project,
        &directory.join(CHANGE_MANIFEST),
        "migration manifest",
    )?;
    let text =
        std::str::from_utf8(&bytes).map_err(|_| SavedChangeError::Unavailable("migration UTF8"))?;
    let plan = plan_change_migration(text, &context, &directory, selection)?;
    inputs(&project, &plan.checked)?;
    Ok((plan, bytes))
}
