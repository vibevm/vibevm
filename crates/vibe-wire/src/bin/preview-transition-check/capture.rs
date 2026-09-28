//! Exact Git and file witnesses for the captured Developer Preview 1 subject.

use std::fs;
use std::path::Path;
use std::process::Command;

use sha2::{Digest, Sha256};
use vibe_wire::generated::preview::e1::capture::Capture;
use vibe_wire::generated::preview::e1::transition::Transition;

use super::CheckFailure;

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn git(dir: &Path, args: &[&str]) -> Result<Vec<u8>, CheckFailure> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .map_err(|_| CheckFailure::new("git-unavailable", "git", "Git could not be started"))?;
    if !output.status.success() {
        return Err(CheckFailure::new(
            "git-object-missing",
            "git",
            "Required captured Git object is unavailable",
        ));
    }
    Ok(output.stdout)
}

fn git_oid(dir: &Path, expression: &str) -> Result<String, CheckFailure> {
    Ok(
        String::from_utf8_lossy(&git(dir, &["rev-parse", expression])?)
            .trim()
            .into(),
    )
}

fn git_blob(dir: &Path, expression: &str) -> Result<Vec<u8>, CheckFailure> {
    git(dir, &["show", expression])
}

pub(super) fn verify(
    transition_path: &Path,
    transition: &Transition,
) -> Result<Vec<String>, CheckFailure> {
    let release_dir = transition_path.parent().unwrap_or(Path::new("."));
    let captured = fs::read(release_dir.join(&transition.dp1.source)).map_err(|_| {
        CheckFailure::new(
            "capture-read",
            "dp1",
            "Registered capture file is unavailable",
        )
    })?;
    if sha256(&captured) != transition.dp1.capture_file_sha256 {
        return Err(CheckFailure::new(
            "capture-digest",
            "dp1",
            "Capture bytes differ from the transition index",
        ));
    }
    let capture: Capture = serde_json::from_slice(&captured)
        .map_err(|error| CheckFailure::new("capture-shape", "dp1", error.to_string()))?;
    capture
        .validate_preview()
        .map_err(|error| CheckFailure::new("capture-meaning", error.field, error.reason))?;
    if capture.capture_id != transition.dp1.capture_id
        || capture.repository_commit_sha1 != transition.dp1.commit_sha1
        || capture.repository_tree_sha1 != transition.dp1.git_tree_sha1
    {
        return Err(CheckFailure::new(
            "capture-binding",
            "dp1",
            "Capture identity differs from the transition index",
        ));
    }
    let raw = fs::read(release_dir.join(&capture.source_record)).map_err(|_| {
        CheckFailure::new(
            "raw-capture-read",
            "dp1",
            "Original capture observation is unavailable",
        )
    })?;
    if sha256(&raw) != capture.source_record_sha256 {
        return Err(CheckFailure::new(
            "raw-capture-digest",
            "dp1",
            "Original observation bytes changed",
        ));
    }

    let root = String::from_utf8_lossy(&git(release_dir, &["rev-parse", "--show-toplevel"])?)
        .trim()
        .to_owned();
    let root = Path::new(&root);
    let commit = capture.repository_commit_sha1.as_str();
    if git_oid(root, &format!("{commit}^{{tree}}"))? != capture.repository_tree_sha1 {
        return Err(CheckFailure::new(
            "repository-tree",
            "dp1",
            "Commit tree differs from captured identity",
        ));
    }
    for package in &capture.packages {
        if git_oid(root, &format!("{commit}:{}", package.slot))? != package.git_tree_sha1 {
            return Err(CheckFailure::new(
                "package-tree",
                &package.coordinate,
                "Package subtree differs from captured identity",
            ));
        }
        let manifest = git_blob(root, &format!("{commit}:{}/vibe.toml", package.slot))?;
        if sha256(&manifest) != package.manifest_sha256 {
            return Err(CheckFailure::new(
                "package-manifest",
                &package.coordinate,
                "Package manifest bytes differ",
            ));
        }
    }
    for file in &capture.files {
        if git_oid(root, &format!("{commit}:{}", file.path))? != file.git_blob_sha1 {
            return Err(CheckFailure::new(
                "source-blob",
                &file.path,
                "Source blob differs from captured identity",
            ));
        }
        if sha256(&git_blob(root, &format!("{commit}:{}", file.path))?) != file.sha256 {
            return Err(CheckFailure::new(
                "source-bytes",
                &file.path,
                "Source bytes differ from captured hash",
            ));
        }
    }
    Ok(vec![
        "capture-shape-and-meaning".into(),
        "original-observation-digest".into(),
        "repository-tree".into(),
        "package-subtree-and-manifest-identities".into(),
        "source-blob-and-byte-identities".into(),
        "source-declared-surface-linkage".into(),
    ])
}
