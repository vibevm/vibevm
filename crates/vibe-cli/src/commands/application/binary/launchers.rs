//! Platform launcher wrappers for immutable application generations.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#binary-ownership");

use super::*;

pub(super) fn direct_launcher_body(runner: &Path, os: &str) -> Result<Vec<u8>> {
    if os == "windows" {
        bail!("direct archive Windows runner requires a literal-argument native launcher");
    }
    Ok(format!("#!/bin/sh\nexec {} \"$@\"\n", sh_quote(runner)).into_bytes())
}

pub(super) fn stable_launcher_name_for(bundle_entry: &Path, os: &str) -> &'static str {
    let extension = bundle_entry
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase);
    if os == "windows" {
        match extension.as_deref() {
            Some("ps1") => "launch.ps1",
            None | Some("cmd" | "bat" | "exe") => "launch.cmd",
            _ => "launch.sh",
        }
    } else {
        "launch.sh"
    }
}

pub(super) fn stable_launcher_body(bundle_entry: &Path, os: &str) -> Vec<u8> {
    match stable_launcher_name_for(bundle_entry, os) {
        "launch.cmd" => format!(
            "@echo off\r\nsetlocal DisableDelayedExpansion\r\n\"{}\" %*\r\n",
            bundle_entry.display().to_string().replace('%', "%%"),
        ).into_bytes(),
        "launch.ps1" => format!(
            "$ErrorActionPreference = 'Stop'\r\n$global:LASTEXITCODE = 0\r\n& '{}' @args\r\nexit $LASTEXITCODE\r\n",
            bundle_entry.to_string_lossy().replace('\'', "''"),
        ).into_bytes(),
        _ => format!("#!/bin/sh\nexec {} \"$@\"\n", sh_quote(bundle_entry)).into_bytes(),
    }
}

fn sh_quote(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', "'\\''"))
}

#[cfg(unix)]
pub(super) fn make_launchers_executable<'a>(paths: impl Iterator<Item = &'a Path>) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    for path in paths {
        let mut permissions = fs::metadata(path)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions)?;
    }
    Ok(())
}

#[cfg(not(unix))]
pub(super) fn make_launchers_executable<'a>(_paths: impl Iterator<Item = &'a Path>) -> Result<()> {
    Ok(())
}
