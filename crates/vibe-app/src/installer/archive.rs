//! Zip and tar.gz payload extraction. Plain files and directories with
//! portable, non-traversing, case-unique relative names are accepted, and
//! so are symbolic links that stay inside the payload (`links`); hard
//! links and special entries are refused.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#installer-separation");

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};

use specmark::spec;
use vibe_core::manifest::is_windows_unsafe_component;

use crate::error::InstallerError;

mod links;
pub use links::LinkMode;

/// The archive container of an owned archive method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#installer-separation")]
pub enum ArchiveFormat {
    Zip,
    TarGz,
}

/// What an extraction could not carry over as declared: one line per link
/// the platform's [`LinkMode`] left out.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#installer-separation")]
pub struct Extracted {
    pub notes: Vec<String>,
}

/// Extract `archive` into the new directory `destination`, then keep only
/// `payload_root` (when declared) as the payload: the selected subtree is
/// what `destination` holds on success. Links are made the platform's way
/// ([`LinkMode::native`]).
pub fn extract(
    archive: &Path,
    format: ArchiveFormat,
    payload_root: Option<&Path>,
    destination: &Path,
) -> Result<Extracted, InstallerError> {
    extract_with(
        archive,
        format,
        payload_root,
        destination,
        LinkMode::native(),
    )
}

/// [`extract`] with an explicit [`LinkMode`], so both modes are testable
/// on every platform.
pub fn extract_with(
    archive: &Path,
    format: ArchiveFormat,
    payload_root: Option<&Path>,
    destination: &Path,
    mode: LinkMode,
) -> Result<Extracted, InstallerError> {
    let name = destination
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let unpacked = destination.with_file_name(format!("{name}.unpack"));
    fs::create_dir(&unpacked).map_err(|source| super::io("creating", &unpacked, source))?;
    let result = unpack(archive, format, &unpacked).and_then(|(files, found)| {
        if files == 0 {
            return Err(refuse("the archive contains no files"));
        }
        let base = payload_root
            .map(|root| portable_path(&root.to_string_lossy()))
            .transpose()?;
        let selected = match &base {
            Some(root) => unpacked.join(root),
            None => unpacked.clone(),
        };
        if !selected.is_dir() {
            return Err(refuse(
                "the declared payload_root is not a directory of the archive",
            ));
        }
        let notes = links::place(&selected, base.as_deref(), &found, mode)?;
        fs::rename(&selected, destination)
            .map_err(|source| super::io("placing payload", destination, source))?;
        Ok(Extracted { notes })
    });
    let _ = fs::remove_dir_all(&unpacked);
    result
}

/// Write every plain entry under `root`; links are collected, checked for
/// names like files, and created only after all plain entries exist.
fn unpack(
    archive: &Path,
    format: ArchiveFormat,
    root: &Path,
) -> Result<(usize, Vec<links::Link>), InstallerError> {
    let open = || File::open(archive).map_err(|source| super::io("opening", archive, source));
    let mut names = Names::default();
    let mut files = 0;
    let mut found = Vec::new();
    match format {
        ArchiveFormat::Zip => {
            let mut zip = zip::ZipArchive::new(open()?)
                .map_err(|error| refuse(&format!("unreadable zip: {error}")))?;
            for index in 0..zip.len() {
                let mut entry = zip
                    .by_index(index)
                    .map_err(|error| refuse(&format!("unreadable zip entry: {error}")))?;
                let name = entry.name().to_owned();
                let mode = entry.unix_mode();
                let kind = mode.map(|mode| mode & 0o170000).filter(|kind| *kind != 0);
                if entry.is_dir() {
                    if kind.is_some_and(|kind| kind != 0o040000) {
                        return Err(refuse(&format!("`{name}` is a special directory entry")));
                    }
                    names.check(&name, true)?;
                    continue;
                }
                if kind == Some(0o120000) {
                    let path = names.check(&name, false)?;
                    let size = entry.size();
                    let target = link_target(&mut entry, size, &name)?;
                    found.push(links::Link { path, name, target });
                    continue;
                }
                if kind.is_some_and(|kind| kind != 0o100000) {
                    return Err(refuse(&format!("`{name}` is a special entry")));
                }
                let path = names.check(&name, false)?;
                let size = entry.size();
                write_file(&mut entry, &root.join(path), size, mode)?;
                files += 1;
            }
        }
        ArchiveFormat::TarGz => {
            let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(open()?));
            let entries = tar
                .entries()
                .map_err(|error| refuse(&format!("unreadable tar.gz: {error}")))?;
            for entry in entries {
                let mut entry =
                    entry.map_err(|error| refuse(&format!("unreadable tar entry: {error}")))?;
                let name = entry
                    .path()
                    .map_err(|error| refuse(&format!("unreadable tar path: {error}")))?
                    .to_str()
                    .ok_or_else(|| refuse("a tar path is not UTF-8"))?
                    .to_owned();
                let kind = entry.header().entry_type();
                // PAX global and per-entry extension headers carry metadata
                // for the entries that follow (GitHub tarballs start with
                // `pax_global_header`); they are not payload.
                if matches!(
                    kind,
                    tar::EntryType::XGlobalHeader | tar::EntryType::XHeader
                ) {
                    continue;
                }
                if kind.is_dir() {
                    names.check(&name, true)?;
                    continue;
                }
                if kind == tar::EntryType::Symlink {
                    let path = names.check(&name, false)?;
                    let target = entry
                        .link_name()
                        .map_err(|error| refuse(&format!("unreadable tar link: {error}")))?
                        .ok_or_else(|| refuse(&format!("link `{name}` has no target")))?
                        .to_str()
                        .ok_or_else(|| refuse(&format!("link `{name}` target is not UTF-8")))?
                        .to_owned();
                    found.push(links::Link { path, name, target });
                    continue;
                }
                if !kind.is_file() {
                    return Err(refuse(&format!("`{name}` is a hard link or special entry")));
                }
                let path = names.check(&name, false)?;
                let size = entry.size();
                let mode = entry.header().mode().ok();
                write_file(&mut entry, &root.join(path), size, mode)?;
                files += 1;
            }
        }
    }
    Ok((files, found))
}

