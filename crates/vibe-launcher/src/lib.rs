//! `vibe-launcher` — the shared core behind vibevm's GUI launchers (PROP-043).
//!
//! A launcher is a thin, GUI-subsystem binary (e.g. `vibetree` → `vibe tree -t`,
//! `vibeterm` → `vibe term`) that resolves the active `vibe`, starts a
//! sub-command without a console flash, and reports any failure in a native
//! dialog. The heavy work stays in `vibe`; this crate only resolves it, spawns
//! it windowless, and never dies silently on a double-click.

specmark::scope!("spec://vibevm/modules/vibe-launcher/PROP-043#root");

use std::path::PathBuf;
use std::process::{Command, ExitCode};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

/// Windows `CREATE_NO_WINDOW`: the console-subsystem `vibe` child gets no console
/// of its own, so a GUI launcher never flashes one (PROP-043 #spawn).
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// The launcher core's failure surface (PROP-043 #resolve/#spawn/#report). One
/// enum for the layer; each message cites the violated REQ and a fix surface.
#[derive(Debug, thiserror::Error)]
#[specmark::spec(implements = "spec://vibevm/modules/vibe-launcher/PROP-043#report")]
pub(crate) enum LauncherError {
    #[error(
        "vibe not found (looked at `{looked}`) \
         (violates spec://vibevm/modules/vibe-launcher/PROP-043#resolve; \
          fix: reinstall vibe, or set VIBEVM_HOME to an instance dir)"
    )]
    Unresolved { looked: String },

    #[error(
        "could not start `{vibe}`: {source} \
         (violates spec://vibevm/modules/vibe-launcher/PROP-043#spawn; \
          fix: check the vibe instance is intact — `vibe self doctor`)"
    )]
    Spawn {
        vibe: String,
        #[source]
        source: std::io::Error,
    },

    #[error(
        "`vibe {argv}` exited with {code:?} \
         (violates spec://vibevm/modules/vibe-launcher/PROP-043#report; \
          fix: run `vibe {argv}` from a console to see the underlying error)"
    )]
    ChildFailed { argv: String, code: Option<i32> },
}

/// Run a `vibe` sub-command as a GUI launcher: resolve `vibe`, spawn it without a
/// console, and translate any failure into a native dialog + a failure exit code
/// (PROP-043 #spawn/#report). A launcher binary is the one-liner over this.
///
/// ```no_run
/// // The whole of a launcher binary (e.g. `vibetree`):
/// let code = vibe_launcher::run(&["tree", "-t"]);
/// # let _ = code;
/// ```
#[specmark::spec(implements = "spec://vibevm/modules/vibe-launcher/PROP-043#spawn")]
pub fn run(subcommand: &[&str]) -> ExitCode {
    match try_run(subcommand) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            report(&err.to_string());
            ExitCode::FAILURE
        }
    }
}

/// Terminal-aware launcher entry (VIBE-LAUNCHERS / PROP-042 §5.1): for a launcher
/// whose command has an in-terminal console mode (vibetree → `vibe tree`).
///
/// - Launched **from a terminal** (inside vibeterm, or any shell): run the
///   sub-command **in place** — inherit stdio so the child renders in the current
///   terminal and the shell waits for it. For `vibe tree` this upgrades the
///   terminal (console TUI + the vibetree icon) instead of opening a window.
/// - **Double-clicked** (we own a fresh console alone): hide that console and
///   spawn the app windowless, exactly like [`run`].
///
/// The binary MUST be **console-subsystem** (no `windows_subsystem = "windows"`)
/// for the in-terminal case to work — a GUI-subsystem process the shell does not
/// wait for, so a hosted TUI would race the shell prompt.
///
/// ```no_run
/// // The whole of a console-subsystem launcher binary (e.g. `vibetree`):
/// let code = vibe_launcher::run_terminal_aware(&["tree", "-t"]);
/// # let _ = code;
/// ```
#[specmark::spec(implements = "spec://vibevm/modules/vibe-cli/PROP-042#in-place-upgrade")]
pub fn run_terminal_aware(subcommand: &[&str]) -> ExitCode {
    if in_terminal() {
        return run_inherited(subcommand);
    }
    // Double-click: hide the console Windows just gave us, then open the app.
    #[cfg(windows)]
    win::hide_own_console();
    run(subcommand)
}

/// Run the sub-command in the current terminal: inherit stdio, wait, and mirror
/// its exit code. Failures go to stderr (visible in the terminal), not a dialog.
fn run_inherited(subcommand: &[&str]) -> ExitCode {
    let vibe = match resolve_vibe() {
        Ok(vibe) => vibe,
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::FAILURE;
        }
    };
    match Command::new(&vibe).args(subcommand).status() {
        Ok(status) => ExitCode::from(u8::try_from(status.code().unwrap_or(1)).unwrap_or(1)),
        Err(err) => {
            eprintln!("could not start `{}`: {err}", vibe.display());
            ExitCode::FAILURE
        }
    }
}

/// Whether we were launched from a terminal (run in place) rather than by a
/// double-click (spawn a window). `$VIBETERM` (set inside vibeterm) is definitive;
/// otherwise a Windows console shared with another process (the shell) marks a
/// terminal, and on other platforms a tty stdout does.
fn in_terminal() -> bool {
    if std::env::var_os("VIBETERM").is_some_and(|v| !v.is_empty()) {
        return true;
    }
    #[cfg(windows)]
    {
        // A double-click owns a fresh console alone (count 1); a shell that
        // launched us is attached too (count > 1).
        win::console_process_count() > 1
    }
    #[cfg(not(windows))]
    {
        use std::io::IsTerminal;
        std::io::stdout().is_terminal()
    }
}

