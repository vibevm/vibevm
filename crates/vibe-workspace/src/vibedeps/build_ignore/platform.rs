//! Safe no-follow opens and stable file identities for supported host families.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#IN-SLOT-BUILD");

use std::fs::File;
use std::io;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FileIdentity {
    volume: u64,
    file: u64,
}

#[cfg(unix)]
mod imp {
    use std::fs::OpenOptions;
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};

    use super::*;

    pub(super) fn open_existing_read(path: &Path) -> io::Result<File> {
        options(false, false).open(path).map_err(nofollow_error)
    }

    pub(super) fn open_existing_append(path: &Path) -> io::Result<File> {
        options(true, false).open(path).map_err(nofollow_error)
    }

    pub(super) fn create_new_append(path: &Path) -> io::Result<File> {
        options(true, true).open(path).map_err(nofollow_error)
    }

    fn options(append: bool, create_new: bool) -> OpenOptions {
        let mut options = OpenOptions::new();
        // Nonblocking prevents a race-substituted FIFO or device from hanging
        // the open before by-handle regular-file validation can refuse it.
        options
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
        if append {
            options.append(true);
        }
        if create_new {
            options.create_new(true);
        }
        options
    }

    fn nofollow_error(error: io::Error) -> io::Error {
        if error.raw_os_error() == Some(libc::ELOOP) {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "refusing symbolic-link `.gitignore`; no-follow open detected it",
            )
        } else {
            error
        }
    }

    pub(super) fn identity(file: &File) -> io::Result<FileIdentity> {
        let metadata = file.metadata()?;
        if !metadata.file_type().is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "refusing non-regular `.gitignore` handle",
            ));
        }
        if metadata.nlink() != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "refusing `.gitignore` with {} hardlinks; exactly one hardlink is required",
                    metadata.nlink()
                ),
            ));
        }
        Ok(FileIdentity {
            volume: metadata.dev(),
            file: metadata.ino(),
        })
    }
}

#[cfg(windows)]
mod imp {
    use std::fs::OpenOptions;
    use std::os::windows::fs::OpenOptionsExt;

    use super::*;
    use winapi_util::file;

    // WinBase.h: opening the reparse point itself is the Windows no-follow
    // operation. Kept local so vibe-workspace remains `forbid(unsafe_code)`.
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    const FILE_ATTRIBUTE_DIRECTORY: u64 = 0x0000_0010;
    const FILE_ATTRIBUTE_REPARSE_POINT: u64 = 0x0000_0400;

    pub(super) fn open_existing_read(path: &Path) -> io::Result<File> {
        options(false, false).open(path)
    }

    pub(super) fn open_existing_append(path: &Path) -> io::Result<File> {
        options(true, false).open(path)
    }

    pub(super) fn create_new_append(path: &Path) -> io::Result<File> {
        options(true, true).open(path)
    }

    fn options(append: bool, create_new: bool) -> OpenOptions {
        let mut options = OpenOptions::new();
        options
            .read(true)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
        if append {
            options.append(true);
        }
        if create_new {
            options.create_new(true);
        }
        options
    }

    pub(super) fn identity(file: &File) -> io::Result<FileIdentity> {
        let information = file::information(file)?;
        let attributes = information.file_attributes();
        if attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "refusing reparse-point or symbolic-link `.gitignore` handle",
            ));
        }
        if attributes & FILE_ATTRIBUTE_DIRECTORY != 0 || !file::typ(file)?.is_disk() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "refusing non-regular `.gitignore` handle",
            ));
        }
        let links = information.number_of_links();
        if links != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "refusing `.gitignore` with {links} hardlinks; exactly one hardlink is required"
                ),
            ));
        }
        Ok(FileIdentity {
            volume: information.volume_serial_number(),
            file: information.file_index(),
        })
    }
}

#[cfg(not(any(unix, windows)))]
compile_error!("build-output ignore safety supports Unix and Windows hosts only");

pub(super) fn open_existing_read(path: &Path) -> io::Result<File> {
    imp::open_existing_read(path)
}

pub(super) fn open_existing_append(path: &Path) -> io::Result<File> {
    imp::open_existing_append(path)
}

pub(super) fn create_new_append(path: &Path) -> io::Result<File> {
    imp::create_new_append(path)
}

pub(super) fn identity(file: &File) -> io::Result<FileIdentity> {
    imp::identity(file)
}
