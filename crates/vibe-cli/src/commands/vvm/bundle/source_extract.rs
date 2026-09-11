use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use vibe_publish::release_manifest::{
    DISTRIBUTION_SOURCE_EXPANDED_MAX_BYTES, DISTRIBUTION_SOURCE_MAX_DEPTH,
    DISTRIBUTION_SOURCE_MAX_FILES, DISTRIBUTION_SOURCE_MAX_MATERIALIZED_ENTRIES,
    DISTRIBUTION_SOURCE_MAX_PATH_BYTES,
};
use zip::ZipArchive;

use super::file_verify::copy_bounded;
use crate::commands::vvm::store::open_regular_no_follow;

pub(super) fn extract_source_archive(archive_path: &Path, destination: &Path) -> Result<()> {
    let (file, _) = open_regular_no_follow(archive_path)?;
    let mut archive = ZipArchive::new(file).context("opening nested source ZIP")?;
    validate_archive_structure(&mut archive)?;
    let mut total = 0_u64;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let name = entry.name().to_string();
        let is_dir = entry.is_dir();
        let relative = safe_source_path(&name, is_dir)
            .with_context(|| format!("unsafe source ZIP entry `{name}`"))?;
        let remaining = DISTRIBUTION_SOURCE_EXPANDED_MAX_BYTES
            .checked_sub(total)
            .context("source ZIP exceeds the extracted-size limit")?;
        if entry.size() > remaining {
            bail!("source ZIP entry `{name}` exceeds the remaining extracted-size limit");
        }
        let output = destination.join(&relative);
        if is_dir {
            fs::create_dir_all(&output)?;
            continue;
        }
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&output)
            .with_context(|| format!("creating extracted source `{}`", output.display()))?;
        let copied = copy_bounded(&mut entry, &mut file, remaining)?;
        if copied != entry.size() {
            bail!("source ZIP entry `{name}` changed size while extracting");
        }
        total += copied;
        apply_zip_permissions(&output, entry.unix_mode())?;
    }
    Ok(())
}

fn validate_archive_structure(archive: &mut ZipArchive<fs::File>) -> Result<()> {
    if archive.len() as u64 > DISTRIBUTION_SOURCE_MAX_FILES {
        bail!("source ZIP has too many entries");
    }
    let mut archive_paths = BTreeSet::new();
    let mut directories = BTreeSet::new();
    let mut files = BTreeSet::new();
    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        let name = entry.name();
        let is_dir = entry.is_dir();
        let relative = safe_source_path(name, is_dir)
            .with_context(|| format!("unsafe source ZIP entry `{name}`"))?;
        let key = collision_key(&relative);
        if !archive_paths.insert(key.clone()) || special_file_mode(entry.unix_mode(), is_dir) {
            bail!("duplicate, symlink, or special source ZIP entry `{name}`");
        }
        let parts = key.split('/').collect::<Vec<_>>();
        validate_limits(0, parts.len(), relative.to_string_lossy().len())?;
        for end in 1..parts.len() {
            let parent = parts[..end].join("/");
            if files.contains(&parent) {
                bail!("source ZIP file/directory prefix collision at `{parent}`");
            }
            directories.insert(parent);
        }
        if is_dir {
            if files.contains(&key) {
                bail!("source ZIP file/directory collision at `{key}`");
            }
            directories.insert(key.clone());
        } else if directories.contains(&key) || !files.insert(key.clone()) {
            bail!("source ZIP file/directory collision at `{key}`");
        }
        validate_limits(
            (directories.len() + files.len()) as u64,
            parts.len(),
            relative.to_string_lossy().len(),
        )?;
    }
    Ok(())
}

fn validate_limits(materialized: u64, depth: usize, path_bytes: usize) -> Result<()> {
    if materialized > DISTRIBUTION_SOURCE_MAX_MATERIALIZED_ENTRIES {
        bail!("source ZIP exceeds the materialized entry limit");
    }
    if depth > DISTRIBUTION_SOURCE_MAX_DEPTH {
        bail!("source ZIP path exceeds the directory-depth limit");
    }
    if path_bytes > DISTRIBUTION_SOURCE_MAX_PATH_BYTES {
        bail!("source ZIP path exceeds the byte-length limit");
    }
    Ok(())
}

pub(crate) fn safe_source_path(name: &str, is_dir: bool) -> Option<PathBuf> {
    if name.is_empty()
        || !name.is_ascii()
        || name.starts_with('/')
        || name.starts_with('\\')
        || name.contains('\\')
        || name.contains(':')
        || name.contains('\0')
    {
        return None;
    }
    let normalized = if is_dir {
        name.strip_suffix('/').unwrap_or(name)
    } else {
        name
    };
    if normalized.is_empty()
        || normalized.split('/').any(|part| {
            part.is_empty() || part == "." || part == ".." || unsafe_windows_segment(part)
        })
    {
        return None;
    }
    Some(normalized.split('/').collect())
}

fn unsafe_windows_segment(segment: &str) -> bool {
    if segment.ends_with(['.', ' ']) {
        return true;
    }
    let stem = segment.split('.').next().unwrap_or(segment);
    let stem = stem.to_ascii_uppercase();
    matches!(
        stem.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || (stem.len() == 4
        && (stem.starts_with("COM") || stem.starts_with("LPT"))
        && matches!(stem.as_bytes()[3], b'1'..=b'9'))
}

pub(crate) fn collision_key(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase()
}

pub(crate) fn special_file_mode(mode: Option<u32>, is_dir: bool) -> bool {
    let Some(mode) = mode else {
        return false;
    };
    match mode & 0o170000 {
        0 => false,
        0o040000 => !is_dir,
        0o100000 => is_dir,
        _ => true,
    }
}

fn apply_zip_permissions(path: &Path, mode: Option<u32>) -> Result<()> {
    #[cfg(unix)]
    if let Some(mode) = mode {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(path)?.permissions();
        permissions.set_mode(mode & 0o777);
        fs::set_permissions(path, permissions)?;
    }
    #[cfg(not(unix))]
    let _ = (path, mode);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn materialized_source_limits_accept_exact_boundary_and_reject_next() {
        assert!(
            validate_limits(
                DISTRIBUTION_SOURCE_MAX_MATERIALIZED_ENTRIES,
                DISTRIBUTION_SOURCE_MAX_DEPTH,
                DISTRIBUTION_SOURCE_MAX_PATH_BYTES,
            )
            .is_ok()
        );
        assert!(validate_limits(DISTRIBUTION_SOURCE_MAX_MATERIALIZED_ENTRIES + 1, 1, 1).is_err());
        assert!(validate_limits(1, DISTRIBUTION_SOURCE_MAX_DEPTH + 1, 1).is_err());
        assert!(validate_limits(1, 1, DISTRIBUTION_SOURCE_MAX_PATH_BYTES + 1).is_err());
    }
}
