//! The arguments of `vibe app export` and `vibe app import`, the portable
//! set of a home's applications (PROP-059 `#portable-set`). They live apart
//! from `app.rs` so the verb list stays inside its file budget.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#portable-set");

use std::path::PathBuf;

#[path = "app_portable/export.rs"]
mod export;
pub use export::AppExportArgs;

/// `vibe app import`.
#[derive(Debug, Clone, clap::Args)]
pub struct AppImportArgs {
    /// Per-exact-key public target decisions; fixed before the cohort starts.
    #[arg(long, value_name = "FILE", conflicts_with = "resume")]
    pub target_input: Option<PathBuf>,
    /// Ephemeral per-application secret answers; '-' consumes stdin once.
    #[arg(long, value_name = "FILE")]
    pub secret_input: Option<PathBuf>,
    /// The set `vibe app export` wrote (TOML, or JSON from `--json`).
    #[arg(required_unless_present = "resume", conflicts_with = "resume")]
    pub file: Option<PathBuf>,

    /// Continue one exact user-home import cohort without rereading the original file.
    #[arg(long,value_name="OPERATION_ID",conflicts_with_all=["file","dry_run","registry","accept_license","skip_installed"])]
    pub resume: Option<String>,

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
