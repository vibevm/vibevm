//! Self-installing GUI launchers (PROP-043 #self-install × PROP-019 §2.7):
//! `vibe self update` builds the launcher binaries, places them in the shim dir
//! next to the `vibe` shim, and — on Windows — (re)creates the Start-menu
//! shortcuts. Updating vibe thus refreshes its launchers too, with no manual
//! `cargo build --release -p vibe-launcher` + copy + hand-made shortcut.
//!
//! Best-effort by contract: a launcher that is *running* (locked), a missing
//! resource compiler, or a shortcut failure is a **note**, never an install
//! failure — by the time this runs the new `vibe` instance is already active.
//! The machine-touching work (writing the on-PATH shim dir, creating Start-menu
//! shortcuts) sits behind [`LauncherInstaller`] so the gate injects a no-op and
//! never mutates the real machine — only a live `vibe self update` takes the
//! native path, exactly like the Windows env persister ([`super::env`]).

specmark::scope!("spec://vibevm/modules/vibe-launcher/PROP-043#self-install");

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

use super::model::Profile;
use super::store::VersionStore;
use crate::output;

/// One GUI launcher: its cargo bin name (= exe stem placed in the shim dir) and
/// its human label (the `.lnk` file name / shortcut title).
struct Launcher {
    bin: &'static str,
    label: &'static str,
}

/// The launchers the pipeline maintains (PROP-043): VibeTree, the complex
/// terminal, and the simple frame. One row per launcher — kept in step with
/// `vibe-launcher`'s `[[bin]]` set and its `build.rs` icon table.
const LAUNCHERS: &[Launcher] = &[
    Launcher {
        bin: "vibetree",
        label: "VibeTree",
    },
    Launcher {
        bin: "vibeterm",
        label: "VibeTerm",
    },
    Launcher {
        bin: "vibeframe",
        label: "VibeFrame",
    },
];

/// Builds + places the GUI launchers for the active install (PROP-043
/// #self-install). A crate-internal seam so the gate injects a no-op and never
/// runs cargo, writes the shim dir, or touches the Start menu.
pub(crate) trait LauncherInstaller {
    /// Refresh every launcher: build `vibe-launcher`, place the exes in the shim
    /// dir, and (Windows) (re)create the Start-menu shortcuts. Best-effort — a
    /// per-launcher problem is logged and skipped; the call returns `Ok(())`
    /// unless something wholly unexpected happens.
    fn refresh(
        &self,
        ctx: &output::Context,
        store: &VersionStore,
        source_root: &Path,
        profile: Profile,
    ) -> Result<()>;
}

/// The gate/no-op installer for tests: does nothing (so the gate never runs
/// cargo, writes the shim dir, or touches the Start menu).
#[cfg(test)]
pub(crate) struct SkipLauncherInstaller;

#[cfg(test)]
impl LauncherInstaller for SkipLauncherInstaller {
    fn refresh(
        &self,
        _ctx: &output::Context,
        _store: &VersionStore,
        _source_root: &Path,
        _profile: Profile,
    ) -> Result<()> {
        Ok(())
    }
}

/// The production installer (PROP-043 #self-install).
pub(crate) struct NativeLauncherInstaller;

impl LauncherInstaller for NativeLauncherInstaller {
    fn refresh(
        &self,
        ctx: &output::Context,
        store: &VersionStore,
        source_root: &Path,
        profile: Profile,
    ) -> Result<()> {
        // Build the launcher binaries into the same managed target-dir the
        // `vibe` build used, so they land beside it under `<build>/<profile>/`.
        // A build failure is surfaced as a note, never an install failure.
        let built_dir = match build_launchers(source_root, &store.build_dir(), profile) {
            Ok(dir) => dir,
            Err(e) => {
                ctx.summary(&format!(
                    "note: GUI launchers not refreshed — build failed ({e}); try `vibe self doctor`"
                ));
                return Ok(());
            }
        };

        let shim_dir = store.shim_dir();
        let mut placed = 0u32;
        for l in LAUNCHERS {
            let src = built_dir.join(exe_name(l.bin));
            if !src.is_file() {
                // A platform / toolchain where this launcher was not built.
                continue;
            }
            let dest = shim_dir.join(exe_name(l.bin));
            if let Err(e) = place_exe(&src, &dest) {
                ctx.summary(&format!(
                    "note: {} launcher not refreshed ({e}) — it may be running; \
                     close it and re-run `vibe self update`",
                    l.label
                ));
                continue;
            }
            placed += 1;
            if let Err(e) = install_shortcut(l.label, &dest, &shim_dir) {
                ctx.summary(&format!(
                    "note: {} Start-menu shortcut skipped ({e})",
                    l.label
                ));
            }
        }
        if placed > 0 {
            ctx.summary(&format!(
                "refreshed {placed} GUI launcher{} in {}",
                if placed == 1 { "" } else { "s" },
                shim_dir.display()
            ));
        }
        Ok(())
    }
}

