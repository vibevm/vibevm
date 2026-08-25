//! The orchestrator's error layer — its own variants speak the
//! Class-F product grammar; lower layers pass through transparently
//! (their messages already carry the grammar).

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use specmark::spec;
use thiserror::Error;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail")]
pub enum Error {
    #[error(
        "no packages to install — neither the command line nor any workspace \
         member's [requires] names one \
         (violates spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail; \
          fix: pass `<group>/<name>[@<version>] …` or add entries to \
          [requires].packages in `{manifest_dir}/vibe.toml`)"
    )]
    NothingToInstall { manifest_dir: String },

    #[error(
        "conditional-dep expansion exceeded {iterations} iterations — cascading \
         predicates may form a cycle or runaway chain; pending extras: {pending:?} \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#req-conditional-fixpoint; \
          fix: break the predicate chain in the named packages' \
          [target.\"context(…)\".dependencies] tables)"
    )]
    ConditionalDepRunaway {
        iterations: usize,
        pending: Vec<String>,
    },

    #[error(
        "CLI root `{pkgref}` is missing from the solved graph — the install \
         source returned an incomplete resolution \
         (violates spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail; \
          fix: report this against the InstallSource implementation in use)"
    )]
    RootNotFetched { pkgref: String },

    #[error(
        "could not create the package cache at `{path}`: {source} \
         (violates spec://org.vibevm.core/vibevm/VIBEVM-SPEC#fetching-strategy-and-cache-layout; \
          fix: check the workspace root is writable)"
    )]
    CacheDir {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error(
        "package tag for `{package}` failed to parse: {source} \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#interface-tags; \
          fix: the package's kind/name or capability list is malformed — \
          correct its manifest)"
    )]
    CapabilityTag {
        package: String,
        #[source]
        source: vibe_resolver::TagError,
    },

    #[error(transparent)]
    Core(#[from] vibe_core::Error),

    #[error(transparent)]
    Registry(#[from] vibe_registry::RegistryError),

    #[error(transparent)]
    Solve(#[from] vibe_resolver::SolveError),

    #[error(transparent)]
    Feature(#[from] vibe_resolver::FeatureError),

    #[error(
        "install lifecycle integration failed: {0} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#H-SCRIPT)"
    )]
    Lifecycle(String),

    #[error(transparent)]
    Workspace(#[from] vibe_workspace::WorkspaceError),
}
