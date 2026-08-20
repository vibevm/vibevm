//! `--auto-commit-push` — the index publishes itself. After every
//! successful mutation the server commits the data directory and
//! pushes it to the working copy's configured upstream, so the only
//! manual hole in the publish scenario (carrying the result to the
//! host) closes. This module is the whole capability: the startup
//! preflight (refuse to ship secrets, refuse without a working copy)
//! and the commit-and-push itself.
//!
//! Design, after PROP-005 fact `DATA-DIR-IS-WORKTREE`: the data
//! directory is itself the index's git working copy. `state/` is
//! gitignored — bearer tokens live at `state/admin.tokens` — and the
//! rest is tracked and published. The operator wires the remote and
//! the branch with plain git; `vibe-index` only runs
//! `git add -A && git commit && git push` from inside the data dir
//! (no refspec, no hard-coded remote — Р1). We deliberately do not
//! depend on `vibe-publish` for this: its `commit_and_push` pulls the
//! resolver into an operator tool that does not need it, treats an
//! empty diff as an error, and nails the target to `origin main`.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");

use std::fmt;
use std::path::Path;
use std::process::Command;

use crate::error::Error;
use crate::scanner::git_cli::{binary, is_git_dir};

/// Startup preflight for `--auto-commit-push`. Refuses to boot unless
/// the data directory is a git working copy (Р3) and `state/` — which
/// holds bearer tokens — is gitignored (Р2). One check, at startup: the
/// operator learns the configuration is unsafe *before* the first
/// mutation ships a token, not after.
pub fn preflight(data_dir: &Path) -> crate::error::Result<()> {
    // Р3 — there must be a working copy to commit into.
    if !is_git_dir(data_dir) {
        return Err(Error::InvalidInput(format!(
            "`--auto-commit-push` is set but the data directory `{}` is not a git working copy \
             (no `.git`): the index publishes itself by committing the data directory, so there is \
             nothing to commit. `git init` the data directory and configure a remote before \
             enabling the flag",
            data_dir.display()
        )));
    }
    // Р2 — `git add -A` must not stage the bearer tokens.
    match ignored(data_dir, "state/admin.tokens") {
        Ok(true) => Ok(()),
        Ok(false) => Err(Error::InvalidInput(
            "`--auto-commit-push` is set but `state/` is not gitignored. `git add -A` would stage \
             the bearer tokens at `state/admin.tokens` and push them to the remote — a credential \
             leak. Add `/state/` to the data directory's `.gitignore` (`vibe-index init` writes \
             one) before enabling the flag"
                .to_string(),
        )),
        Err(reason) => Err(Error::InvalidInput(format!(
            "`--auto-commit-push` is set but could not confirm `state/` is gitignored \
             (git check-ignore failed: {reason}). Ensure git is on PATH and `/state/` is in the \
             data directory's `.gitignore` before enabling the flag"
        ))),
    }
}

/// Commit the data directory's changes and push to its upstream. Called
/// from a mutating handler after the index lock is released and the
/// publish lock is held, on a blocking thread. An empty diff is success
/// ([`PublishOutcome::NothingToCommit`] — Р6): a concurrent mutation may
/// already have shipped this change, so the next publish finds nothing
/// to commit, and that is the normal course of events.
///
/// A push failure (no upstream, network, rejected) is returned as
/// [`PublishError`]; the caller logs it at `warn` and counts it but does
/// **not** fail the HTTP request (Р4) — the mutation already succeeded
/// and stays on disk, and a later successful push carries the queued
/// commit forward.
pub fn commit_and_push(data_dir: &Path, message: &str) -> Result<PublishOutcome, PublishError> {
    // Stage every change in the working copy. `state/` is gitignored, so
    // tokens stay out — preflight guarantees it before we ever get here.
    run_git(data_dir, &["add", "-A"])?;
    // Р6 — nothing staged vs HEAD ⇒ an earlier publish already shipped it.
    if nothing_staged(data_dir)? {
        return Ok(PublishOutcome::NothingToCommit);
    }
    run_git(data_dir, &["commit", "--quiet", "-m", message])?;
    // Р1 — push the working copy's branch to its configured upstream.
    run_git(data_dir, &["push", "--quiet"])?;
    Ok(PublishOutcome::Published)
}

