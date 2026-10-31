//! System programs Windows keeps at fixed places under `%SystemRoot%`
//! (`PROP-065#execution`): `cmd.exe` and Windows PowerShell are addressed
//! by their absolute path, so a child that runs under a reduced PATH — a
//! build step exposing only its requirements, a service, a scripted
//! session — still finds them. The bare name stays the fallback when the
//! expected file is not there.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#execution");

use std::path::{Path, PathBuf};

/// `cmd.exe` of the running Windows: `%SystemRoot%\System32\cmd.exe` when
/// it exists, otherwise the bare name for PATH lookup.
pub fn windows_cmd() -> PathBuf {
    windows_system_program(r"System32\cmd.exe", "cmd.exe")
}

/// Windows PowerShell 5.1 of the running Windows:
/// `%SystemRoot%\System32\WindowsPowerShell\v1.0\powershell.exe` when it
/// exists, otherwise the bare name for PATH lookup.
pub fn windows_powershell() -> PathBuf {
    windows_system_program(
        r"System32\WindowsPowerShell\v1.0\powershell.exe",
        "powershell.exe",
    )
}

/// The program at `relative` under `%SystemRoot%`, or `fallback` when the
/// variable is unset or the file is missing.
pub fn windows_system_program(relative: &str, fallback: &str) -> PathBuf {
    let root = std::env::var_os("SystemRoot").map(PathBuf::from);
    resolve_in(root.as_deref(), relative, fallback, |path| path.is_file())
}

/// The pure choice behind [`windows_system_program`]: `root/relative` when
/// `exists` says so, otherwise `fallback`.
pub(crate) fn resolve_in(
    root: Option<&Path>,
    relative: &str,
    fallback: &str,
    exists: impl Fn(&Path) -> bool,
) -> PathBuf {
    match root {
        Some(root) => {
            let candidate = root.join(relative);
            if exists(&candidate) {
                candidate
            } else {
                PathBuf::from(fallback)
            }
        }
        None => PathBuf::from(fallback),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_absolute_path_wins_when_the_file_exists() {
        let root = Path::new(r"C:\Windows");
        let expected = root.join(r"System32\cmd.exe");
        let chosen = resolve_in(Some(root), r"System32\cmd.exe", "cmd.exe", |path| {
            path == expected
        });
        assert_eq!(chosen, expected);
    }

    #[test]
    fn a_missing_file_or_root_falls_back_to_the_bare_name() {
        let missing = resolve_in(
            Some(Path::new(r"C:\Windows")),
            r"System32\WindowsPowerShell\v1.0\powershell.exe",
            "powershell.exe",
            |_| false,
        );
        assert_eq!(missing, PathBuf::from("powershell.exe"));
        let unset = resolve_in(None, r"System32\cmd.exe", "cmd.exe", |_| true);
        assert_eq!(unset, PathBuf::from("cmd.exe"));
    }
}
