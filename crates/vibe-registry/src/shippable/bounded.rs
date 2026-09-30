//! Exact small-tree replay of the frozen recipe-zero package hash.
//!
//! Unlike the legacy WalkDir implementation, this path refuses unlisted
//! children and reads each regular single-link file through a held no-follow
//! capability with an allocation ceiling. It serves one bounded local-source
//! admission, not arbitrary package trees.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use vibe_core::manifest::{Manifest, declarant_path};
use vibe_core::{ContentHash, PackageRef};
use vibe_safefs::Project;

use crate::RegistryError;
use crate::hash_recipe::{LEGACY0_EXCLUDES, RecipeId, order_entries};

const FILE_CAP: usize = 2 * 1024 * 1024;
const TOTAL_CAP: usize = 4 * 1024 * 1024;
const FILE_COUNT_CAP: usize = 8;
const CHILDREN_CAP: usize = 12;
const DEPTH_CAP: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedContentFile {
    pub relative_path: String,
    pub bytes: u64,
    pub sha256: String,
}

/// Replay the exact recipe-zero hash from a complete, small package-tree
/// witness. The expected rows are untrusted data and are checked against a
/// no-follow complete enumeration. A second pass plus root reopen detects
/// cooperating drift during this read; no external-writer content CAS follows.
pub fn compute_content_hash_legacy0_bounded_exact(
    root: &Path,
    package_pin: &PackageRef,
    expected: &[BoundedContentFile],
) -> Result<ContentHash, RegistryError> {
    validate_input(root, package_pin, expected)?;
    let project = Project::open(root).map_err(|error| invalid(root, error))?;
    let identity = project
        .root_identity()
        .map_err(|error| invalid(root, error))?;
    let first = read_complete(&project, expected)?;
    let manifest = first
        .iter()
        .find(|(path, _)| path == "vibe.toml")
        .ok_or_else(|| invalid(root, "package manifest was not captured"))?;
    let text = std::str::from_utf8(&manifest.1).map_err(|error| invalid(root, error))?;
    let authored = Manifest::parse_str(text).map_err(|error| invalid(root, error))?;
    let authored_pin = authored
        .require_package()
        .and_then(|package| package.as_package_ref())
        .map_err(|error| invalid(root, error))?;
    if &authored_pin != package_pin {
        return Err(invalid(root, "package manifest differs from exact pin"));
    }
    let reopened = Project::open(root).map_err(|error| invalid(root, error))?;
    if reopened
        .root_identity()
        .map_err(|error| invalid(root, error))?
        != identity
        || read_complete(&reopened, expected)? != first
    {
        return Err(invalid(
            root,
            "package root or bytes changed during bounded replay",
        ));
    }
    let mut digest = Sha256::new();
    for (path, bytes) in order_entries(RecipeId::Legacy0, first) {
        digest.update(path.as_bytes());
        digest.update([0]);
        digest.update(&bytes);
        digest.update([0]);
    }
    Ok(ContentHash::from_validated(format!(
        "sha256:{:x}",
        digest.finalize()
    )))
}

fn validate_input(
    root: &Path,
    package_pin: &PackageRef,
    expected: &[BoundedContentFile],
) -> Result<(), RegistryError> {
    if !root.is_absolute()
        || !package_pin.is_qualified()
        || !package_pin.version.is_exact_pin()
        || package_pin.kind.is_none()
        || expected.is_empty()
        || expected.len() > FILE_COUNT_CAP
    {
        return Err(invalid(root, "root, pin or file count is invalid"));
    }
    let mut total = 0_u64;
    let mut seen = BTreeSet::new();
    let mut previous: Option<&str> = None;
    for file in expected {
        declarant_path(Path::new(&file.relative_path))
            .map_err(|error| invalid(root, error.reason()))?;
        if file.relative_path.split('/').count() > DEPTH_CAP
            || file
                .relative_path
                .split('/')
                .any(|component| LEGACY0_EXCLUDES.contains(&component))
            || file.bytes > FILE_CAP as u64
            || previous.is_some_and(|path| path >= file.relative_path.as_str())
            || file
                .relative_path
                .match_indices('/')
                .any(|(index, _)| seen.contains(&file.relative_path[..index]))
        {
            return Err(invalid(
                root,
                "file path order, exclusion, depth or size is invalid",
            ));
        }
        let Some(hex) = file.sha256.strip_prefix("sha256:") else {
            return Err(invalid(root, "file SHA-256 prefix is invalid"));
        };
        if hex.len() != 64
            || !hex
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
        {
            return Err(invalid(root, "file SHA-256 shape is invalid"));
        }
        total = total
            .checked_add(file.bytes)
            .ok_or_else(|| invalid(root, "aggregate size overflow"))?;
        previous = Some(&file.relative_path);
        seen.insert(file.relative_path.as_str());
    }
    if total > TOTAL_CAP as u64 || !seen.contains("vibe.toml") {
        return Err(invalid(
            root,
            "aggregate size or package manifest membership is invalid",
        ));
    }
    Ok(())
}

