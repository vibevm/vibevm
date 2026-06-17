//! Thin wrappers over the system `git`, used by the build pipeline
//! (PROP-019 §2.7). vibevm shells out to the user's installed git rather
//! than linking a git library — matching the project's existing tooling and
//! honouring the user's own credentials and host-key config (§2.13).

specmark::scope!("spec://vibevm/common/PROP-019#build");

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

/// Run `git <args>` in `dir`, returning trimmed stdout. A non-zero exit is
/// an error carrying git's stderr.
pub fn run(dir: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .with_context(|| format!("spawning `git {}`", args.join(" ")))?;
    if !output.status.success() {
        bail!(
            "`git {}` failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Resolve a revision to its full commit hash.
pub fn rev_parse(dir: &Path, rev: &str) -> Result<String> {
    run(dir, &["rev-parse", rev])
}

/// The current branch name, or `None` when HEAD is detached.
pub fn current_branch(dir: &Path) -> Option<String> {
    let name = run(dir, &["rev-parse", "--abbrev-ref", "HEAD"]).ok()?;
    if name == "HEAD" || name.is_empty() {
        None
    } else {
        Some(name)
    }
}

/// Clone `url` into `dest` (a full clone, so any ref or commit resolves),
/// recursing submodules for forward-safety (PROP-019 §2.7).
pub fn clone(url: &str, dest: &Path) -> Result<()> {
    let parent = dest.parent().unwrap_or_else(|| Path::new("."));
    let dest_arg = dest.to_string_lossy();
    run(
        parent,
        &["clone", "--recurse-submodules", url, dest_arg.as_ref()],
    )?;
    Ok(())
}

/// Check out a revision (detaching HEAD at a commit).
pub fn checkout(dir: &Path, rev: &str) -> Result<()> {
    run(dir, &["checkout", "--quiet", rev])?;
    Ok(())
}

/// Every tag in the repo.
pub fn list_tags(dir: &Path) -> Result<Vec<String>> {
    Ok(run(dir, &["tag", "--list"])?
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect())
}

/// The full commit a revision resolves to, or `None` if it does not exist.
pub fn verify(dir: &Path, rev: &str) -> Option<String> {
    run(dir, &["rev-parse", "--verify", "--quiet", rev])
        .ok()
        .filter(|s| !s.is_empty())
}
