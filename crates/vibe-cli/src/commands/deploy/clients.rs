//! §6.3.0.6's ONE client-executable resolver — the only place in this
//! process that searches an executable search path.
//!
//! > "The CLI surface resolves them once; every lower cell and provider is
//! > forbidden from calling `dirs::home_dir`, reading
//! > `HOME`/`USERPROFILE`/`CODEX_HOME`/`CLAUDE_CONFIG_DIR`, searching
//! > `PATH`, or finding a real client."
//!
//! Its own cell for two reasons. It is the sanctioned ambient read (this
//! file is a recorded `env_roots` entry, the same standing `commands/term.rs`
//! has for locating a shell), and the resolution itself is PURE: [`locate`]
//! takes the search directories and the executable extensions as
//! parameters, so the law it implements is testable over injected
//! directories rather than over whatever this machine happens to have
//! installed.
//!
//! **What "resolved" means, exactly.** A bare command word is not a
//! resolution. `Command::new("claude")` searches `PATH` *at spawn time, in
//! the provider* — the very lookup the surface exists to have already done,
//! moved somewhere a test cannot see it and an operator cannot predict it.
//! So [`locate`] answers with an ABSOLUTE path or with
//! [`ClientExecutable::Missing`], and never with a relative word.
//!
//! **A missing client is not a run-time failure here.** Three eager
//! refusals at the surface would make `vibe deploy` on an ordinary
//! `deploy:vibe-bin` profile depend on three unrelated CLIs being
//! installed. The typed `Missing` value travels down instead, and the
//! provider that actually selected that client refuses with remediation
//! naming the command word.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS");

use std::path::{Path, PathBuf};

use specmark::spec;
use vibe_lifecycle::{ClientExecutable, ClientExecutables};

/// The three command words §6.3 names, spelled once.
const CLAUDE: &str = "claude";
const CODEX: &str = "codex";
const OPENCODE: &str = "opencode";

/// Resolve all three clients against this process's ambient search path.
///
/// The ONE ambient read of this surface, and the only one in the deploy
/// command family: everything below receives the resolved value.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
pub(crate) fn resolve_clients() -> ClientExecutables {
    let search = search_path();
    let extensions = executable_extensions();
    ClientExecutables {
        claude: locate(CLAUDE, &search, &extensions),
        codex: locate(CODEX, &search, &extensions),
        opencode: locate(OPENCODE, &search, &extensions),
    }
}

/// Find one command word in the given search directories — PURE.
///
/// The rules, in order, and each of them is a decision rather than an
/// accident:
///
/// 1. a search directory that is not ABSOLUTE is skipped. A relative `PATH`
///    entry resolves against the current working directory, so honouring
///    one would make the answer depend on where `vibe` was invoked — the
///    ambient dependency this whole value exists to remove;
/// 2. the extensions are tried in the given order, and the BARE name is
///    tried first. On Windows `PATHEXT` supplies `.COM;.EXE;.BAT;…` and an
///    extensionless file is still executable when named exactly; on Unix
///    the extension list is empty and only the bare name is tried;
/// 3. the first entry that is an executable FILE wins. Directory entries
///    and, on Unix, files without any execute bit are skipped rather than
///    refused: neither is a client, and a later search directory may still
///    hold the real one.
///
/// Symlinks are followed by `is_file`, deliberately: a client installed as
/// a symlink into a version store is the ordinary shape on this machine.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
pub(crate) fn locate(command: &str, search: &[PathBuf], extensions: &[String]) -> ClientExecutable {
    locate_with(command, search, extensions, is_executable_file)
}

/// Pure resolver core with the platform's executability predicate injected.
fn locate_with(
    command: &str,
    search: &[PathBuf],
    extensions: &[String],
    executable: impl Fn(&Path) -> bool,
) -> ClientExecutable {
    for directory in search {
        if !directory.is_absolute() {
            continue;
        }
        if let Some(path) = first_file(directory, command, extensions, &executable) {
            return ClientExecutable::Resolved {
                command: command.to_owned(),
                path,
            };
        }
    }
    ClientExecutable::Missing {
        command: command.to_owned(),
    }
}

/// The first readable file this directory holds for one command word.
fn first_file(
    directory: &Path,
    command: &str,
    extensions: &[String],
    executable: &impl Fn(&Path) -> bool,
) -> Option<PathBuf> {
    let bare = directory.join(command);
    if executable(&bare) {
        return Some(bare);
    }
    for extension in extensions {
        let candidate = directory.join(format!("{command}{extension}"));
        if executable(&candidate) {
            return Some(candidate);
        }
    }
    None
}

/// Whether one PATH candidate is something this platform may execute.
fn is_executable_file(path: &Path) -> bool {
    let Ok(metadata) = path.metadata() else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;

        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

/// This process's `PATH`, split by the platform separator.
fn search_path() -> Vec<PathBuf> {
    std::env::var_os("PATH")
        .map(|raw| std::env::split_paths(&raw).collect())
        .unwrap_or_default()
}

/// The executable extensions this platform recognises.
///
/// `PATHEXT` on Windows, empty everywhere else. It is read here rather than
/// assumed because an operator can extend it, and a client installed as a
/// `.cmd` shim — which is exactly how npm-installed CLIs land on Windows —
/// is only findable through it.
fn executable_extensions() -> Vec<String> {
    if !cfg!(windows) {
        return Vec::new();
    }
    let raw = std::env::var_os("PATHEXT")
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| ".COM;.EXE;.BAT;.CMD".to_owned());
    raw.split(';')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
#[path = "clients/tests.rs"]
mod tests;
