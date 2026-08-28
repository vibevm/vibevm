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
use vibe_registry::{CachedPackage, LocalRegistry, MultiRegistryResolver, RegistryError};

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
///         InstallResolver::Local(..) => "local",
///         InstallResolver::Multi(..) => "multi",
///         InstallResolver::Embedded { .. } => "embedded",
///     }
/// }
/// ```
pub enum InstallResolver {
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
    /// Resolve `pkgref` and insert its content into the machine-global
    /// store under `store_root` (PROP-010 §2.7). `expected_hash`
    /// (typically the lockfile pin for `(pkgref.kind, pkgref.name,
    /// version)`) is forwarded to the multi-registry path's
    /// mirror-aware fetch so a source serving disagreeing bytes can be
    /// skipped in favour of a matching one. The local-directory path
    /// ignores the hint — there's only ever one source on that path,
    /// and integrity is checked against the lockfile pin at apply
    /// time.
    fn resolve_and_fetch(
        &self,
        pkgref: &PackageRef,
        store_root: &Path,
        expected_hash: Option<&str>,
    ) -> Result<CachedPackage, RegistryError> {
        match self {
            InstallResolver::Local(r, _) => {
                let resolved = r.resolve(pkgref)?;
                r.fetch(&resolved, store_root)
            }
            InstallResolver::Multi(m, _) => {
                let resolution = m.resolve(pkgref)?;
                m.fetch_with_expected_hash(&resolution, store_root, expected_hash)
            }
            InstallResolver::Embedded {
                locals,
                project_local_count,
                declared,
                precedence,
                ..
            } => {
                let fetch_local = || -> Result<CachedPackage, RegistryError> {
                    // Walk the local family in order (project-local first,
                    // then vibe-embedded). The first local that serves the
                    // coordinate wins; an absence falls through to the next;
                    // any real failure halts. Provenance tagging:
                    //   index < project_local_count → is_local (portable,
                    //     per-project packages/ — PROP-030 §3.3)
                    //   else → is_embedded (machine-local, vibe's in-tree
                    //     packages — PROP-030 §2)
                    // so the lock records the right source_kind and the
                    // reproducibility guard fires only for the vibe-embedded
                    // half.
                    let mut last_absent: Option<RegistryError> = None;
                    for (idx, local) in locals.iter().enumerate() {
                        match local.resolve(pkgref) {
                            Ok(resolved) => {
                                let mut cached = local.fetch(&resolved, store_root)?;
                                if idx < *project_local_count {
                                    cached.is_local = true;
                                } else {
                                    cached.is_embedded = true;
                                }
                                return Ok(cached);
                            }
                            Err(e) if is_registry_absent(&e) => {
                                last_absent = Some(e);
                            }
                            Err(e) => return Err(e),
                        }
                    }
                    // `last_absent` is always `Some` when `locals` is
                    // non-empty (every local either Ok's or sets it). The
                    // empty-`locals` case is forbidden by the construction
                    // path (build_install_resolver returns Embedded only
                    // when !locals.is_empty()). Fall through to the
                    // declared walk with the typed absence; if somehow
                    // neither is set, propagate as a generic "not here".
                    match last_absent {
                        Some(e) => Err(e),
                        None => Err(RegistryError::UnqualifiedPkgref(pkgref.to_string())),
                    }
                };
                let fetch_declared = || -> Result<CachedPackage, RegistryError> {
                    match declared {
                        Some(m) => {
                            let resolution = m.resolve(pkgref)?;
                            m.fetch_with_expected_hash(&resolution, store_root, expected_hash)
                        }
                        None => {
                            let group = pkgref.group.clone().ok_or_else(|| {
                                RegistryError::UnqualifiedPkgref(pkgref.to_string())
                            })?;
                            Err(RegistryError::UnknownPackage {
                                group,
                                name: pkgref.name.to_string(),
                            })
                        }
                    }
                };
                // Fetch in precedence order, falling through only a genuine
                // "not here" (a real failure halts).
                match precedence {
                    vibe_resolver::EmbeddedPrecedence::EmbeddedFirst => match fetch_local() {
                        Err(e) if is_registry_absent(&e) => fetch_declared(),
                        other => other,
                    },
                    vibe_resolver::EmbeddedPrecedence::EmbeddedLast => match fetch_declared() {
                        Err(e) if is_registry_absent(&e) => fetch_local(),
                        other => other,
                    },
                }
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

    fn materialise_in_place(
        &self,
        pkgref: &PackageRef,
        slot: &std::path::Path,
    ) -> Result<vibe_registry::InPlaceMaterialised, RegistryError> {
        match self {
            // A local-directory registry has no git backend — in-place needs
            // a real git source to clone and incrementally update (PROP-022
            // §2.4).
            InstallResolver::Local(..) => {
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
                let resolution = m.resolve(pkgref)?;
                m.materialise_in_place(&resolution, slot)
            }
            // In-place needs a git backend to clone and incrementally update;
            // the embedded local-directory registry has none. Serve it from
            // the declared walk when that carries the package, else refuse with
            // the same InPlaceUnsupported an explicit `<dir>` install gives.
            InstallResolver::Embedded { declared, .. } => match declared {
                Some(m) => match m.resolve(pkgref) {
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

impl InstallResolver {
    /// The selected `(ProviderCell, solver override)` pair — the one flag
    /// decision all three solve-path methods share (R-001: decided in the
    /// cells module, only routed here).
    fn cell_selection(&self) -> (ProviderCell, Option<&'static str>) {
        match self {
            InstallResolver::Local(_, s) => (ProviderCell::Local, *s),
            InstallResolver::Multi(_, s) => (ProviderCell::Multi, *s),
            InstallResolver::Embedded { solver, .. } => (ProviderCell::Embedded, *solver),
        }
    }

    /// The borrowed [`ProviderResource`] this resolver's variant owns — the
    /// one routing shape `solve` / `manifest_of` / `solve_masked` all hand to
    /// the cells module's construction sites (R-001).
    fn provider_resource(&self) -> ProviderResource<'_> {
        match self {
            InstallResolver::Local(r, _) => ProviderResource::Local(r),
            InstallResolver::Multi(m, _) => ProviderResource::Multi(m),
            InstallResolver::Embedded {
                locals,
                declared,
                precedence,
                short_circuit,
                ..
            } => ProviderResource::Embedded {
                locals: locals.iter().collect(),
                declared: declared.as_deref(),
                precedence: *precedence,
                short_circuit: *short_circuit,
            },
        }
    }

    /// Enumerate every `group` that publishes a package of the bare
    /// `name` — the candidate set short-name resolution (PROP-008
    /// §2.6) walks. The local-directory path scans the registry tree;
    /// the multi-registry path walks each registry's index. The result
    /// is de-duplicated and sorted; `len() > 1` is a collision. Not
    /// part of [`InstallSource`]: qualification is the surface's input
    /// boundary, reached through [`crate::PackageQualifier`].
    pub fn candidate_groups(&self, name: &str) -> Result<Vec<Group>> {
        match self {
            InstallResolver::Local(r, _) => Ok(r.candidate_groups(name)?),
            InstallResolver::Multi(m, _) => Ok(m.resolve_name_candidates(name)),
            InstallResolver::Embedded {
                locals, declared, ..
            } => {
                // The local family is a Vec: union candidate_groups across
                // every local (project-local + vibe-embedded), then layer in
                // the declared walk, then sort + dedup.
                let mut groups = Vec::new();
                for local in locals {
                    groups.extend(local.candidate_groups(name)?);
                }
                if let Some(m) = declared {
                    groups.extend(m.resolve_name_candidates(name));
                }
                groups.sort_by(|a, b| a.as_str().cmp(b.as_str()));
                groups.dedup();
                Ok(groups)
            }
        }
    }
}

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