/// Outcome of [`commit_and_push`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublishOutcome {
    /// A commit was made and pushed.
    Published,
    /// The working tree had no staged changes — already shipped by a
    /// concurrent publish. Success, not an error (Р6).
    NothingToCommit,
}

/// A publish attempt failed. The mutation that triggered it already
/// succeeded and stays on disk; the caller logs this at `warn` and
/// counts it, but does **not** fail the HTTP request (Р4).
#[derive(Debug)]
pub enum PublishError {
    /// A `git` step exited non-zero; `stderr` carries git's message so
    /// the operator can see why (e.g. a rejected push, no upstream).
    Git { step: &'static str, stderr: String },
}

impl fmt::Display for PublishError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PublishError::Git { step, stderr } => {
                write!(f, "git {step} failed: {}", stderr.trim())
            }
        }
    }
}

impl std::error::Error for PublishError {}

/// Run a git step in `data_dir`; map any non-zero exit to a
/// [`PublishError`] carrying git's stderr. Uses `current_dir` rather
/// than `git -C` so a non-UTF-8 data path is not an issue.
fn run_git(data_dir: &Path, args: &[&str]) -> Result<(), PublishError> {
    let out = Command::new(binary())
        .current_dir(data_dir)
        .args(args)
        .output()
        .map_err(|e| PublishError::Git {
            step: step_of(args),
            stderr: format!("could not invoke git: {e}"),
        })?;
    if out.status.success() {
        return Ok(());
    }
    Err(PublishError::Git {
        step: step_of(args),
        stderr: String::from_utf8_lossy(&out.stderr).trim().to_string(),
    })
}

/// `git diff --cached --quiet` semantics: exit 0 ⇒ the index matches
/// HEAD (nothing staged to commit), exit 1 ⇒ staged changes exist.
fn nothing_staged(data_dir: &Path) -> Result<bool, PublishError> {
    let out = Command::new(binary())
        .current_dir(data_dir)
        .args(["diff", "--cached", "--quiet"])
        .output()
        .map_err(|e| PublishError::Git {
            step: "diff",
            stderr: format!("could not invoke git: {e}"),
        })?;
    match out.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => Err(PublishError::Git {
            step: "diff",
            stderr: String::from_utf8_lossy(&out.stderr).trim().to_string(),
        }),
    }
}

