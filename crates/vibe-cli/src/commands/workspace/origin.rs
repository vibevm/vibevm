//! `[origin]` provenance for staged publish nodes — best-effort git
//! probes of the workspace root (PROP-007 §2.7).

specmark::scope!("spec://vibevm/modules/vibe-workspace/PROP-007#surface");

use std::path::Path;
use std::process::Command;

use vibe_core::manifest::Manifest;
use vibe_workspace::Workspace;
use vibe_workspace::publish::OriginInfo;

/// Build the `[origin]` provenance for every staged node.
///
/// `upstream` is the workspace root's `origin` remote URL when the root is a
/// git repository carrying that remote; otherwise it falls back to the root
/// manifest's project/package `name` (a best-effort identity — an external
/// reader at least learns which monorepo this came from by name). `commit`
/// is the root repo's `HEAD` when it is a git repository, else `None`.
pub(super) fn build_origin_info(workspace: &Workspace) -> OriginInfo {
    let upstream = git_remote_origin_url(&workspace.root)
        .unwrap_or_else(|| root_identity_name(&workspace.root_manifest));
    let commit = git_head_commit(&workspace.root);
    OriginInfo {
        upstream,
        commit,
        generated_by: format!("vibe {}", env!("CARGO_PKG_VERSION")),
        generated_at: vibe_core::timestamp::now_utc(),
    }
}

/// The best-effort identity name of the workspace root — its project or
/// package `name`, or a literal `unknown` if the root carries neither (a
/// virtual `[workspace]`-only coordinator).
pub(super) fn root_identity_name(root: &Manifest) -> String {
    if let Some(p) = &root.project {
        return p.name.clone();
    }
    if let Some(p) = &root.package {
        return p.name.clone();
    }
    "unknown".to_string()
}

/// Run `git remote get-url origin` in `dir`. Returns `None` when `dir` is
/// not a git repo, has no `origin` remote, or `git` is unavailable. The URL
/// is used only as the `[origin].upstream` marker value — it is a public
/// remote URL, not a credentialed push URL, and never carries a token.
fn git_remote_origin_url(dir: &Path) -> Option<String> {
    let out = git_in(dir, &["remote", "get-url", "origin"])?;
    let url = out.trim();
    if url.is_empty() {
        return None;
    }
    Some(url.to_string())
}

/// Run `git rev-parse HEAD` in `dir`. Returns `None` when `dir` is not a
/// git repo or `git` is unavailable.
fn git_head_commit(dir: &Path) -> Option<String> {
    let out = git_in(dir, &["rev-parse", "HEAD"])?;
    let sha = out.trim();
    if sha.is_empty() {
        return None;
    }
    Some(sha.to_string())
}

/// Run `git <args>` in `dir`, returning trimmed stdout on success or `None`
/// on any failure (non-zero exit, `git` missing, I/O error). A best-effort
/// probe — a missing git repo is not an error for `vibe workspace publish`,
/// it just means the `[origin]` marker falls back to the root name.
fn git_in(dir: &Path, args: &[&str]) -> Option<String> {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(dir).args(args);
    cmd.env("LC_ALL", "C").env("LANG", "C");
    cmd.env("GIT_TERMINAL_PROMPT", "0");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let output = cmd.output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}
