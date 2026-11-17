//! Argument structs for `vibe user …` (PROP-074 `#USER-PROJECT-ROOT-GRAMMAR`).
//!
//! The grammar is deliberately flag-free: both verbs take the existing global
//! output and cancellation options only. The settings authority is the sole
//! root selector, so there is no positional, no `--path`, no `-g` and no
//! `--force` — a flag that only re-asserts the one choice is surface without
//! a question behind it.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-074#USER-PROJECT-ROOT-GRAMMAR");

use clap::Subcommand;

#[derive(Debug, clap::Args)]
pub struct UserArgs {
    #[command(subcommand)]
    pub command: UserCommand,
}

#[derive(Debug, Subcommand)]
pub enum UserCommand {
    /// Create the personal user project (PROP-074) at the selected settings
    /// root's `user-project`, outside any repository. An absent path or an
    /// existing empty directory is initialized; an existing valid project is
    /// left untouched. No launcher, Git, lockfile or boot output is produced.
    Init,

    /// Report, read-only, whether the personal user project is absent,
    /// initialized or unavailable. Never creates anything.
    Status,

    /// Manage an explicitly bound per-project personal override.
    Override(UserOverrideArgs),
    /// Enable the personal layer for the selected project, without changing override state.
    Enable {
        #[arg(long, default_value = ".")]
        path: std::path::PathBuf,
    },
    /// Disable personal loading for this project, preserving authored records.
    Disable {
        #[arg(long, default_value = ".")]
        path: std::path::PathBuf,
    },
}

#[derive(Debug, clap::Args)]
pub struct UserOverrideArgs {
    #[command(subcommand)]
    pub command: UserOverrideCommand,
}

#[derive(Debug, Subcommand)]
pub enum UserOverrideCommand {
    /// Create an enabled override and bind the selected project.
    Init {
        #[arg(long)]
        alias: String,
        #[arg(long, default_value = ".")]
        path: std::path::PathBuf,
    },
    /// Explicitly attach another checkout to an existing override alias.
    Bind {
        #[arg(long)]
        alias: String,
        #[arg(long, default_value = ".")]
        path: std::path::PathBuf,
    },
    /// Observe the selected project's binding without writes.
    Status {
        #[arg(long, default_value = ".")]
        path: std::path::PathBuf,
    },
    /// Enable an existing valid override for the next session.
    Enable {
        #[arg(long, default_value = ".")]
        path: std::path::PathBuf,
    },
    /// Disable loading without removing any authored files.
    Disable {
        #[arg(long, default_value = ".")]
        path: std::path::PathBuf,
    },
}
