//! Ref-kind-aware clone refresh for [`super::ShellGit`].
//!
//! A package version names a tag, while registry and lock-driven callers may
//! name a branch or exact commit. Those are different namespaces: a tag has
//! no `origin/<name>` remote-tracking ref, and an annotated tag must be peeled
//! to its commit before checkout. This module keeps those decisions beside
//! the fetch refspecs that make them true.
//!
//! Two kinds of destination come through here and they are not the same
//! place. A **data** destination is one vibe owns and reads — a registry
//! working copy, a per-package cache clone, an embedded source — whose files
//! vibe then hashes; it is checked out byte-exact regardless of the machine's
//! Git configuration or the payload's own `.gitattributes`
//! ([`crate::git_backend::verbatim`]). A **slot** destination is the user's own
//! in-place checkout (PROP-022 §2.4), whose identity is its commit and whose
//! files the user opens in their own tools; that one is left to the user's Git
//! exactly as before. Which it is, is the caller's to say — it is never
//! guessed from the path.

use super::reported::Remote;
use super::*;
use crate::git_backend::verbatim;

pub(super) fn bootstrap(
    git: &ShellGit,
    url: &str,
    refname: &str,
    dest: &Path,
    data: bool,
) -> Result<(), GitError> {
    let dest_s = dest.to_string_lossy();
    // Do not let clone's DWIM `--branch` interpretation decide whether the
    // requested name is a tag or branch. Fetch exactly one target below.
    // `--no-tags` also prevents unrelated or stale tags from affecting that
    // decision. Submodules are initialised after the target commit is known.
    git.run_transferring(
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
        Remote::Url(url),
        dest,
    )?;
    // Before the first checkout, so no file is ever written converted. The
    // clone above is `--no-checkout` precisely because the ref is chosen
    // below; that it also leaves the working tree empty until now is what
    // makes a fresh clone byte-exact from the start.
    if data {
        install_verbatim(dest)?;
    }
    fetch_and_checkout(git, dest, refname, data)?;
    sync_submodules(git, dest, data)
}

/// Make `$GIT_DIR/info/attributes` of a vibe-owned clone the verbatim one,
/// reporting whether it had to be changed — `true` says the working copy
/// predates the rule and its files may hold a converted rendering.
fn install_verbatim(dest: &Path) -> Result<bool, GitError> {
    verbatim::install(&dest.join(".git")).map_err(|error| GitError::CommandFailed {
        cmd: "write .git/info/attributes".into(),
        status: -1,
        stderr: error.to_string(),
    })
}

/// Rewrite every tracked file from the index, applying the attributes now in
/// force. `checkout --force` already rewrites the paths it checks out, but a
/// clone is also *read* without a checkout (a registry inside its freshness
/// window), so the repair has to be available on its own.
fn renormalise(git: &ShellGit, dest: &Path) -> Result<(), GitError> {
    // Reading the commit back into the index first is what makes this a
    // repair at all. `checkout-index` writes only entries whose recorded stat
    // does not already match the file on disk — and a working copy an earlier
    // vibe checked out is precisely stat-clean, so `--force` alone skipped
    // every converted file and the copy stayed broken (measured on Windows,
    // where the files were written by that earlier vibe rather than by the
    // test). `read-tree` drops the recorded stat, and then every entry is
    // written.
    if git
        .run(&verbatim::argv(&["read-tree", "HEAD"]), Some(dest))
        .is_err()
    {
        // No commit is checked out here, so there is no tracked file to
        // rewrite — an empty place is not a copy to repair.
        return Ok(());
    }
    git.run(
        &verbatim::argv(&["checkout-index", "--all", "--force"]),
        Some(dest),
    )
    .map(|_| ())
}

/// The data-tree repair as a backend operation: install the attributes and,
/// when they were not already in force, renormalise what an earlier vibe
/// checked out under the user's conversion.
pub(super) fn normalise_owned_worktree(git: &ShellGit, dest: &Path) -> Result<(), GitError> {
    if !dest.join(".git").exists() {
        return Ok(());
    }
    if install_verbatim(dest)? {
        renormalise(git, dest)?;
    }
    Ok(())
}

/// Clone one exact upstream tree for a dependency-declared embedded source.
/// No submodule recursion: the source is opaque content and its own dependency
/// graph is outside the bridge package's authority.
pub(super) fn bootstrap_embedded(
    git: &ShellGit,
    url: &str,
    refname: &str,
    dest: &Path,
) -> Result<(), GitError> {
    let dest_s = dest.to_string_lossy();
    git.run_transferring(
        &[
            "clone",
            "--no-checkout",
            "--no-tags",
            "--",
            url,
            dest_s.as_ref(),
        ],
        None,
        Remote::Url(url),
        dest,
    )?;
    install_verbatim(dest)?;
    fetch_and_checkout(git, dest, refname, true)
}