fn read_complete(
    project: &Project,
    expected: &[BoundedContentFile],
) -> Result<Vec<(String, Vec<u8>)>, RegistryError> {
    let allowed: BTreeSet<&str> = expected
        .iter()
        .map(|file| file.relative_path.as_str())
        .collect();
    let mut found = Vec::new();
    let mut total = 0_usize;
    walk(project, "", &allowed, expected, &mut found, &mut total)?;
    found.sort_by(|a, b| a.0.cmp(&b.0));
    if found.len() != expected.len() {
        return Err(invalid(
            project.root_path(),
            "package file set is incomplete",
        ));
    }
    Ok(found)
}

fn walk(
    project: &Project,
    parent: &str,
    allowed: &BTreeSet<&str>,
    expected: &[BoundedContentFile],
    found: &mut Vec<(String, Vec<u8>)>,
    total: &mut usize,
) -> Result<(), RegistryError> {
    let directory = if parent.is_empty() {
        project
            .root_dir()
            .map_err(|error| invalid(project.root_path(), error))?
    } else {
        let components: Vec<&str> = parent.split('/').collect();
        project
            .dir(&components, false)
            .map_err(|error| invalid(project.root_path(), error))?
    };
    let names = project
        .child_names_bounded(&directory, CHILDREN_CAP)
        .map_err(|error| invalid(project.root_path(), error))?;
    for name in names {
        let relative = if parent.is_empty() {
            name
        } else {
            format!("{parent}/{name}")
        };
        let file = allowed.contains(relative.as_str());
        let directory = allowed.iter().any(|path| {
            path.strip_prefix(&relative)
                .is_some_and(|suffix| suffix.starts_with('/'))
        });
        if file == directory {
            return Err(invalid(
                project.root_path(),
                format!("unknown or conflicting package child `{relative}`"),
            ));
        }
        if directory {
            if relative.split('/').count() >= DEPTH_CAP {
                return Err(invalid(
                    project.root_path(),
                    "package directory exceeds depth cap",
                ));
            }
            let components: Vec<&str> = relative.split('/').collect();
            project
                .dir(&components, false)
                .map_err(|error| invalid(project.root_path(), error))?;
            walk(project, &relative, allowed, expected, found, total)?;
        } else {
            let row = expected
                .iter()
                .find(|row| row.relative_path == relative)
                .ok_or_else(|| invalid(project.root_path(), "listed file lacks expected row"))?;
            let snapshot = project
                .read_file_snapshot_bounded(&relative, FILE_CAP)
                .map_err(|error| invalid(project.root_path(), error))?
                .ok_or_else(|| {
                    invalid(
                        project.root_path(),
                        "listed file disappeared during capture",
                    )
                })?;
            *total = total
                .checked_add(snapshot.bytes.len())
                .ok_or_else(|| invalid(project.root_path(), "aggregate size overflow"))?;
            if *total > TOTAL_CAP
                || snapshot.size != row.bytes
                || format!("sha256:{}", snapshot.sha256) != row.sha256
                || found.len() == FILE_COUNT_CAP
            {
                return Err(invalid(
                    project.root_path(),
                    format!("file `{relative}` differs from expected bounded row"),
                ));
            }
            found.push((relative, snapshot.bytes));
        }
    }
    Ok(())
}

fn invalid(path: &Path, reason: impl std::fmt::Display) -> RegistryError {
    RegistryError::Io {
        path: PathBuf::from(path),
        source: std::io::Error::new(std::io::ErrorKind::InvalidData, reason.to_string()),
    }
}

#[cfg(test)]
mod tests;
