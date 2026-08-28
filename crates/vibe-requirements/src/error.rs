//! The query's typed failures — partial observations live in the report,
//! only broken authority reaches this enum.

use specmark::spec;
use thiserror::Error;

use vibe_facts::RegistryError;
use vibe_wire::behaviour::requirements_report::RequirementsError;

/// Every way the one [`query`](crate::query) call can refuse. Partial
/// generated values (an absent lock, an absent slot, an unreadable or
/// malformed authored source, a missing relation provider) are NOT
/// errors — they are typed source/relation states in the returned
/// report. This enum is reserved for what the packet's central rulings
/// call aborts: an unacceptable query, broken trusted inputs, a
/// malformed lock or registry whose scope has no wire state, and an
/// assembled report that breaks its own wire laws.
#[derive(Debug, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#FACT-QUERY-CONTRACT")]
pub enum QueryError {
    /// The effective query is not one the surfaces would have accepted —
    /// refused by the wire owner BEFORE any filesystem access.
    #[error(
        "the effective query is not one the surfaces would accept \n         (violates spec://org.vibevm.core/vibevm/common/PROP-054#FACT-QUERY-CONTRACT; \n          fix: pass a spec:// prefix, a limit within 1..=256 and the relations flag)"
    )]
    InvalidQuery {
        #[source]
        source: RequirementsError,
    },

    /// The injected lifecycle run id is not the 32-lowercase-hex shape
    /// the evidence wire and lifecycle state hold themselves to.
    #[error(
        "the injected lifecycle run id `{run_id}` is not 32 lowercase hex \n         (violates spec://org.vibevm.core/vibevm/common/PROP-054#FACT-QUERY-CONTRACT; \n          fix: inject the id from the read-only lifecycle peek, or None)"
    )]
    InvalidRunId { run_id: String },

    /// Workspace discovery failed: the selected root is not a node of a
    /// discoverable workspace, or the tree's manifests are malformed.
    #[error(
        "workspace discovery failed for the selected root \n         (violates spec://org.vibevm.core/vibevm/common/PROP-054#FACT-QUERY-CONTRACT; \n          fix: select the root of a workspace node with a readable vibe.toml)"
    )]
    Workspace {
        #[source]
        source: vibe_workspace::WorkspaceError,
    },

    /// The selected node's manifest could not yield a host coordinate.
    #[error(
        "the selected node declares no usable host coordinate \n         (violates spec://org.vibevm.core/vibevm/common/PROP-054#FACT-QUERY-CONTRACT; \n          fix: give the selected vibe.toml a non-empty [project] group and name)"
    )]
    Host {
        #[source]
        source: RegistryError,
    },

    /// A present `vibe.lock` did not parse — the source universe was
    /// never established, and no wire state can say so.
    #[error(
        "reading the workspace lock failed \n         (violates spec://org.vibevm.core/vibevm/common/PROP-054#FACT-QUERY-CONTRACT; \n          fix: repair vibe.lock or remove it to answer the host alone)"
    )]
    Lock {
        #[source]
        source: vibe_core::Error,
    },

    /// A path wearing the lock's name exists but is not a regular
    /// file — malformed scope, never reinterpretable as "no lock".
    #[error(
        "the workspace lock path `{}` is not a regular file \n         (violates spec://org.vibevm.core/vibevm/common/PROP-054#FACT-QUERY-CONTRACT; \n          fix: remove the directory wearing vibe.lock's name, or restore the file)",
        path.display()
    )]
    LockNotFile { path: std::path::PathBuf },

    /// The adoption registry is malformed — the central ruling aborts
    /// the query rather than answer over an unestablished overlay.
    #[error(
        "reading the adoption registry failed \n         (violates spec://org.vibevm.core/vibevm/common/PROP-054#FACT-QUERY-CONTRACT; \n          fix: repair the vibefacts TOML against the closed [[fact]] schema)"
    )]
    Registry {
        #[source]
        source: RegistryError,
    },

    /// An internal law this library cannot represent (a coordinate
    /// under both kinds, a count beyond the wire's `uint32`). Never
    /// ambiguous output.
    #[error(
        "internal query invariant violated: {0} \n         (violates spec://org.vibevm.core/vibevm/common/PROP-054#FACT-QUERY-CONTRACT; \n          fix: report this — the query reached a state its laws forbid)"
    )]
    Invariant(String),

    /// The assembled report broke a relational law of its own wire —
    /// a bug in this library, surfaced rather than shipped.
    #[error(
        "the assembled report broke its own wire laws \n         (violates spec://org.vibevm.core/vibevm/common/PROP-054#FACT-QUERY-CONTRACT; \n          fix: report this — the query library emitted an invalid report)"
    )]
    Wire {
        #[source]
        source: RequirementsError,
    },
}