/// `cargo build [--release] -p vibe-launcher --target-dir <build>` in
/// `source_root`; returns the dir holding the built exes
/// (`<build>/<profile-subdir>`). The launchers share the managed target-dir with
/// the `vibe` build (never the source tree's own `target/`), so a running
/// launcher is not relinked in place (PROP-019 §2.7, §9.3).
fn build_launchers(source_root: &Path, target_dir: &Path, profile: Profile) -> Result<PathBuf> {
    let mut cmd = Command::new("cargo");
    cmd.current_dir(source_root)
        .args(["build", "-p", "vibe-launcher"]);
    if profile == Profile::Release {
        cmd.arg("--release");
    }
    cmd.arg("--target-dir").arg(target_dir);
    let status = cmd
        .status()
        .with_context(|| format!("spawning cargo build in `{}`", source_root.display()))?;
    if !status.success() {
        bail!(
            "cargo build -p vibe-launcher failed (exit {:?})",
            status.code()
        );
    }
    Ok(target_dir.join(profile.target_subdir()))
}

/// The launcher exe file name on this platform.
fn exe_name(stem: &str) -> String {
    if cfg!(windows) {
        format!("{stem}.exe")
    } else {
        stem.to_string()
    }
}

/// Copy `src` onto `dest`, tolerating a `dest` that is a *running* launcher.
/// Windows locks a running exe against overwrite/delete but still allows
/// *renaming* it, so a live `dest` is moved to a unique sidecar first, then the
/// sidecar is dropped again once the new exe is in place — leaving the dir clean
/// when the old launcher was not running (the common case). A sidecar that is
/// still locked (its launcher is running) lingers for the next update's sweep.
/// Returns `dest`.
fn place_exe(src: &Path, dest: &Path) -> Result<PathBuf> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating `{}`", parent.display()))?;
        sweep_sidecars(parent, dest);
    }
    let mut aside = None;
    if dest.exists() {
        let candidate = sidecar_path(dest);
        // Move the (possibly running) old exe aside so the new one can be
        // written; if even the rename fails, try a direct remove (works when it
        // is not actually running).
        if std::fs::rename(dest, &candidate).is_ok() {
            aside = Some(candidate);
        } else {
            let _ = std::fs::remove_file(dest);
        }
    }
    std::fs::copy(src, dest)
        .with_context(|| format!("copying launcher to `{}`", dest.display()))?;
    // Drop the sidecar now if nothing holds it; a locked one waits for the sweep.
    if let Some(aside) = aside {
        let _ = std::fs::remove_file(aside);
    }
    Ok(dest.to_path_buf())
}

/// A unique `<name>.old-<nanos>` sidecar next to `dest`, so moving a running exe
/// aside never collides with a still-locked sidecar from a prior update.
fn sidecar_path(dest: &Path) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let name = dest
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    dest.with_file_name(format!("{name}.old-{nanos}"))
}

