//! Ref-kind-aware clone refresh for [`super::ShellGit`].
//!
//! A package version names a tag, while registry and lock-driven callers may
//! name a branch or exact commit. Those are different namespaces: a tag has
//! no `origin/<name>` remote-tracking ref, and an annotated tag must be peeled
//! to its commit before checkout. This module keeps those decisions beside
//! the fetch refspecs that make them true.

use super::*;

pub(super) fn bootstrap(
    git: &ShellGit,
    url: &str,
    refname: &str,
    dest: &Path,
) -> Result<(), GitError> {
    let dest_s = dest.to_string_lossy();
    // Do not let clone's DWIM `--branch` interpretation decide whether the
    // requested name is a tag or branch. Fetch exactly one target below.
    // `--no-tags` also prevents unrelated or stale tags from affecting that
    // decision. Submodules are initialised after the target commit is known.
    git.run(
        &[
            "clone",
            "--recurse-submodules",
            "--no-checkout",
            "--no-tags",
            "--",
            url,
            dest_s.as_ref(),
        ],
        None,
    )?;
    fetch_and_checkout(git, dest, refname)?;
    sync_submodules(git, dest)
}

pub(super) fn update(git: &ShellGit, dest: &Path, refname: &str) -> Result<(), GitError> {
    fetch_and_checkout(git, dest, refname)?;
    sync_submodules(git, dest)
}

/// Fetch and check out one unambiguous namespace. Unqualified names retain
/// the registry's tag-first convention, but fall through only when git says
/// that exact remote ref is absent; authentication and transport failures are
/// never hidden by another interpretation.
fn fetch_and_checkout(git: &ShellGit, dest: &Path, refname: &str) -> Result<(), GitError> {
    if is_exact_object_id(refname) {
        return fetch_commit(git, dest, refname);
    }
    if let Some(tag) = refname.strip_prefix("refs/tags/") {
        return fetch_tag(git, dest, tag);
    }
    if let Some(branch) = refname.strip_prefix("refs/heads/") {
        return fetch_branch(git, dest, branch);
    }

    match fetch_tag(git, dest, refname) {
        Ok(()) => return Ok(()),
        Err(GitError::RefNotFound { .. }) => {}
        Err(error) => return Err(error),
    }
    match fetch_branch(git, dest, refname) {
        Ok(()) => return Ok(()),
        Err(GitError::RefNotFound { .. }) => {}
        Err(error) => return Err(error),
    }
    // Preserve the historical arbitrary-ref fallback (including abbreviated
    // commit IDs and server-specific advertised refs), but check out exactly
    // the object that this fetch returned rather than inventing `origin/...`.
    fetch_commit(git, dest, refname)
}

fn fetch_tag(git: &ShellGit, dest: &Path, tag: &str) -> Result<(), GitError> {
    let local_ref = format!("refs/tags/{tag}");
    let refspec = format!("+{local_ref}:{local_ref}");
    // The leading `+` is intentional: published tags are expected to be
    // immutable, but if an origin replaces one, a stale local tag must never
    // mask the new bytes (the content-hash gate above this backend still
    // decides whether those bytes are acceptable).
    git.run(
        &["fetch", "--no-tags", "--", "origin", &refspec],
        Some(dest),
    )?;
    let commit = format!("{local_ref}^{{commit}}");
    git.run(&["checkout", "--detach", "--force", &commit], Some(dest))
        .map(|_| ())
}

fn fetch_branch(git: &ShellGit, dest: &Path, branch: &str) -> Result<(), GitError> {
    let source_ref = format!("refs/heads/{branch}");
    let tracking_ref = format!("refs/remotes/origin/{branch}");
    let refspec = format!("+{source_ref}:{tracking_ref}");
    git.run(
        &["fetch", "--prune", "--no-tags", "--", "origin", &refspec],
        Some(dest),
    )?;
    git.run(
        &["checkout", "--force", "-B", branch, &tracking_ref],
        Some(dest),
    )?;
    let upstream = format!("--set-upstream-to=origin/{branch}");
    git.run(&["branch", &upstream, branch], Some(dest))
        .map(|_| ())
}

fn fetch_commit(git: &ShellGit, dest: &Path, commit: &str) -> Result<(), GitError> {
    git.run(&["fetch", "--no-tags", "--", "origin", commit], Some(dest))?;
    // FETCH_HEAD is the exact object transferred by this invocation. Peeling
    // through `^{commit}` refuses blobs/trees and also handles an explicitly
    // fetched annotated object without leaving HEAD attached to a branch.
    git.run(
        &["checkout", "--detach", "--force", "FETCH_HEAD^{commit}"],
        Some(dest),
    )
    .map(|_| ())
}

fn sync_submodules(git: &ShellGit, dest: &Path) -> Result<(), GitError> {
    git.run(
        &["submodule", "update", "--init", "--recursive"],
        Some(dest),
    )
    .map(|_| ())
}

fn is_exact_object_id(refname: &str) -> bool {
    matches!(refname.len(), 40 | 64) && refname.bytes().all(|byte| byte.is_ascii_hexdigit())
}
