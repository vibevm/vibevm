//! The workspace-global mutation lease: the OUTERMOST command lock.
//!
//! `.vibe/lifecycle.toml` is a single-writer record (PROP-054
//! `##PHASE-STATE-HOME`), and an in-process `Mutex` cannot make that true
//! across processes: two CLI/MCP processes can read the same prior state,
//! independently allocate/adopt, then last-writer-wins the other's row
//! through two individually atomic renames. Every mutating lifecycle surface
//! therefore acquires the same capability-safe, nonblocking
//! `.vibe/lifecycle.lock` at the canonical workspace root BEFORE its
//! world/state/identity reads and holds it through the final
//! state/task/report outcome.
//!
//! The lock IS the strengthened `vibe-safefs` primitive — this cell adds the
//! ownership type, not a second implementation. The persistent empty lock
//! file is infrastructure of a mutating command, not lifecycle state: a Busy
//! refusal may have created it (and `.vibe/`), but no run id, state row or
//! outbox byte follows.
//!
//! Lock order: `lifecycle.lock` is outermost. A holder may then take
//! `compile-trace.lock`, `package-skills.lock` or `vibe-boot-artifacts.lock`;
//! no holder of an inner lock ever acquires this one. Nested acquisition in
//! one process is a typed [`LifecycleLeaseError::Busy`] — an OS lock is held
//! by an open file description, so a second same-process handle on the same
//! name is refused by the host itself — and Busy is a refusal, never a
//! reentrancy mechanism.
//!
//! The type is deliberately not `Clone`/`Copy`: an [`std::sync::Arc<LifecycleLease>`]
//! may travel through the run/callback channel, and cloning THAT proves the
//! one OS acquisition — it can never reacquire the file. Putting acquisition
//! inside the state store would be insufficient: identity selection reads
//! state before any store exists, so the proof is threaded from the command
//! boundary instead.

use std::path::{Path, PathBuf};

use specmark::spec;
use vibe_safefs::Project;

/// The lock file name, relative to `.vibe/` under the canonical workspace
/// root. A bare component on purpose: `Project::try_lock` owns the path law.
pub(crate) const LOCK_NAME: &str = "lifecycle.lock";

/// Why a mutating lifecycle command could not take ownership of the
/// workspace. Every variant names the root and cites the state-home law, so
/// an operator reading only the rendered text learns both which tree was
/// contended and that waiting or re-running — not deleting state — is the
/// remedy. `non_exhaustive` on purpose: the lease is a lower cell whose
/// refusal vocabulary may grow (an MCP-era adapter must match it without a
/// crate break), and the exhaustive match already lives here, beside the
/// variants.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-STATE-HOME")]
pub enum LifecycleLeaseError {
    /// Another mutating process owns this workspace's lifecycle state. The
    /// refusal is typed and total: no run id was allocated, no state or
    /// outbox byte moved. The persistent empty `.vibe/lifecycle.lock` may
    /// have been created — it is infrastructure, not lifecycle state.
    #[error(
        "another mutating lifecycle command owns `{root}` (`.vibe/{lock}` is held); this \
         invocation refused before any run id, state row or outbox byte moved \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-STATE-HOME; \
          fix: wait for the other command to finish, then rerun — the lock is released by \
          its completion or its process death, and nested acquisition by the same process \
          is likewise a refusal, never a reentry)",
        lock = LOCK_NAME,
    )]
    Busy { root: PathBuf },
    /// The root itself could not be pinned as a project capability, or the
    /// lock could not be opened/verified. A root problem, never a
    /// state-cache problem: nothing here advises removing `.vibe/`.
    #[error(
        "cannot take the lifecycle mutation lease at `{root}`: {reason} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-STATE-HOME; \
          fix: ensure the workspace root exists, is an absolute directory path and \
          `.vibe/` is writable, then rerun)"
    )]
    Directory { root: PathBuf, reason: String },
    /// A workspace loaded under a root OTHER than the one this command
    /// leased. The one spelling of "which tree does this command own" is the
    /// lease's; a loaded value that disagrees names a pre-lease snapshot or
    /// a foreign tree, and continuing would read and write state beside
    /// another process's lock. `boundary` says where the disagreement was
    /// caught, so the refusal is diagnosable without a debugger.
    #[error(
        "the workspace {boundary} loaded at `{observed}` does not match the mutation lease's \
         root `{leased}`; refusing to touch a tree this command does not own \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-STATE-HOME; \
          fix: stop at this boundary, inspect any earlier effects already reported by the \
          command, and rerun once the workspace is stable — the lease and loaded tree must \
          name one root)"
    )]
    RootMismatch {
        leased: PathBuf,
        observed: PathBuf,
        boundary: &'static str,
    },
}

