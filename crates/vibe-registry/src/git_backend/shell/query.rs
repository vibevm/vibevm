//! Read-only questions about a local checkout — the verbs that only ask,
//! never clone, fetch or reset. Split from `shell.rs` per the file-length
//! budget, and along a real seam: everything here answers "what does this
//! working tree look like right now", using the same spawn hygiene as the
//! rest of the backend.
//!
//! Both verbs share a contract worth stating once. `Ok(None)` means git
//! answered and the answer is "nothing" — no history for that path, no
//! branch to name. `Err` means git could not be asked at all. Callers here
//! ask in order to *decide something optional* — whether a verdict is
//! stale, which cache bucket to use — so they treat both as "unknown" and
//! step aside rather than fail.

use std::path::Path;

use super::ShellGit;
use crate::git_backend::GitError;

impl ShellGit {
    /// Timestamp of the last commit touching `pathspec`, RFC-3339
    /// (`git log -1 --format=%cI -- <pathspec>`), resolved with `repo` as
    /// the working directory — so `pathspec` is read relative to it.
    ///
    /// `Ok(None)` is "that path has no history here"; an `Err` is "git
    /// could not answer at all" — no binary, no repository, an unborn
    /// branch. A caller asking only *did this code move* treats both as
    /// "unknown, skip", never as a failure.
    pub fn last_commit_iso(&self, repo: &Path, pathspec: &str) -> Result<Option<String>, GitError> {
        self.preflight()?;
        let output = self.run(&["log", "-1", "--format=%cI", "--", pathspec], Some(repo))?;
        let stamp = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(if stamp.is_empty() { None } else { Some(stamp) })
    }

    /// The branch checked out in `repo`
    /// (`git symbolic-ref --short -q HEAD`).
    ///
    /// `Ok(None)` is "there is no branch to name": a detached HEAD, which
    /// `-q` reports by exiting non-zero with empty output, or a directory
    /// that is not a checkout at all. An `Err` is only "git could not be
    /// run". A caller keying a cache bucket by branch treats every one of
    /// those the same way — an unnamed bucket — so none of them fails a
    /// run.
    ///
    /// Deliberately on `run_raw`: `symbolic-ref`'s non-zero exit is an
    /// *answer* here, not an error worth classifying.
    pub fn branch(&self, repo: &Path) -> Result<Option<String>, GitError> {
        self.preflight()?;
        let output = self.run_raw(&["symbolic-ref", "--short", "-q", "HEAD"], Some(repo))?;
        if !output.status.success() {
            return Ok(None);
        }
        let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok((!name.is_empty()).then_some(name))
    }
}
