//! The metadata-only branch of the existing install planner.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#OPERATION-EFFECTS");

use super::*;

/// Explicit destinations selected by the preview operation, not by package data.
/// The old store is lookup-only; missing retained metadata can acquire only into
/// the same operation-owned scratch that the caller closes after preview.
#[derive(Clone, Copy)]
pub struct PreviewSourceStorage<'a> {
    pub read_only_store: Option<&'a Path>,
    pub acquisition_store: &'a Path,
}

/// A complete source-selected closure, without publishing package payload.
/// Candidate observations remain held by the source owner for this invocation.
#[derive(Debug)]
pub struct InstallPreview {
    selected: Vec<vibe_resolver::CandidateMetadata>,
}
impl InstallPreview {
    /// Reasons retained by the selected metadata, not a new registry probe.
    ///
    /// ```no_run
    /// # fn read(plan: &vibe_install::InstallPreview, group: &vibe_core::Group) {
    /// let _reasons = plan.package_candidates(group, "demo");
    /// # }
    /// ```
    pub fn package_candidates(
        &self,
        group: &vibe_core::Group,
        name: &str,
    ) -> Option<&[vibe_registry::source_candidates::SourceCandidateObservation]> {
        self.selected
            .iter()
            .find(|metadata| {
                metadata.candidate.group() == group && metadata.candidate.name() == name
            })
            .map(|metadata| metadata.observations.as_slice())
    }

    /// Selected source facts in graph order, without claiming payload acquisition.
    ///
    /// ```
    /// # fn inspect(preview: &vibe_install::InstallPreview) {
    /// for (group, name, version, evidence) in preview.package_sources() {
    ///     let _selection = (group, name, version, evidence.source_url);
    /// }
    /// # }
    /// ```
    pub fn package_sources(
        &self,
    ) -> impl Iterator<
        Item = (
            &vibe_core::Group,
            &str,
            &semver::Version,
            vibe_wire::generated::shared::SourceEvidence,
        ),
    > {
        self.selected.iter().map(|metadata| {
            let candidate = &metadata.candidate;
            (
                candidate.group(),
                candidate.name(),
                candidate.version(),
                candidate.source_evidence().clone(),
            )
        })
    }
}

/// Plan the complete closure using a source with an explicit preview storage
/// capability. Exact retained classification may acquire into that scratch,
/// but never writes a persistent store or publishes a package.
///
/// ```no_run
/// # fn call(source: &dyn vibe_install::InstallSource, root: &std::path::Path,
/// # mut manifest: vibe_core::manifest::Manifest, mut workspace: vibe_workspace::Workspace,
/// # request: vibe_install::InstallRequest, lock: &vibe_core::manifest::Lockfile,
/// # observer: &dyn vibe_install::PlanObserver, storage: vibe_install::PreviewSourceStorage<'_>) {
/// let _preview = vibe_install::preview_prepared(source, root, &mut manifest,
///     &mut workspace, lock, storage, request, vibe_core::manifest::SpecFormat::Mixed, observer);
/// # }
/// ```
#[allow(clippy::too_many_arguments)]
pub fn preview_prepared<S: InstallSource + ?Sized>(
    source: &S,
    project_root: &Path,
    manifest: &mut Manifest,
    workspace: &mut Workspace,
    lockfile: &Lockfile,
    storage: PreviewSourceStorage<'_>,
    request: InstallRequest,
    spec_format: SpecFormat,
    observer: &dyn PlanObserver,
) -> Result<InstallPreview> {
    if !storage.acquisition_store.is_absolute() {
        return Err(crate::Error::PreviewStorageRequired);
    }
    let prepared = prepare_metadata_plan(
        source,
        project_root,
        manifest,
        workspace,
        request,
        spec_format,
        observer,
        PlanningPurpose::Preview(lockfile, storage),
    )?;
    let Some(prepared) = prepared else {
        return Err(vibe_registry::RegistryError::MalformedMeta {
            path: project_root.to_owned(),
            reason: "preview preparation returned an apply-only freshness shortcut".into(),
        }
        .into());
    };
    let mut selected = prepared.closure.selected;
    let packages = prepared
        .closure
        .graph
        .iter()
        .map(|node| {
            selected
                .remove(&(node.group.clone(), node.name.clone()))
                .ok_or_else(|| {
                    Error::from(vibe_registry::RegistryError::MalformedMeta {
                        path: project_root.to_owned(),
                        reason: "preview graph lost its selected metadata".into(),
                    })
                })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(InstallPreview { selected: packages })
}

/// Whether the package closure needs a provider; remap offers have a separate owner.
///
/// ```no_run
/// # fn check(w: &vibe_workspace::Workspace, m: &vibe_core::manifest::Manifest,
/// # lock: &vibe_core::manifest::Lockfile) {
/// let _needed = vibe_install::preview_has_requirements(w, m, false, lock);
/// # }
/// ```
pub fn preview_has_requirements(
    workspace: &Workspace,
    manifest: &Manifest,
    explicit: bool,
    lockfile: &Lockfile,
) -> bool {
    explicit
        || roots::has_declarations(workspace)
        || roots::raw_has_declarations(manifest)
        || !lockfile.meta.root_dependencies.is_empty()
}
