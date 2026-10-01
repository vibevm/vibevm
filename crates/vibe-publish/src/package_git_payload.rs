//! Private local payload custody for a future protected package-Git owner.
//! No Git command, RepoCreator, network call or C.1 Ready/effect grant lives here.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use vibe_safefs::{Project, PublishError, PublishStage};

mod walk;

const FORMAT: &str = "vibevm.package-git-payload/e1";
const TREE_DOMAIN: &[u8] = b"vibevm.package-git-payload-tree/e1\0";
const COMPLETE: &str = "complete.json";
const PAYLOAD: &str = "payload";

/// Every limit is selected by the host; there is no hidden product-size cap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PayloadLimits {
    pub max_files: usize,
    pub max_directories: usize,
    pub max_children_per_directory: usize,
    pub max_depth: usize,
    pub max_path_bytes: usize,
    pub max_file_bytes: u64,
    pub max_total_bytes: u64,
    pub max_manifest_bytes: usize,
}
impl PayloadLimits {
    fn validate(self) -> anyhow::Result<()> {
        if self.max_files == 0
            || self.max_directories == 0
            || self.max_children_per_directory == 0
            || self.max_depth == 0
            || self.max_path_bytes == 0
            || self.max_file_bytes == 0
            || self.max_total_bytes == 0
            || self.max_manifest_bytes == 0
        {
            anyhow::bail!("every package Git payload limit must be explicit and nonzero");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FileRow {
    pub path: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub unix_mode: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Census {
    directories: Vec<String>,
    files: Vec<FileRow>,
    skipped_gitmodules: bool,
    tree_sha256: String,
}

/// Complete *local byte* receipt, with Gitlink provenance deliberately open.
/// `NeedsGitlinkCheck` must never be promoted to a publish effect permit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ProvenanceStatus {
    NeedsGitlinkCheck,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CompleteManifest {
    pub format: String,
    pub source_root_token: String,
    pub destination_root_token: String,
    pub limits: PayloadLimits,
    pub directories: Vec<String>,
    pub files: Vec<FileRow>,
    pub tree_sha256: String,
    pub skipped_gitmodules: bool,
    pub provenance: ProvenanceStatus,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum PayloadStageError {
    #[error("package Git payload refused before destination publication: {0}")]
    Before(String),
    #[error("package Git payload is partial after {staged_files} staged files: {reason}")]
    Partial { staged_files: usize, reason: String },
    #[error(
        "package Git payload publication may have happened after {staged_files} files: {reason}"
    )]
    PossiblyPublished { staged_files: usize, reason: String },
}

fn before(error: impl std::fmt::Display) -> PayloadStageError {
    PayloadStageError::Before(error.to_string())
}
fn partial(staged_files: usize, error: impl std::fmt::Display) -> PayloadStageError {
    PayloadStageError::Partial {
        staged_files,
        reason: error.to_string(),
    }
}
fn possibly(staged_files: usize, error: impl std::fmt::Display) -> PayloadStageError {
    PayloadStageError::PossiblyPublished {
        staged_files,
        reason: error.to_string(),
    }
}

fn copy_error(staged_files: usize, error: PublishError) -> PayloadStageError {
    let stage = error.stage;
    let created = error.created_display();
    let reason = format!("{:#}; created_directories={created:?}", error.into_report());
    match stage {
        PublishStage::BeforePublication if staged_files == 0 && created.is_empty() => {
            PayloadStageError::Before(reason)
        }
        PublishStage::BeforePublication => partial(staged_files, reason),
        PublishStage::PossiblyPublished => possibly(staged_files, reason),
    }
}

/// Stage under one caller-selected run-scoped empty Project. An interrupted
/// attempt leaves inspectable partial state and is never automatically retried.
pub(crate) fn stage_local_payload(
    source: &Project,
    destination: &Project,
    limits: PayloadLimits,
) -> Result<CompleteManifest, PayloadStageError> {
    stage_inner(source, destination, limits, |_, _| Ok(()), |_, _| Ok(()))
}

fn stage_inner(
    source: &Project,
    destination: &Project,
    limits: PayloadLimits,
    mut before_copy: impl FnMut(usize, &FileRow) -> anyhow::Result<()>,
    mut after_copy: impl FnMut(usize, &FileRow) -> anyhow::Result<()>,
) -> Result<CompleteManifest, PayloadStageError> {
    limits.validate().map_err(before)?;
    let source_root_token = source.root_correlation_token().map_err(before)?;
    let initial = walk::scan(source, &limits, true).map_err(before)?;
    if !initial.files.iter().any(|row| row.path == "vibe.toml") {
        return Err(before("complete source census has no vibe.toml"));
    }
    let named_source = Project::open(source.root_path()).map_err(before)?;
    if named_source.root_correlation_token().map_err(before)? != source_root_token {
        return Err(before("source pathname no longer names the held root"));
    }
    let destination_root_token = destination.root_correlation_token().map_err(before)?;
    let named_destination = Project::open(destination.root_path()).map_err(before)?;
    if named_destination.root_correlation_token().map_err(before)? != destination_root_token {
        return Err(before(
            "destination pathname no longer names the held vault",
        ));
    }
    let root = destination.root_dir().map_err(before)?;
    if !destination
        .child_names_bounded(&root, limits.max_children_per_directory)
        .map_err(before)?
        .is_empty()
    {
        return Err(partial(
            0,
            "run-scoped destination is occupied; inspect retained state, never restage",
        ));
    }
    let mut staged_files = 0;
    for file in &initial.files {
        before_copy(staged_files, file).map_err(|error| {
            if staged_files == 0 {
                before(error)
            } else {
                partial(staged_files, error)
            }
        })?;
        let target = format!("{PAYLOAD}/{}", file.path);
        let copied = source
            .copy_stable_file_to_expected_new_bounded(
                &file.path,
                destination,
                &target,
                file.sha256
                    .strip_prefix("sha256:")
                    .expect("captured SHA shape"),
                file.size_bytes,
                limits.max_file_bytes,
                file.unix_mode,
            )
            .map_err(|error| copy_error(staged_files, error))?;
        if copied.0.bytes != file.size_bytes
            || format!("sha256:{}", copied.0.sha256) != file.sha256
            || copied.0.unix_mode != file.unix_mode
        {
            return Err(possibly(
                staged_files + 1,
                "staged file differs from exact census",
            ));
        }
        staged_files += 1;
        after_copy(staged_files, file).map_err(|error| partial(staged_files, error))?;
    }
    // A file copy creates its parents; explicitly retain empty source dirs.
    for directory in &initial.directories {
        let parts = format!("{PAYLOAD}/{directory}");
        let components: Vec<&str> = parts.split('/').collect();
        destination
            .dir(&components, true)
            .map_err(|error| partial(staged_files, error))?;
    }
    let named_source =
        Project::open(source.root_path()).map_err(|error| partial(staged_files, error))?;
    if named_source
        .root_correlation_token()
        .map_err(|error| partial(staged_files, error))?
        != source_root_token
    {
        return Err(partial(
            staged_files,
            "source pathname no longer names the held root",
        ));
    }
    let source_again =
        walk::scan(&named_source, &limits, true).map_err(|error| partial(staged_files, error))?;
    if source_again != initial {
        return Err(partial(
            staged_files,
            "source full census changed during staging",
        ));
    }
    let named_destination =
        Project::open(destination.root_path()).map_err(|error| partial(staged_files, error))?;
    if named_destination
        .root_correlation_token()
        .map_err(|error| partial(staged_files, error))?
        != destination_root_token
    {
        return Err(partial(
            staged_files,
            "destination pathname no longer names the held vault",
        ));
    }
    let payload = named_destination
        .subproject(&[PAYLOAD])
        .map_err(|error| partial(staged_files, error))?;
    let copied =
        walk::scan(&payload, &limits, false).map_err(|error| partial(staged_files, error))?;
    if copied.directories != initial.directories
        || copied.files != initial.files
        || copied.tree_sha256 != initial.tree_sha256
    {
        return Err(partial(
            staged_files,
            "staged payload differs from source census",
        ));
    }
    let manifest = CompleteManifest {
        format: FORMAT.into(),
        source_root_token,
        destination_root_token,
        limits,
        directories: initial.directories,
        files: initial.files,
        tree_sha256: initial.tree_sha256,
        skipped_gitmodules: initial.skipped_gitmodules,
        provenance: ProvenanceStatus::NeedsGitlinkCheck,
    };
    let bytes = serde_json::to_vec(&manifest).map_err(|error| partial(staged_files, error))?;
    if bytes.len() > limits.max_manifest_bytes {
        return Err(partial(
            staged_files,
            "complete manifest exceeds caller-selected cap",
        ));
    }
    destination
        .publish_new_in(&root, COMPLETE, &bytes)
        .map_err(|error| copy_error(staged_files, error))?;
    let named_destination =
        Project::open(destination.root_path()).map_err(|error| possibly(staged_files, error))?;
    if named_destination
        .root_correlation_token()
        .map_err(|error| possibly(staged_files, error))?
        != manifest.destination_root_token
    {
        return Err(possibly(
            staged_files,
            "destination pathname no longer names the held vault",
        ));
    }
    let named_root = named_destination
        .root_dir()
        .map_err(|error| possibly(staged_files, error))?;
    let observed = named_destination
        .read_file_bounded_in(&named_root, COMPLETE, limits.max_manifest_bytes)
        .map_err(|error| possibly(staged_files, error))?;
    if observed.as_deref() != Some(bytes.as_slice()) {
        return Err(possibly(
            staged_files,
            "complete manifest changed on readback",
        ));
    }
    Ok(manifest)
}

#[cfg(test)]
#[path = "package_git_payload/tests.rs"]
mod tests;
