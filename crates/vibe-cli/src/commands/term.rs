//! `vibe term` — launch the vibeterm terminal app hosting a detected shell
//! (PROP-042 §5 `#vibe-term`). Detects the shell, resolves vibeterm (never a
//! `PATH` search), resolves its Electron binary via the app's own `path.txt`,
//! and spawns it detached.

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-042#vibe-term");

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Result, anyhow, bail};
use specmark::spec;

use crate::cli::TermArgs;
use crate::output;

/// Run `vibe term`: resolve vibeterm + its Electron binary, pick the shell (or
/// the `--exec` override), and launch the terminal detached.
#[spec(implements = "spec://vibevm/modules/vibe-cli/PROP-042#vibe-term")]
pub fn run(_ctx: &output::Context, args: TermArgs) -> Result<()> {
    // A bare shell path may contain spaces (`C:\Program Files\…\pwsh.exe`);
    // quote it so vibeterm's `splitCommand` keeps it as one token. A user
    // `--exec` command line is passed through verbatim (they quote as needed).
    let exec = match args.exec {
        Some(cmd) => cmd,
        None => quote_exe(&detect_shell()),
    };
    launch_vibeterm(&exec, args.cols, args.rows)
}

/// Launch vibeterm running `exec` (the command line for its PTY), optionally at
/// `cols×rows`. Shared by `vibe term` and `vibe tree -t` (PROP-042 §5): resolve
/// vibeterm + its Electron binary, spawn it detached, report the pid.
pub(crate) fn launch_vibeterm(exec: &str, cols: Option<u16>, rows: Option<u16>) -> Result<()> {
    let vibeterm = resolve_vibeterm()?;
    let electron = electron_binary(&vibeterm)?;
    let mut cmd = Command::new(&electron);
    cmd.arg(&vibeterm)
        .arg("--exec")
        .arg(exec)
        .current_dir(&vibeterm);
    if let Some(c) = cols {
        cmd.arg("--cols").arg(c.to_string());
    }
    if let Some(r) = rows {
        cmd.arg("--rows").arg(r.to_string());
    }
    let child = cmd
        .spawn()
        .map_err(|e| anyhow!("launching vibeterm via `{}`: {e}", electron.display()))?;
    println!("vibeterm launched (pid {}) — running `{exec}`", child.id());
    Ok(())
}

/// Double-quote an executable path that contains whitespace so vibeterm's
/// `splitCommand` keeps it as one token; already-quoted or space-free paths pass
/// through unchanged.
pub(crate) fn quote_exe(exe: &str) -> String {
    if exe.contains(char::is_whitespace) && !exe.starts_with('"') {
        format!("\"{exe}\"")
    } else {
        exe.to_string()
    }
}

/// Locate the vibeterm app directory without a `PATH` search (PROP-042 §5):
/// `$VIBEVM_VIBETERM` wins; else a dev fallback walks up from the running binary
/// for `research/vibeterm`.
fn resolve_vibeterm() -> Result<PathBuf> {
    if let Some(dir) = std::env::var_os("VIBEVM_VIBETERM") {
        let dir = PathBuf::from(dir);
        if dir.join("package.json").exists() {
            return Ok(dir);
        }
        bail!("$VIBEVM_VIBETERM = `{}` has no package.json", dir.display());
    }
    let exe = std::env::current_exe()?;
    let mut cursor = exe.parent();
    while let Some(dir) = cursor {
        let cand = dir.join("research").join("vibeterm");
        if cand.join("package.json").exists() {
            return Ok(cand);
        }
        cursor = dir.parent();
    }
    bail!(
        "vibeterm not found — set $VIBEVM_VIBETERM to its directory \
         (dev: <repo>/research/vibeterm)"
    )
}

/// Resolve vibeterm's Electron binary through its own `node_modules/electron/
/// path.txt` — the canonical, cross-platform resolution the electron package
/// itself uses.
fn electron_binary(vibeterm: &Path) -> Result<PathBuf> {
    let base = vibeterm.join("node_modules").join("electron");
    let path_txt = base.join("path.txt");
    let rel = std::fs::read_to_string(&path_txt).map_err(|_| {
        anyhow!(
            "vibeterm's Electron is not installed (no `{}`). Run `npm install` in \
             `{}` and follow its README (npm 11 blocks the postinstall).",
            path_txt.display(),
            vibeterm.display()
        )
    })?;
    let bin = base.join("dist").join(rel.trim());
    if !bin.exists() {
        bail!(
            "vibeterm's Electron binary is missing at `{}`",
            bin.display()
        );
    }
    Ok(bin)
}

/// The interactive shell to host (PROP-042 §5): pwsh 7+ preferred on Windows.
#[cfg(windows)]
fn detect_shell() -> String {
    if let Some(pwsh) = pwsh_path() {
        return pwsh;
    }
    let sysroot = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string());
    format!("{sysroot}\\System32\\WindowsPowerShell\\v1.0\\powershell.exe")
}

/// Locate modern PowerShell 7+ (`pwsh`): the standard install locations, then
/// `PATH`. `None` when only Windows PowerShell 5.1 is available.
#[cfg(windows)]
fn pwsh_path() -> Option<String> {
    for var in ["ProgramFiles", "ProgramW6432"] {
        if let Ok(pf) = std::env::var(var) {
            let cand = format!("{pf}\\PowerShell\\7\\pwsh.exe");
            if Path::new(&cand).exists() {
                return Some(cand);
            }
        }
    }
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        let cand = format!("{local}\\Microsoft\\WindowsApps\\pwsh.exe");
        if Path::new(&cand).exists() {
            return Some(cand);
        }
    }
    which_on_path("pwsh.exe")
}

/// The first `PATH` entry containing `name`, if any.
#[cfg(windows)]
fn which_on_path(name: &str) -> Option<String> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).find_map(|dir| {
        let cand = dir.join(name);
        cand.exists().then(|| cand.to_string_lossy().into_owned())
    })
}

/// The interactive shell to host (PROP-042 §5): `$SHELL`, else `/bin/sh`.
#[cfg(not(windows))]
fn detect_shell() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
}
