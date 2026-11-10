//! The one place PROP-076 `#SYS-ARTIFACTS-NOT-TRACKED` is enforced rather
//! than documented: git tracking a generated prompt artifact is an error.
//!
//! Out of line from [`super`] along its own seam. The parent cell validates
//! what the boot directory *holds*; this asks git what it *tracks*, which is
//! a different question with a different source of truth and the only part
//! of the cell that spawns a process.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-ARTIFACTS-NOT-TRACKED");

use std::path::Path;

use crate::{CheckId, CheckReport};

/// Git tracking a generated prompt artifact is an **error**
/// (PROP-076 `#SYS-ARTIFACTS-NOT-TRACKED`).
///
/// The prefix, its sidecar and the agent lane are a deterministic function
/// of committed inputs, so a tracked copy carries no fact its inputs do not.
/// What it does carry is roughly 280 KB of duplicated text in a file no
/// human can usefully merge, and the one state the rule exists to prevent: a
/// stale copy committed and then believed. So this is reported as a failure
/// and not as a note, with the path and the remedy.
///
/// **The boot lane keeps the opposite rule for the opposite reason.**
/// `STATIC.*` and `INDEX.md` are read by a session whose only tool is a file
/// reader, so they must survive a fresh clone; every consumer of a prompt
/// artifact is a program already invoking `vibe`.
///
/// Git is asked, not guessed: there is no second reader of the index here.
/// When `git` is absent, or the directory is no repository, the check is
/// silent — it cannot know, and inventing an answer about a developer's
/// working tree is worse than saying nothing.
pub(super) fn report_tracked_prompt_artifacts(project_root: &Path, report: &mut CheckReport) {
    for relative in vibe_core::layout::current_boot_prompt_artifacts() {
        let shown = relative.to_string_lossy().replace('\\', "/");
        if !git_tracks(project_root, &shown) {
            continue;
        }
        report.err(
            CheckId::BootDirectory,
            Some(relative),
            None,
            format!(
                "git tracks `{shown}`, a generated system-prompt artifact. It is a \
                 deterministic function of committed inputs, so a tracked copy duplicates \
                 text the repository already holds, cannot usefully be merged, and lets a \
                 stale copy be committed and then believed: run `git rm --cached {shown}` \
                 and let the ignore file in the boot directory cover it. `STATIC.*` and \
                 `INDEX.md` are tracked for the opposite reason — a session reads them with \
                 nothing but a file reader."
            ),
        );
    }
}

/// Does git track `relative` under `project_root`? `false` when git cannot
/// answer at all.
fn git_tracks(project_root: &Path, relative: &str) -> bool {
    std::process::Command::new("git")
        .arg("-C")
        .arg(project_root)
        .args(["ls-files", "--error-unmatch", "--", relative])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}
#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;
    use vibe_core::layout;

    use crate::test_support::{opts, write_minimal_project};
    use crate::{CheckId, Severity, check_project};

    /// A tracked generated prompt artifact is an error; the tracked boot
    /// lane is not, for the opposite reason. A project that is no git
    /// repository is silent, because the check cannot know.
    #[test]
    fn git_tracking_a_generated_prompt_artifact_is_an_error() {
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        let boot = project.path().join(layout::current_boot_dir());
        for name in [
            layout::SYSTEM_MD,
            layout::SYSTEM_META_JSON,
            layout::AGENT_LANE_MD,
            layout::INDEX_MD,
        ] {
            fs::write(boot.join(name), "generated\n").unwrap();
        }

        // Not a repository: nothing is claimed either way.
        let before = check_project(project.path(), &opts());
        assert!(
            !before
                .findings
                .iter()
                .any(|f| f.message.contains("a generated system-prompt artifact")),
            "{:?}",
            before.findings
        );

        if !git_init(project.path()) {
            eprintln!("git is unavailable; the tracked half of this test is skipped");
            return;
        }
        let report = check_project(project.path(), &opts());
        let tracked = report
            .findings
            .iter()
            .filter(|f| f.message.contains("a generated system-prompt artifact"))
            .collect::<Vec<_>>();
        assert_eq!(tracked.len(), 3, "got: {tracked:?}");
        for finding in &tracked {
            assert_eq!(finding.severity, Severity::Error);
            assert_eq!(finding.check, CheckId::BootDirectory);
            assert!(
                finding.message.contains("git rm --cached"),
                "{}",
                finding.message
            );
        }
        // INDEX.md is tracked on purpose and is never the path reported.
        assert!(
            !tracked
                .iter()
                .any(|f| f.path.as_deref() == Some(layout::current_boot_index().as_path())),
            "the boot lane keeps the opposite rule: {tracked:?}"
        );
    }

    /// Initialise a repository and stage everything in it. `false` when git
    /// is not available, which leaves the caller to skip rather than fail.
    fn git_init(root: &std::path::Path) -> bool {
        let run = |args: &[&str]| {
            std::process::Command::new("git")
                .arg("-C")
                .arg(root)
                .args(args)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .is_ok_and(|status| status.success())
        };
        run(&["init", "-q"]) && run(&["add", "-A"])
    }
}
