//! How many times a page has been edited since somebody read it aloud —
//! the rule of five (`MAINTENANCE.md` §6, PROP-057
//! `##OBS-MAINTENANCE-TOOLS`).
//!
//! Five small edits since the last reading put a page back in the reading
//! rota, because accumulated patches break the ladder of a page
//! invisibly to each author of a patch. Counting them needs the one thing
//! this project otherwise refuses to depend on: history. The norm settles
//! that for this single row — «by the number of commits on the page since
//! the reading date, when the history is available, and skipped silently
//! otherwise» — so the answer here is an `Option` and never an error.
//!
//! It is a read-only question asked of `git log`, and it is the only
//! place in this library that asks one. It is not a shell: the argument
//! list is fixed, the path is passed after `--` so a file named like an
//! option cannot become one, and nothing of what comes back is executed.
//! A checkout without git, a tree published without its history and a
//! path git does not know all answer the same way — silence.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#OBS-MAINTENANCE-TOOLS");

use std::path::Path;
use std::process::{Command, Stdio};

use chrono::NaiveDate;

/// Commits touching `path` since `since`, or `None` when the history
/// cannot be read.
pub fn edits_since(repo_root: &Path, path: &Path, since: NaiveDate) -> Option<usize> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .arg("log")
        .arg("--format=%H")
        .arg(format!("--since={since}"))
        // Everything after `--` is a path, so a file whose name begins
        // with a dash stays a file.
        .arg("--")
        .arg(path)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|line| !line.trim().is_empty())
            .count(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_directory_that_is_not_a_checkout_answers_with_silence() {
        let tmp = tempfile::tempdir().expect("a temporary directory");
        assert_eq!(
            edits_since(
                tmp.path(),
                &tmp.path().join("page.xml"),
                NaiveDate::from_ymd_opt(2026, 1, 1).expect("a date"),
            ),
            None,
            "no history is «not asked», and the norm says that is skipped in silence"
        );
    }
}
