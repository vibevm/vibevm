//! Thin wrappers over the system `git`, used by the build pipeline
//! (PROP-019 §2.7). vibevm shells out to the user's installed git rather
//! than linking a git library — matching the project's existing tooling and
//! honouring the user's own credentials and host-key config (§2.13).

specmark::scope!("spec://vibevm/common/PROP-019#build");

use std::path::Path;
use std::process::Command;

use specmark::spec;
use thiserror::Error;

/// The git seam's failure surface (PROP-019 §2.7): spawning the system
/// `git` failed, or git ran and exited non-zero. One enum for the layer so
/// a build failure is navigable back to the requirement it serves.
#[derive(Debug, Error)]
#[spec(implements = "spec://vibevm/common/PROP-019#build")]
pub(crate) enum GitError {
    #[error(
        "spawning `git {args}` failed: {source} \
         (violates spec://vibevm/common/PROP-019#build; \
          fix: install git and put it on PATH — see `vibe man doctor`)"
    )]
    Spawn {
        args: String,
        #[source]
        source: std::io::Error,
    },

    #[error(
        "`git {args}` failed: {stderr} \
         (violates spec://vibevm/common/PROP-019#build; \
          fix: check the revision/remote exists and the clone is intact)"
    )]
    Failed { args: String, stderr: String },
}

/// Run `git <args>` in `dir`, returning trimmed stdout. A non-zero exit is
/// an error carrying git's stderr.
pub(crate) fn run(dir: &Path, args: &[&str]) -> Result<String, GitError> {
    let output = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .map_err(|source| GitError::Spawn {
            args: args.join(" "),
            source,
        })?;
    if !output.status.success() {
        return Err(GitError::Failed {
            args: args.join(" "),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Resolve a revision to its full commit hash.
pub(crate) fn rev_parse(dir: &Path, rev: &str) -> Result<String, GitError> {
    run(dir, &["rev-parse", rev])
}

/// The current branch name, or `None` when HEAD is detached.
pub(crate) fn current_branch(dir: &Path) -> Option<String> {
    let name = run(dir, &["rev-parse", "--abbrev-ref", "HEAD"]).ok()?;
    if name == "HEAD" || name.is_empty() {
        None
    } else {
        Some(name)
    }
}

/// Clone `url` into `dest` (a full clone, so any ref or commit resolves),
/// recursing submodules for forward-safety (PROP-019 §2.7).
pub(crate) fn clone(url: &str, dest: &Path) -> Result<(), GitError> {
    let parent = dest.parent().unwrap_or_else(|| Path::new("."));
    let dest_arg = dest.to_string_lossy();
    run(
        parent,
        &["clone", "--recurse-submodules", url, dest_arg.as_ref()],
    )?;
    Ok(())
}

/// Check out a revision (detaching HEAD at a commit).
pub(crate) fn checkout(dir: &Path, rev: &str) -> Result<(), GitError> {
    run(dir, &["checkout", "--quiet", rev])?;
    Ok(())
}

/// Fetch all refs and tags from the default remote (incremental update of a
/// managed clone — no re-clone, PROP-019 §2.16).
pub(crate) fn fetch(dir: &Path) -> Result<(), GitError> {
    run(dir, &["fetch", "--all", "--tags", "--quiet"])?;
    Ok(())
}

/// Every tag in the repo.
pub(crate) fn list_tags(dir: &Path) -> Result<Vec<String>, GitError> {
    Ok(run(dir, &["tag", "--list"])?
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect())
}

/// The full commit a revision resolves to, or `None` if it does not exist.
pub(crate) fn verify(dir: &Path, rev: &str) -> Option<String> {
    run(dir, &["rev-parse", "--verify", "--quiet", rev])
        .ok()
        .filter(|s| !s.is_empty())
}
