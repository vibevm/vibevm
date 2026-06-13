//! The install orchestrator — the plan → apply pipeline behind
//! `vibe install`, extracted from the CLI (VIBEVM-SPEC §5.6, §9.1,
//! §11.1; the 2026-06-12 audit's sketch, built by SHRINK-PLAN v0.2).
//!
//! The crate owns the install *transaction*: deriving the effective
//! root set, the PROP-011 freshness fast path, driving the depsolver,
//! fetching and feature-pinning every node, the PROP-003 §2.6.1
//! conditional-dependency fixpoint, and recording the outcome into
//! `vibe.toml` / `vibe.lock`. It deliberately does NOT own:
//!
//! - **Cell construction** — the R-001 registry. The caller builds its
//!   registry/solver cells and hands them in behind [`InstallSource`];
//!   construction sites stay at the CLI's composition root.
//! - **Interaction** — confirmation prompts, TTY detection, and report
//!   rendering. [`plan`] returns a [`Plan`] the caller presents and
//!   confirms; [`apply`] runs only after the caller said yes. Progress
//!   during planning surfaces through typed [`PlanEvent`]s, not
//!   prints.
//!
//! The split mirrors the original M0 crate of the same name
//! (`plan_install` / `apply_install`), whose materialisation half
//! moved into `vibe-workspace` under the loading model — what returns
//! here is the orchestration layer the audit found tangled into the
//! CLI.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use std::path::Path;

use vibe_core::PackageRef;
use vibe_registry::{CachedPackage, RegistryError};
use vibe_resolver::{ResolvedGraph, SolveError};

mod apply;
mod error;
mod events;
mod fetched;
mod plan;
mod record;

pub use apply::{ApplyReport, apply};
pub use error::Error;
pub use events::{NullObserver, PlanEvent, PlanObserver};
pub use fetched::{Fetched, NodeInstallMeta};
pub use plan::{InstallRequest, Plan, PlannedInstall, plan};
pub use record::{
    exact_pinned_pkgref, finalize_pkgref_for_manifest, merge_manifest_requires,
    merge_root_dependencies, record_git_source,
};

/// The package source an install runs against — the seam between the
/// orchestrator and whatever registry topology the caller composed
/// (R-001: cells are constructed at the caller's composition root and
/// arrive here already built).
///
/// Canonical implementation shape:
///
/// ```no_run
/// use std::path::Path;
/// use vibe_core::PackageRef;
/// use vibe_install::InstallSource;
/// use vibe_registry::{CachedPackage, LocalRegistry, RegistryError};
/// use vibe_resolver::{ResolvedGraph, SolveError};
///
/// struct LocalSource(LocalRegistry);
///
/// impl InstallSource for LocalSource {
///     fn resolve_and_fetch(
///         &self,
///         pkgref: &PackageRef,
///         cache_root: &Path,
///         _expected_hash: Option<&str>,
///     ) -> Result<CachedPackage, RegistryError> {
///         let resolved = self.0.resolve(pkgref)?;
///         self.0.fetch(&resolved, cache_root)
///     }
///
///     fn solve(&self, roots: &[PackageRef]) -> Result<ResolvedGraph, SolveError> {
///         // Build the solver from the caller's selected cells.
///         # let _ = roots;
///         # unimplemented!()
///     }
/// }
/// ```
pub trait InstallSource {
    /// Resolve `pkgref` and materialise its content into the cache.
    /// `expected_hash` (typically the lockfile pin) lets a
    /// mirror-aware source skip a source serving disagreeing bytes.
    fn resolve_and_fetch(
        &self,
        pkgref: &PackageRef,
        cache_root: &Path,
        expected_hash: Option<&str>,
    ) -> Result<CachedPackage, RegistryError>;

    /// Run the depsolver against this source, returning the full
    /// transitive graph the pipeline will fetch and materialise.
    fn solve(&self, roots: &[PackageRef]) -> Result<ResolvedGraph, SolveError>;
}
