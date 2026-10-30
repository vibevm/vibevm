//! Zip and tar.gz payload extraction. Only plain files and directories with
//! portable, non-traversing, case-unique relative names are accepted; links
//! and special entries are refused.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#installer-separation");

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};

use specmark::spec;
use vibe_core::manifest::is_windows_unsafe_component;

use crate::error::InstallerError;

/// The archive container of an owned archive method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#installer-separation")]
pub enum ArchiveFormat {
    Zip,
    TarGz,
}

/// Extract `archive` into the new directory `destination`, then keep only
/// `payload_root` (when declared) as the payload: the selected subtree is
/// what `destination` holds on success.
pub fn extract(
    archive: &Path,
    format: ArchiveFormat,
    payload_root: Option<&Path>,
    destination: &Path,
) -> Result<(), InstallerError> {
    let name = destination
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let unpacked = destination.with_file_name(format!("{name}.unpack"));
    fs::create_dir(&unpacked).map_err(|source| super::io("creating", &unpacked, source))?;
    let result = unpack(archive, format, &unpacked).and_then(|files| {
        if files == 0 {
            return Err(refuse("the archive contains no files"));
        }
        let selected = match payload_root {
            Some(root) => unpacked.join(portable_path(&root.to_string_lossy())?),
            None => unpacked.clone(),
        };
        if !selected.is_dir() {
            return Err(refuse(
                "the declared payload_root is not a directory of the archive",
            ));
        }
        fs::rename(&selected, destination)
            .map_err(|source| super::io("placing payload", destination, source))
    });
    let _ = fs::remove_dir_all(&unpacked);
    result
}

fn unpack(archive: &Path, format: ArchiveFormat, root: &Path) -> Result<usize, InstallerError> {
    let open = || File::open(archive).map_err(|source| super::io("opening", archive, source));
    let mut names = Names::default();
    let mut files = 0;
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
                if kind.is_some_and(|kind| kind != 0o100000) {
                    return Err(refuse(&format!("`{name}` is a link or special entry")));
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
                if !kind.is_file() {
                    return Err(refuse(&format!("`{name}` is a link or special entry")));
                }
                let path = names.check(&name, false)?;
                let size = entry.size();
                let mode = entry.header().mode().ok();
                write_file(&mut entry, &root.join(path), size, mode)?;
                files += 1;
            }
        }
    }
    Ok(files)
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
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    fn zip_with(entries: &[(&str, &[u8])]) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("payload.zip");
        let mut zip = zip::ZipWriter::new(File::create(&path).unwrap());
        for (name, bytes) in entries {
            zip.start_file(*name, SimpleFileOptions::default().unix_permissions(0o755))
                .unwrap();
            zip.write_all(bytes).unwrap();
        }
        zip.finish().unwrap();
        (dir, path)
    }

    #[test]
    fn zip_payload_root_is_selected() {
        let (dir, archive) = zip_with(&[("app/bin/tool", b"tool"), ("app/README", b"r")]);
        let destination = dir.path().join("out");
        extract(
            &archive,
            ArchiveFormat::Zip,
            Some(Path::new("app")),
            &destination,
        )
        .unwrap();
        assert_eq!(fs::read(destination.join("bin/tool")).unwrap(), b"tool");
        assert!(!dir.path().join("out.unpack").exists());
    }

    #[test]
    fn unsafe_entries_are_refused() {
        for entries in [
            vec![("../escape", b"x".as_slice())],
            vec![("a/CON", b"x".as_slice())],
            vec![
                ("Bin/tool", b"x".as_slice()),
                ("bin/other", b"y".as_slice()),
            ],
            vec![("c:/x", b"x".as_slice())],
        ] {
            let (dir, archive) = zip_with(&entries);
            let destination = dir.path().join("out");
            assert!(
                extract(&archive, ArchiveFormat::Zip, None, &destination).is_err(),
                "{entries:?}"
            );
            assert!(!destination.exists());
        }
    }

    #[test]
    fn tar_gz_links_are_refused_and_files_extract() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("payload.tar.gz");
        let encoder = flate2::write::GzEncoder::new(
            File::create(&path).unwrap(),
            flate2::Compression::fast(),
        );
        let mut tar = tar::Builder::new(encoder);
        let mut header = tar::Header::new_gnu();
        header.set_size(4);
        header.set_mode(0o755);
        header.set_cksum();
        tar.append_data(&mut header, "bin/tool", b"tool".as_slice())
            .unwrap();
        tar.into_inner().unwrap().finish().unwrap();
        let destination = dir.path().join("out");
        extract(&path, ArchiveFormat::TarGz, None, &destination).unwrap();
        assert_eq!(fs::read(destination.join("bin/tool")).unwrap(), b"tool");

        let linked = dir.path().join("link.tar.gz");
        let encoder = flate2::write::GzEncoder::new(
            File::create(&linked).unwrap(),
            flate2::Compression::fast(),
        );
        let mut tar = tar::Builder::new(encoder);
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Symlink);
        header.set_size(0);
        header.set_cksum();
        tar.append_link(&mut header, "bin/evil", "/etc/passwd")
            .unwrap();
        tar.into_inner().unwrap().finish().unwrap();
        assert!(
            extract(
                &linked,
                ArchiveFormat::TarGz,
                None,
                &dir.path().join("out2")
            )
            .is_err()
        );

        // A PAX global header, as GitHub's tarballs carry, is metadata and
        // never refused as a special entry.
        let pax = dir.path().join("pax.tar.gz");
        let encoder =
            flate2::write::GzEncoder::new(File::create(&pax).unwrap(), flate2::Compression::fast());
        let mut tar = tar::Builder::new(encoder);
        let mut header = tar::Header::new_ustar();
        header.set_entry_type(tar::EntryType::XGlobalHeader);
        header.set_size(20);
        header.set_cksum();
        tar.append_data(
            &mut header,
            "pax_global_header",
            b"52 comment=abcdef12\n".as_slice(),
        )
        .unwrap();
        let mut header = tar::Header::new_gnu();
        header.set_size(4);
        header.set_mode(0o644);
        header.set_cksum();
        tar.append_data(&mut header, "repo-1.0/README", b"read".as_slice())
            .unwrap();
        tar.into_inner().unwrap().finish().unwrap();
        let out3 = dir.path().join("out3");
        extract(&pax, ArchiveFormat::TarGz, None, &out3).unwrap();
        assert_eq!(fs::read(out3.join("repo-1.0/README")).unwrap(), b"read");
    }
}
