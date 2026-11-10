//! Managed blocks a harness's **downward** discovery would concatenate into
//! a session rooted here (`PROP-076#SYS-NO-RECURSION`).
//!
//! Out of line from [`super`] along its own seam: the parent cell validates
//! the three instruction files at the project root, while this one answers a
//! different question — *which other files would a harness find* — and it is
//! the only part of the cell that needs an opinion about ignore rules.
//!
//! **The walk must see what a harness sees, and no more.** Gemini CLI and
//! Qwen Code honour `.gitignore` while discovering context files, so a
//! directory git does not show a developer is a directory no harness reads —
//! and reporting it is a false finding. Measured on this repository before
//! the rule was applied: **51** warnings, nearly all from agent working
//! copies under `.claude/worktrees/`, which are linked git worktrees of this
//! same repository and are not part of anyone's session.
//!
//! **So git is asked rather than imitated.** `git ls-files --cached --others
//! --exclude-standard` is exactly "the files git would show you":
//! `.gitignore`, `.git/info/exclude` and the user's global excludes, applied
//! by the one authority on them, with no second ignore engine to disagree
//! with it. It also never descends into a nested repository or a linked
//! worktree, so those drop out for free rather than needing a rule. Without
//! git — no binary, or a project that is no repository — the fallback is the
//! plain walk this check always did, plus the one rule that closes the
//! measured bug by itself: a directory holding a `.git` entry is a
//! repository or a linked worktree of its own and is never entered.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-NO-RECURSION");

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::{CheckId, CheckReport};

use super::{INSTRUCTION_FILES, block_end};

/// Report the discoverable blocks below `project_root`.
///
/// Two findings, because two different things are wrong. A nested
/// instruction file below the project root carries a second project's boot
/// rules: the router scopes itself by node id, so this is a cost in tokens
/// rather than a contradiction, and it is a warning. A copy under the live
/// dependency root whose block is **not** neutralised is a regeneration that
/// has not happened, and names the command that fixes it.
pub(super) fn report_discoverable_blocks(project_root: &Path, report: &mut CheckReport) {
    let vibedeps = vibe_core::layout::current_vibedeps_root();
    for relative in discoverable_instruction_files(project_root) {
        let path = project_root.join(&relative);
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        if block_end(&content).is_none() {
            continue;
        }
        let shown = relative.display().to_string().replace('\\', "/");
        if relative.starts_with(&vibedeps) {
            if !content.contains(vibe_workspace::boot_artifacts::NEUTRALISED_BODY) {
                report.warn(
                    CheckId::RedirectBlock,
                    Some(relative),
                    None,
                    format!(
                        "`{shown}` is a materialised dependency copy whose <vibevm> block is not \
                         neutralised, so a harness that discovers instruction files downward \
                         would read another package's boot rules as this project's: run \
                         `vibe install` to rewrite the copy"
                    ),
                );
            }
            continue;
        }
        report.warn(
            CheckId::RedirectBlock,
            Some(relative),
            None,
            format!(
                "`{shown}` carries a <vibevm> block below the project root, and a harness that \
                 discovers instruction files downward concatenates it: the block scopes itself \
                 to its own project, so this costs context rather than contradicting this \
                 project's boot"
            ),
        );
    }
}

/// Every instruction file strictly below `project_root` that a
/// gitignore-honouring downward discovery would reach, sorted, each
/// project-root-relative.
fn discoverable_instruction_files(project_root: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = match git_listed(project_root) {
        Some(listed) => listed,
        None => walked(project_root),
    }
    .into_iter()
    // Applied to **both** answers, because the two reach it differently and
    // the rule is the same rule. Git usually drops a nested repository on
    // its own, but it only recognises one whose gitdir actually resolves —
    // a worktree whose checkout has been moved or whose administrative
    // directory is gone is still not this project's session, and that is
    // exactly the state an abandoned agent working copy is left in.
    .filter(|relative| !inside_nested_repository(project_root, relative))
    .collect();
    found.sort();
    found.dedup();
    found
}

/// Does any directory strictly between `project_root` and `relative` hold a
/// `.git` entry? Then that directory is a repository or a linked worktree of
/// its own, and its instruction files are not this project's.
fn inside_nested_repository(project_root: &Path, relative: &Path) -> bool {
    let mut directory = project_root.to_path_buf();
    let mut components = relative.components().peekable();
    while let Some(component) = components.next() {
        if components.peek().is_none() {
            return false;
        }
        directory.push(component);
        if fs::symlink_metadata(directory.join(".git")).is_ok() {
            return true;
        }
    }
    false
}

