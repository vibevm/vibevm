//! Copy-on-write detachment for hook-bearing hardlink slots.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-022#hardlink");

use std::fs::{self, File, OpenOptions, Permissions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::{SLOT_RECORD_FILENAME, io_err, read_slot_record, sha256_file};
use crate::WorkspaceError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileIdentity {
    volume: u64,
    file: u64,
}

struct OpenedFile {
    file: File,
    identity: FileIdentity,
    links: u64,
    permissions: Permissions,
    modified: SystemTime,
}

/// Detach every recorded payload inode that still has another hardlink.
///
/// Each replacement is copied through an adjacent temporary file, checked
/// against the slot record, synced, and atomically persisted. Private files
/// are left alone, which makes this idempotent between pre- and post-install
/// hooks even when the pre hook deliberately rewrote recorded payload.
pub(crate) fn detach_recorded_hardlinks(slot: &Path) -> Result<(), WorkspaceError> {
    validate_real_directory(slot)?;
    let record = read_slot_record(slot)
        .map_err(|reason| materialisation_error(&slot.join(SLOT_RECORD_FILENAME), reason))?;
    let mut stable_paths = Vec::with_capacity(record.files.len());
    for row in record.files {
        let relative = Path::new(&row.path);
        validate_parent_chain(slot, relative)?;
        let destination = slot.join(relative);
        let OpenedFile {
            mut file,
            identity,
            links,
            permissions,
            modified,
        } = open_recorded_file(&destination).map_err(|error| io_err(&destination, error))?;
        if links == 1 {
            recheck_path_identity(slot, relative, identity)?;
            stable_paths.push((relative.to_path_buf(), identity));
            continue;
        }

        let parent = destination
            .parent()
            .expect("a validated recorded path always has a parent");
        let mut temporary = tempfile::Builder::new()
            .prefix(".vibe-cow-")
            .tempfile_in(parent)
            .map_err(|error| io_err(parent, error))?;
        io::copy(&mut file, temporary.as_file_mut())
            .map_err(|error| io_err(temporary.path(), error))?;
        temporary
            .as_file_mut()
            .flush()
            .map_err(|error| io_err(temporary.path(), error))?;
        fs::set_permissions(temporary.path(), permissions)
            .map_err(|error| io_err(temporary.path(), error))?;
        temporary
            .as_file()
            .set_times(fs::FileTimes::new().set_modified(modified))
            .map_err(|error| io_err(temporary.path(), error))?;
        let actual = sha256_file(temporary.path())
            .map_err(|reason| materialisation_error(temporary.path(), reason))?;
        if actual != row.sha256 {
            return Err(materialisation_error(
                &destination,
                format!(
                    "recorded hardlink hashes to {actual}, slot record expects {}",
                    row.sha256
                ),
            ));
        }
        temporary
            .as_file()
            .sync_all()
            .map_err(|error| io_err(temporary.path(), error))?;
        recheck_path_identity(slot, relative, identity)?;
        validate_parent_chain(slot, relative)?;
        drop(file);
        temporary
            .into_temp_path()
            .persist(&destination)
            .map_err(|error| io_err(&destination, error.error))?;

        validate_parent_chain(slot, relative)?;
        let replacement =
            open_recorded_file(&destination).map_err(|error| io_err(&destination, error))?;
        if replacement.links != 1 {
            return Err(materialisation_error(
                &destination,
                format!(
                    "copy-on-write replacement still has {} hardlinks",
                    replacement.links
                ),
            ));
        }
        if replacement.modified != modified {
            return Err(materialisation_error(
                &destination,
                "copy-on-write replacement did not preserve the recorded payload mtime",
            ));
        }
        let replacement_identity = replacement.identity;
        drop(replacement);
        recheck_path_identity(slot, relative, replacement_identity)?;
        stable_paths.push((relative.to_path_buf(), replacement_identity));
    }
    for (relative, identity) in stable_paths {
        recheck_path_identity(slot, &relative, identity)?;
    }
    Ok(())
}

fn recheck_path_identity(
    slot: &Path,
    relative: &Path,
    expected: FileIdentity,
) -> Result<(), WorkspaceError> {
    validate_parent_chain(slot, relative)?;
    let path = slot.join(relative);
    let opened = open_recorded_file(&path).map_err(|error| io_err(&path, error))?;
    validate_parent_chain(slot, relative)?;
    if opened.identity != expected {
        return Err(materialisation_error(
            &path,
            "recorded payload path changed identity during copy-on-write preparation",
        ));
    }
    Ok(())
}

fn validate_parent_chain(slot: &Path, relative: &Path) -> Result<(), WorkspaceError> {
    let mut current = PathBuf::from(slot);
    if let Some(parent) = relative.parent() {
        for component in parent.components() {
            current.push(component.as_os_str());
            validate_real_directory(&current)?;
        }
    }
    Ok(())
}

fn validate_real_directory(path: &Path) -> Result<(), WorkspaceError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| io_err(path, error))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(materialisation_error(
            path,
            "recorded payload parent is not a real directory",
        ));
    }
    Ok(())
}

