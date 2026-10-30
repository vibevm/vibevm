//! Windows: spawn the shell in the inherited console and wait, consuming this
//! process's own copy of Ctrl+C / Ctrl+Break so the shell alone handles it.

#![allow(unsafe_code)]

use std::io;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, anyhow};

type HandlerRoutine = Option<unsafe extern "system" fn(u32) -> i32>;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn SetConsoleCtrlHandler(handler_routine: HandlerRoutine, add: i32) -> i32;
}

const CTRL_C_EVENT: u32 = 0;
const CTRL_BREAK_EVENT: u32 = 1;

#[specmark::spec(
    deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "unsafe-gate: SetConsoleCtrlHandler requires a system-ABI callback; this stateless one only classifies the event (spec://org.vibevm.core/vibevm/common/PROP-059#shell-command)."
)]
unsafe extern "system" fn console_control(event: u32) -> i32 {
    i32::from(matches!(event, CTRL_C_EVENT | CTRL_BREAK_EVENT))
}

struct Guard;

impl Guard {
    #[specmark::spec(
        deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
        reason = "unsafe-gate: registers the static stateless console callback above; no borrowed state outlives it (spec://org.vibevm.core/vibevm/common/PROP-059#shell-command)."
    )]
    fn install() -> io::Result<Self> {
        // SAFETY: the callback is static, has the required ABI and reads no state.
        if unsafe { SetConsoleCtrlHandler(Some(console_control), 1) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self)
    }
}

impl Drop for Guard {
    #[specmark::spec(
        deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
        reason = "unsafe-gate: removes the same static callback the guard registered (spec://org.vibevm.core/vibevm/common/PROP-059#shell-command)."
    )]
    fn drop(&mut self) {
        // SAFETY: removes exactly the callback registered by `install`.
        let _ = unsafe { SetConsoleCtrlHandler(Some(console_control), 0) };
    }
}

/// Run the shell and exit with its full status; never returns on success.
pub(super) fn run(mut command: Command, shell: &Path) -> Result<()> {
    let _guard = Guard::install().context("installing the console control handler")?;
    let status = command
        .status()
        .with_context(|| format!("starting the shell `{}`", shell.display()))?;
    let code = status
        .code()
        .ok_or_else(|| anyhow!("the shell exited without an exit code"))?;
    // `exit` keeps the guard installed until the process ends, so a queued
    // Ctrl+C cannot replace the shell's status.
    std::process::exit(code)
}
