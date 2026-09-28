//! Exact Git and file witnesses for both captured Developer Preview subjects.

use std::path::Path;
use std::process::Command;

use sha2::{Digest, Sha256};
use vibe_safefs::Project;
use vibe_wire::generated::preview::e1::capture::Capture;
use vibe_wire::generated::preview::e1::transition::{CaptureRef, PendingCaptureState, Transition};

use super::CheckFailure;

pub(super) fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(super) fn git_output(dir: &Path, args: &[&str]) -> Result<std::process::Output, CheckFailure> {
    let mut command = Command::new("git");
    command.env_clear();
    if let Some(path) = std::env::var_os("PATH") {
        command.env("PATH", path);
    }
    #[cfg(windows)]
    for name in ["SystemRoot", "WINDIR"] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    let null = if cfg!(windows) { "NUL" } else { "/dev/null" };
    command.envs([
        ("LC_ALL", "C"),
        ("LANG", "C"),
        ("GIT_NO_LAZY_FETCH", "1"),
        ("GIT_NO_REPLACE_OBJECTS", "1"),
        ("GIT_OPTIONAL_LOCKS", "0"),
        ("GIT_TERMINAL_PROMPT", "0"),
        ("GIT_CONFIG_NOSYSTEM", "1"),
        ("GIT_CONFIG_GLOBAL", null),
        ("GIT_CONFIG_SYSTEM", null),
        ("GIT_GRAFT_FILE", null),
    ]);
    command
        .args([
            "--no-replace-objects",
            "--literal-pathspecs",
            "-c",
            "protocol.allow=never",
        ])
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .map_err(|_| CheckFailure::new("git-unavailable", "git", "Git could not be started"))
}

pub(super) fn git(dir: &Path, args: &[&str]) -> Result<Vec<u8>, CheckFailure> {
    let output = git_output(dir, args)?;
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
    let project = open_project(release_dir)?;
    let references = transition
        .capture_references()
        .map_err(|error| CheckFailure::new("capture-meaning", error.field, error.reason))?;
    let mut claims = verify_ref(release_dir, &project, &transition.dp1, "dp1")?.claims;
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
        claims.extend(verify_ref(release_dir, &project, reference, "dp2")?.claims);
    }
    for reference in &references {
        let endpoint = reference == &transition.dp1
            || (Some(reference.capture_id.as_str()) == transition.dp2.capture_id.as_deref()
                && Some(reference.source.as_str()) == transition.dp2.source.as_deref());
        if !endpoint {
            claims.extend(
                verify_ref(
                    release_dir,
                    &project,
                    reference,
                    &format!("catalogue:{}", reference.capture_id),
                )?
                .claims,
            );
        }
    }
    Ok(claims)
}

pub(super) fn open_project(directory: &Path) -> Result<Project, CheckFailure> {
    let absolute = std::path::absolute(directory).map_err(|_| {
        CheckFailure::new(
            "release-root",
            "preview-transition",
            "Release root could not be resolved",
        )
    })?;
    Project::open(&absolute).map_err(|_| {
        CheckFailure::new(
            "release-root",
            "preview-transition",
            "Release root could not be opened without following links",
        )
    })
}

pub(super) fn read_within(
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
    let Some(directory) = project.dir_if_present(parents).map_err(|_| refused())? else {
        return Err(unavailable());
    };
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

pub(super) fn digest_within(
    project: &Project,
    relative: &str,
    subject: &str,
    missing: &'static str,
) -> Result<String, CheckFailure> {
    let unavailable =
        || CheckFailure::new(missing, subject, "Retained evidence file is unavailable");
    let refused = || {
        CheckFailure::new(
            "evidence-read-refused",
            subject,
            "Evidence must be a stable regular single-link file below its no-follow root",
        )
    };
    let components: Vec<&str> = relative.split('/').collect();
    let Some((name, parents)) = components.split_last() else {
        return Err(unavailable());
    };
    let Some(directory) = project.dir_if_present(parents).map_err(|_| refused())? else {
        return Err(unavailable());
    };
    let digest = project
        .digest_file_in(&directory, name)
        .map_err(|_| refused())?
        .ok_or_else(unavailable)?;
    Ok(digest
        .sha256
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

pub(super) struct VerifiedCapture {
    pub capture: Capture,
    pub claims: Vec<String>,
}

pub(super) fn verify_ref(
    release_dir: &Path,
    project: &Project,
    reference: &CaptureRef,
    subject: &str,
) -> Result<VerifiedCapture, CheckFailure> {
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
    let raw_digest = digest_within(project, &capture.source_record, subject, "raw-capture-read")?;
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
    let claims = [
        "capture-shape-and-meaning",
        "original-observation-digest",
        "repository-tree",
        "package-subtree-and-manifest-identities",
        "source-blob-and-byte-identities",
        "source-declared-surface-linkage",
    ]
    .into_iter()
    .map(|claim| format!("{subject}:{claim}"))
    .collect();
    Ok(VerifiedCapture { capture, claims })
}
