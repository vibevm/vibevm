//! One error enum for the whole crate (campaign rule R-15): every message
//! names the `spec://` it answers to and says what to do next.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#SHELL-SERVE-SOURCES");

use std::path::{Path, PathBuf};

use specmark::spec;

/// What can go wrong between a shell's bytes and a reader that wants
/// them.
///
/// Opening a shell never appears here, and that is deliberate: there is
/// always an answer — the bare fallback — so [`crate::Shell::open`] is
/// infallible and a reader starts on a machine with nothing installed.
/// These are the failures of the surfaces that ASK about a shell: the
/// index that does not parse, the pin that does not, the store directory
/// that holds no shell at all.
#[derive(Debug, thiserror::Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-057#SHELL-PIN")]
pub enum ShellError {
    /// The shell's own index is unreadable.
    #[error(
        "the documentation shell at `{path}` {message} (violates \
         spec://org.vibevm.core/vibevm/common/PROP-057#SHELL-XTASK-EMBED; \
         fix: re-run `cargo xtask embed-doc-shell`, which writes the index)"
    )]
    Index { path: PathBuf, message: String },

    /// The pin beside `vibe` is unreadable.
    #[error(
        "the shell pin `doc-shell.lock` {message} (violates \
         spec://org.vibevm.core/vibevm/common/PROP-057#SHELL-PIN; \
         fix: repair the pin — it is an authored record, not a generated one, \
         and `cargo xtask embed-doc-shell` rewrites its `sha256`)"
    )]
    Pin { message: String },

    /// A store directory was named and holds no shell.
    #[error(
        "`{path}` holds no documentation shell: a shell directory carries `shell.json` and \
         `page-template.html` (violates \
         spec://org.vibevm.core/vibevm/common/PROP-057#SHELL-INSTALL-COMMAND; \
         fix: run `vibe doc shell install` to place one, or serve without it — the bare \
         fallback shell needs nothing)"
    )]
    NoShell { path: PathBuf },

    /// A path under the shell could not be read.
    #[error(
        "cannot read `{path}` of the documentation shell: {message} (violates \
         spec://org.vibevm.core/vibevm/common/PROP-057#SHELL-SERVE-SOURCES; \
         fix: re-place the shell — a half-readable one serves half a page)"
    )]
    Io { path: PathBuf, message: String },
}

impl ShellError {
    /// An unreadable path under the shell.
    pub fn io(path: &Path, error: &std::io::Error) -> ShellError {
        ShellError::Io {
            path: path.to_path_buf(),
            message: error.to_string(),
        }
    }
}

/// The crate's result.
pub type ShellResult<T> = Result<T, ShellError>;
