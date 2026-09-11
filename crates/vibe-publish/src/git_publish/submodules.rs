//! Gitlink provenance captured before publication flattens working trees.

use std::path::Path;
use std::process::{Command, Output};

use serde::Serialize;

use crate::PublishError;

/// One populated, clean submodule copied into a release as ordinary files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SubmoduleProvenance {
    /// Forward-slashed path relative to the publication source root.
    pub path: String,
    /// Exact gitlink object id recorded by the containing repository.
    pub commit: String,
}

/// Capture the gitlinks whose populated working trees publication will
/// flatten. Non-Git directories simply produce an empty report. Once a
/// gitlink is visible, the checkout must be clean and exactly at its indexed
/// commit; otherwise no SHA truthfully identifies the bytes being copied.
pub fn inspect(source_dir: &Path) -> Result<Vec<SubmoduleProvenance>, PublishError> {
    let Some(index) = probe(source_dir, &["ls-files", "--stage", "-z"])? else {
        return Ok(Vec::new());
    };
    let mut submodules = Vec::new();
    for record in index.stdout.split(|byte| *byte == 0) {
        if record.is_empty() {
            continue;
        }
        let Some(tab) = record.iter().position(|byte| *byte == b'\t') else {
            continue;
        };
        let header = std::str::from_utf8(&record[..tab]).map_err(|error| {
            PublishError::Git(format!(
                "cannot decode gitlink index metadata as UTF-8: {error}"
            ))
        })?;
        let mut fields = header.split_whitespace();
        let mode = fields.next().unwrap_or_default();
        let commit = fields.next().unwrap_or_default();
        if mode != "160000" || commit.is_empty() {
            continue;
        }
        let rel = std::str::from_utf8(&record[tab + 1..]).map_err(|error| {
            PublishError::Git(format!("cannot decode gitlink path as UTF-8: {error}"))
        })?;
        let checkout = source_dir.join(rel);
        // An uninitialised submodule may leave an empty directory. It has no
        // bytes to flatten, hence no vendoring event to claim.
        if !checkout.is_dir() || !checkout.join(".git").exists() {
            continue;
        }

        let head = required(&checkout, &["rev-parse", "--verify", "HEAD^{commit}"])?;
        let actual = String::from_utf8_lossy(&head.stdout).trim().to_string();
        if !actual.eq_ignore_ascii_case(commit) {
            return Err(PublishError::Git(format!(
                "submodule `{rel}` is checked out at `{actual}`, but its gitlink pins `{commit}`; initialise/update it before publishing"
            )));
        }
        let status = required(
            &checkout,
            &["status", "--porcelain=v1", "--untracked-files=all"],
        )?;
        if !status.stdout.is_empty() {
            return Err(PublishError::Git(format!(
                "submodule `{rel}` has uncommitted or untracked bytes; clean it before publishing so `vendored at {commit}` stays truthful"
            )));
        }
        submodules.push(SubmoduleProvenance {
            path: rel.replace('\\', "/"),
            commit: commit.to_ascii_lowercase(),
        });
    }
    submodules.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(submodules)
}

fn probe(cwd: &Path, args: &[&str]) -> Result<Option<Output>, PublishError> {
    match command(cwd, args).output() {
        Ok(output) if output.status.success() => Ok(Some(output)),
        Ok(_) | Err(_) => Ok(None),
    }
}

fn required(cwd: &Path, args: &[&str]) -> Result<Output, PublishError> {
    let output = command(cwd, args)
        .output()
        .map_err(|error| PublishError::Git(format!("spawning git {}: {error}", args.join(" "))))?;
    if !output.status.success() {
        return Err(PublishError::Git(format!(
            "git {} failed while verifying submodule provenance: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(output)
}

fn command(cwd: &Path, args: &[&str]) -> Command {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(cwd)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GCM_INTERACTIVE", "Never")
        .env("LC_ALL", "C")
        .env("LANG", "C");
    command
}