/// A zip link stores its target as the entry's contents.
fn link_target(input: &mut impl Read, size: u64, name: &str) -> Result<String, InstallerError> {
    if size > links::MAX_TARGET_BYTES {
        return Err(refuse(&format!("link `{name}` has an oversized target")));
    }
    let mut target = String::new();
    input
        .take(links::MAX_TARGET_BYTES)
        .read_to_string(&mut target)
        .map_err(|_| refuse(&format!("link `{name}` target is not UTF-8")))?;
    Ok(target)
}

/// Duplicate and case-conflict detection across all entries.
#[derive(Default)]
struct Names {
    seen: BTreeSet<String>,
    prefixes: BTreeMap<String, String>,
}

impl Names {
    fn check(&mut self, name: &str, directory: bool) -> Result<PathBuf, InstallerError> {
        let trimmed = if directory {
            name.trim_end_matches('/')
        } else {
            name
        };
        let path = portable_path(trimmed)?;
        if !self.seen.insert(trimmed.to_ascii_lowercase()) && !directory {
            return Err(refuse(&format!(
                "`{name}` is duplicated or case-conflicting"
            )));
        }
        let mut prefix = String::new();
        for component in trimmed.split('/') {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(component);
            let previous = self
                .prefixes
                .insert(prefix.to_ascii_lowercase(), prefix.clone());
            if previous.is_some_and(|previous| previous != prefix) {
                return Err(refuse(&format!(
                    "`{name}` has a case-conflicting component"
                )));
            }
        }
        Ok(path)
    }
}

/// A relative archive path with portable components only.
pub fn portable_path(value: &str) -> Result<PathBuf, InstallerError> {
    if value.is_empty() || value.contains('\\') || value.starts_with('/') || value.contains(':') {
        return Err(refuse(&format!(
            "`{value}` is not a portable relative path"
        )));
    }
    let path = PathBuf::from(value);
    let portable = path.components().all(|part| match part {
        Component::Normal(name) => name
            .to_str()
            .is_some_and(|name| !is_windows_unsafe_component(name)),
        _ => false,
    });
    if !portable {
        return Err(refuse(&format!(
            "`{value}` traverses or has an unsafe component"
        )));
    }
    Ok(path)
}

fn write_file(
    input: &mut impl Read,
    destination: &Path,
    size: u64,
    mode: Option<u32>,
) -> Result<(), InstallerError> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|source| super::io("creating", parent, source))?;
    }
    let mut output = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(destination)
        .map_err(|source| super::io("extracting", destination, source))?;
    let copied = io::copy(&mut input.take(size.saturating_add(1)), &mut output)
        .map_err(|source| super::io("extracting", destination, source))?;
    if copied != size {
        return Err(refuse(&format!(
            "`{}` does not match its declared size",
            destination.display()
        )));
    }
    set_mode(destination, mode)
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: Option<u32>) -> Result<(), InstallerError> {
    use std::os::unix::fs::PermissionsExt;
    match mode {
        Some(mode) => fs::set_permissions(path, fs::Permissions::from_mode(mode & 0o777))
            .map_err(|source| super::io("setting permissions on", path, source)),
        None => Ok(()),
    }
}

#[cfg(not(unix))]
fn set_mode(_path: &Path, _mode: Option<u32>) -> Result<(), InstallerError> {
    Ok(())
}

fn refuse(reason: &str) -> InstallerError {
    InstallerError::Archive {
        reason: reason.into(),
    }
}

#[cfg(test)]
mod tests;
