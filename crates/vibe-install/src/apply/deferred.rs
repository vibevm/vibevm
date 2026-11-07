//! Deferred in-place materialisation after install consent.

use super::*;

pub(super) struct NativePlacement {
    group: vibe_core::Group,
    name: String,
    version: semver::Version,
    _held: vibe_registry::PreparedInPlace,
    staged: vibe_registry::git_backend::shell::native_admission::NativeMaterialisation,
}
impl vibe_workspace::install::PreparedSlotPlacement for NativePlacement {
    fn coordinate(&self) -> (&vibe_core::Group, &str, &semver::Version) {
        (&self.group, &self.name, &self.version)
    }
    fn destination(&self) -> &Path {
        self.staged.destination()
    }
    fn staged_path(&self) -> &Path {
        self.staged.staged_path()
    }
    fn validate(&self) -> std::result::Result<(), String> {
        self.staged
            .validate_before_publication()
            .map_err(|e| e.to_string())
    }
    fn changed(&self) -> bool {
        self.staged.changed()
    }
}

/// All source/transform admission precedes caller metadata publication.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-MATERIALISATION-LAW"
)]
pub(super) fn stage_deferred_native(
    fetched: &[Fetched],
    resolution: &mut [ResolvedDep],
) -> Result<Vec<NativePlacement>> {
    let mut staged = Vec::new();
    for fetched in fetched {
        let Some(held) = &fetched.in_place else {
            continue;
        };
        let Some(candidate) = held.stage_native()? else {
            continue;
        };
        let source = &fetched.cached.resolved;
        let mut rows = resolution.iter_mut().filter(|r| {
            r.group == source.group && r.name == source.name && r.version == source.version
        });
        let row = rows.next().ok_or_else(|| Error::RootNotFetched {
            pkgref: format!("{}/{}", source.group, source.name),
        })?;
        if rows.next().is_some() {
            return Err(Error::RootNotFetched {
                pkgref: format!("duplicate {}/{}", source.group, source.name),
            });
        }
        row.in_place_changed = Some(candidate.changed());
        staged.push(NativePlacement {
            group: source.group.clone(),
            name: source.name.clone(),
            version: source.version.clone(),
            _held: held.clone(),
            staged: candidate,
        });
    }
    if !staged.is_empty() && fetched.iter().filter(|f| f.in_place.is_some()).count() != staged.len()
    {
        return Err(vibe_registry::RegistryError::Git(vibe_registry::GitError::NativeContext{reason:"mixed deferred legacy and native materialisation requires a shared rollback capability"}).into());
    }
    Ok(staged)
}

pub(super) fn workspace_error(error: vibe_workspace::WorkspaceError) -> Error {
    match error {
        vibe_workspace::WorkspaceError::InstallRollback { primary, reason } => {
            Error::RollbackFailed {
                primary: Box::new(Error::Workspace(*primary)),
                reason,
            }
        }
        error => Error::Workspace(error),
    }
}

/// Apply retained in-place preparations. No source/ref selection or network
/// fallback occurs here. The completed placement updates the same fetched and
/// resolution rows consumed by metadata, boot and hook owners.
pub(super) fn materialise_deferred_in_place<S: InstallSource + ?Sized>(
    _source: &S,
    workspace: &Workspace,
    fetched: &mut [Fetched],
    resolution: &mut [ResolvedDep],
    native: &[NativePlacement],
) -> Result<()> {
    for (i, f) in fetched.iter_mut().enumerate() {
        let Some(prepared) = &f.in_place else {
            continue;
        };
        if native.iter().any(|n| {
            n.group == f.cached.resolved.group
                && n.name == f.cached.resolved.name
                && n.version == f.cached.resolved.version
        }) {
            continue;
        }
        let pkgref = exact_pinned_pkgref(&ResolvedNode {
            group: f.cached.resolved.group.clone(),
            name: f.cached.resolved.name.clone(),
            version: f.cached.resolved.version.clone(),
            dependencies: Vec::new(),
            is_root: false,
        });
        crate::configuration_admission::package_manifest(&pkgref, prepared.manifest())?;
        let placed = prepared.materialise()?;
        vibedeps::ensure_gitignored(
            &workspace.root,
            &vibedeps::in_place_slot_rel_path(
                &f.cached.package_meta().group,
                &f.cached.resolved.name,
            ),
        )?;
        // Overwrite the lockfile-carried provenance with the freshly-fetched
        // values; the resolution's manifest follows so boot / hook composition
        // reads the updated tree. The version stays the solver's pick.
        f.cached.manifest = placed.manifest.clone();
        f.cached.content_hash = placed.content_hash;
        f.cached.source_uri = placed.source_uri;
        f.cached.source_ref = Some(placed.source_ref);
        f.cached.resolved_commit = placed.resolved_commit;
        resolution[i].manifest = placed.manifest;
        resolution[i].in_place_changed = Some(placed.changed);
    }
    Ok(())
}
