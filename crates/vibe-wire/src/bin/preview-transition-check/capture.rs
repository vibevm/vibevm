//! Exact Git and file witnesses for both captured Developer Preview subjects.

use std::path::Path;
use std::process::Command;

use sha2::{Digest, Sha256};
use vibe_safefs::Project;
use vibe_wire::generated::preview::e1::capture::Capture;
use vibe_wire::generated::preview::e1::transition::{CaptureRef, PendingCaptureState, Transition};

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
    let absolute = std::path::absolute(release_dir).map_err(|_| {
        CheckFailure::new(
            "release-root",
            "preview-transition",
            "Release root could not be resolved",
        )
    })?;
    let project = Project::open(&absolute).map_err(|_| {
        CheckFailure::new(
            "release-root",
            "preview-transition",
            "Release root could not be opened without following links",
        )
    })?;
    let references = transition
        .capture_references()
        .map_err(|error| CheckFailure::new("capture-meaning", error.field, error.reason))?;
    let mut claims = verify_ref(release_dir, &project, &transition.dp1, "dp1")?;
    if !matches!(transition.dp2.state, PendingCaptureState::Pending) {
        let Some(reference) = references.iter().find(|reference| {
            Some(reference.capture_id.as_str()) == transition.dp2.capture_id.as_deref()
                && Some(reference.source.as_str()) == transition.dp2.source.as_deref()
        }) else {
            return Err(CheckFailure::new(
                "capture-binding",
                "dp2",
                "Capture identity is incomplete",
            ));
        };
        claims.extend(verify_ref(release_dir, &project, reference, "dp2")?);
    }
    for reference in &references {
        let endpoint = reference == &transition.dp1
            || (Some(reference.capture_id.as_str()) == transition.dp2.capture_id.as_deref()
                && Some(reference.source.as_str()) == transition.dp2.source.as_deref());
        if !endpoint {
            claims.extend(verify_ref(
                release_dir,
                &project,
                reference,
                &format!("catalogue:{}", reference.capture_id),
            )?);
        }
    }
    Ok(claims)
}

fn read_within(
    project: &Project,
    relative: &str,
    subject: &str,
    code: &'static str,
) -> Result<Vec<u8>, CheckFailure> {
    let unavailable = || CheckFailure::new(code, subject, "Retained evidence file is unavailable");
    let refused = || {
        CheckFailure::new(
            "evidence-read-refused",
            subject,
            "Evidence must be a stable regular single-link file below the no-follow release root",
        )
    };
    let components: Vec<&str> = relative.split('/').collect();
    let Some((name, parents)) = components.split_last() else {
        return Err(unavailable());
    };
    let directory = project.dir(parents, false).map_err(|_| refused())?;
    let Some((_, observed_len)) = project
        .inspect_file_in(&directory, name)
        .map_err(|_| refused())?
    else {
        return Err(unavailable());
    };
    let limit = usize::try_from(observed_len).map_err(|_| refused())?;
    project
        .read_file_snapshot_bounded_in(&directory, name, limit)
        .map_err(|_| refused())?
        .map(|snapshot| snapshot.bytes)
        .ok_or_else(unavailable)
}

fn verify_ref(
    release_dir: &Path,
    project: &Project,
    reference: &CaptureRef,
    subject: &str,
) -> Result<Vec<String>, CheckFailure> {
    let captured = read_within(project, &reference.source, subject, "capture-read")?;
    if sha256(&captured) != reference.capture_file_sha256 {
        return Err(CheckFailure::new(
            "capture-digest",
            subject,
            "Capture bytes differ from the transition index",
        ));
    }
    let capture: Capture = serde_json::from_slice(&captured).map_err(|_| {
        CheckFailure::new(
            "capture-shape",
            subject,
            "Capture does not match the registered shape",
        )
    })?;
    capture.validate_preview().map_err(|error| {
        CheckFailure::new(
            "capture-meaning",
            format!("{subject}.{}", error.field),
            error.reason,
        )
    })?;
    if capture.capture_id != reference.capture_id
        || capture.repository_commit_sha1 != reference.commit_sha1
        || capture.repository_tree_sha1 != reference.git_tree_sha1
    {
        return Err(CheckFailure::new(
            "capture-binding",
            subject,
            "Capture identity differs from the transition index",
        ));
    }
    let components: Vec<&str> = capture.source_record.split('/').collect();
    let Some((name, parents)) = components.split_last() else {
        return Err(CheckFailure::new(
            "raw-capture-read",
            subject,
            "Original observation reference is missing",
        ));
    };
    let directory = project.dir(parents, false).map_err(|_| {
        CheckFailure::new(
            "evidence-read-refused",
            subject,
            "Original observation directory must remain below the no-follow release root",
        )
    })?;
    let raw = project.digest_file_in(&directory, name)
        .map_err(|_| CheckFailure::new("evidence-read-refused", subject, "Original observation must be a stable regular single-link file below the no-follow release root"))?
        .ok_or_else(|| CheckFailure::new("raw-capture-read", subject, "Original observation is unavailable"))?;
    let raw_digest: String = raw
        .sha256
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    if raw_digest != capture.source_record_sha256 {
        return Err(CheckFailure::new(
            "raw-capture-digest",
            subject,
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
            subject,
            "Commit tree differs from captured identity",
        ));
    }
    for package in &capture.packages {
        if git_oid(root, &format!("{commit}:{}", package.slot))? != package.git_tree_sha1 {
            return Err(CheckFailure::new(
                "package-tree",
                format!("{subject}:{}", package.coordinate),
                "Package subtree differs from captured identity",
            ));
        }
        let manifest = git_blob(root, &format!("{commit}:{}/vibe.toml", package.slot))?;
        if sha256(&manifest) != package.manifest_sha256 {
            return Err(CheckFailure::new(
                "package-manifest",
                format!("{subject}:{}", package.coordinate),
                "Package manifest bytes differ",
            ));
        }
    }
    for file in &capture.files {
        if git_oid(root, &format!("{commit}:{}", file.path))? != file.git_blob_sha1 {
            return Err(CheckFailure::new(
                "source-blob",
                format!("{subject}:{}", file.path),
                "Source blob differs from captured identity",
            ));
        }
        if sha256(&git_blob(root, &format!("{commit}:{}", file.path))?) != file.sha256 {
            return Err(CheckFailure::new(
                "source-bytes",
                format!("{subject}:{}", file.path),
                "Source bytes differ from captured hash",
            ));
        }
    }
    Ok([
        "capture-shape-and-meaning",
        "original-observation-digest",
        "repository-tree",
        "package-subtree-and-manifest-identities",
        "source-blob-and-byte-identities",
        "source-declared-surface-linkage",
    ]
    .into_iter()
    .map(|claim| format!("{subject}:{claim}"))
    .collect())
}
