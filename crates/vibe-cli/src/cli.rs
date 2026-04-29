//! Command-line argument schema.
//!
//! Spec: `VIBEVM-SPEC.md` §9.1.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "vibe",
    version = env!("CARGO_PKG_VERSION"),
    about = "The disciplined runtime for spec-driven vibecoding.",
    long_about = "vibevm: a CLI software project manager for spec-driven AI-assisted development.\n\
                  Manages installable building blocks — flows, feats, stacks, tools — and assembles\n\
                  them into project-level spec content that AI agents read at session boot."
)]
pub struct Cli {
    /// Produce machine-readable JSON output.
    #[arg(long, global = true)]
    pub json: bool,

    /// Reduce output to a single summary line (useful in scripts / CI).
    #[arg(long, global = true, conflicts_with = "json")]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Scaffold a new vibevm project in the target directory.
    Init(InitArgs),

    /// List the packages recorded in the project's lockfile.
    List(ListArgs),

    /// Install one or more packages into the current project.
    Install(InstallArgs),

    /// Remove an installed package from the current project.
    Uninstall(UninstallArgs),

    /// Manage the registry cache (clone, sync).
    Registry(RegistryArgs),

    /// Print version information.
    Version,
}

#[derive(Debug, clap::Args)]
pub struct RegistryArgs {
    #[command(subcommand)]
    pub command: RegistrySubcommand,
}

#[derive(Debug, Subcommand)]
pub enum RegistrySubcommand {
    /// Force a `git fetch` on the configured registry cache.
    Sync(RegistrySyncArgs),

    /// Publish a package directory as a tagged release in the
    /// configured registry organization. Maintainers only — needs a
    /// publish token (see RUNTIME-GUIDE.md).
    Publish(RegistryPublishArgs),
}

#[derive(Debug, clap::Args)]
pub struct RegistrySyncArgs {
    /// Directory of the project (defaults to current).
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}

#[derive(Debug, clap::Args)]
pub struct RegistryPublishArgs {
    /// Path to the package directory (containing `vibe-package.toml`).
    #[arg(required = true)]
    pub source: PathBuf,

    /// Name of the `[[registry]]` to publish into. Defaults to the
    /// first registry in `vibe.toml`.
    #[arg(long = "registry")]
    pub registry: Option<String>,

    /// Project root with `vibe.toml`. Defaults to current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// Describe what would happen but make no API calls or pushes.
    #[arg(long = "dry-run")]
    pub dry_run: bool,
}

#[derive(Debug, clap::Args)]
pub struct InitArgs {
    /// Directory to initialize (defaults to the current working directory).
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// Pre-set the active stack name (still requires installation separately).
    #[arg(long)]
    pub stack: Option<String>,

    /// Project name; defaults to the basename of the target directory.
    #[arg(long)]
    pub name: Option<String>,

    /// Override the default registry URL written into `vibe.toml`.
    /// When unset, `vibe init` writes the public `vibespecs`
    /// organization on GitHub (`https://github.com/vibespecs`).
    /// Conflicts with `--no-registry`.
    #[arg(long = "registry-url", conflicts_with = "no_registry")]
    pub registry_url: Option<String>,

    /// Override the default ref (`main`) recorded under `[registry]`.
    /// Conflicts with `--no-registry`.
    #[arg(long = "registry-ref", conflicts_with = "no_registry")]
    pub registry_ref: Option<String>,

    /// Do not write a `[registry]` section into `vibe.toml`. The
    /// project will then require `--registry <path>` on every
    /// `vibe install`, or a manual edit to `vibe.toml` later.
    #[arg(long = "no-registry")]
    pub no_registry: bool,
}

#[derive(Debug, clap::Args)]
pub struct ListArgs {
    /// Filter by package kind (flow, feat, stack, tool).
    #[arg(long)]
    pub kind: Option<String>,

    /// Directory of the project (defaults to current).
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}

#[derive(Debug, clap::Args)]
pub struct InstallArgs {
    /// One or more package references, each `<kind>:<name>[@<version>]`.
    #[arg(required = true)]
    pub packages: Vec<String>,

    /// Directory of the project (defaults to current).
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// Path to a local-directory registry (M0 only; M1 adds git registry).
    #[arg(long)]
    pub registry: Option<PathBuf>,

    /// Skip the interactive confirmation prompt (non-interactive envs).
    #[arg(long, alias = "yes")]
    pub assume_yes: bool,
}

#[derive(Debug, clap::Args)]
pub struct UninstallArgs {
    /// Package reference `<kind>:<name>` (version is ignored on uninstall).
    pub package: String,

    /// Directory of the project (defaults to current).
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// Skip the interactive confirmation prompt.
    #[arg(long, alias = "yes")]
    pub assume_yes: bool,
}
