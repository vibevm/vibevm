//! Incremental reconciliation of a materialiser-owned slot footprint.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#DIFF-MATERIALISE");

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use vibe_safefs::SourceObservation;

use sha2::{Digest, Sha256};
use vibe_core::ContentHash;

use super::{
    CopyMode, SLOT_RECORD_FILENAME, SlotRecord, io_err, read_slot_record, sha256_file,
    write_slot_record,
};
use crate::{WorkspaceError, path_to_slash};

#[cfg(test)]
mod apply_no_replace_tests;
#[cfg(test)]
mod mode_tests;
#[cfg(test)]
mod no_replace_tests;
mod placement;
mod plan;
mod report;
use placement::place_atomically;

use plan::DiffPlan;
pub(crate) use report::MaterialiseReport;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Placement {
    Keep,
    Replace,
    CreateNew,
}

/// Content prepared and hashed before the slot is mutated.
pub(super) struct PreparedSlotFile {
    path: PathBuf,
    sha256: String,
    content: PreparedContent,
}

enum PreparedContent {
    Observed {
        observation: Arc<SourceObservation>,
        path: PathBuf,
        mode: CopyMode,
    },
    Source {
        path: PathBuf,
        mode: CopyMode,
    },
    Bytes(Vec<u8>),
}

impl PreparedSlotFile {
    pub(super) fn from_observation(
        path: PathBuf,
        source: PathBuf,
        mode: CopyMode,
        observation: Arc<SourceObservation>,
    ) -> Self {
        Self {
            path,
            sha256: sha256_bytes(observation.bytes()),
            content: PreparedContent::Observed {
                observation,
                path: source,
                mode,
            },
        }
    }
    pub(super) fn from_source(
        path: PathBuf,
        source: PathBuf,
        mode: CopyMode,
    ) -> Result<Self, WorkspaceError> {
        let sha256 =
            sha256_file(&source).map_err(|reason| WorkspaceError::SpecMaterialization {
                path: source.clone(),
                reason: format!("incoming payload cannot be hashed: {reason}"),
            })?;
        Ok(Self {
            path,
            sha256,
            content: PreparedContent::Source { path: source, mode },
        })
    }

    pub(super) fn from_bytes(path: PathBuf, bytes: Vec<u8>) -> Self {
        let sha256 = sha256_bytes(&bytes);
        Self {
            path,
            sha256,
            content: PreparedContent::Bytes(bytes),
        }
    }

    pub(super) fn path_wire(&self) -> String {
        path_to_slash(&self.path)
    }

    pub(super) fn sha256(&self) -> &str {
        &self.sha256
    }
}

/// Order prepared payloads into the canonical persisted order: ascending
/// flattened forward-slash path — the order `validate_file_rows` enforces
/// and every payload aggregate hash consumes. Host `Path` order compares
/// component-wise and diverges from it whenever a directory name prefixes a
/// sibling file (`a/x` sorts before `a.md` component-wise, after it
/// flattened), so every `PreparedSlotFile` ordering flows through here —
/// never through a hand-written `Path` comparator.
pub(super) fn sort_prepared_files(files: &mut [PreparedSlotFile]) {
    files.sort_by_cached_key(PreparedSlotFile::path_wire);
}

/// Walk and hash a mixed source tree without touching its destination slot.
pub(super) fn prepare_source_tree(
    root: &Path,
    mode: CopyMode,
    policy: &super::source_policy::SourcePolicy,
) -> Result<Vec<PreparedSlotFile>, WorkspaceError> {
    let mut files = Vec::new();
    collect_source_files(root, root, mode, policy, &mut files)?;
    sort_prepared_files(&mut files);
    Ok(files)
}

/// Recipe-0 aggregate identity over prepared output bytes.
pub(super) fn compute_prepared_payload_hash(
    files: &[PreparedSlotFile],
) -> Result<ContentHash, WorkspaceError> {
    let mut hasher = Sha256::new();
    for file in files {
        hasher.update(file.path_wire().as_bytes());
        hasher.update([0]);
        match &file.content {
            PreparedContent::Observed { observation, .. } => hasher.update(observation.bytes()),
            PreparedContent::Bytes(bytes) => hasher.update(bytes),
            PreparedContent::Source { path, .. } => {
                hash_file_into(path, &mut hasher).map_err(|reason| {
                    WorkspaceError::SpecMaterialization {
                        path: path.clone(),
                        reason: format!("incoming payload cannot be aggregated: {reason}"),
                    }
                })?;
            }
        }
        hasher.update([0]);
    }
    Ok(ContentHash::from_validated(format!(
        "sha256:{}",
        super::slot_record::lower_hex(&hasher.finalize())
    )))
}