/// Resolve → spawn → wait; the fallible half of [`run`].
fn try_run(subcommand: &[&str]) -> Result<(), LauncherError> {
    let vibe = resolve_vibe()?;
    let mut cmd = Command::new(&vibe);
    cmd.args(subcommand);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let status = cmd.status().map_err(|source| LauncherError::Spawn {
        vibe: vibe.display().to_string(),
        source,
    })?;
    if status.success() {
        Ok(())
    } else {
        Err(LauncherError::ChildFailed {
            argv: subcommand.join(" "),
            code: status.code(),
        })
    }
}

/// Locate the active `vibe`, self-relative first (PROP-043 #resolve): `VIBEVM_HOME`
/// wins (CI / an explicit instance); else the VVM `current` pointer next to the
/// launcher's own `…/opt/bin`; else a `PATH` search as a last resort.
fn resolve_vibe() -> Result<PathBuf, LauncherError> {
    let mut looked: Vec<String> = Vec::new();

    if let Some(home) = std::env::var_os("VIBEVM_HOME") {
        let cand = PathBuf::from(home).join(vibe_exe());
        if cand.is_file() {
            return Ok(cand);
        }
        looked.push(cand.display().to_string());
    }

    if let Some(cand) = via_current_pointer() {
        if cand.is_file() {
            return Ok(cand);
        }
        looked.push(cand.display().to_string());
    }

    if let Some(cand) = via_path() {
        return Ok(cand);
    }
    looked.push("PATH".to_string());

    Err(LauncherError::Unresolved {
        looked: looked.join(", "),
    })
}

/// The launcher lives in `…/opt/bin/<name>.exe`; the VVM active instance is named
/// by `…/opt/vibevm/current` (a file holding the instance dir). Read it and join
/// `vibe[.exe]` — the same derivation the `vibe` shim uses (PROP-043 #resolve).
fn via_current_pointer() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let bin_dir = exe.parent()?;
    let pointer = bin_dir.parent()?.join("vibevm").join("current");
    let instance = std::fs::read_to_string(&pointer).ok()?;
    let instance = instance.trim();
    if instance.is_empty() {
        return None;
    }
    Some(PathBuf::from(instance).join(vibe_exe()))
}

/// First `PATH` entry that holds `vibe[.exe]`, if any (the fallback resolver).
fn via_path() -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).find_map(|dir| {
        let cand = dir.join(vibe_exe());
        cand.is_file().then_some(cand)
    })
}

/// The platform's `vibe` binary file name.
fn vibe_exe() -> &'static str {
    if cfg!(windows) { "vibe.exe" } else { "vibe" }
}

/// Surface a failure where a user of a windowless process can see it: a native
/// dialog on Windows, stderr elsewhere (PROP-043 #report).
#[specmark::spec(
    deviates = "spec://vibevm/modules/vibe-launcher/PROP-043#report",
    reason = "one isolated MessageBoxW FFI call with a documented SAFETY invariant; a dedicated audit crate for a single dialog is not warranted"
)]
fn report(message: &str) {
    #[cfg(windows)]
    // SAFETY: MessageBoxW with a null owner and two valid NUL-terminated UTF-16
    // buffers that outlive the call; no state is shared across threads.
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
        let text: Vec<u16> = message.encode_utf16().chain(std::iter::once(0)).collect();
        // Neutral title: the core is shared by every launcher (VibeTree, VibeTerm).
        let title: Vec<u16> = "vibevm".encode_utf16().chain(std::iter::once(0)).collect();
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
    #[cfg(not(windows))]
    eprintln!("{message}");
}

/// Windows console probes for [`run_terminal_aware`]: tell a double-click (our own
/// fresh console) from a shell launch, and hide our console on the double-click
/// path so none lingers behind the spawned app.
#[cfg(windows)]
mod win {
    /// The number of processes attached to our console. `1` means we own a fresh
    /// console alone — a double-click; `> 1` means a shell launched us into its
    /// console (a terminal). A missing console reports `0`.
    #[specmark::spec(
        deviates = "spec://vibevm/modules/vibe-launcher/PROP-043#spawn",
        reason = "unsafe-gate: one FFI call to GetConsoleProcessList into a fixed stack buffer; the API's whole contract is a valid writable buffer + its length, both satisfied — a dedicated audit crate for a single console probe is not warranted"
    )]
    pub(super) fn console_process_count() -> u32 {
        use windows_sys::Win32::System::Console::GetConsoleProcessList;
        let mut pids = [0u32; 4];
        // SAFETY: GetConsoleProcessList writes up to `pids.len()` PIDs into the
        // valid, aligned stack buffer and returns the true count; no aliasing.
        unsafe { GetConsoleProcessList(pids.as_mut_ptr(), pids.len() as u32) }
    }

    /// Hide our own console window (the double-click path), so no console lingers
    /// behind the spawned app. Called only when we own the console alone, so it
    /// never hides a shell's window.
    #[specmark::spec(
        deviates = "spec://vibevm/modules/vibe-launcher/PROP-043#spawn",
        reason = "unsafe-gate: two FFI calls (GetConsoleWindow, ShowWindow) on our own console HWND; a null handle is a no-op and no Rust-owned memory is touched — a dedicated audit crate for hiding one window is not warranted"
    )]
    pub(super) fn hide_own_console() {
        use windows_sys::Win32::System::Console::GetConsoleWindow;
        use windows_sys::Win32::UI::WindowsAndMessaging::{SW_HIDE, ShowWindow};
        // SAFETY: GetConsoleWindow returns our console HWND or null; ShowWindow on
        // it (or a null no-op) touches no Rust-owned memory.
        unsafe {
            let hwnd = GetConsoleWindow();
            if !hwnd.is_null() {
                ShowWindow(hwnd, SW_HIDE);
            }
        }
    }
}
