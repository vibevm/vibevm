//! Safe no-follow opens and stable single-link file identities.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FileIdentity {
    volume: u64,
    file: u64,
}

pub(crate) fn preflight_absent_or_regular(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Err(invalid("refusing symbolic-link or reparse-point path"))
        }
        Ok(metadata) if !metadata.file_type().is_file() => {
            Err(invalid("refusing non-file/non-regular file path"))
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

pub(crate) fn open_existing_read(path: &Path) -> io::Result<File> {
    imp::open(path, Access::Read, false).and_then(validate_open)
}

pub(crate) fn open_existing_read_write(path: &Path) -> io::Result<File> {
    imp::open(path, Access::ReadWrite, false).and_then(validate_open)
}

pub(crate) fn open_existing_append(path: &Path) -> io::Result<File> {
    imp::open(path, Access::Append, false).and_then(validate_open)
}

pub(crate) fn create_new_read_write(path: &Path) -> io::Result<File> {
    imp::open(path, Access::ReadWrite, true).and_then(validate_open)
}

pub(crate) fn create_new_append(path: &Path) -> io::Result<File> {
    imp::open(path, Access::Append, true).and_then(validate_open)
}

pub(crate) fn identity(file: &File) -> io::Result<FileIdentity> {
    imp::identity(file)
}

pub(crate) fn assert_path_identity(path: &Path, expected: FileIdentity) -> io::Result<()> {
    preflight_absent_or_regular(path)?;
    let file = open_existing_read(path)?;
    let actual = identity(&file)?;
    if actual == expected {
        Ok(())
    } else {
        Err(invalid(
            "pathname no longer names the validated open file handle",
        ))
    }
}

pub(crate) fn read_optional(path: &Path) -> io::Result<Option<Vec<u8>>> {
    preflight_absent_or_regular(path)?;
    let mut file = match open_existing_read(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let identity = identity(&file)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    assert_path_identity(path, identity)?;
    Ok(Some(bytes))
}

fn validate_open(file: File) -> io::Result<File> {
    identity(&file)?;
    Ok(file)
}

#[derive(Clone, Copy)]
enum Access {
    Read,
    ReadWrite,
    Append,
}

fn invalid(reason: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, reason)
}

#[cfg(unix)]
mod imp {
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};

    use super::*;

    pub(super) fn open(path: &Path, access: Access, create_new: bool) -> io::Result<File> {
        let mut options = OpenOptions::new();
        options
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .create_new(create_new);
        match access {
            Access::Read => {}
            Access::ReadWrite => {
                options.write(true);
            }
            Access::Append => {
                options.append(true);
            }
        }
        options.open(path).map_err(|error| {
            if error.raw_os_error() == Some(libc::ELOOP) {
                invalid("refusing symbolic-link path; no-follow open detected it")
            } else {
                error
            }
        })
    }

    pub(super) fn identity(file: &File) -> io::Result<FileIdentity> {
        let metadata = file.metadata()?;
        if !metadata.file_type().is_file() {
            return Err(invalid("refusing non-regular file handle"));
        }
        if metadata.nlink() != 1 {
            return Err(invalid(&format!(
                "refusing file with {} hardlinks; exactly one is required",
                metadata.nlink()
            )));
        }
        Ok(FileIdentity {
            volume: metadata.dev(),
            file: metadata.ino(),
        })
    }
}

#[cfg(windows)]
mod imp {
    use std::os::windows::fs::OpenOptionsExt;

    use super::*;
    use winapi_util::file;

    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    const FILE_ATTRIBUTE_DIRECTORY: u64 = 0x0000_0010;
    const FILE_ATTRIBUTE_REPARSE_POINT: u64 = 0x0000_0400;

    pub(super) fn open(path: &Path, access: Access, create_new: bool) -> io::Result<File> {
        let mut options = OpenOptions::new();
        options
            .read(true)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
            .create_new(create_new);
        match access {
            Access::Read => {}
            Access::ReadWrite => {
                options.write(true);
            }
            Access::Append => {
                options.append(true);
            }
        }
        options.open(path)
    }

    pub(super) fn identity(file: &File) -> io::Result<FileIdentity> {
        let information = file::information(file)?;
        let attributes = information.file_attributes();
        if attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(invalid(
                "refusing reparse-point or symbolic-link file handle",
            ));
        }
        if attributes & FILE_ATTRIBUTE_DIRECTORY != 0 || !file::typ(file)?.is_disk() {
            return Err(invalid("refusing non-regular file handle"));
        }
        let links = information.number_of_links();
        if links != 1 {
            return Err(invalid(&format!(
                "refusing file with {links} hardlinks; exactly one is required"
            )));
        }
        Ok(FileIdentity {
            volume: information.volume_serial_number(),
            file: information.file_index(),
        })
    }
}

#[cfg(not(any(unix, windows)))]
compile_error!("safe file identity supports Unix and Windows hosts only");
