//! Incremental reconciliation of a materialiser-owned slot footprint.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#DIFF-MATERIALISE");

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use vibe_core::ContentHash;

use super::{
    CopyMode, SLOT_RECORD_FILENAME, SlotRecord, io_err, read_slot_record, sha256_file,
    write_slot_record,
};
use crate::{WorkspaceError, path_to_slash};

mod plan;
mod report;

use plan::DiffPlan;
pub(crate) use report::MaterialiseReport;

/// Content prepared and hashed before the slot is mutated.
pub(super) struct PreparedSlotFile {
    path: PathBuf,
    sha256: String,
    content: PreparedContent,
}

enum PreparedContent {
    Source { path: PathBuf, mode: CopyMode },
    Bytes(Vec<u8>),
}

impl PreparedSlotFile {
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
) -> Result<Vec<PreparedSlotFile>, WorkspaceError> {
    let mut files = Vec::new();
    collect_source_files(root, root, mode, &mut files)?;
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
    let old_record = inspect_existing_record(slot)?;
    let footprint = files.iter().map(|file| file.path.clone()).collect();

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

    let plan = DiffPlan::build(slot, files, &old_record)?;
    let removed = remove_stale_files(slot, &plan.stale)?;
    let mut written = Vec::new();
    for (file, keep) in files.iter().zip(plan.keep) {
        if !keep {
            place_atomically(slot, file)?;
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

fn inspect_destination(
    path: &Path,
    old: Option<&super::SlotFile>,
    incoming_hash: &str,
    stale_scaffold: bool,
) -> Result<bool, WorkspaceError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if old.is_none() && metadata.file_type().is_file() => {
            let actual = sha256_file(path).map_err(|reason| materialisation_error(path, reason))?;
            if actual == incoming_hash {
                Ok(true)
            } else {
                Err(materialisation_error(
                    path,
                    "incoming payload collides with a different unrecorded on-disk file",
                ))
            }
        }
        Ok(metadata) if old.is_none() && metadata.is_dir() && stale_scaffold => Ok(false),
        Ok(_) if old.is_none() => Err(materialisation_error(
            path,
            "incoming payload collides with an unrecorded on-disk path",
        )),
        Ok(metadata) if metadata.is_dir() => Err(materialisation_error(
            path,
            "recorded payload path became a directory; refusing to remove unrecorded contents",
        )),
        Ok(metadata) if !metadata.file_type().is_file() && !metadata.file_type().is_symlink() => {
            Err(materialisation_error(
                path,
                "recorded payload path is neither a regular file nor a symbolic link",
            ))
        }
        Ok(metadata) if metadata.file_type().is_symlink() => Ok(false),
        Ok(_) => {
            let old = old.expect("the unrecorded case returned above");
            if old.sha256 != incoming_hash {
                return Ok(false);
            }
            let actual = sha256_file(path).map_err(|reason| materialisation_error(path, reason))?;
            Ok(actual == incoming_hash)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(io_err(path, error)),
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
        place_atomically(slot, file)?;
    }
    Ok(())
}

fn place_atomically(slot: &Path, file: &PreparedSlotFile) -> Result<(), WorkspaceError> {
    let destination = slot.join(&file.path);
    let parent = destination
        .parent()
        .expect("a prepared relative file always has a parent");
    fs::create_dir_all(parent).map_err(|error| io_err(parent, error))?;
    match &file.content {
        PreparedContent::Bytes(bytes) => {
            let mut temporary = staged_file(parent)?;
            temporary
                .as_file_mut()
                .write_all(bytes)
                .map_err(|error| io_err(temporary.path(), error))?;
            persist_staged(temporary, &destination, &file.sha256)
        }
        PreparedContent::Source {
            path,
            mode: CopyMode::Copy,
        } => {
            let temporary = staged_file(parent)?;
            fs::copy(path, temporary.path()).map_err(|error| io_err(temporary.path(), error))?;
            persist_staged(temporary, &destination, &file.sha256)
        }
        PreparedContent::Source {
            path,
            mode: CopyMode::Hardlink,
        } => place_hardlink(path, &destination, parent, &file.sha256),
    }
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
    temporary
        .into_temp_path()
        .persist(destination)
        .map_err(|error| io_err(destination, error.error))?;
    Ok(())
}

fn place_hardlink(
    source: &Path,
    destination: &Path,
    parent: &Path,
    expected_hash: &str,
) -> Result<(), WorkspaceError> {
    let source_hash =
        sha256_file(source).map_err(|reason| materialisation_error(source, reason))?;
    if source_hash != expected_hash {
        return Err(materialisation_error(
            destination,
            format!(
                "incoming payload changed while materialising: prepared {expected_hash}, source {source_hash}"
            ),
        ));
    }
    match fs::remove_file(destination) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(io_err(destination, error)),
    }
    if fs::hard_link(source, destination).is_ok() {
        let actual = sha256_file(destination)
            .map_err(|reason| materialisation_error(destination, reason))?;
        if actual == expected_hash {
            return Ok(());
        }
        let _ = fs::remove_file(destination);
        return Err(materialisation_error(
            destination,
            format!(
                "incoming payload changed while materialising: prepared {expected_hash}, linked {actual}"
            ),
        ));
    }
    let temporary = staged_file(parent)?;
    fs::copy(source, temporary.path()).map_err(|error| io_err(temporary.path(), error))?;
    persist_staged(temporary, destination, expected_hash)
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

fn collect_source_files(
    directory: &Path,
    root: &Path,
    mode: CopyMode,
    files: &mut Vec<PreparedSlotFile>,
) -> Result<(), WorkspaceError> {
    for entry in fs::read_dir(directory).map_err(|error| io_err(directory, error))? {
        let entry = entry.map_err(|error| io_err(directory, error))?;
        if entry.file_name() == ".git" {
            continue;
        }
        let source = entry.path();
        let file_type = entry.file_type().map_err(|error| io_err(&source, error))?;
        if file_type.is_dir() {
            collect_source_files(&source, root, mode, files)?;
        } else if file_type.is_file() {
            let relative = source
                .strip_prefix(root)
                .expect("a walked source path remains below its root")
                .to_path_buf();
            files.push(PreparedSlotFile::from_source(relative, source, mode)?);
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
