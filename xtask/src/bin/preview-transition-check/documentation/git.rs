//! Local immutable Git census and byte identity, never mutable worktree intake.

use super::{CheckFailure, CheckReport, CheckRequest, FindingState, capture, fault, pending};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use vibe_wire::generated::preview::e1::documentation_capture::{
    DocumentationCapture, SourceSubject,
};

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-081#DOCUMENTATION-INPUT-CHECK");

pub(super) struct VerifiedSource {
    pub files: BTreeMap<String, Vec<u8>>,
}

use super::super::git_entry;
use git_entry::Entry;

fn entry<'a>(
    row: &'a [u8],
    id: &str,
    report: &mut CheckReport,
) -> Result<Option<Entry<'a>>, CheckFailure> {
    match git_entry::decode(row) {
        Ok(entry) => Ok(Some(entry)),
        Err(git_entry::DecodeError::PathEncoding) => {
            pending(
                report,
                "documentation-git-path-unsupported",
                id,
                "Captured path is not representable by the UTF-8 wire format",
                FindingState::Unsupported,
            );
            Ok(None)
        }
        Err(error) => Err(fault("documentation-git-row", id, error.reason())),
    }
}

fn text(bytes: &[u8], id: &str) -> Result<String, CheckFailure> {
    let value = std::str::from_utf8(bytes).map_err(|_| {
        fault(
            "documentation-git-output",
            id,
            "Git identity output cannot be represented",
        )
    })?;
    Ok(value.strip_suffix('\n').unwrap_or(value).to_owned())
}

