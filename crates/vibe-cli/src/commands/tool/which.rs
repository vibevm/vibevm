//! Resolving a bare command name on `PATH` the way the operating system
//! does (`PROP-068#proxy-tool`), for `vibe tool px exec <command>`.
//!
//! On Unix that is the `PATH` entries in order. On Windows it is the
//! `PATH` entries with `PATHEXT` applied: a name that already ends in a
//! listed extension is tried as it stands, and otherwise each extension
//! is appended — never the bare name, because Windows cannot start an
//! extensionless file and a payload directory on `PATH` may hold one
//! (Node ships `npm` beside `npm.cmd`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-068#proxy-tool");

use std::path::{Path, PathBuf};

/// The extensions Windows tries when the environment names none.
pub(crate) const DEFAULT_PATHEXT: &str = ".COM;.EXE;.BAT;.CMD";

/// The names to look for, in order, for the command `name` under
/// `pathext` (ignored off Windows).
///
/// ```ignore
/// // Windows: a name without a listed extension is never tried bare.
/// assert_eq!(candidates("npm", Some(".EXE;.CMD")), ["npm.EXE", "npm.CMD"]);
/// // One that already carries a listed extension is tried as it stands.
/// assert_eq!(candidates("npm.cmd", Some(".EXE;.CMD")), ["npm.cmd"]);
/// ```
pub(crate) fn candidates(name: &str, pathext: Option<&str>) -> Vec<String> {
    if !cfg!(windows) {
        return vec![name.to_owned()];
    }
    let pathext = pathext
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .unwrap_or(DEFAULT_PATHEXT);
    let extensions: Vec<&str> = pathext
        .split(';')
        .map(str::trim)
        .filter(|extension| extension.starts_with('.') && extension.len() > 1)
        .collect();
    if extensions.iter().any(|extension| {
        name.to_ascii_lowercase()
            .ends_with(&extension.to_ascii_lowercase())
    }) {
        return vec![name.to_owned()];
    }
    extensions
        .iter()
        .map(|extension| format!("{name}{extension}"))
        .collect()
}

/// Whether `name` is a bare command name rather than a path: a path is
/// run as written and never looked for on `PATH`.
pub(crate) fn is_bare(name: &str) -> bool {
    !name.contains('/') && !name.contains('\\') && Path::new(name).components().count() == 1
}

/// The file `name` resolves to under `path` and `pathext`, or `None`.
/// A name that is already a path is returned as it stands.
pub(crate) fn locate(name: &str, path: Option<&str>, pathext: Option<&str>) -> Option<PathBuf> {
    if !is_bare(name) {
        return Some(PathBuf::from(name));
    }
    let names = candidates(name, pathext);
    for dir in std::env::split_paths(path.unwrap_or_default()) {
        if dir.as_os_str().is_empty() {
            continue;
        }
        for candidate in &names {
            let full = dir.join(candidate);
            if full.is_file() {
                return Some(full);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_is_not_looked_for_on_path() {
        assert!(!is_bare("./node"));
        assert!(!is_bare(r"C:\tools\node.exe"));
        assert!(is_bare("node"));
        assert_eq!(
            locate("./node", Some("/nowhere"), None),
            Some(PathBuf::from("./node"))
        );
    }

    #[test]
    fn windows_applies_pathext_and_never_the_bare_name() {
        if !cfg!(windows) {
            assert_eq!(candidates("npm", Some(".EXE;.CMD")), ["npm"]);
            return;
        }
        assert_eq!(candidates("npm", Some(".EXE;.CMD")), ["npm.EXE", "npm.CMD"]);
        // A listed extension already on the name: tried as it stands.
        assert_eq!(candidates("npm.cmd", Some(".EXE;.CMD")), ["npm.cmd"]);
        assert_eq!(candidates("NPM.CMD", Some(".exe;.cmd")), ["NPM.CMD"]);
        // One that is not listed is a name, not an extension.
        assert_eq!(candidates("a.b", Some(".EXE")), ["a.b.EXE"]);
        // No PATHEXT in the environment: the four Windows always tries.
        assert_eq!(candidates("x", None), ["x.COM", "x.EXE", "x.BAT", "x.CMD"]);
        // Entries that are not extensions are ignored, never widened.
        assert_eq!(candidates("x", Some(".EXE; ;junk;.")), ["x.EXE"]);
    }

    #[test]
    fn the_first_path_entry_that_holds_the_file_wins() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        let name = if cfg!(windows) { "tool.cmd" } else { "tool" };
        std::fs::write(second.path().join(name), "x").unwrap();
        let path = std::env::join_paths([first.path(), second.path()]).unwrap();
        let path = path.to_str().unwrap();
        assert_eq!(
            locate("tool", Some(path), Some(".CMD")),
            Some(second.path().join(name))
        );
        // An empty entry is skipped rather than read as the current
        // directory, which is not on PATH on either platform.
        assert_eq!(locate("tool", Some(""), Some(".CMD")), None);
        assert_eq!(locate("absent", Some(path), Some(".CMD")), None);
    }
}