/// Ask git. `None` when git cannot answer — no binary, or no repository —
/// which is the only case the fallback walk exists for.
fn git_listed(project_root: &Path) -> Option<Vec<PathBuf>> {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(project_root)
        // `--cached` keeps tracked files; `--others --exclude-standard`
        // adds the untracked ones git would show, and nothing it would not.
        .args([
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
            "--",
        ])
        .stderr(Stdio::null());
    for name in INSTRUCTION_FILES {
        // Without `:(glob)` magic a `*` in a pathspec matches `/` too, so
        // this is "at any depth" — and the leading `*/` is what makes it
        // *strictly below* the root, which is the only part that matters.
        command.arg(format!("*/{name}"));
    }
    let output = command.output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(
        String::from_utf8_lossy(&output.stdout)
            .split('\0')
            .filter(|line| !line.is_empty())
            .map(PathBuf::from)
            .collect(),
    )
}

/// The fallback: the plain walk, never entering a directory that is a
/// repository or a linked worktree of its own.
fn walked(project_root: &Path) -> Vec<PathBuf> {
    walkdir::WalkDir::new(project_root)
        .max_depth(12)
        .into_iter()
        .filter_entry(|entry| !skipped(entry.path()))
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| {
            let relative = entry.path().strip_prefix(project_root).ok()?.to_path_buf();
            let named = relative
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| INSTRUCTION_FILES.contains(&name));
            (named && relative.components().count() >= 2).then_some(relative)
        })
        .collect()
}

/// Directories no harness discovery and no check needs to walk.
///
/// The `.git` **entry** test is the load-bearing one: a linked worktree
/// carries a `.git` *file* and a clone carries a `.git` *directory*, so
/// either way the directory below is a separate repository whose instruction
/// files are not this project's.
fn skipped(path: &Path) -> bool {
    let named = path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| matches!(name, ".git" | "target" | "node_modules" | ".vibe"));
    named || (path.is_dir() && fs::symlink_metadata(path.join(".git")).is_ok())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use tempfile::tempdir;

    use crate::test_support::{opts, write_minimal_project};
    use crate::{CheckId, Severity, check_project};

    const BLOCK: &str = "# Mine\n\n<vibevm>\nrouter\n</vibevm>\n";

    /// Neither an ignored directory nor a nested worktree is anyone's
    /// session, so neither is reported — while an ordinary nested project
    /// still is. The git half and the fallback half are asserted over the
    /// same fixture, because the two paths must agree.
    #[test]
    fn an_ignored_directory_and_a_nested_worktree_are_not_discoverable() {
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        fs::write(project.path().join("CLAUDE.md"), BLOCK).unwrap();
        fs::write(project.path().join(".gitignore"), "/ignored/\n").unwrap();

        for directory in ["ignored/deep", "worktrees/agent", "research/demo"] {
            fs::create_dir_all(project.path().join(directory)).unwrap();
            fs::write(project.path().join(directory).join("AGENTS.md"), BLOCK).unwrap();
        }
        // A linked worktree carries a `.git` file naming its gitdir.
        fs::write(
            project.path().join("worktrees/agent/.git"),
            "gitdir: /elsewhere/.git/worktrees/agent\n",
        )
        .unwrap();

        // The fallback path first: no repository here yet, so the walk runs.
        // It skips the nested worktree — the rule that closes the measured
        // bug on its own — and it does **not** honour the ignore file, which
        // is the stated limit of having no second ignore engine: without git
        // there is no repository for those rules to be the rules of.
        assert_eq!(
            discovered(project.path()),
            vec![
                "ignored/deep/AGENTS.md".to_owned(),
                "research/demo/AGENTS.md".to_owned()
            ],
            "the fallback walk must skip a directory that is its own worktree"
        );

        if !git_init(project.path()) {
            eprintln!("git is unavailable; the git half of this test is skipped");
            return;
        }
        assert_eq!(
            discovered(project.path()),
            vec!["research/demo/AGENTS.md".to_owned()],
            "git must not show an ignored directory or a nested worktree"
        );
    }

    /// The reported nested paths, in order, as the finding names them.
    fn discovered(root: &Path) -> Vec<String> {
        let report = check_project(root, &opts());
        let mut shown: Vec<String> = report
            .findings
            .iter()
            .filter(|finding| {
                finding.check == CheckId::RedirectBlock
                    && finding.severity == Severity::Warning
                    && finding.message.contains("below the project root")
            })
            .map(|finding| {
                finding
                    .path
                    .as_ref()
                    .expect("a discoverable finding names its path")
                    .display()
                    .to_string()
                    .replace('\\', "/")
            })
            .collect();
        shown.sort();
        shown
    }

    fn git_init(root: &Path) -> bool {
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
