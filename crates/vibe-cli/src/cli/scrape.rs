//! `vibe scrape` argument grammar (PROP-056).

use std::path::PathBuf;

use clap::{Args, Subcommand};

/// Plan, execute, recover, or manage the contract for a project scrape.
#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct ScrapeArgs {
    /// Render the complete deterministic plan without writing anything.
    #[arg(long, conflicts_with = "recover")]
    pub plan: bool,

    /// Create a scraped project at this new, absent directory.
    #[arg(long, value_name = "DIR", conflicts_with_all = ["in_place", "recover"])]
    pub output: Option<PathBuf>,

    /// Scrape the selected project through the recoverable in-place transaction.
    #[arg(long, conflicts_with_all = ["output", "recover"])]
    pub in_place: bool,

    /// Settle the pending user-local transaction for the selected project.
    #[arg(
        long,
        conflicts_with_all = ["plan", "output", "in_place", "contract", "assume_yes"]
    )]
    pub recover: bool,

    /// Read this contract instead of `vibevm/scrape/contract.toml`.
    #[arg(long, value_name = "FILE", conflicts_with = "recover")]
    pub contract: Option<PathBuf>,

    /// Project root to inspect or mutate. Recovery requires this explicitly.
    #[arg(long, value_name = "ROOT")]
    pub path: Option<PathBuf>,

    /// Explicitly authorize the destructive in-place transaction.
    #[arg(
        long,
        requires = "in_place",
        conflicts_with_all = ["plan", "recover"]
    )]
    pub assume_yes: bool,

    #[command(subcommand)]
    pub command: Option<ScrapeCommand>,
}

#[derive(Debug, Subcommand)]
pub enum ScrapeCommand {
    /// Create or validate the project-owned scrape contract.
    Contract(ScrapeContractArgs),
}

#[derive(Debug, Args)]
pub struct ScrapeContractArgs {
    #[command(subcommand)]
    pub command: ScrapeContractCommand,
}

#[derive(Debug, Subcommand)]
pub enum ScrapeContractCommand {
    /// Write the conservative default contract, refusing an existing file.
    Init(ScrapeContractInitArgs),

    /// Parse the contract and prepare its plan without writing anything.
    Check(ScrapeContractCheckArgs),
}

#[derive(Debug, Args)]
pub struct ScrapeContractInitArgs {
    /// Project root that will own the default contract.
    #[arg(long, value_name = "ROOT", default_value = ".")]
    pub path: PathBuf,
}

#[derive(Debug, Args)]
pub struct ScrapeContractCheckArgs {
    /// Project root whose contract is checked.
    #[arg(long, value_name = "ROOT", default_value = ".")]
    pub path: PathBuf,

    /// Read this contract instead of `vibevm/scrape/contract.toml`.
    #[arg(long, value_name = "FILE")]
    pub contract: Option<PathBuf>,
}
