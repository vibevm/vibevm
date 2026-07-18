//! `vibe-launcher` — the shared core behind vibevm's GUI launchers (PROP-043).
//!
//! A launcher is a thin, GUI-subsystem binary (e.g. `vibetree`) that resolves the
//! active `vibe`, starts a sub-command without a console flash, and reports any
//! failure in a native dialog. The heavy work stays in `vibe`; this crate only
//! resolves it, spawns it windowless, and never dies silently on a double-click.

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
        let title: Vec<u16> = "VibeTree"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
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
