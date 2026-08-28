//! The CLI's lease epoch: the read-only locator that decides WHICH root a
//! mutating command must own, and the one acquisition helper every mutating
//! boundary goes through.
//!
//! Split from `inputs.rs` when that cell reached its line budget: input
//! normalisation and lease location are different laws — the locator answers
//! "which tree", the acquisition answers "who owns it" — and the lease half
//! is the piece R7.4 §2.1 keeps growing.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use vibe_workspace::Workspace;

/// The read-only locator epoch's ONE answer: which absolute root a mutating
/// command must lease before it may read anything execution-shaped.
///
/// The canonical workspace root when discovery succeeds; the canonical
/// selected root when it fails — the SAME fallback law the state root has
/// always had (a project outside any discoverable workspace still gets its
/// `.vibe/lifecycle.toml`). Discovery here reads the tree but decides
/// nothing downstream: the manifest, workspace and state that execution
/// consumes are loaded AFTER the lease, from the command's own snapshot.
pub(crate) fn lease_root(project_root: &Path) -> PathBuf {
    Workspace::discover(project_root)
        .map(|workspace| workspace.root)
        .unwrap_or_else(|_| project_root.to_path_buf())
}

/// Acquire the workspace-global mutation lease at the locator's root — the
/// outermost command lock, held through the final report/error/trace
/// finalisation. The typed refusal (Busy) renders with its own PROP-054
/// citation; this boundary adds only the command-side context.
pub(crate) fn acquire_lease(
    project_root: &Path,
) -> Result<std::sync::Arc<vibe_lifecycle::LifecycleLease>> {
    let root = lease_root(project_root);
    vibe_lifecycle::LifecycleLease::acquire(&root)
        .map(std::sync::Arc::new)
        .with_context(|| {
            format!(
                "taking the lifecycle mutation lease at `{}`",
                root.display()
            )
        })
}

/// One real lease for the unit-test fixtures that must carry the proof as a
/// value (an `InstallRunContext` built by hand). Acquired ONCE per test
/// process; these fixtures never dispatch through it, so sharing one
/// acquisition across them is exactly the Arc proof the production channel
/// carries.
///
/// The owner RETAINS its `TempDir` (a plain `keep()` leak would orphan the
/// directory even when nothing else wants it). Statics are never dropped in
/// today's Rust, so process-exit cleanup is not delivered — the temp
/// filesystem's own sweeper is the backstop — but the ownership is honest:
/// the day static destructors run, this directory is cleaned with the rest.
#[cfg(test)]
pub(crate) fn test_lease() -> std::sync::Arc<vibe_lifecycle::LifecycleLease> {
    /// The retained owner: the lease AND the directory it was taken over,
    /// so both live (and, if statics ever drop, die) together.
    struct LeaseOwner {
        lease: std::sync::Arc<vibe_lifecycle::LifecycleLease>,
        _dir: tempfile::TempDir,
    }
    static OWNER: std::sync::OnceLock<LeaseOwner> = std::sync::OnceLock::new();
    OWNER
        .get_or_init(|| {
            let dir = tempfile::tempdir().expect("a temp root for the test lease");
            let lease = vibe_lifecycle::LifecycleLease::acquire(dir.path())
                .expect("the retained test root is leasable");
            LeaseOwner {
                lease: std::sync::Arc::new(lease),
                _dir: dir,
            }
        })
        .lease
        .clone()
}
