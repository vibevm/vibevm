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
pub mod configuration_admission;
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
    apply_with_spec_format_and_lifecycle_observed_traced_prepared, publish_admitted_manifest,
    publish_empty_legacy_lock,
};
pub use error::Error;
pub use events::{NullObserver, PlanEvent, PlanObserver};
pub use fetched::{Fetched, NodeInstallMeta};
pub use lifecycle::{
    InstallProgress, InstallSlotLifecycle, NoSlotLifecycleObserver, SlotLifecycleObserver,
    SlotLifecyclePlan, SlotLifecyclePlanEntry, SlotLifecycleReport,
};
pub use plan::{
    InstallRequest, Plan, PlannedInstall, freshness_prepared, plan, plan_prepared_with_spec_format,
    plan_prepared_without_publication, plan_with_spec_format,
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

    /// Observed sibling of [`Self::resolve_and_fetch`]. Implementations with
    /// internal registry/network stages override this; the compatibility
    /// default preserves every existing source implementation.
    fn resolve_and_fetch_with_progress(
        &self,
        pkgref: &PackageRef,
        store_root: &Path,
        expected_hash: Option<&str>,
        _progress: &vibe_core::progress::Progress,
    ) -> Result<CachedPackage, RegistryError> {
        self.resolve_and_fetch(pkgref, store_root, expected_hash)
    }

    /// Run the depsolver against this source, returning the full
    /// transitive graph the pipeline will fetch and materialise.
    fn solve(&self, roots: &[PackageRef]) -> Result<ResolvedGraph, SolveError>;

    /// Resolve actual common consumer inputs through the selected supported cell.
    /// The operation-owned snapshot must be reused for held and masked retries.
    /// Legacy implementations explicitly refuse instead of dropping intent.
    fn solve_configured(
        &self,
        _roots: &[PackageRef],
        _snapshot: &vibe_resolver::configuration::ConfigurationSnapshot,
        _blocked: &BTreeSet<(String, String)>,
    ) -> Result<ResolvedGraph, SolveError> {
        Err(vibe_resolver::DepProviderError::Configuration(
            vibe_core::manifest::PackageConfigurationError::Unsupported {
                package: "workspace dependency graph".into(),
                reason: "the selected source/solver does not consume package configuration".into(),
            },
        )
        .into())
    }

    /// Actual configured selection already bound during this planning solve.
    /// A legacy source returns none; callers still enforce package admission.
    fn configuration_for_node(
        &self,
        _node: &vibe_resolver::ResolvedNode,
    ) -> Result<Option<vibe_core::manifest::PackageConfiguration>, SolveError> {
        Ok(None)
    }

    /// Read one solved package's manifest through the source's metadata-only
    /// provider path. This must never fetch or materialise package content.
    fn manifest_of(&self, pkg: &PackageRef) -> Result<Manifest, SolveError>;

    /// Bind prospective in-place bytes and their immutable source revision
    /// without changing the project checkout. Protected consumers admit the
    /// returned manifest before calling its bound `materialise` operation.
    /// A metadata-only lookup or the old slot is not this proof.
    fn prepare_in_place(
        &self,
        pkgref: &PackageRef,
        _slot: &Path,
    ) -> Result<vibe_registry::PreparedInPlace, RegistryError> {
        let group = pkgref
            .group
            .clone()
            .ok_or_else(|| RegistryError::UnqualifiedPkgref(pkgref.to_string()))?;
        Err(RegistryError::InPlaceUnsupported {
            group,
            name: pkgref.name.to_string(),
        })
    }

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
    /// so a version bump on a giant repo transfers only changed objects.
    /// This compatibility/source operation does not perform package configuration
    /// admission. Protected install/update consumers use `prepare_in_place`
    /// followed by its bound materialisation instead. Returns the slot's
    /// manifest + provenance for the lockfile; a source with no git backend
    /// (the local-directory registry) errors.
    fn materialise_in_place(
        &self,
        pkgref: &PackageRef,
        slot: &Path,
    ) -> Result<InPlaceMaterialised, RegistryError>;
}
