//! Positive environment for Cargo commands that execute package-supplied code.
//!
//! Binary delivery and native cdylib builds have the same trust/root posture:
//! both execute Cargo inside a providing package and inherit only the process,
//! temporary-directory, locale, and Rust toolchain inputs they need.  Keeping
//! that policy here prevents the two callers from drifting, especially around
//! Windows linker and SDK discovery.

use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::process::Command;

/// The complete ambient environment admitted to package-supplied Cargo.
pub(crate) const CARGO_BUILD_ENV_KEYS: &[&str] = &[
    "PATH",
    "SystemRoot",
    "WINDIR",
    "HOME",
    "USERPROFILE",
    "TEMP",
    "TMP",
    "LANG",
    "LC_ALL",
    "CARGO_HOME",
    "RUSTUP_HOME",
    "RUSTUP_TOOLCHAIN",
];

#[derive(Clone, Debug, Default)]
pub(crate) struct CargoToolchainEnvironment {
    pub(crate) path_entries: Vec<PathBuf>,
    pub(crate) library_dirs: Vec<PathBuf>,
}

/// Construct a command with the positive package-build environment applied.
///
/// The program remains an argv executable, never a shell string. Callers add
/// arguments, cwd and stdio after construction.
#[must_use]
pub fn package_cargo_command(program: impl AsRef<OsStr>) -> Command {
    let parent = std::env::vars_os().collect::<Vec<_>>();
    let toolchain = system_cargo_toolchain(&parent);
    let mut command = Command::new(program);
    command
        .env_clear()
        .envs(cargo_build_environment(parent, &toolchain));
    command
}

pub(crate) fn cargo_build_environment(
    parent: impl IntoIterator<Item = (OsString, OsString)>,
    toolchain: &CargoToolchainEnvironment,
) -> Vec<(OsString, OsString)> {
    let parent = parent.into_iter().collect::<Vec<_>>();
    let mut environment = CARGO_BUILD_ENV_KEYS
        .iter()
        .filter_map(|allowed| {
            parent
                .iter()
                .find(|(key, _)| environment_key_matches(key, allowed))
                .map(|(_, value)| (OsString::from(allowed), value.clone()))
        })
        .collect::<Vec<_>>();
    extend_path(&mut environment, toolchain.path_entries.iter().cloned());
    if !toolchain.library_dirs.is_empty()
        && let Ok(libraries) = std::env::join_paths(&toolchain.library_dirs)
    {
        // MSVC's linker does not search PATH for the Windows SDK libraries.
        // This value is derived only from discovered toolchain directories;
        // an ambient parent LIB is never copied into package build code.
        environment.push(("LIB".into(), libraries));
    }
    environment
}

pub(crate) fn environment_key_matches(actual: &OsStr, allowed: &str) -> bool {
    if cfg!(windows) {
        actual.to_string_lossy().eq_ignore_ascii_case(allowed)
    } else {
        actual == OsStr::new(allowed)
    }
}

fn extend_path(
    environment: &mut [(OsString, OsString)],
    additions: impl IntoIterator<Item = PathBuf>,
) {
    let Some((_, path)) = environment
        .iter_mut()
        .find(|(key, _)| environment_key_matches(key, "PATH"))
    else {
        return;
    };
    // Toolchain executables win over same-named ambient utilities (Git for
    // Windows ships a POSIX `link.exe`). The inherited PATH remains fallback
    // so cargo/rustup stay discoverable.
    let inherited = std::env::split_paths(path).collect::<Vec<_>>();
    let mut entries = Vec::new();
    for addition in additions {
        if !entries.iter().any(|entry| entry == &addition) {
            entries.push(addition);
        }
    }
    for entry in inherited {
        if !entries.iter().any(|existing| existing == &entry) {
            entries.push(entry);
        }
    }
    if let Ok(joined) = std::env::join_paths(entries) {
        *path = joined;
    }
}

#[cfg(not(windows))]
pub(crate) fn system_cargo_toolchain(
    _parent: &[(OsString, OsString)],
) -> CargoToolchainEnvironment {
    CargoToolchainEnvironment::default()
}

#[cfg(windows)]
pub(crate) fn system_cargo_toolchain(parent: &[(OsString, OsString)]) -> CargoToolchainEnvironment {
    use std::sync::OnceLock;

    static DISCOVERED: OnceLock<CargoToolchainEnvironment> = OnceLock::new();
    DISCOVERED
        .get_or_init(|| discover_msvc_toolchain(parent))
        .clone()
}

