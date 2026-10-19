//! Place prepared payloads under either the legacy replace or strict
//! descriptor-owned no-replace publication policy.

use std::fs;
use std::io::Write;
use std::path::Path;

use super::{
    CopyMode, PreparedContent, PreparedSlotFile, PublicationPolicy, io_err, materialisation_error,
    sha256_file,
};
use crate::WorkspaceError;

pub(super) fn place_all(
    slot: &Path,
    files: &[PreparedSlotFile],
    policy: PublicationPolicy,
) -> Result<(), WorkspaceError> {
    for file in files {
        place_atomically(slot, file, policy == PublicationPolicy::Legacy)?;
    }
    Ok(())
}

pub(super) fn place_atomically(
    slot: &Path,
    file: &PreparedSlotFile,
    replace: bool,
) -> Result<(), WorkspaceError> {
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
            persist_staged(temporary, &destination, &file.sha256, replace)
        }
        PreparedContent::Source {
            path,
            mode: CopyMode::Copy,
        } => {
            let temporary = staged_file(parent)?;
            fs::copy(path, temporary.path()).map_err(|error| io_err(temporary.path(), error))?;
            persist_staged(temporary, &destination, &file.sha256, replace)
        }
        PreparedContent::Source {
            path,
            mode: CopyMode::Hardlink,
        } => place_hardlink(path, &destination, parent, &file.sha256, replace),
    }
}

/// A strict recorded hardlink update keeps the old exact-owned file until a
/// fully copied and hash-checked replacement is ready. The legacy hardlink
/// path still uses its existing remove-then-link behavior.
pub(super) fn place_strict_recorded(
    slot: &Path,
    file: &PreparedSlotFile,
) -> Result<(), WorkspaceError> {
    let PreparedContent::Source {
        path,
        mode: CopyMode::Hardlink,
    } = &file.content
    else {
        return place_atomically(slot, file, true);
    };
    let destination = slot.join(&file.path);
    let parent = destination
        .parent()
        .expect("a prepared relative file has a parent");
    fs::create_dir_all(parent).map_err(|error| io_err(parent, error))?;
    let temporary = staged_file(parent)?;
    fs::copy(path, temporary.path()).map_err(|error| io_err(temporary.path(), error))?;
    persist_staged(temporary, &destination, &file.sha256, true)
}

fn staged_file(parent: &Path) -> Result<tempfile::NamedTempFile, WorkspaceError> {
    tempfile::Builder::new()
        .prefix(".vibe-place-")
        .rand_bytes(super::path_limits::STAGING_RANDOM_BYTES)
        .tempfile_in(parent)
        .map_err(|error| io_err(parent, error))
}

fn persist_staged(
    temporary: tempfile::NamedTempFile,
    destination: &Path,
    expected_hash: &str,
    replace: bool,
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
    let temporary = temporary.into_temp_path();
    if replace {
        temporary
            .persist(destination)
            .map_err(|error| io_err(destination, error.error))?;
    } else {
        temporary
            .persist_noclobber(destination)
            .map_err(|error| occupied_or_io(destination, error.error))?;
    }
    Ok(())
}

fn occupied_or_io(path: &Path, error: std::io::Error) -> WorkspaceError {
    if error.kind() == std::io::ErrorKind::AlreadyExists {
        materialisation_error(
            path,
            "strict descriptor publication found an occupied new output path",
        )
    } else {
        io_err(path, error)
    }
}

fn place_hardlink(
    source: &Path,
    destination: &Path,
    parent: &Path,
    expected_hash: &str,
    replace: bool,
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
    if replace {
        match fs::remove_file(destination) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(io_err(destination, error)),
        }
    }
    match fs::hard_link(source, destination) {
        Ok(()) => {
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
        Err(error) if !replace && error.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(occupied_or_io(destination, error));
        }
        Err(_) => {}
    }
    let temporary = staged_file(parent)?;
    fs::copy(source, temporary.path()).map_err(|error| io_err(temporary.path(), error))?;
    persist_staged(temporary, destination, expected_hash, replace)
}