pub(super) fn update(
    git: &ShellGit,
    dest: &Path,
    refname: &str,
    data: bool,
) -> Result<(), GitError> {
    // A clone made by an earlier vibe carries the user's conversion. Putting
    // the attributes in force before the checkout makes the refresh itself the
    // repair for every path it rewrites; `renormalise` then covers the rest,
    // including a refresh that found nothing new upstream.
    let converted = data && install_verbatim(dest)?;
    fetch_and_checkout(git, dest, refname, data)?;
    if converted {
        renormalise(git, dest)?;
    }
    sync_submodules(git, dest, data)
}

/// Fetch and check out one unambiguous namespace. Unqualified names retain
/// the registry's tag-first convention, but fall through only when git says
/// that exact remote ref is absent; authentication and transport failures are
/// never hidden by another interpretation.
fn fetch_and_checkout(
    git: &ShellGit,
    dest: &Path,
    refname: &str,
    data: bool,
) -> Result<(), GitError> {
    if is_exact_object_id(refname) {
        return fetch_commit(git, dest, refname, data);
    }
    if let Some(tag) = refname.strip_prefix("refs/tags/") {
        return fetch_tag(git, dest, tag, data);
    }
    if let Some(branch) = refname.strip_prefix("refs/heads/") {
        return fetch_branch(git, dest, branch, data);
    }

    match fetch_tag(git, dest, refname, data) {
        Ok(()) => return Ok(()),
        Err(GitError::RefNotFound { .. }) => {}
        Err(error) => return Err(error),
    }
    match fetch_branch(git, dest, refname, data) {
        Ok(()) => return Ok(()),
        Err(GitError::RefNotFound { .. }) => {}
        Err(error) => return Err(error),
    }
    // Preserve the historical arbitrary-ref fallback (including abbreviated
    // commit IDs and server-specific advertised refs), but check out exactly
    // the object that this fetch returned rather than inventing `origin/...`.
    fetch_commit(git, dest, refname, data)
}

/// The arguments of a command that writes the working tree: byte-exact for a
/// data destination, the user's own Git for a slot.
fn writing<'a>(args: &[&'a str], data: bool) -> Vec<&'a str> {
    if data {
        verbatim::argv(args)
    } else {
        args.to_vec()
    }
}

fn fetch_tag(git: &ShellGit, dest: &Path, tag: &str, data: bool) -> Result<(), GitError> {
    let local_ref = format!("refs/tags/{tag}");
    let refspec = format!("+{local_ref}:{local_ref}");
    // The leading `+` is intentional: published tags are expected to be
    // immutable, but if an origin replaces one, a stale local tag must never
    // mask the new bytes (the content-hash gate above this backend still
    // decides whether those bytes are acceptable).
    git.run_transferring(
        &["fetch", "--no-tags", "--", "origin", &refspec],
        Some(dest),
        Remote::Origin,
        dest,
    )?;
    let commit = format!("{local_ref}^{{commit}}");
    git.run(
        &writing(&["checkout", "--detach", "--force", &commit], data),
        Some(dest),
    )
    .map(|_| ())
}

fn fetch_branch(git: &ShellGit, dest: &Path, branch: &str, data: bool) -> Result<(), GitError> {
    let source_ref = format!("refs/heads/{branch}");
    let tracking_ref = format!("refs/remotes/origin/{branch}");
    let refspec = format!("+{source_ref}:{tracking_ref}");
    git.run_transferring(
        &["fetch", "--prune", "--no-tags", "--", "origin", &refspec],
        Some(dest),
        Remote::Origin,
        dest,
    )?;
    git.run(
        &writing(&["checkout", "--force", "-B", branch, &tracking_ref], data),
        Some(dest),
    )?;
    let upstream = format!("--set-upstream-to=origin/{branch}");
    git.run(&["branch", &upstream, branch], Some(dest))
        .map(|_| ())
}

fn fetch_commit(git: &ShellGit, dest: &Path, commit: &str, data: bool) -> Result<(), GitError> {
    git.run_transferring(
        &["fetch", "--no-tags", "--", "origin", commit],
        Some(dest),
        Remote::Origin,
        dest,
    )?;
    // FETCH_HEAD is the exact object transferred by this invocation. Peeling
    // through `^{commit}` refuses blobs/trees and also handles an explicitly
    // fetched annotated object without leaving HEAD attached to a branch.
    git.run(
        &writing(
            &["checkout", "--detach", "--force", "FETCH_HEAD^{commit}"],
            data,
        ),
        Some(dest),
    )
    .map(|_| ())
}

fn sync_submodules(git: &ShellGit, dest: &Path, data: bool) -> Result<(), GitError> {
    // The `-c` flags reach the submodule checkouts too: git exports its
    // command-line configuration to its children through
    // `GIT_CONFIG_PARAMETERS`. A `.gitattributes` *inside* a submodule is the
    // one rendering this does not reach (B-289).
    git.run(
        &writing(&["submodule", "update", "--init", "--recursive"], data),
        Some(dest),
    )
    .map(|_| ())
}

fn is_exact_object_id(refname: &str) -> bool {
    matches!(refname.len(), 40 | 64) && refname.bytes().all(|byte| byte.is_ascii_hexdigit())
}
