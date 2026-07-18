//! `vibe term` — launch the vibeterm terminal app hosting a detected shell
//! (PROP-042 §5 `#vibe-term`). Detects the shell, resolves vibeterm (never a
//! `PATH` search), resolves its Electron binary via the app's own `path.txt`,
//! and spawns it detached.

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-042#vibe-term");

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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
    // Visible, no control server: a human uses it directly and resizes it live.
    let child = spawn_vibeterm(exec, cols, rows, false, false)?;
    println!("vibeterm launched (pid {}) — running `{exec}`", child.id());
    Ok(())
}

/// Spawn vibeterm detached and return the child handle. `control` adds
/// `--control` so vibeterm starts its AIUI control server + discovery file;
/// `headless` adds `--headless` so no OS window pops up (a control session is
/// driven over HTTP and read from the headless mirror, PROP-042 §4). The two are
/// orthogonal: a headless control session is the default agent case, but
/// `vibe aiui open --visible` runs control **visible** so a human can watch and
/// resize it live. `vibe term` / `vibe tree -t` pass neither. Shared by all three.
pub(crate) fn spawn_vibeterm(
    exec: &str,
    cols: Option<u16>,
    rows: Option<u16>,
    control: bool,
    headless: bool,
) -> Result<std::process::Child> {
    let vibeterm = resolve_vibeterm()?;
    let electron = electron_binary(&vibeterm)?;
    let mut cmd = Command::new(&electron);
    // Dev layout: `electron <appdir>` (Electron resolves via node_modules/electron).
    // Packaged layout: electron-packager put the binary at the dir root and it
    // auto-loads resources/app, so NO app-path argument is passed.
    if vibeterm.shape == VibetermShape::Dev {
        cmd.arg(&vibeterm.dir);
    }
    cmd.arg("--exec")
        .arg(exec)
        .current_dir(&vibeterm.dir)
        // Detach the child's stdio. vibeterm is a GUI process that long-outlives
        // this launcher; if it inherited our pipes it would hold them open, so a
        // `vibe aiui open` whose stdout is captured (e.g. `pid=$(vibe aiui
        // open)`) would hang until vibeterm exits, and Electron's GPU-cache
        // chatter would spam the launcher's console.
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Some(c) = cols {
        cmd.arg("--cols").arg(c.to_string());
    }
    if let Some(r) = rows {
        cmd.arg("--rows").arg(r.to_string());
    }
    if control {
        cmd.arg("--control");
        // In control mode, also open a Chrome DevTools Protocol endpoint so an
        // external agent (chromiumoxide) can attach and read the live page's
        // real state (xterm grid cols/cell metrics, DOM layout) without OCR.
        // We pick the loopback port synchronously here and pass it so vibeterm
        // publishes it in its discovery file; the agent reads that. A tiny race
        // exists between this bind and Electron's, acceptable on loopback.
        let cdp = pick_free_loopback_port()?;
        cmd.arg("--cdp-port").arg(cdp.to_string());
    }
    if headless {
        cmd.arg("--headless");
    }
    cmd.spawn()
        .map_err(|e| anyhow!("launching vibeterm via `{}`: {e}", electron.display()))
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

/// Bind a transient loopback TCP socket at an OS-chosen port and return that
/// port, purely to hand Electron a likely-free `--remote-debugging-port` for
/// the CDP endpoint. The socket is dropped immediately, so a small race with
/// Electron's own bind remains — acceptable on loopback for a dev/debug endpoint.
pub(crate) fn pick_free_loopback_port() -> Result<u16> {
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0))
        .map_err(|e| anyhow!("finding a free loopback port for CDP: {e}"))?;
    listener
        .local_addr()
        .map(|a| a.port())
        .map_err(|e| anyhow!("resolving the CDP port: {e}"))
}

/// Whether a resolved vibeterm is the unpackaged dev source (`apps/vibeterm`,
/// Electron resolved through `node_modules/electron/path.txt`) or a packaged,
/// self-contained build (electron binary at the dir root, `resources/app/`
/// inside — produced by `apps/vibeterm/scripts/package.mjs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VibetermShape {
    Dev,
    Packaged,
}

