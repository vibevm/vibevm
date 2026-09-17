//! Composition-root inputs for documentation commands.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-LIBRARY");

use std::ffi::OsString;
use std::path::PathBuf;
use vibe_core::progress::Progress;

/// Ambient values resolved by the CLI and handed to the documentation layer.
#[derive(Debug, Clone, Default)]
pub struct DocEnv {
    /// Invocation-owned observation tree supplied by the CLI composition root.
    pub progress: Progress,
    /// `$VIBE_SETTINGS`, when the operator relocated the settings dir.
    pub settings: Option<OsString>,
    /// The operator's home, for the real `~/.vibe` the tripwire guards.
    pub home: Option<OsString>,
    /// The system temporary directory — where sandboxes go by default.
    pub temp: PathBuf,
    /// The working directory, which is the source tree during a panel run.
    pub cwd: Option<PathBuf>,
    /// The running binary: the default subject of every example.
    pub current_exe: Option<PathBuf>,
    /// The process id, so two runs on one machine cannot share a sandbox.
    pub pid: u32,
    /// `$VIBEVM_INSTALL_ROOT/opt`, else `~/.vibe/opt` — where a downloaded
    /// reader shell is kept.
    pub install_root: Option<PathBuf>,
    /// Has the invocation declared that nobody is at the keyboard?
    pub unattended: bool,
    /// Is the invocation printing a machine document?
    pub json: bool,
}
