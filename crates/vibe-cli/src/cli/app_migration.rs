//! The argument surface of `vibe app migrate` (PROP-075
//! `#MIGRATION-FAILURE`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#MIGRATION-FAILURE");

/// Flags of `vibe app migrate SLOT FROM TO`.
#[derive(Debug, clap::Args)]
pub struct AppMigrateArgs {
    /// The application slot whose state moves, as `vibe app alternatives`
    /// names it.
    pub slot: String,

    /// The old version, as a version or a whole installation key.
    pub from: String,

    /// The new version, as a version or a whole installation key. The same
    /// version as `FROM` is admitted: that is a maintenance entry, which
    /// carries state into the version it is already in.
    pub to: String,

    /// Run exactly this declared migration entry; repeatable. Naming an
    /// entry is itself the consent to run it.
    #[arg(long = "only", value_name = "ID", conflicts_with = "all")]
    pub only: Vec<String>,

    /// Run every entry the new version declares.
    #[arg(long)]
    pub all: bool,

    /// Run the entries an attended offer would preselect, without asking.
    #[arg(long)]
    pub yes: bool,

    /// Reuse the kept work directory of this version pair and re-run only
    /// what has not succeeded — the import, when the export already did.
    #[arg(long)]
    pub resume: bool,

    /// Resolve both versions' packages from this local registry instead of
    /// the configured registries. A migration step is the package's own
    /// script, so both package trees must be reachable.
    #[arg(long)]
    pub registry: Option<std::path::PathBuf>,
}