#[cfg(unix)]
fn open_recorded_file(path: &Path) -> io::Result<OpenedFile> {
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};

    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(nofollow_error)?;
    let metadata = file.metadata()?;
    if !metadata.file_type().is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "recorded payload is not a regular file",
        ));
    }
    Ok(OpenedFile {
        file,
        identity: FileIdentity {
            volume: metadata.dev(),
            file: metadata.ino(),
        },
        links: metadata.nlink(),
        permissions: metadata.permissions(),
        modified: metadata.modified()?,
    })
}

#[cfg(unix)]
fn nofollow_error(error: io::Error) -> io::Error {
    if error.raw_os_error() == Some(libc::ELOOP) {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "recorded payload is a symbolic link",
        )
    } else {
        error
    }
}

#[cfg(windows)]
fn open_recorded_file(path: &Path) -> io::Result<OpenedFile> {
    use std::os::windows::fs::OpenOptionsExt;
    use winapi_util::file;

    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    const FILE_ATTRIBUTE_DIRECTORY: u64 = 0x0000_0010;
    const FILE_ATTRIBUTE_REPARSE_POINT: u64 = 0x0000_0400;

    let file = OpenOptions::new()
        .read(true)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)?;
    let information = file::information(&file)?;
    let attributes = information.file_attributes();
    if attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "recorded payload is a reparse point or symbolic link",
        ));
    }
    if attributes & FILE_ATTRIBUTE_DIRECTORY != 0 || !file::typ(&file)?.is_disk() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "recorded payload is not a regular disk file",
        ));
    }
    let metadata = file.metadata()?;
    Ok(OpenedFile {
        file,
        identity: FileIdentity {
            volume: information.volume_serial_number(),
            file: information.file_index(),
        },
        links: information.number_of_links(),
        permissions: metadata.permissions(),
        modified: metadata.modified()?,
    })
}

#[cfg(not(any(unix, windows)))]
compile_error!("hardlink copy-on-write safety supports Unix and Windows hosts only");

fn materialisation_error(path: &Path, reason: impl Into<String>) -> WorkspaceError {
    WorkspaceError::SpecMaterialization {
        path: path.to_path_buf(),
        reason: reason.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_recheck_refuses_a_swapped_payload_path() {
        let slot = tempfile::tempdir().unwrap();
        let path = slot.path().join("payload.txt");
        fs::write(&path, "first").unwrap();
        let identity = open_recorded_file(&path).unwrap().identity;
        fs::remove_file(&path).unwrap();
        fs::write(&path, "replacement").unwrap();

        let error =
            recheck_path_identity(slot.path(), Path::new("payload.txt"), identity).unwrap_err();
        assert!(error.to_string().contains("changed identity"), "{error}");
    }

    #[test]
    fn identity_recheck_revalidates_the_parent_chain() {
        let slot = tempfile::tempdir().unwrap();
        let parent = slot.path().join("nested");
        fs::create_dir(&parent).unwrap();
        let path = parent.join("payload.txt");
        fs::write(&path, "first").unwrap();
        let identity = open_recorded_file(&path).unwrap().identity;
        fs::remove_file(&path).unwrap();
        fs::remove_dir(&parent).unwrap();
        fs::write(&parent, "not a directory").unwrap();

        let error = recheck_path_identity(slot.path(), Path::new("nested/payload.txt"), identity)
            .unwrap_err();
        assert!(
            error.to_string().contains("not a real directory"),
            "{error}"
        );
    }
}