/// Reconcile exactly the old and incoming recorded footprints, then write the
/// new record last. Paths absent from the old record are never removed.
pub(super) fn reconcile_slot(
    slot: &Path,
    files: &[PreparedSlotFile],
    record: &SlotRecord,
) -> Result<MaterialiseReport, WorkspaceError> {
    reconcile_slot_mode(slot, files, record, false)
}

pub(super) fn reconcile_declared_slot(
    slot: &Path,
    files: &[PreparedSlotFile],
    record: &SlotRecord,
) -> Result<MaterialiseReport, WorkspaceError> {
    reconcile_slot_mode(slot, files, record, true)
}

fn reconcile_slot_mode(
    slot: &Path,
    files: &[PreparedSlotFile],
    record: &SlotRecord,
    strict_new: bool,
) -> Result<MaterialiseReport, WorkspaceError> {
    let old_record = inspect_existing_record(slot)?;
    let footprint = files.iter().map(|file| file.path.clone()).collect();

    if strict_new && old_record.is_none() {
        let mut empty = record.clone();
        empty.files.clear();
        let plan = DiffPlan::build(slot, files, &empty, true)?;
        fs::create_dir_all(slot).map_err(|error| io_err(slot, error))?;
        let mut written = Vec::new();
        for (file, disposition) in files.iter().zip(plan.dispositions) {
            if disposition != Placement::Keep {
                place_atomically(slot, file, disposition)?;
                written.push(file.path.clone());
            }
        }
        super::slot_record::write_slot_record_new(slot, record).map_err(|reason| {
            materialisation_error(&slot.join(SLOT_RECORD_FILENAME), format!("record publication refused after payload paths already published: {:?}; {reason}", written))
        })?;
        return Ok(MaterialiseReport::new(
            footprint,
            written,
            Vec::new(),
            false,
            None,
            record,
        ));
    }

    let Some(old_record) = old_record else {
        let migrated = slot.exists();
        migrate_unrecorded_slot(slot)?;
        place_all(slot, files)?;
        write_record_last(slot, record)?;
        let written = files.iter().map(|file| file.path.clone()).collect();
        return Ok(MaterialiseReport::new(
            footprint,
            written,
            Vec::new(),
            migrated,
            None,
            record,
        ));
    };

    let plan = DiffPlan::build(slot, files, &old_record, strict_new)?;
    let removed = remove_stale_files(slot, &plan.stale)?;
    let mut written = Vec::new();
    for (file, disposition) in files.iter().zip(plan.dispositions) {
        if disposition != Placement::Keep {
            place_atomically(slot, file, disposition)?;
            written.push(file.path.clone());
        }
    }
    write_record_last(slot, record)?;
    Ok(MaterialiseReport::new(
        footprint,
        written,
        removed,
        false,
        Some(&old_record),
        record,
    ))
}

fn inspect_existing_record(slot: &Path) -> Result<Option<SlotRecord>, WorkspaceError> {
    match fs::symlink_metadata(slot) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            return Err(materialisation_error(
                slot,
                "dependency slot is not a real directory",
            ));
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(io_err(slot, error)),
    }
    let record_path = slot.join(SLOT_RECORD_FILENAME);
    match fs::symlink_metadata(&record_path) {
        Ok(_) => read_slot_record(slot)
            .map(Some)
            .map_err(|reason| materialisation_error(&record_path, reason)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(io_err(&record_path, error)),
    }
}

fn directory_contains_only_stale(
    path: &Path,
    slot: &Path,
    stale: &BTreeSet<String>,
) -> Result<bool, WorkspaceError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(io_err(path, error)),
    };
    if !metadata.is_dir() {
        return Ok(false);
    }
    let mut saw_stale = false;
    for entry in fs::read_dir(path).map_err(|error| io_err(path, error))? {
        let entry = entry.map_err(|error| io_err(path, error))?;
        let child = entry.path();
        let file_type = entry.file_type().map_err(|error| io_err(&child, error))?;
        if file_type.is_dir() {
            if !directory_contains_only_stale(&child, slot, stale)? {
                return Ok(false);
            }
            saw_stale = true;
        } else {
            let relative = child
                .strip_prefix(slot)
                .expect("a walked slot path remains below its root");
            if !stale.contains(&path_to_slash(relative)) {
                return Ok(false);
            }
            saw_stale = true;
        }
    }
    Ok(saw_stale)
}

