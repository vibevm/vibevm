//! Existing source dispatch helpers; retained selection remains in one owner.
specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");
use super::*;

impl InstallResolver {
    pub(super) fn held_for(&self, pkg: &PackageRef) -> Option<&MultiRegistryResolver> {
        let group = pkg.group.as_ref()?;
        let held = match self {
            Self::Multi(held, _) | Self::LockedLocal { held, .. } => Some(held.as_ref()),
            Self::Embedded { declared, .. } => declared.as_deref(),
            Self::Local(..) => None,
        }?;
        held.has_locked_package(group, pkg.name.as_str())
            .then_some(held)
    }
    pub(super) fn resolve_and_fetch_inner(
        &self,
        pkgref: &PackageRef,
        store_root: &Path,
        expected_hash: Option<&str>,
        progress: Option<&vibe_core::progress::Progress>,
    ) -> Result<CachedPackage, RegistryError> {
        if let Some(held) = self.held_for(pkgref) {
            let selected = held.resolve_for(pkgref, ResolutionPurpose::DownloadSource)?;
            return held.fetch_with_expected_hash(&selected, store_root, expected_hash);
        }
        match self {
            InstallResolver::Local(r, _) | InstallResolver::LockedLocal { local: r, .. } => {
                let resolved = r.resolve(pkgref)?;
                r.fetch(&resolved, store_root)
            }
            InstallResolver::Multi(m, _) => {
                let resolution = match progress {
                    Some(progress) => m.resolve_for_with_progress(
                        pkgref,
                        ResolutionPurpose::DownloadSource,
                        progress,
                    )?,
                    None => m.resolve_for(pkgref, ResolutionPurpose::DownloadSource)?,
                };
                match progress {
                    Some(progress) => m.fetch_with_expected_hash_progress(
                        &resolution,
                        store_root,
                        expected_hash,
                        progress,
                    ),
                    None => m.fetch_with_expected_hash(&resolution, store_root, expected_hash),
                }
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
                            let resolution = match progress {
                                Some(progress) => m.resolve_for_with_progress(
                                    pkgref,
                                    ResolutionPurpose::DownloadSource,
                                    progress,
                                )?,
                                None => m.resolve_for(pkgref, ResolutionPurpose::DownloadSource)?,
                            };
                            match progress {
                                Some(progress) => m.fetch_with_expected_hash_progress(
                                    &resolution,
                                    store_root,
                                    expected_hash,
                                    progress,
                                ),
                                None => m.fetch_with_expected_hash(
                                    &resolution,
                                    store_root,
                                    expected_hash,
                                ),
                            }
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

    /// The selected `(ProviderCell, solver override)` pair — the one flag
    /// decision all three solve-path methods share (R-001: decided in the
    /// cells module, only routed here).
    pub(super) fn cell_selection(&self) -> (ProviderCell, Option<&'static str>) {
        match self {
            InstallResolver::Local(_, s) => (ProviderCell::Local, *s),
            InstallResolver::LockedLocal { solver, .. } => (ProviderCell::Embedded, *solver),
            InstallResolver::Multi(_, s) => (ProviderCell::Multi, *s),
            InstallResolver::Embedded { solver, .. } => (ProviderCell::Embedded, *solver),
        }
    }

    /// The borrowed [`ProviderResource`] this resolver's variant owns — the
    /// one routing shape `solve` / `manifest_of` / `solve_masked` all hand to
    /// the cells module's construction sites (R-001).
    pub(super) fn provider_resource(&self) -> ProviderResource<'_> {
        match self {
            InstallResolver::Local(r, _) => ProviderResource::Local(r),
            InstallResolver::LockedLocal { local, held, .. } => ProviderResource::Embedded {
                locals: vec![local],
                declared: Some(held),
                precedence: vibe_resolver::EmbeddedPrecedence::EmbeddedFirst,
                short_circuit: true,
            },
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
            InstallResolver::Local(r, _) | InstallResolver::LockedLocal { local: r, .. } => {
                Ok(r.candidate_groups(name)?)
            }
            InstallResolver::Multi(m, _) => Ok(m.resolve_name_candidates(name)?),
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
                    groups.extend(m.resolve_name_candidates(name)?);
                }
                groups.sort_by(|a, b| a.as_str().cmp(b.as_str()));
                groups.dedup();
                Ok(groups)
            }
        }
    }
}
