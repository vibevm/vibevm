//! The [`InstallResolver`] — the local / multi-registry / embedded dispatch
//! behind the [`vibe_install::InstallSource`] seam, moved verbatim from
//! `vibe-cli/src/commands/install/resolver.rs` (R7.4 A15a). Construction
//! lives in [`crate::builder`]; the cell selection its solve paths route
//! through lives in [`crate::cells`] (R-001).

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::Result;
use vibe_core::{Group, PackageRef};
use vibe_install::InstallSource;
use vibe_registry::{
    CachedPackage, LocalRegistry, MultiRegistryResolver, RegistryError, ResolutionPurpose,
};

use crate::cells::{
    ProviderCell, ProviderResource, dep_solver, metadata_manifest_cell, selection_flags,
    solve_masked_cell,
};

/// Either a M0-shape local-directory registry (used by an explicit registry
/// path and the in-tree fixture path) or a full PROP-002 multi-registry
/// resolver covering the `[[registry]]` / `[[mirror]]` / `[[override]]`
/// sections in `vibe.toml`. The orchestrator consumes it through the
/// [`InstallSource`] seam; construction (and the R-001 cell selection its
/// solve paths read) lives in this crate.
///
/// ```
/// use vibe_package_source::InstallResolver;
///
/// // Match on the variant a caller owns; the type deliberately is not
/// // `Debug` (it holds live registry handles).
/// fn name_of(resolver: &InstallResolver) -> &'static str {
///     match resolver {
///         InstallResolver::Local(..) | InstallResolver::LockedLocal { .. } => "local",
///         InstallResolver::Multi(..) => "multi",
///         InstallResolver::Embedded { .. } => "embedded",
///     }
/// }
/// ```
pub enum InstallResolver {
    /// Explicit local discovery with held sources taking priority per coordinate.
    LockedLocal {
        local: LocalRegistry,
        held: Box<MultiRegistryResolver>,
        solver: Option<&'static str>,
    },
    /// The local-directory registry plus the optional solver cell
    /// name threaded through to the R-001 selection seam.
    Local(LocalRegistry, Option<&'static str>),
    // Boxed: `MultiRegistryResolver` is by far the larger variant
    // (it carries the registry list plus the override / git-source /
    // path-source maps), so an unboxed enum would bloat every
    // `InstallResolver` value to the size of the multi-registry path.
    Multi(Box<MultiRegistryResolver>, Option<&'static str>),
    /// PROP-030: the embedded local-directory registry (a source install's
    /// in-tree `packages/`) composed with an optional declared multi-registry
    /// walk at the origin-selected precedence. `declared = None` is the
    /// no-`[[registry]]` project where the local family stands alone. The
    /// Vec is the ordered local-registry family — project-local first (when
    /// `<project_root>/packages/` is discovered, PROP-030 §3.3), then
    /// vibe-embedded. The composite at the resolver layer (PROP-030 §3)
    /// honours this ordering: the first local wins a clash inside the family.
    Embedded {
        locals: Vec<LocalRegistry>,
        /// PROP-030 §3.3: how many leading entries of `locals` are the
        /// project-local registry (0 when only vibe-embedded is in the
        /// family, 1 when project-packages were discovered). The fetch path
        /// tags the resolved package `is_local` (portable) for an index < this
        /// count, else `is_embedded` (machine-local) — so the lock records the
        /// right `source_kind` and the reproducibility guard fires only for
        /// the vibe-embedded half.
        project_local_count: usize,
        declared: Option<Box<MultiRegistryResolver>>,
        precedence: vibe_resolver::EmbeddedPrecedence,
        /// PROP-030 §3.1: when set (`--embedded-short-circuit`), version
        /// enumeration stops at the embedded registry for any coordinate it
        /// serves, so the declared walk (and its network round-trip) is
        /// consulted only for packages the embedded registry lacks.
        short_circuit: bool,
        solver: Option<&'static str>,
    },
}

impl InstallSource for InstallResolver {
    fn source_solve_limits(
        &self,
    ) -> Result<vibe_resolver::SourceSolveLimits, vibe_resolver::SolveError> {
        Ok(vibe_resolver::SourceSolveLimits::standard_operation())
    }
    fn solve_metadata(
        &self,
        request: &vibe_resolver::SourceSolveRequest<'_>,
    ) -> Result<vibe_resolver::SourceResolvedGraph, vibe_resolver::SolveError> {
        let (cell, solver) = self.cell_selection();
        let flags = selection_flags(cell, solver);
        crate::cells::solve_metadata_cell(&flags, self, request)
    }
    fn acquire_candidate_for_install(
        &self,
        candidate: &vibe_registry::source_candidates::RetainedSourceCandidate,
        store_root: &Path,
        expected_hash: Option<&str>,
        slot: &Path,
        progress: &vibe_core::progress::Progress,
    ) -> Result<vibe_registry::PackageAcquisition, RegistryError> {
        use vibe_core::manifest::CanonicalSourceBinding;
        let local_root = match candidate.binding() {
            CanonicalSourceBinding::DirectoryRegistry { root }
            | CanonicalSourceBinding::Local { root }
            | CanonicalSourceBinding::Embedded { root } => Some(root),
            _ => None,
        };
        match self {
            Self::Local(local, _) => local.fetch_candidate_for_install(
                candidate,
                store_root,
                expected_hash,
                slot,
                progress,
            ),
            Self::LockedLocal { local, held, .. } => {
                if held.has_locked_package(candidate.group(), candidate.name())
                    || local_root.is_none()
                {
                    held.fetch_candidate_for_install(
                        candidate,
                        store_root,
                        expected_hash,
                        slot,
                        progress,
                    )
                } else {
                    local.fetch_candidate_for_install(
                        candidate,
                        store_root,
                        expected_hash,
                        slot,
                        progress,
                    )
                }
            }
            Self::Multi(owner, _) => owner.fetch_candidate_for_install(
                candidate,
                store_root,
                expected_hash,
                slot,
                progress,
            ),
            Self::Embedded {
                locals, declared, ..
            } => {
                if let Some(owner) = declared
                    .as_deref()
                    .filter(|owner| owner.has_locked_package(candidate.group(), candidate.name()))
                {
                    return owner.fetch_candidate_for_install(
                        candidate,
                        store_root,
                        expected_hash,
                        slot,
                        progress,
                    );
                }
                if let Some(root) = local_root
                    && let Some(local) = locals.iter().find(|local| local.root() == root)
                {
                    return local.fetch_candidate_for_install(
                        candidate,
                        store_root,
                        expected_hash,
                        slot,
                        progress,
                    );
                }
                if let Some(owner) = declared.as_deref() {
                    return owner.fetch_candidate_for_install(
                        candidate,
                        store_root,
                        expected_hash,
                        slot,
                        progress,
                    );
                }
                Err(RegistryError::MalformedMeta {
                    path: slot.into(),
                    reason: "retained source has no matching operation owner".into(),
                })
            }
        }
    }

    fn replays_locked_source(&self, pkg: &PackageRef) -> bool {
        self.held_for(pkg)
            .is_some_and(|held| held.replays_locked_source(pkg))
    }

    fn acquire_for_install(
        &self,
        pkgref: &PackageRef,
        store_root: &Path,
        expected_hash: Option<&str>,
        slot: &Path,
        progress: &vibe_core::progress::Progress,
    ) -> Result<vibe_registry::PackageAcquisition, RegistryError> {
        if let Some(held) = self.held_for(pkgref) {
            let selected = held.resolve_for_with_progress(
                pkgref,
                ResolutionPurpose::DownloadSource,
                progress,
            )?;
            return held.fetch_for_install(&selected, store_root, expected_hash, slot, progress);
        }
        match self {
            InstallResolver::Multi(registry, _) => {
                let selected = registry.resolve_for_with_progress(
                    pkgref,
                    ResolutionPurpose::DownloadSource,
                    progress,
                )?;
                registry.fetch_for_install(&selected, store_root, expected_hash, slot, progress)
            }
            // Embedded/local precedence remains its single incumbent algorithm.
            _ => self
                .resolve_and_fetch_with_progress(pkgref, store_root, expected_hash, progress)
                .map(|cached| vibe_registry::PackageAcquisition::Cached(Box::new(cached))),
        }
    }
    /// Resolve `pkgref` and insert its content into the machine-global
    /// store under `store_root` (PROP-010 §2.7). `expected_hash`
    /// (typically the lockfile pin for `(pkgref.kind, pkgref.name,
    /// version)`) is forwarded to the multi-registry path's
    /// mirror-aware fetch so a source serving disagreeing bytes can be
    /// skipped in favour of a matching one. The local-directory path
    /// ignores the hint — there's only ever one source on that path.
    /// No path refuses on a mismatch: the returned `content_hash` is the
    /// hash of the bytes fetched, and a caller that must reproduce the
    /// lock (`vibe reinstall`) compares it with the pin itself.
    fn resolve_and_fetch(
        &self,
        pkgref: &PackageRef,
        store_root: &Path,
        expected_hash: Option<&str>,
    ) -> Result<CachedPackage, RegistryError> {
        self.resolve_and_fetch_inner(pkgref, store_root, expected_hash, None)
    }

    fn resolve_and_fetch_with_progress(
        &self,
        pkgref: &PackageRef,
        store_root: &Path,
        expected_hash: Option<&str>,
        progress: &vibe_core::progress::Progress,
    ) -> Result<CachedPackage, RegistryError> {
        match self {
            InstallResolver::Multi(..) => {
                self.resolve_and_fetch_inner(pkgref, store_root, expected_hash, Some(progress))
            }
            // Local/embedded dispatch already has its own precedence and
            // provenance rules. Keep that one algorithm and wrap it with an
            // honest component task instead of duplicating the walk here.
            _ => {
                let task = progress.task(format!("Fetching {}", pkgref.qualified_name()));
                let result = self.resolve_and_fetch_inner(
                    pkgref,
                    store_root,
                    expected_hash,
                    Some(&task.progress()),
                );
                match &result {
                    Ok(cached) if cached.is_local || cached.is_embedded => {
                        task.skip("reused local package source")
                    }
                    Ok(_) => task.finish(),
                    Err(_) => task.fail("package fetch failed"),
                }
                result
            }
        }
    }

    fn solve(
        &self,
        roots: &[PackageRef],
    ) -> Result<vibe_resolver::ResolvedGraph, vibe_resolver::SolveError> {
        // Cell selection lives in the cells module (R-001); this
        // match only routes the resource the caller already owns.
        let (provider_cell, solver_override) = self.cell_selection();
        let flags = selection_flags(provider_cell, solver_override);
        dep_solver(&flags, self.provider_resource()).solve(roots)
    }

    fn solve_configured(
        &self,
        roots: &[PackageRef],
        snapshot: &vibe_resolver::configuration::ConfigurationSnapshot,
        blocked: &BTreeSet<(String, String)>,
    ) -> Result<vibe_resolver::ResolvedGraph, vibe_resolver::SolveError> {
        let (provider_cell, solver_override) = self.cell_selection();
        let flags = selection_flags(provider_cell, solver_override);
        crate::cells::solve_configured_cell(
            &flags,
            self.provider_resource(),
            roots,
            snapshot,
            blocked,
        )
    }

    fn manifest_of(
        &self,
        pkg: &PackageRef,
    ) -> Result<vibe_core::manifest::Manifest, vibe_resolver::SolveError> {
        // Cell selection lives in the cells module (R-001); this method
        // only routes the resource the resolver already owns.
        metadata_manifest_cell(self.provider_resource(), pkg)
    }

    fn solve_masked(
        &self,
        roots: &[PackageRef],
        blocked: &BTreeSet<(String, String)>,
    ) -> Result<vibe_resolver::ResolvedGraph, vibe_resolver::SolveError> {
        let (provider_cell, solver_override) = self.cell_selection();
        let flags = selection_flags(provider_cell, solver_override);
        solve_masked_cell(&flags, self.provider_resource(), roots, blocked)
    }

    fn prepare_in_place(
        &self,
        pkgref: &PackageRef,
        slot: &std::path::Path,
    ) -> Result<vibe_registry::PreparedInPlace, RegistryError> {
        if let Some(held) = self.held_for(pkgref) {
            let selected = held.resolve_for(pkgref, ResolutionPurpose::CheckoutSource)?;
            return held.prepare_in_place(&selected, slot);
        }
        let unsupported = || {
            let group = pkgref
                .group
                .clone()
                .ok_or_else(|| RegistryError::UnqualifiedPkgref(pkgref.to_string()))?;
            Err(RegistryError::InPlaceUnsupported {
                group,
                name: pkgref.name.to_string(),
            })
        };
        match self {
            InstallResolver::Local(..) | InstallResolver::LockedLocal { .. } => unsupported(),
            InstallResolver::Multi(registry, _) => {
                let resolution = registry.resolve_for(pkgref, ResolutionPurpose::CheckoutSource)?;
                registry.prepare_in_place(&resolution, slot)
            }
            InstallResolver::Embedded {
                declared: Some(registry),
                ..
            } => match registry.resolve_for(pkgref, ResolutionPurpose::CheckoutSource) {
                Ok(resolution) => registry.prepare_in_place(&resolution, slot),
                Err(error) if is_registry_absent(&error) => unsupported(),
                Err(error) => Err(error),
            },
            InstallResolver::Embedded { declared: None, .. } => unsupported(),
        }
    }

    fn materialise_in_place(
        &self,
        pkgref: &PackageRef,
        slot: &std::path::Path,
    ) -> Result<vibe_registry::InPlaceMaterialised, RegistryError> {
        if let Some(held) = self.held_for(pkgref) {
            let selected = held.resolve_for(pkgref, ResolutionPurpose::CheckoutSource)?;
            return held.prepare_in_place(&selected, slot)?.materialise();
        }
        match self {
            // A local-directory registry has no git backend — in-place needs
            // a real git source to clone and incrementally update (PROP-022
            // §2.4).
            InstallResolver::Local(..) | InstallResolver::LockedLocal { .. } => {
                let group = pkgref
                    .group
                    .clone()
                    .ok_or_else(|| RegistryError::UnqualifiedPkgref(pkgref.to_string()))?;
                Err(RegistryError::InPlaceUnsupported {
                    group,
                    name: pkgref.name.to_string(),
                })
            }
            InstallResolver::Multi(m, _) => {
                let resolution = m.resolve_for(pkgref, ResolutionPurpose::CheckoutSource)?;
                m.materialise_in_place(&resolution, slot)
            }
            // In-place needs a git backend to clone and incrementally update;
            // the embedded local-directory registry has none. Serve it from
            // the declared walk when that carries the package, else refuse with
            // the same InPlaceUnsupported an explicit `<dir>` install gives.
            InstallResolver::Embedded { declared, .. } => match declared {
                Some(m) => match m.resolve_for(pkgref, ResolutionPurpose::CheckoutSource) {
                    Ok(resolution) => m.materialise_in_place(&resolution, slot),
                    Err(e) if is_registry_absent(&e) => {
                        let group = pkgref
                            .group
                            .clone()
                            .ok_or_else(|| RegistryError::UnqualifiedPkgref(pkgref.to_string()))?;
                        Err(RegistryError::InPlaceUnsupported {
                            group,
                            name: pkgref.name.to_string(),
                        })
                    }
                    Err(e) => Err(e),
                },
                None => {
                    let group = pkgref
                        .group
                        .clone()
                        .ok_or_else(|| RegistryError::UnqualifiedPkgref(pkgref.to_string()))?;
                    Err(RegistryError::InPlaceUnsupported {
                        group,
                        name: pkgref.name.to_string(),
                    })
                }
            },
        }
    }
}

mod operations;

/// The registry errors that mean "this source does not serve the
/// coordinate" — the embedded/declared composition falls through these and
/// halts on anything else (PROP-002 §2.3.1 fall-through set).
pub(crate) fn is_registry_absent(err: &RegistryError) -> bool {
    matches!(
        err,
        RegistryError::UnknownPackage { .. }
            | RegistryError::NoMatchingVersion { .. }
            | RegistryError::PackageNotFoundEverywhere { .. }
    )
}

#[cfg(test)]
#[path = "source_progress_tests.rs"]
mod progress_tests;

mod locked;
