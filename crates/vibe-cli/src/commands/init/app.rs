//! The `app` package scaffold
//! ([PROP-059](../../../../../vibevm/vibespecs/common/PROP-059-user-applications.xml)).
//!
//! A package of kind `app` with nothing but `[package]` is a valid
//! product, yet `vibe app plan` and `vibe install -g` refuse it: there is
//! no `[application]` slot and no installer option to choose. A client
//! that creates an application package (the desktop shell's wizard) would
//! then have to patch the manifest by hand before the first plan. So the
//! scaffold writes the smallest declaration that plans: the slot named
//! after the package, one installer option for the host the command runs
//! on, and one `recipe` step whose script it also writes.
//!
//! Nothing optional is invented. No launcher (the validators do not ask
//! for one, and an executable name would be a guess), no icon (the
//! directory is created so the author knows where one goes), no features,
//! parameters or result: an option with steps and no `result` owns its
//! `$VIBE_OUT`, which the recipe step is allowed to write.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#declaration");

use std::path::Path;

use anyhow::Result;
use vibe_core::manifest::{Manifest, StepKind};

use super::helpers::*;
use crate::output;

/// The host's platform id and the name a client shows for it. The three
/// predefined ids are the ones installer selection detects
/// (PROP-059 `#installer-options`); any other host gets its own OS name
/// for both, which is still a portable token and selectable explicitly.
fn host_platform() -> (&'static str, &'static str) {
    match std::env::consts::OS {
        "linux" => ("linux", "Linux"),
        "macos" => ("macos", "macOS"),
        "windows" => ("windows", "Windows"),
        other => (other, other),
    }
}

/// The interpreter of the recipe and the script it runs. Windows has no
/// `/bin/sh` to rely on, so its scaffold is a PowerShell script.
fn recipe() -> (&'static str, &'static str) {
    if cfg!(windows) {
        ("windows_powershell", "recipes/build.ps1")
    } else {
        ("posix_sh", "recipes/build.sh")
    }
}

/// The `[application]` and `[[installer_options]]` tables, as TOML text
/// that follows every other table of a manifest. `name` is a validated
/// package name, so it is already a portable token and needs no quoting
/// beyond the plain string.
pub(super) fn manifest_tail(name: &str) -> String {
    let (platform, platform_name) = host_platform();
    let (interpreter, script) = recipe();
    let arch = std::env::consts::ARCH;
    format!(
        "\n[application]\nid = \"{name}\"\n\n\
         [[installer_options]]\nid = \"main\"\nname = \"Main\"\n\
         platform = \"{platform}\"\nplatform_name = \"{platform_name}\"\narch = \"{arch}\"\n\n\
         [[installer_options.steps]]\nid = \"build\"\ntitle = \"Build\"\nkind = \"recipe\"\n\
         interpreter = \"{interpreter}\"\nscript = \"{script}\"\n"
    )
}

/// True when the manifest's installer options run the scaffold's recipe.
/// Re-initialising a hand-written application package must not drop a
/// script its manifest never names.
pub(super) fn declares_scaffold(manifest: &Manifest) -> bool {
    let (_, script) = recipe();
    manifest.installer_options.iter().any(|option| {
        option.steps.iter().any(|step| {
            matches!(
                &step.kind,
                StepKind::Recipe { script: declared, .. } if declared == Path::new(script)
            )
        })
    })
}

/// Write the recipe script and the icon directory under `pkg_dir`.
pub(super) fn create_app_files(
    ctx: &output::Context,
    project_root: &Path,
    pkg_dir: &Path,
    name: &str,
) -> Result<Vec<Outcome>> {
    let (_, script) = recipe();
    let script_path = pkg_dir.join(script);
    let body = format!(
        "# The `build` step of the `main` installer option: it runs while the\n\
         # application is installed and writes what gets placed into $VIBE_OUT.\n\
         echo \"Building {name}\"\n"
    );
    let text = if cfg!(windows) {
        body
    } else {
        format!("#!/bin/sh\n{body}")
    };
    let recipe_outcome = ensure_file(ctx, project_root, &script_path, &text, "build recipe")?;
    if recipe_outcome.action == Action::Created {
        mark_executable(&script_path)?;
    }
    // The doc scaffold keeps its empty `media/` the same way, so git
    // tracks the directory the author is expected to fill.
    let icons = ensure_file(
        ctx,
        project_root,
        &pkg_dir.join("icons").join(".gitkeep"),
        "",
        "icon directory",
    )?;
    Ok(vec![recipe_outcome, icons])
}

/// The recipe runs as `/bin/sh <script>`, so the bit is not needed by
/// vibe; it lets the author run the script by hand while writing it.
#[cfg(unix)]
fn mark_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = std::fs::metadata(path)?.permissions();
    permissions.set_mode(permissions.mode() | 0o111);
    std::fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(not(unix))]
fn mark_executable(_path: &Path) -> Result<()> {
    Ok(())
}
