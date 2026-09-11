//! Deterministic ZIP construction and strict release/source ZIP reading.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read, Write};
use std::path::{Component, Path};

use anyhow::{Context, Result, bail, ensure};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime, ZipArchive, ZipWriter};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ArchiveEntry {
    pub(crate) name: String,
    pub(crate) bytes: Vec<u8>,
    pub(crate) mode: u32,
}

/// Write one byte-stable ZIP: sorted names, fixed DOS epoch, DEFLATE level 9,
/// and only the declared Unix permission bits.
pub(crate) fn write_zip(entries: &[ArchiveEntry]) -> Result<Vec<u8>> {
    write_zip_with(entries, CompressionMethod::Deflated, Some(9))
}

/// Write the canonical nested source ZIP without compression. STORED makes
/// its bytes architecture-independent by contract; each outer platform ZIP
/// still DEFLATE-compresses this complete entry for release transport.
pub(crate) fn write_stored_zip(entries: &[ArchiveEntry]) -> Result<Vec<u8>> {
    write_zip_with(entries, CompressionMethod::Stored, None)
}

fn write_zip_with(
    entries: &[ArchiveEntry],
    method: CompressionMethod,
    level: Option<i64>,
) -> Result<Vec<u8>> {
    ensure!(
        entries.len() < usize::from(u16::MAX),
        "ZIP contains too many entries for the non-ZIP64 release format"
    );
    let mut ordered = entries.iter().collect::<Vec<_>>();
    ordered.sort_by(|left, right| left.name.as_bytes().cmp(right.name.as_bytes()));
    for entry in &ordered {
        validate_archive_path(&entry.name)?;
        ensure!(
            entry.bytes.len() < u32::MAX as usize,
            "ZIP entry `{}` requires ZIP64",
            entry.name
        );
    }
    for pair in ordered.windows(2) {
        ensure!(
            pair[0].name != pair[1].name,
            "duplicate ZIP entry path `{}`",
            pair[0].name
        );
    }

    let cursor = Cursor::new(Vec::new());
    let mut writer = ZipWriter::new(cursor);
    let timestamp = DateTime::from_date_and_time(1980, 1, 1, 0, 0, 0)
        .context("constructing fixed ZIP timestamp")?;
    for entry in ordered {
        let mut options = SimpleFileOptions::default()
            .compression_method(method)
            .last_modified_time(timestamp)
            .unix_permissions(entry.mode & 0o777);
        if let Some(level) = level {
            options = options.compression_level(Some(level));
        }
        writer
            .start_file(&entry.name, options)
            .with_context(|| format!("starting ZIP entry `{}`", entry.name))?;
        writer
            .write_all(&entry.bytes)
            .with_context(|| format!("writing ZIP entry `{}`", entry.name))?;
    }
    let bytes = writer
        .finish()
        .context("finishing deterministic ZIP")?
        .into_inner();
    ensure!(
        bytes.len() < u32::MAX as usize,
        "release ZIP requires ZIP64"
    );
    Ok(bytes)
}

/// Read regular files from a ZIP while refusing every path or file type that
/// cannot be safely materialised as the committed source/release tree.
pub(crate) fn read_zip(bytes: &[u8]) -> Result<Vec<ArchiveEntry>> {
    read_zip_inner(bytes, None)
}

/// Read only the named entries, rejecting an entry from central-directory
/// metadata before allocation when its expanded size exceeds its independent
/// limit.
pub(crate) fn read_zip_bounded(
    bytes: &[u8],
    limits: &BTreeMap<String, u64>,
) -> Result<Vec<ArchiveEntry>> {
    read_zip_inner(bytes, Some(limits))
}

fn read_zip_inner(
    bytes: &[u8],
    limits: Option<&BTreeMap<String, u64>>,
) -> Result<Vec<ArchiveEntry>> {
    let mut archive =
        ZipArchive::new(Cursor::new(bytes)).context("opening ZIP central directory")?;
    ensure!(
        archive.len() < usize::from(u16::MAX),
        "ZIP contains too many entries for the non-ZIP64 release format"
    );
    let mut names = BTreeSet::new();
    let mut entries = Vec::new();
    for index in 0..archive.len() {
        let mut file = archive
            .by_index(index)
            .with_context(|| format!("opening ZIP entry {index}"))?;
        if file.is_dir() {
            continue;
        }
        let name = file.name().to_string();
        validate_archive_path(&name).with_context(|| format!("unsafe ZIP entry path `{name}`"))?;
        ensure!(
            names.insert(name.clone()),
            "duplicate ZIP entry path `{name}`"
        );
        let mode = file.unix_mode().unwrap_or(0o644);
        if let Some(limits) = limits {
            let max = limits
                .get(&name)
                .with_context(|| format!("ZIP contains undeclared entry `{name}`"))?;
            ensure!(
                file.size() <= *max,
                "ZIP entry `{name}` declares {} expanded bytes, exceeding its independent {max}-byte limit",
                file.size()
            );
        }
        ensure!(
            mode & 0o170000 != 0o120000,
            "ZIP entry `{name}` is a symbolic link; release archives admit regular files only"
        );
        ensure!(
            file.size() < u64::from(u32::MAX) && file.compressed_size() < u64::from(u32::MAX),
            "ZIP entry `{name}` requires ZIP64"
        );
        let expected_size = usize::try_from(file.size())
            .with_context(|| format!("ZIP entry `{name}` is too large for this host"))?;
        let mut content = Vec::with_capacity(expected_size);
        file.read_to_end(&mut content)
            .with_context(|| format!("reading ZIP entry `{name}`"))?;
        ensure!(
            content.len() == expected_size,
            "ZIP entry `{name}` declared {expected_size} bytes but yielded {}",
            content.len()
        );
        entries.push(ArchiveEntry {
            name,
            bytes: content,
            mode: mode & 0o777,
        });
    }
    entries.sort_by(|left, right| left.name.as_bytes().cmp(right.name.as_bytes()));
    Ok(entries)
}

fn validate_archive_path(name: &str) -> Result<()> {
    ensure!(!name.is_empty(), "archive path is empty");
    ensure!(!name.contains('\\'), "archive path contains a backslash");
    ensure!(!name.ends_with('/'), "archive path names a directory");
    ensure!(
        !name.contains(':'),
        "archive path contains a platform prefix"
    );
    ensure!(
        !name.chars().any(char::is_control),
        "archive path contains a control character"
    );
    let path = Path::new(name);
    ensure!(!path.is_absolute(), "archive path is absolute");
    for component in path.components() {
        match component {
            Component::Normal(_) => {}
            Component::CurDir => bail!("archive path contains `.`"),
            Component::ParentDir => bail!("archive path contains `..`"),
            Component::RootDir | Component::Prefix(_) => bail!("archive path is rooted"),
        }
    }
    ensure!(
        !name
            .split('/')
            .any(|segment| segment.is_empty() || matches!(segment, "." | "..")),
        "archive path contains an empty, `.` or `..` segment"
    );
    Ok(())
}

#[cfg(test)]
#[path = "archive/tests.rs"]
mod tests;
