//! Locating and spawning the npm toolchain on every platform.
//!
//! Node tools install as `node_modules/.bin/<tool>` (POSIX) plus a
//! `<tool>.cmd` shim (Windows) — a bare `Command::new("<tool>")` cannot
//! spawn the `.cmd` form (the PROP-015 mcp lesson), so every spawn goes
//! through here. Resolution is strictly project-local: the tool the
//! floor runs is the tool the project pinned, never a global stray.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The project-local path of an npm-installed tool, if present.
pub(crate) fn local_tool(root: &Path, tool: &str) -> Option<PathBuf> {
    let bin = root.join("node_modules").join(".bin");
    if cfg!(windows) {
        let cmd = bin.join(format!("{tool}.cmd"));
        if cmd.exists() {
            return Some(cmd);
        }
    }
    let plain = bin.join(tool);
    if plain.exists() { Some(plain) } else { None }
}

/// A ready-to-run command for a project-local npm tool. `None` when the
/// tool is not installed — the caller renders the recipe.
pub(crate) fn tool_command(root: &Path, tool: &str) -> Option<Command> {
    let path = local_tool(root, tool)?;
    if cfg!(windows) && path.extension().is_some_and(|e| e == "cmd") {
        let mut cmd = Command::new("cmd");
        cmd.arg("/c").arg(path);
        cmd.current_dir(root);
        return Some(cmd);
    }
    let mut cmd = Command::new(path);
    cmd.current_dir(root);
    Some(cmd)
}

/// A `node` command rooted at the project.
pub(crate) fn node_command(root: &Path) -> Command {
    let mut cmd = Command::new("node");
    cmd.current_dir(root);
    cmd
}
