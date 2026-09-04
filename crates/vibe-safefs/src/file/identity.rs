//! What the OS calls "the same filesystem object", and how to ask.
//!
//! Split out of the publication cell it serves so neither outgrows the
//! file-length budget, and because everything here is one question with two
//! platform answers: two names that report this pair are one file, however
//! differently they are spelled — a hard link, a case-folding volume, a
//! junction one level up, an 8.3 short spelling, a bind mount.

use std::path::Path;

use anyhow::{Context, Result};

/// Opaque OS identity: volume + file index on Windows, device and inode on
/// Unix. Equality is public; the platform representation is not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileIdentity {
    volume: u64,
    index: u64,
}

impl FileIdentity {
    pub(crate) fn identity_bytes(self) -> [u8; 16] {
        let mut bytes = [0_u8; 16];
        bytes[..8].copy_from_slice(&self.volume.to_be_bytes());
        bytes[8..].copy_from_slice(&self.index.to_be_bytes());
        bytes
    }
}

/// The identity a path reports, with any injected alias applied.
///
/// In a shipped build the hook is compiled out to a `const None`, so this is
/// the identity unchanged and the alias arm is dead code the optimiser drops.
/// Where the hook is live, every member of one group reports a volume no real
/// volume serial or `dev_t` can hold — so an injected alias is total inside its
/// group and can never collide with a genuine identity outside it.
pub(crate) fn with_alias(identity: FileIdentity, relative: &str) -> FileIdentity {
    match crate::identity_hook::identity_alias(relative) {
        Some(group) => FileIdentity {
            volume: u64::MAX,
            index: group,
        },
        None => identity,
    }
}

#[cfg(windows)]
pub(crate) fn file_identity(file: &std::fs::File, display: &Path) -> Result<FileIdentity> {
    let information = winapi_util::file::information(file)
        .with_context(|| format!("inspecting `{}`", display.display()))?;
    Ok(FileIdentity {
        volume: information.volume_serial_number(),
        index: information.file_index(),
    })
}

#[cfg(not(windows))]
pub(crate) fn file_identity(file: &std::fs::File, display: &Path) -> Result<FileIdentity> {
    use std::os::unix::fs::MetadataExt;
    let metadata = file
        .metadata()
        .with_context(|| format!("inspecting `{}`", display.display()))?;
    Ok(FileIdentity {
        volume: metadata.dev(),
        index: metadata.ino(),
    })
}

#[cfg(windows)]
pub(crate) fn number_of_links(
    file: &std::fs::File,
    _metadata: &std::fs::Metadata,
    display: &Path,
) -> Result<u64> {
    let information = winapi_util::file::information(file)
        .with_context(|| format!("inspecting `{}`", display.display()))?;
    Ok(information.number_of_links())
}

#[cfg(not(windows))]
pub(crate) fn number_of_links(
    _file: &std::fs::File,
    metadata: &std::fs::Metadata,
    _display: &Path,
) -> Result<u64> {
    use std::os::unix::fs::MetadataExt;
    Ok(metadata.nlink())
}

#[cfg(windows)]
pub(crate) fn is_not_empty(error: &std::io::Error) -> bool {
    error.raw_os_error() == Some(145)
}

#[cfg(not(windows))]
pub(crate) fn is_not_empty(error: &std::io::Error) -> bool {
    error.raw_os_error() == Some(39)
}