/// `git check-ignore --quiet <path>` semantics: exit 0 ⇒ `path` is
/// ignored, exit 1 ⇒ not ignored. Any other outcome is returned as a
/// `reason` string the caller folds into its error message.
fn ignored(data_dir: &Path, path: &str) -> Result<bool, String> {
    let out = Command::new(binary())
        .current_dir(data_dir)
        .args(["check-ignore", "--quiet", path])
        .output()
        .map_err(|e| format!("could not invoke git: {e}"))?;
    match out.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        Some(code) => Err(format!(
            "exit code {code}: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )),
        None => Err("terminated by signal".to_string()),
    }
}

/// The human label for a git subcommand, for error messages.
fn step_of(args: &[&str]) -> &'static str {
    match args.first().copied() {
        Some("add") => "add",
        Some("commit") => "commit",
        Some("push") => "push",
        Some("diff") => "diff",
        _ => "step",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn git_available() -> bool {
        Command::new("git").arg("--version").output().is_ok()
    }

    fn init_repo(path: &Path) {
        for args in [
            &["init", "--quiet", "-b", "main"][..],
            &["config", "user.email", "t@t.invalid"][..],
            &["config", "user.name", "T"][..],
        ] {
            let s = Command::new("git")
                .current_dir(path)
                .args(args)
                .status()
                .unwrap();
            assert!(s.success(), "git {:?} in {}", args, path.display());
        }
    }

    fn commit_all(path: &Path, msg: &str) {
        for args in [&["add", "-A"][..], &["commit", "--quiet", "-m", msg][..]] {
            let s = Command::new("git")
                .current_dir(path)
                .args(args)
                .status()
                .unwrap();
            assert!(s.success());
        }
    }

    /// Acceptance point 2 / Р3: a non-git data directory refuses to
    /// start, and the message says what to do. No git binary is needed
    /// — `is_git_dir` is a directory test.
    #[test]
    fn preflight_rejects_non_git_data_dir() {
        let dir = tempdir().unwrap();
        let msg = preflight(dir.path()).unwrap_err().to_string();
        assert!(msg.contains("not a git working copy"), "got: {msg}");
        assert!(msg.contains(".git"), "got: {msg}");
    }

    /// Acceptance point 3 / Р2: a git data directory whose `state/` is
    /// not gitignored refuses to start, naming the token leak.
    #[test]
    fn preflight_rejects_unignored_state_dir() {
        if !git_available() {
            return;
        }
        let dir = tempdir().unwrap();
        init_repo(dir.path());
        // No .gitignore ⇒ state/ is not ignored.
        let msg = preflight(dir.path()).unwrap_err().to_string();
        assert!(msg.contains("state/"), "got: {msg}");
        assert!(msg.contains("token"), "got: {msg}");
    }

    /// Positive control: with `/state/` ignored, preflight passes.
    #[test]
    fn preflight_accepts_gitignored_state_dir() {
        if !git_available() {
            return;
        }
        let dir = tempdir().unwrap();
        init_repo(dir.path());
        std::fs::write(dir.path().join(".gitignore"), "/state/\n").unwrap();
        preflight(dir.path()).expect("preflight passes when state/ is ignored");
    }

    /// Acceptance point 6 / Р6: an empty diff is success
    /// (`NothingToCommit`), not an error. No remote is configured, so a
    /// mistaken commit-then-push would surface here — proving the empty
    /// diff short-circuits before any push.
    #[test]
    fn empty_diff_is_reported_as_success() {
        if !git_available() {
            return;
        }
        let dir = tempdir().unwrap();
        init_repo(dir.path());
        std::fs::write(dir.path().join(".gitignore"), "/state/\n").unwrap();
        std::fs::write(dir.path().join("repomd.json"), "{}\n").unwrap();
        commit_all(dir.path(), "initial");
        // Nothing changed since the initial commit ⇒ empty diff.
        let outcome = commit_and_push(dir.path(), "index: noop").unwrap();
        assert_eq!(outcome, PublishOutcome::NothingToCommit);
    }

    /// B-072 — the fix's whole point, at the publish seam: an
    /// identical re-upsert writes ZERO bytes (`write_to` compares the
    /// projection against the disk before writing), so the publisher
    /// finds a clean worktree and reports `NothingToCommit` (Р6).
    /// Before the fix the fresh `generated_at` alone dirtied
    /// `repomd.json`, and publishing a no-op mutation became a
    /// timestamp-only commit.
    #[test]
    fn identical_reupsert_is_nothing_to_commit() {
        if !git_available() {
            return;
        }
        use crate::index::memory::{Index, WriteCtx};
        use crate::types::{NamingConvention, PackageKind, VersionEntry};

        let dir = tempdir().unwrap();
        init_repo(dir.path());
        std::fs::write(dir.path().join(".gitignore"), "/state/\n").unwrap();

        // A one-entry catalog through the real writer path, committed.
        let at = chrono::Utc::now();
        let mut idx = Index::new(
            "vibespecs",
            "https://example.invalid",
            NamingConvention::Fqdn,
            at,
        );
        let make_entry = |at| {
            VersionEntry::minimal(
                PackageKind::Flow,
                "org.vibevm".parse().unwrap(),
                "wal",
                "0.1.0".parse().unwrap(),
                at,
            )
        };
        assert!(idx.upsert(make_entry(at)), "the first insert changes state");
        idx.write_to(dir.path(), &WriteCtx { at }).unwrap();
        commit_all(dir.path(), "initial catalog");

        // The identical repeat, an hour later by the wall clock: the
        // upsert no-ops, the write compares and writes nothing.
        let later = at + chrono::Duration::hours(1);
        assert!(
            !idx.upsert(make_entry(at)),
            "the identical repeat changes no state"
        );
        idx.write_to(dir.path(), &WriteCtx { at: later }).unwrap();

        let outcome = commit_and_push(dir.path(), "index: noop").unwrap();
        assert_eq!(outcome, PublishOutcome::NothingToCommit);
    }
}
