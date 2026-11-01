//! The arguments of `vibe app export` and `vibe app import`, the portable
//! set of a home's applications (PROP-059 `#portable-set`). They live apart
//! from `app.rs` so the verb list stays inside its file budget.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#portable-set");

use std::path::PathBuf;

/// `vibe app export`.
#[derive(Debug, Clone, clap::Args)]
pub struct AppExportArgs {
    /// Write the set to this file instead of standard output.
    #[arg(long, short = 'o', value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// Include hidden installations and those a requirement pulled in.
    #[arg(long)]
    pub all: bool,

    /// Leave the launcher arrangement (collections, pins, overrides) out.
    #[arg(long)]
    pub installations_only: bool,
}

/// `vibe app import`.
#[derive(Debug, Clone, clap::Args)]
pub struct AppImportArgs {
    /// The set `vibe app export` wrote (TOML, or JSON from `--json`).
    pub file: PathBuf,

    /// Print what each row would do and change nothing.
    #[arg(long)]
    pub dry_run: bool,

    /// Resolve the packages from this local registry first.
    #[arg(long, value_name = "DIR")]
    pub registry: Option<PathBuf>,

    /// Accept every installer's declared license without being asked.
    #[arg(long)]
    pub accept_license: bool,

    /// Keep an application installed with another choice as it is instead
    /// of refusing its row.
    #[arg(long)]
    pub skip_installed: bool,
}
