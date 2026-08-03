//! Argument structs for `vibe workspace …` (PROP-007).
//!
//! Split from the `cli` hub along command-family lines; the hub
//! re-exports everything, so `crate::cli::X` paths are unchanged.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#command-summary");

use std::path::PathBuf;

use clap::Subcommand;

#[derive(Debug, clap::Args)]
pub struct WorkspaceArgs {
    #[command(subcommand)]
    pub command: WorkspaceSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum WorkspaceSubcommand {
    /// Publish the workspace's self-publishing members. Discovers the
    /// workspace enclosing the current directory, selects every node
    /// (root and members) carrying `[package]` whose `publish` posture
    /// does not exclude it, orders them dependency-first via inter-member
    /// `path` dependencies, and publishes each as its own repository in
    /// the workspace's primary `[[registry]]` org — reusing the same
    /// per-package machinery as `vibe registry publish`. Each published
    /// copy carries an `[origin]` provenance marker, a "generated copy"
    /// README banner, and a `.github/PULL_REQUEST_TEMPLATE.md` STOP
    /// notice. Publishing is **not atomic**: on the first failure the
    /// command stops and reports which nodes were already published and
    /// which remain (PROP-007 §2.7). Maintainers only — needs the same
    /// publish token used by `vibe registry publish`.
    Publish(WorkspacePublishArgs),
}

#[derive(Debug, clap::Args)]
pub struct WorkspacePublishArgs {
    /// Restrict the publish to a single workspace node by its
    /// root-relative path (`.` selects the workspace root, e.g.
    /// `packages/flow-wal` selects that member). When omitted, every
    /// self-publishing node is published. A node whose `publish`
    /// posture excludes it is reported as skipped even when named
    /// explicitly here.
    #[arg(long = "member")]
    pub member: Option<String>,

    /// Project directory to discover the workspace from. Discovery
    /// walks up to the enclosing `[workspace]`. Defaults to the
    /// current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// Describe what would be published — selection, dependency order,
    /// staged content — but make no API calls and push nothing. No
    /// repository is created.
    #[arg(long = "dry-run")]
    pub dry_run: bool,
}