fn inspect_stale_paths(slot: &Path, stale: &[PathBuf]) -> Result<(), WorkspaceError> {
    for relative in stale {
        inspect_stale_parents(slot, relative)?;
        let path = slot.join(relative);
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.is_dir() => {
                return Err(materialisation_error(
                    &path,
                    "recorded stale payload became a directory; refusing to remove unrecorded contents",
                ));
            }
            Ok(metadata)
                if !metadata.file_type().is_file() && !metadata.file_type().is_symlink() =>
            {
                return Err(materialisation_error(
                    &path,
                    "recorded stale payload is neither a regular file nor a symbolic link",
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(io_err(&path, error)),
        }
    }
    Ok(())
}

fn inspect_stale_parents(slot: &Path, relative: &Path) -> Result<(), WorkspaceError> {
    let mut parents = relative
        .ancestors()
        .skip(1)
        .filter(|path| !path.as_os_str().is_empty())
        .collect::<Vec<_>>();
    parents.reverse();
    for parent in parents {
        let path = slot.join(parent);
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_dir() => {}
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(materialisation_error(
                    &path,
                    "recorded payload parent became a symbolic link; refusing to escape the slot",
                ));
            }
            Ok(_) => {
                return Err(materialisation_error(
                    &path,
                    "recorded payload parent is not a directory",
                ));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(io_err(&path, error)),
        }
    }
    Ok(())
}

fn inspect_incoming_parents(
    slot: &Path,
    incoming: &BTreeSet<String>,
    old: &BTreeMap<&str, &super::SlotFile>,
    stale: &[PathBuf],
) -> Result<(), WorkspaceError> {
    let stale: BTreeSet<String> = stale.iter().map(|path| path_to_slash(path)).collect();
    for wire in incoming {
        let path = Path::new(wire);
        for parent in path
            .ancestors()
            .skip(1)
            .filter(|path| !path.as_os_str().is_empty())
        {
            let parent_wire = path_to_slash(parent);
            match fs::symlink_metadata(slot.join(parent)) {
                Ok(metadata) if metadata.is_dir() => {}
                Ok(_) if old.contains_key(parent_wire.as_str()) && stale.contains(&parent_wire) => {
                }
                Ok(_) => {
                    return Err(materialisation_error(
                        &slot.join(parent),
                        "incoming payload parent collides with an unrecorded on-disk path",
                    ));
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(io_err(&slot.join(parent), error)),
            }
        }
    }
    Ok(())
}

fn refuse_incoming_topology_collisions(
    slot: &Path,
    incoming: &BTreeSet<String>,
) -> Result<(), WorkspaceError> {
    for path in incoming {
        for parent in Path::new(path)
            .ancestors()
            .skip(1)
            .filter(|path| !path.as_os_str().is_empty())
        {
            if incoming.contains(&path_to_slash(parent)) {
                return Err(materialisation_error(
                    &slot.join(path),
                    "incoming footprint maps both a file and one of its descendants",
                ));
            }
        }
    }
    Ok(())
}

fn migrate_unrecorded_slot(slot: &Path) -> Result<(), WorkspaceError> {
    if slot.exists() {
        fs::remove_dir_all(slot).map_err(|error| io_err(slot, error))?;
    }
    fs::create_dir_all(slot).map_err(|error| io_err(slot, error))
}

fn place_all(slot: &Path, files: &[PreparedSlotFile]) -> Result<(), WorkspaceError> {
    for file in files {
        place_atomically(slot, file, Placement::Replace)?;
    }
    Ok(())
}

fn staged_file(parent: &Path) -> Result<tempfile::NamedTempFile, WorkspaceError> {
    tempfile::Builder::new()
        .prefix(".vibe-place-")
        .tempfile_in(parent)
        .map_err(|error| io_err(parent, error))
}

fn persist_staged(
    temporary: tempfile::NamedTempFile,
    destination: &Path,
    expected_hash: &str,
) -> Result<(), WorkspaceError> {
    persist_staged_mode(temporary, destination, expected_hash, false)
}

fn persist_staged_mode(
    temporary: tempfile::NamedTempFile,
    destination: &Path,
    expected_hash: &str,
    create_new: bool,
) -> Result<(), WorkspaceError> {
    let actual = sha256_file(temporary.path())
        .map_err(|reason| materialisation_error(temporary.path(), reason))?;
    if actual != expected_hash {
        return Err(materialisation_error(
            destination,
            format!(
                "incoming payload changed while materialising: prepared {}, placed {actual}",
                expected_hash
            ),
        ));
    }
    temporary
        .as_file()
        .sync_all()
        .map_err(|error| io_err(temporary.path(), error))?;
    let staged = temporary.into_temp_path();
    let published = if create_new {
        staged.persist_noclobber(destination)
    } else {
        staged.persist(destination)
    };
    published.map_err(|error| io_err(destination, error.error))?;
    Ok(())
}

fn remove_stale_files(slot: &Path, stale: &[PathBuf]) -> Result<Vec<PathBuf>, WorkspaceError> {
    let mut stale = stale.to_vec();
    stale.sort_by_key(|path| std::cmp::Reverse(path.components().count()));
    let mut removed = Vec::new();
    for relative in stale {
        let path = slot.join(&relative);
        match fs::remove_file(&path) {
            Ok(()) => {
                removed.push(relative);
                prune_empty_parents(slot, path.parent());
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(io_err(&path, error)),
        }
    }
    removed.sort();
    Ok(removed)
}

fn prune_empty_parents(slot: &Path, mut parent: Option<&Path>) {
    while let Some(path) = parent {
        if path == slot || fs::remove_dir(path).is_err() {
            break;
        }
        parent = path.parent();
    }
}

fn write_record_last(slot: &Path, record: &SlotRecord) -> Result<(), WorkspaceError> {
    write_slot_record(slot, record)
        .map_err(|reason| materialisation_error(&slot.join(SLOT_RECORD_FILENAME), reason))
}

#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#FORMAT-SIMPLE"
)]
fn collect_source_files(
    directory: &Path,
    root: &Path,
    mode: CopyMode,
    policy: &super::source_policy::SourcePolicy,
    files: &mut Vec<PreparedSlotFile>,
) -> Result<(), WorkspaceError> {
    for entry in fs::read_dir(directory).map_err(|error| io_err(directory, error))? {
        let entry = entry.map_err(|error| io_err(directory, error))?;
        if entry.file_name() == ".git" {
            continue;
        }
        let source = entry.path();
        if super::derived::is_generated_boot_artifact(root, &source) {
            continue;
        }
        let file_type = entry.file_type().map_err(|error| io_err(&source, error))?;
        if file_type.is_dir() {
            collect_source_files(&source, root, mode, policy, files)?;
        } else if file_type.is_file() {
            let relative = source
                .strip_prefix(root)
                .expect("a walked source path remains below its root")
                .to_path_buf();
            let observed = policy.captured(&relative);
            // A copy's managed boot block is neutralised on the way in, so
            // the bytes a harness's downward discovery concatenates do not
            // describe another project's boot (PROP-076 `#SYS-NO-RECURSION`).
            // Only an instruction file is read for this, and only its block
            // changes.
            if super::instruction_block::is_instruction_file(&relative) {
                let bytes = match &observed {
                    Some(observation) => observation.bytes().to_vec(),
                    None => fs::read(&source).map_err(|error| io_err(&source, error))?,
                };
                if let Some(neutralised) = super::instruction_block::neutralise(&bytes) {
                    files.push(PreparedSlotFile::from_bytes(relative, neutralised));
                    continue;
                }
            }
            files.push(if let Some(observation) = observed {
                PreparedSlotFile::from_observation(relative, source, mode, observation)
            } else {
                PreparedSlotFile::from_source(relative, source, mode)?
            });
        }
    }
    Ok(())
}

fn hash_file_into(path: &Path, hasher: &mut Sha256) -> Result<(), String> {
    let mut file = fs::File::open(path)
        .map_err(|error| format!("cannot open `{}`: {error}", path.display()))?;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("cannot read `{}`: {error}", path.display()))?;
        if read == 0 {
            return Ok(());
        }
        hasher.update(&buffer[..read]);
    }
}

fn sha256_bytes(bytes: &[u8]) -> String {
    super::slot_record::lower_hex(&Sha256::digest(bytes))
}

fn materialisation_error(path: &Path, reason: impl Into<String>) -> WorkspaceError {
    WorkspaceError::SpecMaterialization {
        path: path.to_path_buf(),
        reason: reason.into(),
    }
}
