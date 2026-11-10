//! Resolving the node every `vibe prompt` verb is about, and the one
//! question about it that needs a subprocess.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-OPERATIONS-SURFACE");

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use vibe_core::manifest::{Manifest, TargetOs};
use vibe_workspace::Workspace;
use vibe_workspace::system_prompt::{CheckRequest, TrackedPaths};

/// One selected node, held open for the whole command.
pub(crate) struct Selected {
    /// The held workspace, which `build` needs and the read-only verbs do
    /// not — they take the three paths below.
    pub(crate) workspace: Workspace,
    /// The node's canonical workspace-relative path (`.` for the root).
    pub(crate) node_rel: String,
    /// The node's absolute directory.
    pub(crate) node_dir: PathBuf,
    /// The node's own manifest — the authority for `[system_prompt]`.
    pub(crate) manifest: Manifest,
    /// Whether the workspace root is a git working tree, decided once.
    pub(crate) git: bool,
}

/// Resolve the node: an explicit `--node` wins, otherwise the selection the
/// workspace itself discovers from the directory.
pub(crate) fn select(path: &Path, node: Option<&str>) -> Result<Selected> {
    let selected = Workspace::discover_selected(path).context("discovering the workspace")?;
    let workspace = selected.workspace;
    let node_rel = match node {
        Some(explicit) => {
            let canonical = explicit.trim_end_matches('/');
            let canonical = if canonical.is_empty() { "." } else { canonical };
            if canonical != "." && workspace.member_by_rel_path(canonical).is_none() {
                bail!(
                    "`--node {explicit}` is not a node of this workspace; its nodes are `.`{}",
                    workspace
                        .iter_nodes()
                        .filter(|(rel, _)| *rel != ".")
                        .map(|(rel, _)| format!(", `{rel}`"))
                        .collect::<String>()
                );
            }
            canonical.to_owned()
        }
        None => selected.selected.as_str().to_owned(),
    };
    let node_dir = workspace.node_abs_path(&node_rel);
    let manifest = workspace
        .member_by_rel_path(&node_rel)
        .map(|member| member.manifest.clone())
        .unwrap_or_else(|| workspace.root_manifest.clone());
    let git = workspace.root.join(".git").exists();
    Ok(Selected {
        node_rel,
        node_dir,
        manifest,
        git,
        workspace,
    })
}

impl Selected {
    /// The freshness request for this node.
    ///
    /// `session_os` is this process's own operating system, because a
    /// `vibe prompt` verb runs in the session it is answering about. It is
    /// `std::env::consts::OS` — a **compile-time** constant, not an
    /// environment read and not a detection step — and it feeds exactly one
    /// sentence, the host mismatch of `#SYS-HOST-AND-TARGET`. A verb that
    /// takes an explicit session-OS input replaces it with that input;
    /// nothing here decides a gate.
    pub(crate) fn request<'a>(
        &'a self,
        tracked: &'a dyn TrackedPaths,
        model_window: Option<usize>,
    ) -> CheckRequest<'a> {
        CheckRequest {
            node_rel: &self.node_rel,
            node_dir: &self.node_dir,
            workspace_root: &self.workspace.root,
            manifest: &self.manifest,
            model_window,
            session_os: TargetOs::current(),
            tracked,
        }
    }

    /// Which generated artifacts git tracks, as this invocation can observe
    /// it. Asked through `git ls-files` rather than through a second ignore
    /// engine of our own, and only where a `.git` entry exists; without one
    /// the answer is "cannot tell", which is reported as unknown rather than
    /// as clean (`#SYS-ARTIFACTS-NOT-TRACKED`).
    pub(crate) fn tracked(&self) -> GitTracked {
        GitTracked {
            paths: if self.git {
                self.git_ls_files()
            } else {
                Vec::new()
            },
            known: self.git,
        }
    }

    fn git_ls_files(&self) -> Vec<String> {
        let boot = vibe_core::layout::current_boot_dir();
        let names = [
            vibe_core::layout::SYSTEM_MD,
            vibe_core::layout::SYSTEM_META_JSON,
            vibe_core::layout::AGENT_LANE_MD,
        ];
        let mut command = Command::new("git");
        command
            .arg("-C")
            .arg(&self.workspace.root)
            .args(["ls-files", "--cached", "--"]);
        for name in names {
            let relative = if self.node_rel == "." {
                boot.join(name)
            } else {
                Path::new(&self.node_rel).join(&boot).join(name)
            };
            command.arg(relative);
        }
        command
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| {
                String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .map(|line| line.trim().replace('\\', "/"))
                    .filter(|line| !line.is_empty())
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// What `git ls-files` said, as a [`TrackedPaths`].
#[derive(Default)]
pub(crate) struct GitTracked {
    paths: Vec<String>,
    known: bool,
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