#[cfg(windows)]
fn discover_msvc_toolchain(parent: &[(OsString, OsString)]) -> CargoToolchainEnvironment {
    let mut candidates = Vec::new();
    for key in ["ProgramFiles", "ProgramFiles(x86)", "ProgramW6432"] {
        let Some((_, value)) = parent
            .iter()
            .find(|(actual, _)| environment_key_matches(actual, key))
        else {
            continue;
        };
        collect_msvc_linker_dirs(
            &PathBuf::from(value).join("Microsoft Visual Studio"),
            &mut candidates,
        );
    }
    candidates.sort();
    candidates.dedup();
    let linker_dir = candidates
        .iter()
        .rev()
        .find(|path| path.ends_with("bin/Hostx64/x64"))
        .or_else(|| candidates.last())
        .cloned();
    let mut library_dirs = Vec::new();
    if let Some(linker_dir) = &linker_dir
        && let Some(msvc_version_root) = linker_dir.ancestors().nth(3)
    {
        let libraries = msvc_version_root.join("lib").join(msvc_architecture());
        if libraries.is_dir() {
            library_dirs.push(libraries);
        }
    }
    collect_windows_sdk_library_dirs(parent, &mut library_dirs);
    library_dirs.sort();
    library_dirs.dedup();
    CargoToolchainEnvironment {
        path_entries: linker_dir.into_iter().collect(),
        library_dirs,
    }
}

#[cfg(windows)]
fn collect_windows_sdk_library_dirs(
    parent: &[(OsString, OsString)],
    library_dirs: &mut Vec<PathBuf>,
) {
    for key in ["ProgramFiles(x86)", "ProgramFiles", "ProgramW6432"] {
        let Some((_, value)) = parent
            .iter()
            .find(|(actual, _)| environment_key_matches(actual, key))
        else {
            continue;
        };
        let root = PathBuf::from(value).join("Windows Kits/10/Lib");
        let Ok(entries) = std::fs::read_dir(root) else {
            continue;
        };
        let mut versions = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect::<Vec<_>>();
        versions.sort();
        let Some(latest) = versions.last() else {
            continue;
        };
        for family in ["ucrt", "um"] {
            let directory = latest.join(family).join(msvc_architecture());
            if directory.is_dir() {
                library_dirs.push(directory);
            }
        }
    }
}

#[cfg(windows)]
fn msvc_architecture() -> &'static str {
    if cfg!(target_arch = "x86_64") {
        "x64"
    } else if cfg!(target_arch = "x86") {
        "x86"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "arm"
    }
}

#[cfg(windows)]
fn collect_msvc_linker_dirs(root: &std::path::Path, candidates: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_msvc_linker_dirs(&path, candidates);
        } else if path
            .file_name()
            .is_some_and(|name| name.eq_ignore_ascii_case("link.exe"))
            && let Some(parent) = path.parent()
        {
            candidates.push(parent.to_path_buf());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_cargo_environment_is_positive_and_toolchain_first() {
        let temp = tempfile::tempdir().unwrap();
        let ambient = temp.path().join("ambient");
        let toolchain = temp.path().join("toolchain");
        let libraries = temp.path().join("libraries");
        let parent = vec![
            (
                OsString::from("PATH"),
                std::env::join_paths([ambient.clone()]).unwrap(),
            ),
            (OsString::from("TEMP"), OsString::from("allowed")),
            (OsString::from("RUSTFLAGS"), OsString::from("--cfg leaked")),
            (OsString::from("LIB"), OsString::from("ambient-library")),
            (
                OsString::from("CARGO_REGISTRIES_PRIVATE_TOKEN"),
                OsString::from("secret"),
            ),
        ];
        let environment = cargo_build_environment(
            parent,
            &CargoToolchainEnvironment {
                path_entries: vec![toolchain.clone()],
                library_dirs: vec![libraries.clone()],
            },
        );
        let names = environment
            .iter()
            .map(|(key, _)| key.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert_eq!(names, ["PATH", "TEMP", "LIB"]);
        let path = environment
            .iter()
            .find(|(key, _)| environment_key_matches(key, "PATH"))
            .unwrap();
        assert_eq!(
            std::env::split_paths(&path.1).collect::<Vec<_>>(),
            [toolchain, ambient]
        );
        let lib = environment.iter().find(|(key, _)| key == "LIB").unwrap();
        assert_eq!(
            std::env::split_paths(&lib.1).collect::<Vec<_>>(),
            [libraries]
        );
    }
}
