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

use specmark::spec;

/// The read-only locator epoch's ONE answer: which absolute root a mutating
/// command must lease before it may read anything execution-shaped.
///
/// The canonical workspace root when discovery succeeds; the canonical
/// selected root when it fails — the SAME fallback law the state root has
/// always had (a project outside any discoverable workspace still gets its
/// `.vibe/lifecycle.toml`). Discovery here reads the tree but decides
/// nothing downstream: the manifest, workspace and state that execution
/// consumes are loaded AFTER the lease, from the command's own snapshot.
/// ```
/// use vibe_orchestrator::lease_root;
/// let root = lease_root(std::path::Path::new("."));
/// assert!(root.is_absolute() || root == std::path::Path::new("."));
/// ```
#[must_use]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
pub fn lease_root(project_root: &Path) -> PathBuf {
    Workspace::discover(project_root)
        .map(|workspace| workspace.root)
        .unwrap_or_else(|_| project_root.to_path_buf())
}

/// Acquire the workspace-global mutation lease at the locator's root — the
/// outermost command lock, held through the final report/error/trace
/// finalisation. The typed refusal (Busy) renders with its own PROP-054
/// citation; this boundary adds only the command-side context.
/// ```no_run
/// let lease = vibe_orchestrator::acquire_lease(std::path::Path::new("."))?;
/// assert!(std::sync::Arc::strong_count(&lease) >= 1);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
pub fn acquire_lease(
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
