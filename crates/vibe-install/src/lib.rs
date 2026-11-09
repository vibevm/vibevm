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
mod source_admission;
mod visibility_projection;

pub use apply::{
    ApplyReport, LockPublication, PreparedApplyReport, ProjectRecordCapture,
    ProjectRecordPublication, SlotLifecycleSeams, apply, apply_with_spec_format,
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
    InstallPreview, InstallRequest, Plan, PlannedInstall, PreviewSourceStorage, freshness_prepared,
    plan, plan_prepared_with_spec_format, plan_prepared_without_publication, plan_with_spec_format,
    preview_has_requirements, preview_prepared,
};
pub use record::{
    exact_pinned_pkgref, finalize_pkgref_for_manifest, merge_manifest_requires,
    merge_root_dependencies, record_git_source,
};
pub use slot_verify::resolution_slots_verified;
pub use visibility_projection::{
    FilteringDepProvider, Projection, ProjectionInput, metadata_manifest, project,
    solve_visible_graph, solve_with_visibility_mask,
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
    /// Whether this exact request replays held source authority. Default sources
    /// retain their existing incremental preparation path.
    fn replays_locked_source(&self, _pkgref: &PackageRef) -> bool {
        false
    }

    /// For an explicit request this operation moves at its held source: that
    /// source and whether it serves an admitted version. Default sources hold
    /// no lock and report `None`.
    fn locked_update_availability(
        &self,
        _pkgref: &PackageRef,
    ) -> Result<Option<vibe_registry::LockedUpdateAvailability>, RegistryError> {
        Ok(None)
    }

    /// Explicit operation resource budget; unsupported source modes refuse.
    fn source_solve_limits(&self) -> Result<vibe_resolver::SourceSolveLimits, SolveError> {
        Err(vibe_resolver::DepProviderError::Other(
            "selected source/solver has no metadata operation budget".into(),
        )
        .into())
    }

    /// Resolve and retain metadata without acquiring a payload. Implementations
    /// must not fall back to semver-only selection or an unconstrained fetch.
    fn solve_metadata(
        &self,
        _request: &vibe_resolver::SourceSolveRequest<'_>,
    ) -> Result<vibe_resolver::SourceResolvedGraph, SolveError> {
        Err(vibe_resolver::DepProviderError::Other(
            "selected source/solver has no retained metadata relation".into(),
        )
        .into())
    }

    /// Acquire exactly the operation-owned candidate retained by metadata solve.
    fn acquire_candidate_for_install(
        &self,
        _candidate: &vibe_registry::source_candidates::RetainedSourceCandidate,
        _store_root: &Path,
        _expected_hash: Option<&str>,
        _slot: &Path,
        _progress: &vibe_core::progress::Progress,
    ) -> Result<vibe_registry::PackageAcquisition, RegistryError> {
        Err(RegistryError::MalformedMeta { path: _slot.into(), reason: "selected source does not support retained-candidate acquisition; no name/version fallback is allowed".into() })
    }

    /// One selected acquisition; native in-place objects remain retained until
    /// checked workspace apply. Legacy/local implementations keep their fetch.
    fn acquire_for_install(
        &self,
        pkgref: &PackageRef,
        store_root: &Path,
        expected_hash: Option<&str>,
        _slot: &Path,
        progress: &vibe_core::progress::Progress,
    ) -> Result<vibe_registry::PackageAcquisition, RegistryError> {
        self.resolve_and_fetch_with_progress(pkgref, store_root, expected_hash, progress)
            .map(|cached| vibe_registry::PackageAcquisition::Cached(Box::new(cached)))
    }
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

/// Held coordinates whose locked source is an in-workspace `file://`
/// working tree (PROP-011 §2.6): mutable, so each install refreshes them from
/// that same directory instead of replaying the held hash. `in-place` giants
/// are excluded; they refresh through `vibe update`.
///
/// ```
/// // An empty held lock has nothing to refresh from the workspace.
/// let rows: Vec<vibe_core::manifest::LockedPackage> = Vec::new();
/// let mutable = vibe_install::mutable_locked_sources(std::path::Path::new("."), &rows);
/// assert!(mutable.is_empty());
/// ```
pub fn mutable_locked_sources(
    root: &std::path::Path,
    rows: &[vibe_core::manifest::LockedPackage],
) -> Vec<String> {
    rows.iter()
        .filter(|row| {
            !row.materialization.is_in_place()
                && vibe_workspace::freshness::is_in_workspace_file_source(
                    row.source_url.as_str(),
                    root,
                )
        })
        .map(|row| format!("{}/{}", row.group, row.name))
        .collect()
}

/// Workspace-owned checkout addresses passed to source observation, never persisted.
/// ```
/// let paths = vibe_install::locked_checkout_paths(std::path::Path::new("."), &[]);
/// assert!(paths.is_empty());
/// ```
pub fn locked_checkout_paths(
    root: &std::path::Path,
    rows: &[vibe_core::manifest::LockedPackage],
) -> std::collections::HashMap<String, std::path::PathBuf> {
    rows.iter()
        .filter(|row| row.materialization.is_in_place())
        .map(|row| {
            (
                format!("{}/{}", row.group, row.name),
                vibe_workspace::vibedeps::in_place_slot_abs_path(
                    root,
                    &row.group,
                    row.name.as_str(),
                ),
            )
        })
        .collect()
}
