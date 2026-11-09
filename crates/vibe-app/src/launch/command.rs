//! The PATH lookup of a launcher that names a command instead of a file in
//! the installed payload (`PROP-065#launcher-command`).
//!
//! `vibe-app-proxy` mirrors these rules in
//! `crates/vibe-cli/src/bin/vibe-app-proxy/resolve.rs`; the proxy depends on
//! no vibe crate, so the two implementations stay separate and must stay in
//! agreement.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#launcher-command");

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use crate::resolve::strip_verbatim;

/// The default Windows `PATHEXT` when the environment carries none.
pub(crate) const DEFAULT_PATHEXT: &str = ".COM;.EXE;.BAT;.CMD";

/// Where a launcher command is looked up: the `PATH` to walk, the `PATHEXT`
/// Windows appends, and one file that is never chosen — the running proxy's
/// own image, so a proxy for `NAME` whose launcher runs `NAME` never
/// re-enters itself. Only that one file is skipped: the proxies of the
/// other installed applications live in the same directory, and a managed
/// runtime installed by name is reached through exactly those.
///
/// A search built by hand reads no environment at all — the directories to
/// walk are data:
///
/// ```
/// use std::ffi::OsString;
///
/// use vibe_app::CommandSearch;
///
/// let search = CommandSearch {
///     path: Some(OsString::from("/vibe-no-such-directory")),
///     ..CommandSearch::default()
/// };
/// assert_eq!(
///     search.locate("vibe-no-such-command").unwrap_err(),
///     "command `vibe-no-such-command` is not on PATH"
/// );
/// ```
#[derive(Debug, Clone, Default)]
pub struct CommandSearch {
    pub path: Option<OsString>,
    pub pathext: Option<OsString>,
    pub exclude: Option<PathBuf>,
}

impl CommandSearch {
    /// The search this process's environment describes, skipping no file.
    #[specmark::spec(
        deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
        reason = "A `command:NAME` launcher is resolved against the caller's own PATH, which is the fact being read (spec://org.vibevm.core/vibevm/common/PROP-065#launcher-command); PATH and PATHEXT are read once here and resolution receives them as values, so a caller with its own search builds `CommandSearch` directly."
    )]
    pub fn from_env() -> Self {
        Self {
            path: std::env::var_os("PATH"),
            pathext: std::env::var_os("PATHEXT"),
            exclude: None,
        }
    }

    /// The same search with the file `image` never chosen.
    pub fn excluding(mut self, image: &Path) -> Self {
        self.exclude = Some(image.to_path_buf());
        self
    }

    /// The absolute, canonical path of `name` on this search's `PATH`, or
    /// why there is none.
    pub fn locate(&self, name: &str) -> Result<PathBuf, String> {
        locate(
            name,
            self.path.as_deref(),
            self.pathext.as_deref(),
            self.exclude.as_deref(),
        )
        .ok_or_else(|| format!("command `{name}` is not on PATH"))
    }
}

/// The file names tried for `name`, in order. Off Windows that is the name
/// itself. On Windows a name that already ends in a listed `PATHEXT`
/// extension is tried as it stands, and a bare name is tried only with each
/// extension appended — never on its own, because Windows cannot start an
/// extensionless file, and a payload directory on PATH may well hold one
/// (Node's own `npm` beside `npm.cmd`).
fn candidates(name: &str, pathext: Option<&OsStr>) -> Vec<String> {
    if !cfg!(windows) {
        return vec![name.to_owned()];
    }
    let extensions = pathext
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| DEFAULT_PATHEXT.to_owned());
    let listed: Vec<&str> = extensions
        .split(';')
        .map(str::trim)
        .filter(|extension| extension.len() > 1 && extension.starts_with('.'))
        .collect();
    let lower = name.to_ascii_lowercase();
    if listed
        .iter()
        .any(|extension| lower.ends_with(&extension.to_ascii_lowercase()))
    {
        return vec![name.to_owned()];
    }
    listed
        .into_iter()
        .map(|extension| format!("{name}{extension}"))
        .collect()
}

/// Whether `candidate` is a file this platform can execute.
fn executable_file(candidate: &Path) -> bool {
    if !candidate.is_file() {
        return false;
    }
    #[cfg(unix)]
    let runnable = {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(candidate)
            .map(|meta| meta.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    };
    #[cfg(not(unix))]
    let runnable = true;
    runnable
}

/// The canonical form of `path`, or the path itself when it cannot be read.
fn resolved(path: &Path) -> PathBuf {
    std::fs::canonicalize(path)
        .map(strip_verbatim)
        .unwrap_or_else(|_| path.to_path_buf())
}

fn locate(
    name: &str,
    path: Option<&OsStr>,
    pathext: Option<&OsStr>,
    exclude: Option<&Path>,
) -> Option<PathBuf> {
    let names = candidates(name, pathext);
    let skipped = exclude.map(resolved);
    for directory in std::env::split_paths(path.unwrap_or(OsStr::new(""))) {
        if directory.as_os_str().is_empty() {
            continue;
        }
        for candidate in &names {
            let file = directory.join(candidate);
            if !executable_file(&file) {
                continue;
            }
            let file = resolved(&file);
            if skipped.as_deref() == Some(file.as_path()) {
                continue;
            }
            return Some(file);
        }
    }
    None
}

#[cfg(all(test, unix))]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    fn script(dir: &Path, name: &str, mode: u32) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, "#!/bin/sh\nexit 0\n").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
        path
    }

    fn search(dirs: &[&Path]) -> CommandSearch {
        CommandSearch {
            path: Some(std::env::join_paths(dirs).unwrap()),
            pathext: None,
            exclude: None,
        }
    }

    #[test]
    fn the_first_executable_entry_wins_and_the_excluded_image_is_skipped() {
        let home = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(home.path()).unwrap();
        let (own, other) = (root.join("own"), root.join("other"));
        fs::create_dir_all(&own).unwrap();
        fs::create_dir_all(&other).unwrap();
        let image = script(&own, "node", 0o755);
        let real = script(&other, "node", 0o755);
        let search = search(&[&own, &other]);

        assert_eq!(search.locate("node").unwrap(), image);
        assert_eq!(
            search.clone().excluding(&image).locate("node").unwrap(),
            real
        );
        assert_eq!(
            search.locate("npm").unwrap_err(),
            "command `npm` is not on PATH"
        );
    }

    #[test]
    fn a_sibling_in_the_excluded_image_s_own_directory_still_wins() {
        // The proxies of every installed application share one directory:
        // only the running image is skipped, so a managed runtime's own
        // proxy beside it is exactly what a `command:NAME` launcher finds.
        let home = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(home.path()).unwrap();
        let image = script(&root, "codkit", 0o755);
        let node = script(&root, "node", 0o755);
        let search = search(&[&root]).excluding(&image);
        assert_eq!(search.locate("node").unwrap(), node);
        assert!(search.locate("codkit").is_err());
    }

    #[test]
    fn a_file_without_the_executable_bit_is_not_a_command() {
        let home = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(home.path()).unwrap();
        script(&root, "node", 0o644);
        assert!(search(&[&root]).locate("node").is_err());
    }

    #[test]
    fn the_bare_name_is_the_only_candidate_off_windows() {
        // On Windows the same input yields `node.EXE` / `node.CMD` and never
        // the bare `node`; that half is covered by the CLI fixture there.
        assert_eq!(candidates("node", Some(OsStr::new(".EXE;.CMD"))), ["node"]);
        assert_eq!(candidates("npm.cmd", None), ["npm.cmd"]);
    }
}
