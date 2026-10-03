//! Explicit on-demand machine frame grammar.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-LOCAL-FRAME");

use clap::{Args, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Args)]
pub struct MachineArgs {
    #[command(subcommand)]
    pub command: MachineSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum MachineSubcommand {
    /// Exchange generated frames until stdin EOF; pure planner by default.
    Session(MachineSessionArgs),
}

#[derive(Debug, Args)]
pub struct MachineSessionArgs {
    /// Explicit absolute project directory containing vibe.toml.
    #[arg(long, value_parser = absolute_root, conflicts_with_all = ["json", "quiet", "verbose", "invoked_by"])]
    pub root: PathBuf,
    /// Explicit cooperative same-project fixed Validate host mode.
    #[arg(long, value_enum)]
    pub fixed_validate: Option<FixedValidateMode>,
    /// Explicit cooperative same-project fixed package host mode.
    #[arg(long, value_enum, conflicts_with = "fixed_validate")]
    pub fixed_package: Option<FixedPackageMode>,
    /// First-issue lifetime in milliseconds (1..=86400000); issue only.
    #[arg(long, value_parser = clap::value_parser!(u64).range(1..=86_400_000))]
    pub ttl_ms: Option<u64>,
    /// Permit the caller's exact approval of a captured Validate script.
    #[arg(long, requires = "fixed_validate")]
    pub approve_captured_script: bool,
    /// Permit a typed detach selection.
    #[arg(long, requires = "fixed_validate")]
    pub allow_detach: bool,
    /// Permit a typed cancellation before dispatch.
    #[arg(long, requires = "fixed_validate")]
    pub allow_cancel_pre_dispatch: bool,
    /// Permit a typed cancellation at a deferred boundary.
    #[arg(long, requires = "fixed_validate")]
    pub allow_cancel_deferred_boundary: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum FixedValidateMode {
    Issue,
    Reattach,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum FixedPackageMode {
    Issue,
    Reattach,
}

fn absolute_root(value: &str) -> Result<PathBuf, String> {
    let root = PathBuf::from(value);
    if !root.is_absolute() {
        return Err("violates REQ spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-TRUSTED-CONTEXT: root must be absolute; fix surface: --root <absolute-project-root>".to_owned());
    }
    Ok(root)
}
