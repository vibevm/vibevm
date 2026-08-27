//! The typed refusals of lifecycle-state I/O, split from `store.rs` when the
//! transaction half outgrew the 600-line budget. The store owns the FILE;
//! this cell owns what an operator learns when the file refuses.

use std::path::PathBuf;

use specmark::spec;
use thiserror::Error;

/// What one bounded re-read proved after a publication crossed the rename
/// boundary and then failed (PROP-054 `##PHASE-STATE-HOME`): the outcome a
/// caller cannot re-derive, carried beside the original failure rather than
/// inferred from prose.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-STATE-HOME")]
pub enum PostPublicationRecovery {
    /// The exact candidate bytes are durable: the write in fact landed, and
    /// the store adopted the candidate in memory. Memory and disk agree.
    CandidateAdopted,
    /// The exact prior bytes — or the prior absence — are durable: the write
    /// in fact did not land, and the store retained the prior state. Memory
    /// and disk agree.
    PriorRetained,
    /// The durable state is neither the candidate nor the prior bytes (or it
    /// could not be proven safe to read at all). The store is POISONED: it
    /// refuses every further mutation rather than write over a state it
    /// cannot describe.
    Poisoned { reason: String },
}

impl std::fmt::Display for PostPublicationRecovery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CandidateAdopted => write!(
                f,
                "a bounded re-read proved the exact candidate bytes durable, so the candidate \
                 was adopted in memory",
            ),
            Self::PriorRetained => write!(
                f,
                "a bounded re-read proved the exact prior bytes durable, so the prior state was \
                 retained",
            ),
            Self::Poisoned { reason } => write!(
                f,
                "a bounded re-read proved a state that is neither the candidate nor the prior \
                 bytes ({reason}), so the store is poisoned and refuses further writes",
            ),
        }
    }
}

/// Every typed refusal produced while opening, reading, validating,
/// publishing or recovering lifecycle state, plus fresh run-id allocation.
/// Publication variants carry [`vibe_safefs::PublishStage`] directly so the
/// rename boundary is matchable data, never inferred from prose.
#[derive(Debug, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-STATE-HOME")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#REF-LIFECYCLE-TOML")]
#[non_exhaustive]
pub enum LifecycleStateError {
    #[error(
        "cannot read lifecycle state `{path}`: {source} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-STATE-HOME; \
          fix: remove this erasable cache and rerun the lifecycle)"
    )]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error(
        "cannot open the lifecycle workspace root `{path}`: {reason} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-STATE-HOME; \
          fix: pass the canonical ABSOLUTE workspace root — the directory that already exists \
          on disk; this is a root problem, not a state-file problem, so the state cache is \
          irrelevant here)"
    )]
    Root { path: PathBuf, reason: String },
    #[error(
        "lifecycle state `{path}` is not valid UTF-8 \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#REF-LIFECYCLE-TOML; \
          fix: remove this erasable cache and rerun the lifecycle)"
    )]
    NotUtf8 { path: PathBuf },
    #[error(
        "malformed lifecycle state `{path}`: {reason} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#REF-LIFECYCLE-TOML; \
          fix: remove this erasable cache and rerun the lifecycle)"
    )]
    Malformed { path: PathBuf, reason: String },
    #[error(
        "unsupported lifecycle state schema {schema} in `{path}`; this build supports schema 1 \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#REF-LIFECYCLE-TOML; \
          fix: remove this erasable cache and rerun the lifecycle)"
    )]
    Unsupported { path: PathBuf, schema: u32 },
    #[error(
        "cannot encode lifecycle state `{path}`: {reason} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#REF-LIFECYCLE-TOML; \
          fix: report this generated-wire serialization failure)"
    )]
    Encode { path: PathBuf, reason: String },
    #[error(
        "candidate lifecycle state `{path}` encodes to {size} bytes, over the {cap}-byte state \
         read cap \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-STATE-HOME; \
          fix: this store never publishes bytes it could not read back — reduce the state's rows \
          or report this runaway state)"
    )]
    TooLarge {
        path: PathBuf,
        size: usize,
        cap: usize,
    },
    #[error(
        "lifecycle state `{path}` violates the delegated-run invariant: {reason} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#REF-AGENT-RESUME; \
          fix: remove this erasable cache and rerun the lifecycle)"
    )]
    Invariant { path: PathBuf, reason: String },
    #[error(
        "lifecycle state publication `{path}` failed: {failure} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-STATE-HOME; \
          fix: ensure `.vibe/` is writable and rerun)"
    )]
    Publication {
        path: PathBuf,
        /// How far the publication got — the typed stage, not only the prose
        /// inside the rendered failure. `BeforePublication` means the
        /// destination is provably unchanged.
        stage: vibe_safefs::PublishStage,
        failure: String,
    },
    #[error(
        "cannot confirm lifecycle state publication `{path}`: {publication}; {recovery} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-STATE-HOME; \
          fix: the recovery line says what is durable; rerun the lifecycle against it)"
    )]
    PostPublication {
        path: PathBuf,
        /// How far the publication got — the typed stage. Always
        /// [`vibe_safefs::PublishStage::PossiblyPublished`] on this variant;
        /// carried as a field so a caller matches the stage, never prose.
        stage: vibe_safefs::PublishStage,
        /// The original publication failure, preserved verbatim — including
        /// how far the publication got — so the recovery beside it can be
        /// audited against the failure that prompted it.
        publication: String,
        recovery: PostPublicationRecovery,
    },
    #[error(
        "this lifecycle state store is poisoned and refuses further writes to `{path}`: {reason} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-STATE-HOME; \
          fix: compare `.vibe/lifecycle.toml` with the last proven state this store still \
          exposes, remove the erasable cache if it is foreign, and rerun the lifecycle)"
    )]
    Poisoned { path: PathBuf, reason: String },
    #[error(
        "cannot allocate a fresh run-id scratch directory under `{path}`: {reason} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#REF-AGENT-RESUME; \
          fix: ensure the SELECTED project root is writable and rerun the lifecycle)"
    )]
    Allocation { path: PathBuf, reason: String },
}
