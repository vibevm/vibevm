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

    /// Not a failure: a hosted `agent` row at a slot point parked for the
    /// hosting agent, so the install stopped AT THAT POINT and the chain went
    /// no further.
    ///
    /// How much is already durable depends on WHICH point, and this type does
    /// not pretend otherwise. A `slot:pre-install` park stops before the
    /// remaining slots are materialised and therefore before the lockfile
    /// barrier and every post-barrier row; a `slot:post-install` park happens
    /// AFTER the apply is durable — slots placed, lock written — and stops the
    /// rows that would have followed it. `progress` below is the
    /// boundary-measured record of whichever of those actually happened; the
    /// caller reports that and exits 0.
    #[error(
        "the install lifecycle parked run `{}` for the hosting agent at a slot point; the chain \
         stopped there and the report carries the progress measured up to it; {} task(s) await \
         it, then resume with `{}` \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-HANDSHAKE)",
        delegation.run_id,
        delegation.tasks.len(),
        delegation.resume,
    )]
    Delegated {
        delegation: Box<vibe_lifecycle::Delegation>,
        reports: Vec<crate::SlotLifecycleReport>,
        /// What the install really did before the park, measured at the
        /// mutation boundary rather than assumed from the point. A
        /// post-install park represents a COMPLETE materialisation; a
        /// pre-install park represents whatever was placed before it;
        /// reporting either as nothing would be the dishonest half of an
        /// honest handoff.
        progress: Box<crate::InstallProgress>,
    },

    /// A slot row FAILED, carrying the rows and the boundary-measured
    /// progress the outermost command needs to report it. A failure is an
    /// outcome: `vibe install` renders its own document for it rather than
    /// letting the removal of the per-row echo take the machine record with it.
    #[error(
        "{source} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#FAILURE-BY-PHASE; \
          fix: apply the inner failure's remediation and rerun the install)"
    )]
    SlotFailed {
        #[source]
        source: Box<Error>,
        reports: Vec<crate::SlotLifecycleReport>,
        progress: Box<crate::InstallProgress>,
    },

    #[error(transparent)]
    Workspace(#[from] vibe_workspace::WorkspaceError),
}
