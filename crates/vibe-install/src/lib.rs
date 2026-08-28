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
//!   construction sites live in the shared `vibe-package-source`
//!   composition (since R7.4 A15a) — no surface constructs a cell of its
//!   own.
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

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use std::collections::BTreeSet;
use std::path::Path;

use vibe_core::PackageRef;
use vibe_core::manifest::Manifest;
use vibe_registry::{CachedPackage, InPlaceMaterialised, RegistryError};
use vibe_resolver::{ResolvedGraph, SolveError};

mod apply;
mod error;
mod events;
mod fetched;
mod lifecycle;
mod plan;
mod record;
mod slot_verify;
mod visibility_projection;

pub use apply::{
    ApplyReport, PreparedApplyReport, SlotLifecycleSeams, apply, apply_with_spec_format,
    apply_with_spec_format_and_hook_output, apply_with_spec_format_and_lifecycle_observed,
    apply_with_spec_format_and_lifecycle_observed_traced,
    apply_with_spec_format_and_lifecycle_observed_traced_prepared,
};
pub use error::Error;
pub use events::{NullObserver, PlanEvent, PlanObserver};
pub use fetched::{Fetched, NodeInstallMeta};
pub use lifecycle::{
    InstallProgress, InstallSlotLifecycle, NoSlotLifecycleObserver, SlotLifecycleObserver,
    SlotLifecyclePlan, SlotLifecyclePlanEntry, SlotLifecycleReport,
};
pub use plan::{
    InstallRequest, Plan, PlannedInstall, plan, plan_prepared_with_spec_format,
    plan_with_spec_format,
};
pub use record::{
    exact_pinned_pkgref, finalize_pkgref_for_manifest, merge_manifest_requires,
    merge_root_dependencies, record_git_source,
};
pub use visibility_projection::{
    FilteringDepProvider, Projection, ProjectionInput, metadata_manifest, project,
    solve_with_visibility_mask,
};

/// The package source an install runs against — the seam between the
/// orchestrator and whatever registry topology the caller composed
/// (R-001: cells are constructed at the caller's composition root and
/// arrive here already built).
///
/// Canonical implementation shape:
///
/// ```no_run
/// use std::collections::BTreeSet;
/// use std::path::Path;
/// use vibe_core::{PackageRef, manifest::Manifest};
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
///         store_root: &Path,
///         _expected_hash: Option<&str>,
///     ) -> Result<CachedPackage, RegistryError> {
///         let resolved = self.0.resolve(pkgref)?;
///         self.0.fetch(&resolved, store_root)
///     }
///
///     fn solve(&self, roots: &[PackageRef]) -> Result<ResolvedGraph, SolveError> {
///         // Build the solver from the caller's selected cells.
///         # let _ = roots;
///         # unimplemented!()
///     }
///
///     fn manifest_of(&self, pkg: &PackageRef) -> Result<Manifest, SolveError> {
///         # let _ = pkg;
///         # unimplemented!()
///     }
///
///     fn solve_masked(
///         &self,
///         roots: &[PackageRef],
///         blocked: &BTreeSet<(String, String)>,
///     ) -> Result<ResolvedGraph, SolveError> {
///         # let _ = (roots, blocked);
///         # unimplemented!()
///     }
///
///     fn materialise_in_place(
///         &self,
///         pkgref: &PackageRef,
///         slot: &Path,
///     ) -> Result<vibe_registry::InPlaceMaterialised, RegistryError> {
///         // A local-directory registry has no git backend; a git-backed
///         // source clones / incrementally updates the slot here.
///         # let _ = (pkgref, slot);
///         unimplemented!()
///     }
/// }
/// ```
pub trait InstallSource {
    /// Resolve `pkgref` and insert its content into the machine-global
    /// store (`~/.vibe/cache/`, PROP-010 §2.7) under `store_root`.
    /// `expected_hash` (typically the lockfile pin) lets a
    /// mirror-aware source skip a source serving disagreeing bytes.
    fn resolve_and_fetch(
        &self,
        pkgref: &PackageRef,
        store_root: &Path,
        expected_hash: Option<&str>,
    ) -> Result<CachedPackage, RegistryError>;

    /// Run the depsolver against this source, returning the full
    /// transitive graph the pipeline will fetch and materialise.
    fn solve(&self, roots: &[PackageRef]) -> Result<ResolvedGraph, SolveError>;

    /// Read one solved package's manifest through the source's metadata-only
    /// provider path. This must never fetch or materialise package content.
    fn manifest_of(&self, pkg: &PackageRef) -> Result<Manifest, SolveError>;

    /// Re-run the selected solver cell while masking the named
    /// `parent -> target` edges in every parent manifest it reads.
    fn solve_masked(
        &self,
        roots: &[PackageRef],
        blocked: &BTreeSet<(String, String)>,
    ) -> Result<ResolvedGraph, SolveError>;

    /// Place an `in-place` package (PROP-022 §2.4) directly into its project
    /// `slot`: a fresh `git clone --recurse-submodules` when the slot is
    /// absent, an incremental `git fetch` + checkout when it already carries
    /// `.git`. Bypasses the cache clone + the `.git`-stripped `copy`,
    /// so a version bump on a giant repo transfers only changed objects. Used
    /// by scoped `vibe update <pkg>` and the general `vibe install` re-resolve
    /// (via [`apply`](crate::apply), which defers the in-place fetch past the
    /// plan) for a package the lockfile records as in-place. Returns the slot's
    /// manifest + provenance for the lockfile; a source with no git backend
    /// (the local-directory registry) errors.
    fn materialise_in_place(
        &self,
        pkgref: &PackageRef,
        slot: &Path,
    ) -> Result<InPlaceMaterialised, RegistryError>;
}
