//! The system-prompt cell — the generated prefix of PROP-076 is the
//! composition of its own current inputs, and nobody committed it.
//!
//! **Why this cell exists when `vibe prompt check` already answers.** That
//! command answers one node, on request, and exits non-zero on any verdict
//! but `equal` — which is what a wrapper gates on. `vibe check` is the
//! report a developer reads without knowing to ask, so a stale prefix shows
//! up beside the project's other findings rather than only in a verb nobody
//! ran (`SP-FMT-008`).
//!
//! **A project that has opted in is the only project reported.** Presence of
//! the authored `SYSTEM-TEMPLATE.md` is the whole opt-in (`#SYS-OPT-IN`), so
//! a project without one is silent — the feature is free for every project
//! that does not want it, and a finding would be the one cost it must not
//! have. The single exception is a project that opted **out** and left an
//! artifact behind: the inputs say there should be none, and saying nothing
//! about a file on disk that nothing can explain is the quiet failure
//! `#SYS-REMOVAL-AND-CONFLICT` exists to prevent.
//!
//! **Severities, and which of them the spec chose.** An artifact git tracks
//! is an **error**: `#SYS-ARTIFACTS-NOT-TRACKED` says in those words that
//! `vibe check` reports a tracked generated prefix as one, because a tracked
//! copy is the one state the rule exists to prevent. Everything else is a
//! **warning**, which PROP-076 does not state and this cell decides: a stale
//! or missing prefix is fully derivable from committed inputs by one named
//! command, it cannot make a build, an install or a boot wrong — nothing
//! reads it but a program that is already running `vibe` — and the caller
//! who needs a hard gate has `vibe prompt check`'s non-zero exit. Making it
//! an error would fail `vibe check` for every opted-in project between an
//! input edit and the next install, which is an ordinary state rather than a
//! defect.
//!
//! **One node, the project root.** vibe-check is project-file only and does
//! not load a workspace; a member node's own prefix is
//! `vibe prompt check --node <rel>`'s answer. The root is where the prefix
//! that a caller sends lives.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-READ-ONLY-CHECK");

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use specmark::{cell, spec};
use vibe_core::manifest::Manifest;
use vibe_workspace::system_prompt::{
    CheckRequest, CheckVerdict, PromptCheck, TrackedPaths, check_node_prompt,
};

use crate::{Check, CheckId, CheckOptions, CheckReport};

/// The [`CheckId::SystemPrompt`] cell.
#[cell(seam = "Check", variant = "system-prompt")]
pub struct SystemPromptCheck;

impl Check for SystemPromptCheck {
    fn id(&self) -> CheckId {
        CheckId::SystemPrompt
    }

    /// PROP-076 `SP-FMT-008` — the freshness verdict of the generated
    /// prefix, its sidecar and the agent lane, in the ordinary report.
    #[spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-READ-ONLY-CHECK",
        r = 1
    )]
    fn run(&self, project_root: &Path, _opts: &CheckOptions, report: &mut CheckReport) {
        // A manifest that does not parse is `manifest_validity`'s finding,
        // and reporting it twice under two ids is two answers to one
        // question.
        let Ok(manifest) = Manifest::read(project_root.join(Manifest::FILENAME)) else {
            return;
        };
        let tracked = GitTracked::of(project_root);
        let request = CheckRequest {
            node_rel: ".",
            node_dir: project_root,
            workspace_root: project_root,
            manifest: &manifest,
            // The lane's size against a stated context window is a
            // `vibe prompt check` observation, not a project defect.
            model_window: None,
            tracked: &tracked,
        };
        let answer = match check_node_prompt(&request) {
            Ok(answer) => answer,
            Err(error) => {
                // Freshness could not be established at all. That is a
                // warning for the same reason staleness is: it says
                // nothing about the project's correctness, only that this
                // cell cannot answer.
                report.warn(
                    CheckId::SystemPrompt,
                    Some(boot_relative(vibe_core::layout::SYSTEM_MD)),
                    None,
                    format!("the generated system prompt could not be checked: {error}"),
                );
                return;
            }
        };
        if !reportable(&answer) {
            return;
        }
        for message in messages(&answer) {
            let path = Some(boot_relative(vibe_core::layout::SYSTEM_MD));
            match answer.verdict {
                CheckVerdict::Conflict(_) => {
                    report.err(CheckId::SystemPrompt, path, None, message);
                }
                _ => report.warn(CheckId::SystemPrompt, path, None, message),
            }
        }
    }
}

/// Whether this project is one the cell speaks about at all.
///
/// Opted in, or opted out with an artifact still on disk. A project that
/// never opted in and holds none of the three is not reported — the whole
/// feature is invisible to it.
fn reportable(answer: &PromptCheck) -> bool {
    answer.opted_in
        || answer.system.present
        || answer.meta.present
        || answer.lane.present
        || matches!(answer.verdict, CheckVerdict::Conflict(_))
}

/// The findings for one verdict: the verdict's own name, each of its
/// reasons, and the exact `vibe prompt build` that resolves it.
///
/// `equal` is no finding at all. Every other verdict carries its reasons
/// already rendered as sentences, and they are not summarised here: a
/// verdict with the wrong reason is one nobody can act on.
fn messages(answer: &PromptCheck) -> Vec<String> {
    if answer.verdict.is_equal() {
        return Vec::new();
    }
    let remedy = answer.remedy();
    answer
        .verdict
        .reasons()
        .into_iter()
        .map(|reason| {
            format!(
                "the generated system prompt is {}: {reason} — run `{remedy}`",
                answer.verdict.as_str()
            )
        })
        .collect()
}

/// One generated artifact's path as the report shows it: project-relative,
/// forward-slashed, under the live boot directory.
fn boot_relative(name: &str) -> PathBuf {
    vibe_core::layout::current_boot_dir().join(name)
}

/// Which of the three generated paths git tracks, asked of git.
///
/// A second reader of git's index would be a second answer to one question,
/// and a project that is no repository, or a machine with no `git`, gets
/// "cannot tell" rather than an invented "clean" — inventing an answer about
/// a developer's working tree is worse than saying nothing
/// (`#SYS-ARTIFACTS-NOT-TRACKED`).
struct GitTracked {
    paths: Vec<String>,
    known: bool,
}

impl GitTracked {
    fn of(project_root: &Path) -> Self {
        let boot = vibe_core::layout::current_boot_dir();
        let mut command = Command::new("git");
        command
            .arg("-C")
            .arg(project_root)
            .args(["ls-files", "--cached", "-z", "--"])
            .stderr(Stdio::null());
        for name in [
            vibe_core::layout::SYSTEM_MD,
            vibe_core::layout::SYSTEM_META_JSON,
            vibe_core::layout::AGENT_LANE_MD,
        ] {
            command.arg(boot.join(name));
        }
        match command.output() {
            Ok(output) if output.status.success() => Self {
                paths: String::from_utf8_lossy(&output.stdout)
                    .split('\0')
                    .map(|entry| entry.trim().replace('\\', "/"))
                    .filter(|entry| !entry.is_empty())
                    .collect(),
                known: true,
            },
            _ => Self {
                paths: Vec::new(),
                known: false,
            },
        }
    }
}

impl TrackedPaths for GitTracked {
    fn is_tracked(&self, path: &Path) -> Option<bool> {
        if !self.known {
            return None;
        }
        let spelling = path.to_string_lossy().replace('\\', "/");
        Some(self.paths.iter().any(|tracked| spelling.ends_with(tracked)))
    }
}

#[cfg(test)]
#[path = "system_prompt/tests.rs"]
mod tests;