/// Best-effort removal of `<dest>.old-*` sidecars left by earlier updates; a
/// still-locked one (its process is alive) simply fails to delete and waits.
fn sweep_sidecars(parent: &Path, dest: &Path) {
    let Some(name) = dest.file_name().map(|s| s.to_string_lossy().into_owned()) else {
        return;
    };
    let needle = format!("{name}.old-");
    let Ok(entries) = std::fs::read_dir(parent) else {
        return;
    };
    for entry in entries.flatten() {
        if entry.file_name().to_string_lossy().starts_with(&needle) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// Create/replace the Start-menu shortcut for a launcher (PROP-043
/// #self-install). Windows only for now (Mac/Linux tracked separately) — a
/// no-op elsewhere, where the exe is still placed in the shim dir.
#[cfg(windows)]
fn install_shortcut(label: &str, target_exe: &Path, working_dir: &Path) -> Result<()> {
    // Reuse the codebase's established "shell out to PowerShell for durable
    // machine edits" approach (cf. `super::env`): WScript.Shell writes a `.lnk`
    // into the per-user Start-menu `Programs\vibevm` group — no admin, no COM
    // plumbing, no extra dependency.
    let script = format!(
        "$ErrorActionPreference='Stop'; \
         $group = Join-Path ([Environment]::GetFolderPath('Programs')) 'vibevm'; \
         New-Item -ItemType Directory -Force -Path $group | Out-Null; \
         $lnk = Join-Path $group '{label}.lnk'; \
         $s = (New-Object -ComObject WScript.Shell).CreateShortcut($lnk); \
         $s.TargetPath = '{target}'; \
         $s.IconLocation = '{target},0'; \
         $s.WorkingDirectory = '{workdir}'; \
         $s.Description = '{label}'; \
         $s.Save()",
        label = ps_quote(label),
        target = ps_quote(&target_exe.display().to_string()),
        workdir = ps_quote(&working_dir.display().to_string()),
    );
    let out = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()
        .context("spawning powershell to create the Start-menu shortcut")?;
    if !out.status.success() {
        bail!("{}", String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(())
}

/// Non-Windows: Start-menu / `.desktop` / `.app` shortcuts are tracked
/// separately (out of scope now); the exe is already placed in the shim dir.
#[cfg(not(windows))]
fn install_shortcut(_label: &str, _target_exe: &Path, _working_dir: &Path) -> Result<()> {
    Ok(())
}

/// Single-quote-escape for a PowerShell single-quoted string literal.
#[cfg(windows)]
fn ps_quote(value: &str) -> String {
    value.replace('\'', "''")
}

#[cfg(test)]
mod tests {
    use super::*;
    use specmark::verifies;

    /// A first placement creates `dest`; a second placement over the existing
    /// (here unlocked) `dest` replaces it via the rename-aside path.
    #[test]
    #[verifies("spec://vibevm/modules/vibe-launcher/PROP-043#self-install", r = 1)]
    fn place_exe_creates_then_replaces() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src.bin");
        let dest = tmp.path().join("bin").join("launcher.exe");

        std::fs::write(&src, b"one").unwrap();
        let got = place_exe(&src, &dest).unwrap();
        assert_eq!(got, dest);
        assert_eq!(std::fs::read(&dest).unwrap(), b"one");

        std::fs::write(&src, b"two").unwrap();
        place_exe(&src, &dest).unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), b"two");

        // The replaced exe's sidecar was dropped again (nothing held it), so the
        // dir is left clean — only the launcher itself remains.
        let lingering = std::fs::read_dir(dest.parent().unwrap())
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().contains(".old-"))
            .count();
        assert_eq!(lingering, 0, "no sidecar should linger when unlocked");
    }

    /// Stale `<name>.old-*` sidecars from earlier updates are swept on the next
    /// placement.
    #[test]
    #[verifies("spec://vibevm/modules/vibe-launcher/PROP-043#self-install", r = 1)]
    fn place_exe_sweeps_stale_sidecars() {
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("launcher.exe");
        let src = tmp.path().join("src.bin");
        std::fs::write(&dest, b"old").unwrap();
        std::fs::write(tmp.path().join("launcher.exe.old-123"), b"stale").unwrap();
        std::fs::write(&src, b"new").unwrap();

        place_exe(&src, &dest).unwrap();

        assert!(!tmp.path().join("launcher.exe.old-123").exists());
        assert_eq!(std::fs::read(&dest).unwrap(), b"new");
    }

    #[test]
    fn exe_name_is_platform_shaped() {
        if cfg!(windows) {
            assert_eq!(exe_name("vibetree"), "vibetree.exe");
        } else {
            assert_eq!(exe_name("vibetree"), "vibetree");
        }
    }
}
