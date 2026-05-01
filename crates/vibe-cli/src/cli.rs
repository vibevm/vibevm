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

    /// Print the project's configured `[[registry]]` / `[[mirror]]` /
    /// `[[override]]` entries and the host adapter each registry will
    /// dispatch to.
    List(RegistryListArgs),

    /// Add a new `[[registry]]` block to `vibe.toml`.
    Add(RegistryAddArgs),

    /// Add a `[[mirror]]` block targeting a registry (or `*` for any).
    SetMirror(RegistrySetMirrorArgs),

    /// Remove a `[[registry]]` or `[[mirror]]` block from `vibe.toml`.
    Remove(RegistryRemoveArgs),
}

#[derive(Debug, clap::Args)]
pub struct RegistrySyncArgs {
    /// Directory of the project (defaults to current).
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}

#[derive(Debug, clap::Args)]
pub struct RegistryListArgs {
    /// Project root with `vibe.toml`. Defaults to current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}

#[derive(Debug, clap::Args)]
pub struct RegistryAddArgs {
    /// Local alias for the new registry. Used in lockfile `registry`
    /// fields and to target `[[mirror]]` / `[[override]]` entries.
    pub name: String,

    /// Organization-root URL — any git URL `git` accepts
    /// (`git@host:org`, `ssh://...`, `https://...`).
    pub url: String,

    /// Registry-level ref (reserved for a future registry-metadata
    /// branch). Defaults to `main`.
    #[arg(long = "ref")]
    pub registry_ref: Option<String>,

    /// Naming convention mapping `<kind>:<name>` to a repo name under
    /// the org. One of `kind-name` (default), `name`, `kind/name`.
    #[arg(long = "naming")]
    pub naming: Option<String>,

    /// Where to insert the new registry in the priority list.
    /// `primary` makes it the first registry (the new default for
    /// publish + the first stop on resolve fallback). `append` adds
    /// it at the end. Defaults to `append`.
    #[arg(long = "position", default_value = "append")]
    pub position: String,

    /// Project root with `vibe.toml`. Defaults to current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}

#[derive(Debug, clap::Args)]
pub struct RegistrySetMirrorArgs {
    /// Target registry name (matches a `[[registry]].name`) or `*` for
    /// any registry.
    pub of: String,

    /// Mirror URL. Any git URL `git` accepts.
    pub url: String,

    /// Priority within the target registry's mirror chain — lower =
    /// tried first. Defaults to 0.
    #[arg(long = "priority", default_value_t = 0)]
    pub priority: i32,

    /// Project root with `vibe.toml`. Defaults to current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}

#[derive(Debug, clap::Args)]
pub struct RegistryRemoveArgs {
    /// What to remove. Subcommand-style: `registry <name>` removes the
    /// `[[registry]]` with that name; `mirror <of> <url>` removes the
    /// `[[mirror]]` block matching exactly on `(of, url)`.
    #[command(subcommand)]
    pub target: RegistryRemoveTarget,
}

#[derive(Debug, Subcommand)]
pub enum RegistryRemoveTarget {
    /// Remove a `[[registry]]` named `<NAME>`. Refuses if any
    /// `[[mirror]]` targets this registry by name (those would be
    /// orphaned). Wildcard `of = "*"` mirrors are unaffected.
    Registry(RegistryRemoveRegistryArgs),

    /// Remove a `[[mirror]]` exactly matching `(<OF>, <URL>)`.
    Mirror(RegistryRemoveMirrorArgs),
}

#[derive(Debug, clap::Args)]
pub struct RegistryRemoveRegistryArgs {
    /// `[[registry]].name` to remove.
    pub name: String,

    /// Project root with `vibe.toml`. Defaults to current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}

#[derive(Debug, clap::Args)]
pub struct RegistryRemoveMirrorArgs {
    /// `[[mirror]].of` of the entry to remove.
    pub of: String,

    /// `[[mirror]].url` of the entry to remove (exact match).
    pub url: String,

    /// Project root with `vibe.toml`. Defaults to current directory.
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
