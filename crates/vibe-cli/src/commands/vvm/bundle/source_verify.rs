use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::Path;

use anyhow::{Result, bail};
use vibe_publish::release_manifest::{
    DISTRIBUTION_SOURCE_EXPANDED_MAX_BYTES, DISTRIBUTION_SOURCE_MAX_DEPTH,
    DISTRIBUTION_SOURCE_MAX_FILES, DISTRIBUTION_SOURCE_MAX_MATERIALIZED_ENTRIES,
    DISTRIBUTION_SOURCE_MAX_PATH_BYTES,
};
use zip::ZipArchive;

use super::{
    collision_key, copy_and_hash_bounded, hash_regular_exact, safe_source_path, special_file_mode,
};
use crate::commands::vvm::store::open_regular_no_follow;

pub(super) fn source_matches_archive(archive_path: &Path, source_root: &Path) -> Result<bool> {
    if !source_root.is_dir() {
        return Ok(false);
    }
    let (file, _) = open_regular_no_follow(archive_path)?;
    let mut archive = ZipArchive::new(file)?;
    if archive.len() as u64 > DISTRIBUTION_SOURCE_MAX_FILES {
        return Ok(false);
    }
    let mut expected = BTreeSet::new();
    let mut total = 0_u64;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let name = entry.name().to_string();
        let is_dir = entry.is_dir();
        let Some(relative) = safe_source_path(&name, is_dir) else {
            return Ok(false);
        };
        if check_scan_limits(
            0,
            relative.components().count(),
            relative.to_string_lossy().len(),
        )
        .is_err()
        {
            return Ok(false);
        }
        if special_file_mode(entry.unix_mode(), is_dir) {
            return Ok(false);
        }
        let actual = source_root.join(&relative);
        let metadata = match fs::symlink_metadata(&actual) {
            Ok(metadata) => metadata,
            _ => return Ok(false),
        };
        if !mode_matches(&metadata, entry.unix_mode()) {
            return Ok(false);
        }
        if is_dir {
            if !metadata.file_type().is_dir() {
                return Ok(false);
            }
            continue;
        }
        if !metadata.file_type().is_file() || !expected.insert(collision_key(&relative)) {
            return Ok(false);
        }
        let remaining = DISTRIBUTION_SOURCE_EXPANDED_MAX_BYTES.saturating_sub(total);
        if entry.size() > remaining {
            return Ok(false);
        }
        let (entry_size, entry_digest) =
            copy_and_hash_bounded(&mut entry, &mut io::sink(), remaining)?;
        let actual_digest = hash_regular_exact(&actual, entry.size(), remaining, false)?;
        if entry_size != entry.size()
            || entry_size != metadata.len()
            || entry_digest != actual_digest
        {
            return Ok(false);
        }
        total += entry_size;
    }
    let mut actual = BTreeSet::new();
    collect_source_files(source_root, source_root, &mut actual)?;
    Ok(expected == actual)
}

fn mode_matches(metadata: &fs::Metadata, expected: Option<u32>) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        expected.is_none_or(|mode| metadata.permissions().mode() & 0o777 == mode & 0o777)
    }
    #[cfg(not(unix))]
    {
        let _ = (metadata, expected);
        true
    }
}

fn collect_source_files(root: &Path, dir: &Path, files: &mut BTreeSet<String>) -> Result<()> {
    let mut pending = vec![(dir.to_path_buf(), 0_usize)];
    let mut visited = 0_u64;
    while let Some((directory, depth)) = pending.pop() {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            visited += 1;
            let relative = entry.path().strip_prefix(root)?.to_path_buf();
            let child_depth = depth + 1;
            check_scan_limits(visited, child_depth, relative.to_string_lossy().len())?;
            let metadata = fs::symlink_metadata(entry.path())?;
            if metadata.file_type().is_symlink() {
                bail!("extracted source contains a symlink");
            }
            if metadata.is_dir() {
                pending.push((entry.path(), child_depth));
            } else if metadata.is_file() {
                if !files.insert(collision_key(&relative)) {
                    bail!("extracted source contains colliding paths");
                }
            } else {
                bail!("extracted source contains a special file");
            }
        }
    }
    Ok(())
}

fn check_scan_limits(entries: u64, depth: usize, path_bytes: usize) -> Result<()> {
    if entries > DISTRIBUTION_SOURCE_MAX_MATERIALIZED_ENTRIES {
        bail!("extracted source exceeds the maximum entry count");
    }
    if depth > DISTRIBUTION_SOURCE_MAX_DEPTH {
        bail!("extracted source exceeds the maximum directory depth");
    }
    if path_bytes > DISTRIBUTION_SOURCE_MAX_PATH_BYTES {
        bail!("extracted source path exceeds the maximum length");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracted_tree_scan_limits_fail_closed() {
        assert!(
            check_scan_limits(
                DISTRIBUTION_SOURCE_MAX_MATERIALIZED_ENTRIES,
                DISTRIBUTION_SOURCE_MAX_DEPTH,
                DISTRIBUTION_SOURCE_MAX_PATH_BYTES,
            )
            .is_ok()
        );
        assert!(check_scan_limits(DISTRIBUTION_SOURCE_MAX_MATERIALIZED_ENTRIES + 1, 1, 1).is_err());
        assert!(check_scan_limits(1, DISTRIBUTION_SOURCE_MAX_DEPTH + 1, 1).is_err());
        assert!(check_scan_limits(1, 1, DISTRIBUTION_SOURCE_MAX_PATH_BYTES + 1).is_err());
    }
}
