//! Small, exact local package-tree witness for a protected fixture source.
//!
//! Complete bounded no-follow enumeration is a cooperative drift check. It
//! does not make source reads and later installation one atomic transaction.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use thiserror::Error;
use vibe_core::PackageRef;
use vibe_core::manifest::{Manifest, declarant_path};
use vibe_safefs::{FileIdentity, Project};

const FILE_CAP: u64 = 2 * 1024 * 1024;
const TOTAL_CAP: u64 = 4 * 1024 * 1024;
const FILE_COUNT_CAP: usize = 8;
const CHILDREN_CAP: usize = 12;
const DEPTH_CAP: usize = 4;
const MANIFEST_CAP: usize = 1024 * 1024;
const TREE_DOMAIN: &[u8] = b"vibevm.lifecycle.package-tree/1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageTreeFileWitness {
    pub relative_path: String,
    pub bytes: u64,
    pub sha256: String,
}

/// One exact allowed file set and its complete observed tree. Files are
/// sorted by portable UTF-8 path; the root identity is opaque and compared
/// locally, never serialized as an authority claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageTreeWitness {
    pub root: PathBuf,
    pub root_identity: FileIdentity,
    pub package_pin: PackageRef,
    pub files: Vec<PackageTreeFileWitness>,
    pub tree_sha256: String,
}

#[derive(Debug, Error)]
pub enum PackageTreeWitnessError {
    #[error("invalid fixture package-tree selection: {0}")]
    InvalidSelection(String),
    #[error("capturing fixture package tree failed: {0}")]
    Capture(String),
    #[error("fixture package tree changed: {0}")]
    Drift(String),
}

/// Capture a small source tree whose complete expected file membership is
/// supplied by the admitting fixture. Unknown children, including unknown
/// directories, refuse. The selected package's own manifest must agree with
/// the exact kind/group/name/version pin.
pub fn capture_package_tree_witness(
    root: &Path,
    package_pin: &PackageRef,
    allowed_files: &[&str],
) -> Result<PackageTreeWitness, PackageTreeWitnessError> {
    validate_selection(root, package_pin, allowed_files)?;
    let project = Project::open(root).map_err(capture_error)?;
    let root_identity = project.root_identity().map_err(capture_error)?;
    let allowed: BTreeSet<String> = allowed_files
        .iter()
        .map(|path| (*path).to_owned())
        .collect();
    let files = walk(&project, &allowed)?;
    check_manifest(&project, package_pin, &files)?;
    // Reopen the pathname and enumerate again. This is a narrow stability
    // check across the capture, not an exclusion of non-cooperating writers.
    let reopened = Project::open(root).map_err(capture_error)?;
    if reopened.root_identity().map_err(capture_error)? != root_identity
        || walk(&reopened, &allowed)? != files
    {
        return Err(PackageTreeWitnessError::Drift(
            "root identity or file census changed during capture".into(),
        ));
    }
    let tree_sha256 = tree_digest(&files);
    Ok(PackageTreeWitness {
        root: root.to_path_buf(),
        root_identity,
        package_pin: package_pin.clone(),
        files,
        tree_sha256,
    })
}

/// Reobserve through the same bounded no-follow path before source effects.
/// A matching digest alone is insufficient: root identity, pin and full file
/// membership also have to match.
pub fn recheck_package_tree_witness(
    expected: &PackageTreeWitness,
) -> Result<(), PackageTreeWitnessError> {
    let allowed: Vec<&str> = expected
        .files
        .iter()
        .map(|file| file.relative_path.as_str())
        .collect();
    let observed = capture_package_tree_witness(&expected.root, &expected.package_pin, &allowed)?;
    if &observed != expected {
        return Err(PackageTreeWitnessError::Drift(
            "root identity, package pin, file census or bytes differ".into(),
        ));
    }
    Ok(())
}

fn validate_selection(
    root: &Path,
    package_pin: &PackageRef,
    allowed_files: &[&str],
) -> Result<(), PackageTreeWitnessError> {
    let invalid = |reason: &str| PackageTreeWitnessError::InvalidSelection(reason.into());
    if !root.is_absolute() {
        return Err(invalid("root must be absolute"));
    }
    if !package_pin.is_qualified()
        || !package_pin.version.is_exact_pin()
        || package_pin.kind.is_none()
    {
        return Err(invalid(
            "package pin must name exact kind, group, name and version",
        ));
    }
    if allowed_files.is_empty() || allowed_files.len() > FILE_COUNT_CAP {
        return Err(invalid("file count is empty or over the fixture cap"));
    }
    let mut seen = BTreeSet::new();
    for path in allowed_files {
        declarant_path(Path::new(path)).map_err(|error| invalid(error.reason()))?;
        if path.split('/').count() > DEPTH_CAP || !seen.insert(*path) {
            return Err(invalid("duplicate or over-depth fixture path"));
        }
    }
    if !seen.contains("vibe.toml") {
        return Err(invalid("exact package manifest must be in file set"));
    }
    Ok(())
}

