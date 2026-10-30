//! The child shell: the caller's parent process when it is a recognised
//! interactive shell, otherwise the fixed platform fallback.

use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

#[cfg(target_os = "linux")]
use linux as native;
#[cfg(target_os = "macos")]
use macos as native;
#[cfg(windows)]
use windows as native;

/// The executable `vibe shell` starts.
pub(super) fn shell() -> io::Result<PathBuf> {
    Ok(select(native::parent_image(), native::fallback()?))
}

/// The observed parent image when it is an absolute path to a recognised
/// shell, otherwise the fallback.
fn select(parent: Option<PathBuf>, fallback: PathBuf) -> PathBuf {
    match parent {
        Some(image) if image.is_absolute() && recognised(&image) => image,
        _ => fallback,
    }
}

fn recognised(image: &Path) -> bool {
    let Some(name) = image.file_name().and_then(OsStr::to_str) else {
        return false;
    };
    if cfg!(windows) {
        ["cmd.exe", "powershell.exe", "pwsh.exe"]
            .iter()
            .any(|shell| name.eq_ignore_ascii_case(shell))
    } else {
        matches!(name, "bash" | "zsh")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_recognised_absolute_shells_are_selected() {
        let fallback = PathBuf::from(if cfg!(windows) {
            r"C:\Windows\System32\cmd.exe"
        } else {
            "/bin/sh"
        });
        for image in [None, Some("bash"), Some("/usr/bin/python3")] {
            let chosen = select(image.map(PathBuf::from), fallback.clone());
            assert_eq!(chosen, fallback, "{image:?}");
        }
        let shells: &[&str] = if cfg!(windows) {
            &[
                r"C:\Windows\System32\CMD.EXE",
                r"C:\Program Files\PowerShell\7\pwsh.exe",
            ]
        } else {
            &["/usr/bin/bash", "/bin/zsh"]
        };
        for shell in shells {
            assert_eq!(
                select(Some(PathBuf::from(shell)), fallback.clone()),
                PathBuf::from(shell)
            );
        }
    }
}