/// A resolved vibeterm: its directory and whether it is the dev or packaged form.
pub(crate) struct Vibeterm {
    pub(crate) dir: PathBuf,
    pub(crate) shape: VibetermShape,
}

/// The executable name a packaged vibeterm ships: electron-packager names it
/// after the app (`vibeterm`), so a packaged build carries `vibeterm(.exe)` at
/// its dir root. That exe is the one signal present in BOTH packaging layouts —
/// the unpacked `resources/app/` tree and the asar-packed `resources/app.asar`.
pub(crate) fn packaged_exe_name() -> &'static str {
    if cfg!(windows) {
        "vibeterm.exe"
    } else {
        "vibeterm"
    }
}

/// Classify a dir as dev or packaged by electron-packager's signature: a
/// packaged build carries the app-named binary at its root (PROP-042 §5 names
/// "electron binary at its root" as the distinguishing feature). Keying off the
/// exe — not `resources/app/package.json` — is what lets an **asar-packed**
/// build be recognized: `@electron/packager` v20 defaults asar ON, so the app
/// lives in `resources/app.asar` with no unpacked `app/package.json`, and the
/// old probe wrongly demoted such an instance to `Dev` (the bug that made
/// `vibe term` / `vibe tree -t` fail from a correctly-packaged instance). A dev
/// source dir has no app-named exe, so it still falls to `Dev`.
fn classify_vibeterm(dir: &Path) -> VibetermShape {
    if dir.join(packaged_exe_name()).is_file() {
        VibetermShape::Packaged
    } else {
        VibetermShape::Dev
    }
}

/// Locate the vibeterm app directory without a `PATH` search (PROP-042 §5):
/// `$VIBEVM_VIBETERM` wins (an explicit override); else an installed `vibe`
/// finds the packaged `vibeterm/` shipped inside its own instance dir
/// (`selfloc::derive_self`); else a dev fallback walks up from the running
/// binary for `apps/vibeterm` — works for a `target/debug` or `target/release`
/// build alike, since both sit under the repo root.
fn resolve_vibeterm() -> Result<Vibeterm> {
    // Tier 1 — explicit override (dev or packaged).
    if let Some(dir) = std::env::var_os("VIBEVM_VIBETERM") {
        let dir = PathBuf::from(dir);
        let dev_ok = dir.join("package.json").exists();
        // A packaged build is recognized by its app-named exe at the root, not
        // by `resources/app/package.json` — the latter is absent in an
        // asar-packed build (see `classify_vibeterm`).
        let pkg_ok = dir.join(packaged_exe_name()).is_file();
        if !dev_ok && !pkg_ok {
            bail!(
                "$VIBEVM_VIBETERM = `{}` is neither a dev app nor a packaged build",
                dir.display()
            );
        }
        let shape = classify_vibeterm(&dir);
        return Ok(Vibeterm { dir, shape });
    }
    // Tier 2 — packaged alongside the running binary in its VVM instance.
    if let Some(loc) =
        crate::commands::vvm::selfloc::derive_self(std::env::current_exe().ok().as_deref())
    {
        let cand = loc.home.join("vibeterm");
        if matches!(classify_vibeterm(&cand), VibetermShape::Packaged) {
            return Ok(Vibeterm {
                dir: cand,
                shape: VibetermShape::Packaged,
            });
        }
    }
    // Tier 3 — dev walk-up for apps/vibeterm.
    let exe = std::env::current_exe()?;
    let mut cursor = exe.parent();
    while let Some(dir) = cursor {
        let cand = dir.join("apps").join("vibeterm");
        if cand.join("package.json").exists() {
            return Ok(Vibeterm {
                dir: cand,
                shape: VibetermShape::Dev,
            });
        }
        cursor = dir.parent();
    }
    bail!(
        "vibeterm not found — set $VIBEVM_VIBETERM to its directory \
         (dev: <repo>/apps/vibeterm; packaged: the instance's vibeterm/)"
    )
}

