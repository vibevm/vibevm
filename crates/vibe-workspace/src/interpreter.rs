//! Script interpreter selection: which program runs a package script on this
//! host, and which of the `.sh` / `.ps1` files beside its base path it runs.
//!
//! The one owner of that choice, shared by the lifecycle `script` handler and
//! `vibe run`. [`InterpreterProbe`] is the seam that keeps the selection
//! testable without depending on the machine's `PATH`.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#H-SCRIPT");

use std::path::{Path, PathBuf};
use std::process::Command;

use specmark::spec;

/// The host-OS family that drives interpreter selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    /// macOS / Linux — `.sh` via `bash`.
    Unix,
    /// Windows — `.sh` via Git Bash first, else `.ps1` via PowerShell.
    Windows,
}

impl Platform {
    /// The platform this process runs on.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#H-SCRIPT")]
    pub fn current() -> Self {
        if cfg!(windows) {
            Platform::Windows
        } else {
            Platform::Unix
        }
    }
}

/// How a script is invoked on this host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptInvocation {
    /// Interpreter program — `bash` or `powershell`.
    pub interpreter: String,
    /// The resolved script file (with extension).
    pub script: PathBuf,
}

/// Probe whether an interpreter program is usable on this host. Seam so
/// tests assert selection without depending on the machine's `PATH`.
///
/// ```
/// use vibe_workspace::interpreter::InterpreterProbe;
///
/// // The canonical use is a test double standing in for the machine `PATH`:
/// // here only `bash` is considered installed.
/// struct OnlyBash;
/// impl InterpreterProbe for OnlyBash {
///     fn has(&self, program: &str) -> bool {
///         program == "bash"
///     }
/// }
///
/// assert!(OnlyBash.has("bash"));
/// assert!(!OnlyBash.has("powershell"));
/// ```
pub trait InterpreterProbe {
    fn has(&self, program: &str) -> bool;
}

/// Production probe: spawn `<program> --version`.
pub struct SystemProbe;

impl InterpreterProbe for SystemProbe {
    fn has(&self, program: &str) -> bool {
        Command::new(program)
            .args(interpreter_probe_args(program))
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

fn interpreter_probe_args(program: &str) -> &'static [&'static str] {
    if cfg!(windows) && program.eq_ignore_ascii_case("powershell") {
        &[
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "$null = $PSVersionTable.PSVersion",
        ]
    } else {
        &["--version"]
    }
}

/// Resolve the interpreter + script for a base path on a given platform.
/// `None` means a script was declared but no usable interpreter/script pair
/// exists; the caller owns the refusal.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#H-SCRIPT")]
pub fn select_invocation(
    root: &Path,
    base: &Path,
    platform: Platform,
    probe: &dyn InterpreterProbe,
) -> Option<ScriptInvocation> {
    let sh = root.join(base).with_extension("sh");
    let ps1 = root.join(base).with_extension("ps1");
    let sh_via_bash = || {
        (sh.is_file() && probe.has("bash")).then(|| ScriptInvocation {
            interpreter: "bash".to_string(),
            script: sh.clone(),
        })
    };
    match platform {
        // Unix: `.sh` via bash, or nothing.
        Platform::Unix => sh_via_bash(),
        // Windows: prefer `.sh` via Git Bash; fall back to `.ps1` via
        // PowerShell when no usable bash/script pair exists.
        Platform::Windows => sh_via_bash().or_else(|| {
            (ps1.is_file() && probe.has("powershell")).then(|| ScriptInvocation {
                interpreter: "powershell".to_string(),
                script: ps1.clone(),
            })
        }),
    }
}

#[cfg(test)]
#[path = "interpreter/tests.rs"]
mod tests;