fn walk(
    project: &Project,
    allowed: &BTreeSet<String>,
) -> Result<Vec<PackageTreeFileWitness>, PackageTreeWitnessError> {
    let mut found = Vec::new();
    let mut total = 0_u64;
    walk_dir(project, "", allowed, &mut found, &mut total)?;
    found.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    if found.len() != allowed.len() {
        return Err(PackageTreeWitnessError::Drift(
            "one or more declared fixture files are absent".into(),
        ));
    }
    Ok(found)
}

fn walk_dir(
    project: &Project,
    parent: &str,
    allowed: &BTreeSet<String>,
    found: &mut Vec<PackageTreeFileWitness>,
    total: &mut u64,
) -> Result<(), PackageTreeWitnessError> {
    let directory = if parent.is_empty() {
        project.root_dir().map_err(capture_error)?
    } else {
        let components: Vec<&str> = parent.split('/').collect();
        project.dir(&components, false).map_err(capture_error)?
    };
    let names = project
        .child_names_bounded(&directory, CHILDREN_CAP)
        .map_err(capture_error)?;
    for name in names {
        let relative = if parent.is_empty() {
            name
        } else {
            format!("{parent}/{name}")
        };
        let is_file = allowed.contains(&relative);
        let is_directory = allowed.iter().any(|path| {
            path.strip_prefix(&relative)
                .is_some_and(|suffix| suffix.starts_with('/'))
        });
        if is_file == is_directory {
            return Err(PackageTreeWitnessError::Capture(format!(
                "unknown or conflicting fixture child `{relative}`"
            )));
        }
        if is_directory {
            if relative.split('/').count() >= DEPTH_CAP {
                return Err(PackageTreeWitnessError::Capture(format!(
                    "fixture directory `{relative}` exceeds depth cap"
                )));
            }
            let components: Vec<&str> = relative.split('/').collect();
            project.dir(&components, false).map_err(capture_error)?;
            walk_dir(project, &relative, allowed, found, total)?;
        } else {
            let state = project
                .stable_file_state_bounded(&relative, FILE_CAP)
                .map_err(capture_error)?
                .ok_or_else(|| {
                    PackageTreeWitnessError::Drift(format!(
                        "fixture file `{relative}` disappeared during capture"
                    ))
                })?;
            *total = total.checked_add(state.bytes).ok_or_else(|| {
                PackageTreeWitnessError::Capture("fixture aggregate bytes overflow".into())
            })?;
            if *total > TOTAL_CAP || found.len() == FILE_COUNT_CAP {
                return Err(PackageTreeWitnessError::Capture(
                    "fixture aggregate bytes or file count exceeds cap".into(),
                ));
            }
            found.push(PackageTreeFileWitness {
                relative_path: relative,
                bytes: state.bytes,
                sha256: format!("sha256:{}", state.sha256),
            });
        }
    }
    Ok(())
}

fn check_manifest(
    project: &Project,
    package_pin: &PackageRef,
    files: &[PackageTreeFileWitness],
) -> Result<(), PackageTreeWitnessError> {
    let snapshot = project
        .read_file_snapshot_bounded("vibe.toml", MANIFEST_CAP)
        .map_err(capture_error)?
        .ok_or_else(|| PackageTreeWitnessError::Drift("package manifest disappeared".into()))?;
    let row = files
        .iter()
        .find(|file| file.relative_path == "vibe.toml")
        .ok_or_else(|| PackageTreeWitnessError::Drift("package manifest not enumerated".into()))?;
    if row.bytes != snapshot.size || row.sha256 != format!("sha256:{}", snapshot.sha256) {
        return Err(PackageTreeWitnessError::Drift(
            "package manifest differs from tree census".into(),
        ));
    }
    let text = std::str::from_utf8(&snapshot.bytes).map_err(capture_error)?;
    let manifest = Manifest::parse_str(text).map_err(capture_error)?;
    let authored = manifest.require_package().map_err(capture_error)?;
    if authored.as_package_ref().map_err(capture_error)? != *package_pin {
        return Err(PackageTreeWitnessError::Capture(
            "package manifest identity differs from exact pin".into(),
        ));
    }
    Ok(())
}

fn tree_digest(files: &[PackageTreeFileWitness]) -> String {
    let mut digest = Sha256::new();
    frame(&mut digest, TREE_DOMAIN);
    for file in files {
        frame(&mut digest, file.relative_path.as_bytes());
        frame(&mut digest, &file.bytes.to_be_bytes());
        frame(&mut digest, file.sha256.as_bytes());
    }
    format!("sha256:{:x}", digest.finalize())
}

fn frame(digest: &mut Sha256, bytes: &[u8]) {
    digest.update((bytes.len() as u64).to_be_bytes());
    digest.update(bytes);
}

fn capture_error(error: impl std::fmt::Display) -> PackageTreeWitnessError {
    PackageTreeWitnessError::Capture(error.to_string())
}

#[cfg(test)]
mod tests;