/// Resolve vibeterm's Electron binary. Dev: through its own
/// `node_modules/electron/path.txt` (the resolution the electron package uses).
/// Packaged: the binary sits at the dir root (electron-packager lays it there).
fn electron_binary(v: &Vibeterm) -> Result<PathBuf> {
    match v.shape {
        VibetermShape::Packaged => {
            // electron-packager names the executable after the app name —
            // `vibeterm` (`apps/vibeterm/scripts/package.mjs` passes `'vibeterm'`
            // as the packager name) — so a packaged build ships `vibeterm.exe`,
            // NOT `electron.exe`. Looking for `electron.exe` here was the bug
            // that made `vibe tree -t` from an instance fail to spawn.
            let bin = v.dir.join(packaged_exe_name());
            if !bin.is_file() {
                bail!(
                    "packaged vibeterm's binary is missing at `{}`",
                    bin.display()
                );
            }
            Ok(bin)
        }
        VibetermShape::Dev => {
            let base = v.dir.join("node_modules").join("electron");
            let path_txt = base.join("path.txt");
            let rel = std::fs::read_to_string(&path_txt).map_err(|_| {
                anyhow!(
                    "vibeterm's Electron is not installed (no `{}`). Run `npm install` in \
                     `{}` and follow its README (npm 11 blocks the postinstall).",
                    path_txt.display(),
                    v.dir.display()
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
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// A packaged build ships `vibeterm.exe` (named after the app by
    /// electron-packager), NOT `electron.exe` — `electron_binary` must resolve
    /// the app-named binary (PROP-042 §5). This is the regression that made
    /// `vibe tree -t` from an instance fail to spawn.
    #[test]
    fn packaged_electron_binary_is_app_named() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("vibeterm-win32-x64");
        fs::create_dir_all(dir.join("resources").join("app")).unwrap();
        fs::write(
            dir.join("resources").join("app").join("package.json"),
            b"{}",
        )
        .unwrap();
        let exe_name = if cfg!(windows) {
            "vibeterm.exe"
        } else {
            "vibeterm"
        };
        fs::write(dir.join(exe_name), b"binary").unwrap();
        // classify agrees it is packaged.
        assert_eq!(classify_vibeterm(&dir), VibetermShape::Packaged);
        let v = Vibeterm {
            dir: dir.clone(),
            shape: VibetermShape::Packaged,
        };
        let bin = electron_binary(&v).expect("packaged binary resolves");
        assert_eq!(bin.file_name().unwrap(), exe_name);
    }

    /// A packaged dir without the app-named binary errors clearly, never hangs.
    #[test]
    fn packaged_missing_binary_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("vibeterm-win32-x64");
        fs::create_dir_all(dir.join("resources").join("app")).unwrap();
        fs::write(
            dir.join("resources").join("app").join("package.json"),
            b"{}",
        )
        .unwrap();
        let v = Vibeterm {
            dir,
            shape: VibetermShape::Packaged,
        };
        assert!(electron_binary(&v).is_err());
    }

    /// An asar-packed build (`@electron/packager` v20's default) has
    /// `resources/app.asar` and NO unpacked `resources/app/package.json` — only
    /// the app-named exe at root. `classify` must still call it Packaged and
    /// `electron_binary` must resolve the exe. This is the regression that made
    /// `vibe term` / `vibe tree -t` fail from a correctly-packaged instance.
    #[test]
    fn classify_recognizes_asar_packed_build() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("vibeterm-win32-x64");
        fs::create_dir_all(dir.join("resources")).unwrap();
        fs::write(dir.join("resources").join("app.asar"), b"asar").unwrap();
        fs::write(dir.join(packaged_exe_name()), b"binary").unwrap();
        // The old signal is absent — proving we no longer depend on it.
        assert!(
            !dir.join("resources")
                .join("app")
                .join("package.json")
                .is_file()
        );
        assert_eq!(classify_vibeterm(&dir), VibetermShape::Packaged);
        let v = Vibeterm {
            dir: dir.clone(),
            shape: VibetermShape::Packaged,
        };
        assert_eq!(
            electron_binary(&v).unwrap().file_name().unwrap(),
            packaged_exe_name()
        );
    }

    /// A dev source dir (a `package.json` at root, no app-named exe) stays Dev —
    /// the classifier must not mistake the source app for a packaged build.
    #[test]
    fn classify_dev_source_dir_stays_dev() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        fs::write(dir.join("package.json"), b"{}").unwrap();
        assert_eq!(classify_vibeterm(dir), VibetermShape::Dev);
    }
}