/// One workspace's outermost mutation lease: the pinned root capability plus
/// the exclusive OS guard over `.vibe/lifecycle.lock`.
///
/// Non-`Clone`/`Copy` on purpose — both fields are move-only handles, so the
/// only way to share the one acquisition is an `Arc` over the whole value.
/// Dropping the last owner (or the process dying) releases the OS lock.
#[derive(Debug)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-STATE-HOME")]
pub struct LifecycleLease {
    /// The pinned workspace-root capability. Every production read and write
    /// this lease authorises — state I/O above all — goes through it, never
    /// through a second capability opened from the same path.
    project: Project,
    /// Intentionally live: dropping it (or process death) releases the lock.
    _guard: vibe_safefs::LockGuard,
}

impl LifecycleLease {
    /// Pin the root capability and take `.vibe/lifecycle.lock` WITHOUT
    /// blocking. `root` must be the canonical ABSOLUTE workspace root — the
    /// caller's read-only locator epoch decides it once, before anything
    /// execution-shaped is read.
    ///
    /// `Ok(None)`-shaped contention surfaces as the typed
    /// [`LifecycleLeaseError::Busy`]; a root that cannot be pinned or a lock
    /// that cannot be opened/verified is
    /// [`LifecycleLeaseError::Directory`] with the safefs chain rendered.
    pub fn acquire(root: &Path) -> Result<Self, LifecycleLeaseError> {
        let project = Project::open(root).map_err(|error| LifecycleLeaseError::Directory {
            root: root.to_path_buf(),
            reason: format!("{error:#}"),
        })?;
        match project.try_lock(LOCK_NAME) {
            Ok(Some(guard)) => Ok(Self {
                project,
                _guard: guard,
            }),
            Ok(None) => Err(LifecycleLeaseError::Busy {
                root: root.to_path_buf(),
            }),
            Err(error) => Err(LifecycleLeaseError::Directory {
                root: root.to_path_buf(),
                reason: format!("{error:#}"),
            }),
        }
    }

    /// The canonical absolute workspace root this lease pins — the one root
    /// every post-acquire workspace load must agree with.
    #[must_use]
    pub fn root(&self) -> &Path {
        self.project.root_path()
    }

    /// The ONE root-agreement gate every post-acquisition workspace load
    /// owes. `observed` is the root a loaded workspace (or a plan built from
    /// one) claims; a disagreement is the typed
    /// [`LifecycleLeaseError::RootMismatch`] naming `boundary` — the one
    /// place this check lives, so no call site hand-rolls its own spelling
    /// of the refusal again.
    pub fn ensure_root(
        &self,
        observed: &Path,
        boundary: &'static str,
    ) -> Result<(), LifecycleLeaseError> {
        if observed == self.root() {
            return Ok(());
        }
        Err(LifecycleLeaseError::RootMismatch {
            leased: self.root().to_path_buf(),
            observed: observed.to_path_buf(),
            boundary,
        })
    }

    /// The pinned capability proof. State I/O goes through this exact
    /// capability rather than opening a second one from the same path, so
    /// the tree a leased command mutates is the tree it pinned.
    pub(crate) fn project(&self) -> &Project {
        &self.project
    }
}

#[cfg(test)]
#[path = "lease/tests.rs"]
mod tests;
