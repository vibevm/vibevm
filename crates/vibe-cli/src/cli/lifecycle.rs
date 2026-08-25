//! Argument grammar for the nine-phase default lifecycle and its clean prefix.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#CHAIN-GENERAL");

use std::path::PathBuf;

use super::InstallArgs;

/// Shared flags for every default-lifecycle verb except `install`.
///
/// Package references and install-only source mutation flags deliberately do
/// not occur here: the lifecycle's prerequisite install reads the manifest,
/// while only the explicit `vibe install` verb may change it.
#[derive(Debug, Clone, clap::Args)]
pub struct LifecycleArgs {
    /// Directory of the project (defaults to current).
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// Path to a local-directory registry.
    #[arg(long)]
    pub registry: Option<PathBuf>,

    /// Skip interactive confirmation for the prerequisite install.
    #[arg(long, alias = "yes")]
    pub assume_yes: bool,

    /// Override the project's resolved language preference for this run.
    #[arg(long)]
    pub language: Option<String>,

    /// Activate features on every root package during prerequisite install.
    #[arg(long, value_delimiter = ',')]
    pub features: Vec<String>,

    /// Do not activate `[features].default` during prerequisite install.
    #[arg(long)]
    pub no_default_features: bool,

    /// Activate every non-private feature during prerequisite install.
    #[arg(long)]
    pub all_features: bool,

    /// Halt on public-registry authentication failures.
    #[arg(long)]
    pub auth_required: bool,

    /// Select the dependency solver used by prerequisite install.
    #[arg(long, value_name = "naive|sat|resolvo")]
    pub solver: Option<String>,

    /// Prefer the embedded registry over declared registries.
    #[arg(long)]
    pub prefer_embedded: bool,

    /// Prefer declared registries over the embedded registry.
    #[arg(long)]
    pub no_prefer_embedded: bool,

    /// Ignore the ambient embedded registry.
    #[arg(long)]
    pub no_default_registry: bool,

    /// Short-circuit registry traffic for embedded coordinates.
    #[arg(long)]
    pub embedded_short_circuit: bool,

    /// Prefer project-local packages over other local sources.
    #[arg(long)]
    pub prefer_local: bool,

    /// Ignore project-local packages.
    #[arg(long)]
    pub no_prefer_local: bool,

    /// Run install hooks regardless of package group.
    #[arg(long)]
    pub allow_hooks: bool,

    /// Ignore reusable lifecycle fingerprints for this invocation.
    #[arg(long)]
    pub force: bool,
}

impl LifecycleArgs {
    /// Build the one existing install implementation's argument shape without
    /// admitting pkgrefs or any manifest-mutating install-only flag.
    pub(crate) fn install_args(&self) -> InstallArgs {
        InstallArgs {
            packages: Vec::new(),
            path: self.path.clone(),
            registry: self.registry.clone(),
            assume_yes: self.assume_yes,
            language: self.language.clone(),
            features: self.features.clone(),
            no_default_features: self.no_default_features,
            all_features: self.all_features,
            exact: false,
            auth_required: self.auth_required,
            solver: self.solver.clone(),
            prefer_embedded: self.prefer_embedded,
            no_prefer_embedded: self.no_prefer_embedded,
            no_default_registry: self.no_default_registry,
            offline: false,
            embedded_short_circuit: self.embedded_short_circuit,
            prefer_local: self.prefer_local,
            no_prefer_local: self.no_prefer_local,
            git: None,
            tag: None,
            branch: None,
            rev: None,
            git_auth: None,
            git_token_env: None,
            allow_hooks: self.allow_hooks,
            force: self.force,
        }
    }
}

/// Arguments for `vibe clean [<phase> …]`.
#[derive(Debug, clap::Args)]
pub struct CleanArgs {
    /// Directory of the project. When left at `.`, a chained phase's own
    /// `--path` selects the project for both lifecycles.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// Skip the clean confirmation.
    #[arg(long, alias = "yes")]
    pub assume_yes: bool,

    /// The default-lifecycle phase to run after the wipe.
    #[command(subcommand)]
    pub chain: Option<CleanChain>,
}

/// Any default-lifecycle phase may follow the independent clean lifecycle.
#[derive(Debug, clap::Subcommand)]
pub enum CleanChain {
    Validate(LifecycleArgs),
    Install(InstallArgs),
    Generate(LifecycleArgs),
    Build(LifecycleArgs),
    Test(LifecycleArgs),
    Create(LifecycleArgs),
    Verify(LifecycleArgs),
    Package(LifecycleArgs),
    Deploy(LifecycleArgs),
}
