//! Locating and spawning the go toolchain on every platform.
//!
//! Unlike the npm world there is no project-local `node_modules/.bin`:
//! go, gofmt, staticcheck and exhaustive are machine tools. Resolution
//! order: the documented env overrides (`GO_AI_NATIVE_GO` for the go
//! binary — a machine that keeps go off PATH points here; gofmt is
//! derived as its sibling), then PATH. An absent tool is the caller's
//! recipe-carrying failure, never a silent skip.

use std::path::{Path, PathBuf};
use std::process::Command;

/// A `go` command rooted at the project (env override or PATH).
pub(crate) fn go_command(root: &Path) -> Command {
    let mut cmd = Command::new(go_ai_native_extract_bridge::go_binary());
    cmd.current_dir(root);
    cmd
}

/// A `gofmt` command rooted at the project. When the go binary was
/// resolved through the env override, gofmt is taken from the same
/// directory (the toolchain ships them side by side); otherwise PATH.
pub(crate) fn gofmt_command(root: &Path) -> Command {
    let go = go_ai_native_extract_bridge::go_binary();
    let path = PathBuf::from(&go);
    let gofmt = match path.parent() {
        Some(dir) if dir.as_os_str().len() > 0 => {
            let sibling = dir.join(if cfg!(windows) { "gofmt.exe" } else { "gofmt" });
            if sibling.exists() {
                sibling
            } else {
                PathBuf::from("gofmt")
            }
        }
        _ => PathBuf::from("gofmt"),
    };
    let mut cmd = Command::new(gofmt);
    cmd.current_dir(root);
    cmd
}

/// A PATH-resolved evidence-provider command (staticcheck,
/// exhaustive) rooted at the project. Spawn failure is the caller's
/// signal that the tool is absent — the recipe travels with the step.
pub(crate) fn path_tool(root: &Path, tool: &str) -> Command {
    let mut cmd = Command::new(tool);
    cmd.current_dir(root);
    cmd
}
