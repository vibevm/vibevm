//! Grammar for `vibe prompt`: the explicit operations surface of the
//! cache-stable system-prompt prefix (PROP-076 `#SYS-OPERATIONS-SURFACE`,
//! `#SYS-OPERATIONS-PATH-AND-FLAGS`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-OPERATIONS-SURFACE");

use std::path::PathBuf;

/// Arguments for `vibe prompt`.
#[derive(Debug, clap::Args)]
pub struct PromptArgs {
    #[command(subcommand)]
    pub command: PromptCommand,
}

/// Which artifact `vibe prompt path` is asked about. There are two because
/// there are two artifacts: the lane is what a launch delivery points at,
/// and the system prefix is what an API caller sends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum PromptPart {
    /// `vibevm/vibespecs/boot/AGENT-LANE.md`.
    Lane,
    /// `vibevm/vibespecs/boot/SYSTEM.md`.
    System,
}

#[derive(Debug, clap::Subcommand)]
pub enum PromptCommand {
    /// Regenerate the node's prefix, sidecar and agent lane explicitly,
    /// through the one boot transaction. Ends with one summary line naming
    /// what happened and where.
    Build {
        /// The node, as a workspace-relative path. Default: the selected one.
        #[arg(long, value_name = "REL")]
        node: Option<String>,
        /// The project directory to work in.
        #[arg(long, default_value = ".")]
        path: PathBuf,
    },

    /// Compare the current inputs with the published artifacts, read-only.
    /// Writes nothing, opens no connection and invokes no compile provider.
    Check {
        #[arg(long, value_name = "REL")]
        node: Option<String>,
        #[arg(long, default_value = ".")]
        path: PathBuf,
        /// A model context window in bytes, against which the lane's size
        /// is reported. Absent reports the size and no verdict about it.
        #[arg(long, value_name = "BYTES")]
        model_window: Option<usize>,
    },

    /// Return one request's materials — bands P, U and G separately, with
    /// their observed digests and the recommended cache breakpoints. Never a
    /// ready-to-send request, and never a rebuild.
    Prepare {
        #[arg(long, value_name = "REL")]
        node: Option<String>,
        #[arg(long, default_value = ".")]
        path: PathBuf,
        /// The session's operating system, as `os=<name>`. A gated row is
        /// decided only against an explicitly supplied target; the
        /// generator's own operating system is never used.
        #[arg(long, value_name = "os=<name>")]
        session_target: Option<String>,
    },

    /// Print the absolute path of a generated artifact, with its size, hash
    /// and freshness. A stale or missing artifact is reported as stale or
    /// missing with the build that fixes it, never as an empty path.
    Path {
        /// Which artifact. Default: the lane.
        #[arg(long, value_enum, default_value = "lane")]
        part: PromptPart,
        #[arg(long, value_name = "REL")]
        node: Option<String>,
        #[arg(long, default_value = ".")]
        path: PathBuf,
    },

    /// Print ready-to-paste launch flags for a harness kind or a named
    /// launcher — flags only, never a program.
    Flags {
        /// A harness kind (`claude`, `codex`) or a launcher name.
        #[arg(long, value_name = "KIND-OR-LAUNCHER")]
        harness: String,
        #[arg(long, value_name = "REL")]
        node: Option<String>,
        #[arg(long, default_value = ".")]
        path: PathBuf,
    },
}