pub(super) fn verify(
    input: &DocumentationCapture,
    request: &CheckRequest,
    product_root: &Path,
    report: &mut CheckReport,
) -> Result<Option<VerifiedSource>, CheckFailure> {
    let id = input.capture_id.as_str();
    let SourceSubject::Git(subject) = &input.source_subject else {
        pending(
            report,
            "documentation-content-recipe-unsupported",
            id,
            "Content-tree identity has no bound existing recipe owner",
            FindingState::Unsupported,
        );
        return Ok(None);
    };
    let root = if subject.repository_key == "product" {
        product_root.to_path_buf()
    } else {
        let Some(value) = request
            .repository_roots
            .as_ref()
            .and_then(|roots| roots.get(&subject.repository_key))
        else {
            pending(
                report,
                "documentation-repository-root-missing",
                id,
                "Repository key has no explicit retrieval binding; no fallback was attempted",
                FindingState::Pending,
            );
            return Ok(None);
        };
        PathBuf::from(value)
    };
    if !root.is_absolute() {
        return Err(fault(
            "documentation-repository-root",
            id,
            "Supplied source repository root must be absolute",
        ));
    }
    let _pinned = capture::open_project(&root)?;
    let top = text(&capture::git(&root, &["rev-parse", "--show-toplevel"])?, id)?;
    if Path::new(&top) != root {
        return Err(fault(
            "documentation-repository-selection",
            id,
            "Git discovered a parent instead of the explicitly selected root",
        ));
    }
    let commit = subject.commit_sha1.as_str();
    if capture::git(&root, &["cat-file", "-t", commit])? != b"commit\n" {
        return Err(fault(
            "documentation-commit-kind",
            id,
            "Selected Git object is not an actual commit",
        ));
    }
    let tree = text(
        &capture::git(&root, &["rev-parse", &format!("{commit}^{{tree}}")])?,
        id,
    )?;
    if tree != subject.tree_sha1 {
        return Err(fault(
            "documentation-git-tree",
            id,
            "Selected commit tree differs from documentation capture",
        ));
    }
    let prefix = input.source_root_components.join("/");
    if !prefix.is_empty() {
        let selected = text(
            &capture::git(&root, &["rev-parse", &format!("{commit}:{prefix}")])?,
            id,
        )?;
        if capture::git(&root, &["cat-file", "-t", &selected])? != b"tree\n" {
            return Err(fault(
                "documentation-source-root",
                id,
                "Source components do not select a captured directory",
            ));
        }
    }
    let mut args = vec!["ls-tree", "-r", "-z", "--full-tree", commit];
    if !prefix.is_empty() {
        args.extend(["--", prefix.as_str()]);
    }
    let rows = capture::git(&root, &args)?;
    let mut observed = BTreeMap::new();
    let mut supported = true;
    for row in git_entry::rows(&rows) {
        let Some(row) = entry(row, id, report)? else {
            supported = false;
            continue;
        };
        let repository_path = row.repository_path;
        let relative = if prefix.is_empty() {
            repository_path
        } else {
            repository_path
                .strip_prefix(&format!("{prefix}/"))
                .ok_or_else(|| {
                    fault(
                        "documentation-git-scope",
                        id,
                        "Git census path escaped selected source-root coordinates",
                    )
                })?
        };
        if !row.regular() {
            pending(
                report,
                "documentation-git-entry-unsupported",
                &format!("{id}:{relative}"),
                "Symlink/gitlink/nonregular Git entry is not admitted or followed",
                FindingState::Unsupported,
            );
            supported = false;
            continue;
        }
        if observed
            .insert(relative.to_owned(), row.oid.to_owned())
            .is_some()
        {
            return Err(fault(
                "documentation-git-row",
                id,
                "Git census repeats a path",
            ));
        }
    }
    if !supported {
        return Ok(None);
    }
    let declared: BTreeSet<_> = input.files.iter().map(|file| file.path.as_str()).collect();
    if declared != observed.keys().map(String::as_str).collect() {
        return Err(fault(
            "documentation-file-census",
            id,
            "Captured source-root file membership differs from complete declared inventory",
        ));
    }
    let mut files = BTreeMap::new();
    for file in &input.files {
        let oid = &observed[&file.path];
        if file.git_blob_sha1.as_deref() != Some(oid) {
            return Err(fault(
                "documentation-file-blob",
                &format!("{id}:{}", file.path),
                "File Git blob differs from observed captured tree",
            ));
        }
        let bytes = capture::git(&root, &["cat-file", "blob", oid])?;
        if capture::sha256(&bytes) != file.sha256 {
            return Err(fault(
                "documentation-file-digest",
                &format!("{id}:{}", file.path),
                "File bytes differ from declared SHA-256",
            ));
        }
        report
            .checked_claims
            .push(format!("documentation-file-bytes:{id}:{}", file.path));
        files.insert(file.path.clone(), bytes);
    }
    for input_row in &input.build_inputs {
        let rows = capture::git(
            &root,
            &[
                "ls-tree",
                "-z",
                "--full-tree",
                commit,
                "--",
                &input_row.path,
            ],
        )?;
        let mut entries = git_entry::rows(&rows);
        let raw = entries.next().ok_or_else(|| {
            fault(
                "documentation-build-entry",
                id,
                "Build input is absent from immutable tree",
            )
        })?;
        if entries.next().is_some() {
            return Err(fault(
                "documentation-build-entry",
                id,
                "Build input does not select exactly one captured entry",
            ));
        }
        let Some(row) = entry(raw, id, report)? else {
            return Ok(None);
        };
        if row.repository_path != input_row.path {
            return Err(fault(
                "documentation-build-entry",
                id,
                "Build input selects a different captured path",
            ));
        }
        if !row.regular() {
            pending(
                report,
                "documentation-build-entry-unsupported",
                &format!("{id}:{}", input_row.path),
                "Nonregular build input is not admitted or followed",
                FindingState::Unsupported,
            );
            return Ok(None);
        }
        let oid = row.oid.to_owned();
        if input_row.git_blob_sha1.as_deref() != Some(oid.as_str()) {
            return Err(fault(
                "documentation-build-blob",
                id,
                "Build input blob identity differs from immutable source",
            ));
        }
        let bytes = capture::git(&root, &["cat-file", "blob", &oid])?;
        if capture::sha256(&bytes) != input_row.sha256 {
            return Err(fault(
                "documentation-build-digest",
                id,
                "Build input bytes differ from retained digest",
            ));
        }
        report.checked_claims.push(format!(
            "documentation-build-input-bytes:{id}:{}",
            input_row.path
        ));
    }
    report.checked_claims.extend([
        format!("documentation-git-tree:{id}"),
        format!("documentation-file-census:{id}"),
    ]);
    Ok(Some(VerifiedSource { files }))
}
