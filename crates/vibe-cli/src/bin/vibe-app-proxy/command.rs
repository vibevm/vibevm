//! Resolving a command on the caller's own PATH: what a `command:NAME`
//! launcher names (`PROP-065#launcher-command`), and what a proxy of a
//! switched-off slot hands the command to (`PROP-059#slot-off`).

use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};

use super::{Launch, strip_verbatim};

/// The prefix of a launcher `executable` naming a command resolved on PATH
/// at launch instead of a payload file (`PROP-065#launcher-command`); the
/// spelling of `vibe_core::manifest::LAUNCHER_COMMAND_PREFIX`.
const COMMAND_PREFIX: &str = "command:";

/// The default Windows `PATHEXT` when the environment carries none.
const DEFAULT_PATHEXT: &str = ".COM;.EXE;.BAT;.CMD";

/// The command a `command:NAME` executable names.
pub(super) fn launcher_command(executable: &str) -> Option<&str> {
    executable
        .strip_prefix(COMMAND_PREFIX)
        .filter(|name| !name.is_empty())
}

/// The `PATH` and `PATHEXT` a launcher command is looked up on, read once
/// at the process boundary.
pub(crate) struct CommandEnv {
    pub path: Option<OsString>,
    pub pathext: Option<OsString>,
}

impl CommandEnv {
    #[specmark::spec(
        deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
        reason = "A `command:NAME` launcher is resolved against the caller's own PATH (spec://org.vibevm.core/vibevm/common/PROP-065#launcher-command); the standalone proxy reads PATH and PATHEXT once at its process boundary and resolution receives them as values."
    )]
    pub(crate) fn from_env() -> Self {
        Self {
            path: std::env::var_os("PATH"),
            pathext: std::env::var_os("PATHEXT"),
        }
    }
}

pub(super) fn executable_file(candidate: &Path) -> bool {
    if !candidate.is_file() {
        return false;
    }
    #[cfg(unix)]
    let runnable = {
        use std::os::unix::fs::PermissionsExt;
        fs::metadata(candidate)
            .map(|meta| meta.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    };
    #[cfg(not(unix))]
    let runnable = true;
    runnable
}

/// The canonical form of `path`, or the path itself when it cannot be read.
pub(super) fn resolved_path(path: &Path) -> PathBuf {
    fs::canonicalize(path)
        .map(strip_verbatim)
        .unwrap_or_else(|_| path.to_path_buf())
}

/// The proxy directories a fall-through already passed through, a
/// `PATH`-separated list handed to the child so a chain of proxies for one
/// switched-off slot always walks forward along PATH and terminates
/// (`PROP-059#slot-off`). Internal plumbing between proxy images.
pub(crate) const SKIP_VARIABLE: &str = "VIBE_APP_PROXY_SKIP";

/// Hand `command` to the next same-named program on PATH after this proxy's
/// own directory: what a switched-off slot does, so the system program the
/// slot shadowed is reached again (`PROP-059#slot-off`). The proxy's own
/// directory and every directory an earlier hop already left are skipped,
/// and the list is passed on, so the walk cannot turn back.
#[specmark::spec(
    deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "A switched-off slot hands its command to the next one on the caller's own PATH (spec://org.vibevm.core/vibevm/common/PROP-059#slot-off); the standalone proxy reads PATH, PATHEXT and the walked-chain variable once at its process boundary and passes them on as values."
)]
pub(super) fn fall_through(exe: &Path, command: &str, slot: &str) -> Result<Launch, String> {
    let env = CommandEnv::from_env();
    let mut skipped: Vec<PathBuf> = std::env::var_os(SKIP_VARIABLE)
        .map(|value| {
            std::env::split_paths(&value)
                .map(|dir| resolved_path(&dir))
                .collect()
        })
        .unwrap_or_default();
    if let Some(dir) = exe.parent() {
        skipped.push(resolved_path(dir));
    }
    let names = command_candidates(command, env.pathext.as_deref());
    let own = resolved_path(exe);
    for directory in std::env::split_paths(env.path.as_deref().unwrap_or(OsStr::new(""))) {
        if directory.as_os_str().is_empty() {
            continue;
        }
        let resolved_dir = resolved_path(&directory);
        if skipped.contains(&resolved_dir) {
            continue;
        }
        for candidate in &names {
            let file = directory.join(candidate);
            if !executable_file(&file) {
                continue;
            }
            let file = resolved_path(&file);
            if file == own {
                continue;
            }
            skipped.push(resolved_dir);
            let skip = std::env::join_paths(&skipped)
                .map_err(|error| format!("composing {SKIP_VARIABLE}: {error}"))?;
            return Ok(Launch {
                executable: file,
                args: Vec::new(),
                working_directory: None,
                skip: Some(skip),
            });
        }
    }
    Err(format!(
        "slot `{slot}` is switched off and no other `{command}` is on PATH after `{}`; switch the \
         slot back on with `vibe app use {slot} --default`, or put a `{command}` on PATH",
        exe.parent().unwrap_or(exe).display()
    ))
}

/// `name` on `env`'s PATH, never choosing `own_image` — this very proxy —
/// so a proxy for `name` whose launcher runs `name` cannot re-enter itself.
/// Nothing else in its directory is skipped: the proxies of the other
/// installed applications live there, and a runtime a user installed by
/// name is reached through exactly those. The rules mirror
/// `vibe_app::CommandSearch`; `command_candidates` holds the `PATHEXT` half,
/// and the first executable file wins.
pub(crate) fn locate_command(
    name: &str,
    launcher: &str,
    env: &CommandEnv,
    own_image: Option<&Path>,
) -> Result<PathBuf, String> {
    let names = command_candidates(name, env.pathext.as_deref());
    let skipped = own_image.map(resolved_path);
    for directory in std::env::split_paths(env.path.as_deref().unwrap_or(OsStr::new(""))) {
        if directory.as_os_str().is_empty() {
            continue;
        }
        for candidate in &names {
            let file = directory.join(candidate);
            if !executable_file(&file) {
                continue;
            }
            let file = resolved_path(&file);
            if skipped.as_deref() == Some(file.as_path()) {
                continue;
            }
            return Ok(file);
        }
    }
    Err(format!(
        "command `{name}` is not on PATH, so launcher `{launcher}` cannot run; put a `{name}` on \
         PATH, or install an application package whose public commands include it \
         (`vibe install -g GROUP/NAME`)"
    ))
}

/// Off Windows the name itself; on Windows a name already ending in a
/// listed `PATHEXT` extension as it stands, and a bare name only with each
/// extension appended — never on its own, since Windows cannot start an
/// extensionless file (Node's payload holds `npm` beside `npm.cmd`).
fn command_candidates(name: &str, pathext: Option<&OsStr>) -> Vec<String> {
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
