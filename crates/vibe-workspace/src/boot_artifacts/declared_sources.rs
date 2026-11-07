//! Install-selected declared inputs carried into the existing artifact readers.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-INSTALLED-READ",
    r = 1
);
use crate::boot::{BootEntry, BootProvenance};
use crate::install::ResolvedDep;
use crate::{WorkspaceError, vibedeps};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use vibe_core::manifest::{SourceInterpretation, SpecFormat};
use vibe_safefs::Project;
use vibe_spec::{
    FileResolver, PreparedInstalledSource, PreparedOutputRow, SelectedPackage, SelfCoordinate,
    SpecAddress,
};
mod generated;
mod source;
pub(crate) use source::BootSectionSource;

#[derive(Debug, Clone)]
pub(crate) struct DeclaredBootSources {
    project: Arc<Project>,
    root: PathBuf,
    selected: BTreeMap<(String, String), SelectedPackage>,
    bindings: BTreeMap<(String, String), PreparedInstalledSource>,
    generated: Arc<BTreeMap<PathBuf, generated::GeneratedObservation>>,
}
impl DeclaredBootSources {
    pub(crate) fn validate_emission(
        resolution: &[ResolvedDep],
        sources: Option<&Self>,
    ) -> Result<(), WorkspaceError> {
        for dep in resolution.iter().filter(|dep| {
            dep.manifest
                .package
                .as_ref()
                .is_some_and(|package| package.has_explicit_source_policy())
        }) {
            let selected_hash = dep.source_hash.as_ref().ok_or_else(|| {
                error(
                    &dep.content_dir,
                    "declared unit has no independently selected source hash",
                )
            })?;
            let binding = sources
                .and_then(|sources| {
                    sources
                        .bindings
                        .get(&(dep.group.to_string(), dep.name.clone()))
                })
                .ok_or_else(|| {
                    error(
                        &dep.content_dir,
                        "declared unit has no independently selected read binding",
                    )
                })?;
            let expected = binding.expectation();
            if &expected.source_hash != selected_hash
                || expected.version != dep.version.to_string()
                || expected.kind != dep.kind
            {
                return Err(error(
                    &dep.content_dir,
                    "unit emission selection differs from prepared read binding",
                ));
            }
        }
        Ok(())
    }
    pub(crate) fn prepare(
        root: &Path,
        resolution: &[ResolvedDep],
        input_format: Option<SpecFormat>,
    ) -> Result<Option<Self>, WorkspaceError> {
        if !resolution.iter().any(|dep| {
            dep.manifest
                .package
                .as_ref()
                .is_some_and(|p| p.has_explicit_source_policy())
        }) {
            return Ok(None);
        }
        // Only the selected workspace is a caller root; slots are held descendants.
        let physical = root.canonicalize().map_err(|e| error(root, e))?;
        let project = Arc::new(Project::open(&physical).map_err(|e| error(root, e))?);
        let mut selected = BTreeMap::new();
        let mut bindings = BTreeMap::new();
        for dep in resolution {
            let relative = if dep
                .manifest
                .package
                .as_ref()
                .is_some_and(|p| p.materialization.is_in_place())
            {
                vibedeps::in_place_slot_rel_path(&dep.group, &dep.name)
            } else {
                vibedeps::slot_rel_path(&dep.group, &dep.name, &dep.version)
            };
            let key = (dep.group.to_string(), dep.name.clone());
            if selected
                .insert(
                    key.clone(),
                    SelectedPackage::new(dep.version.to_string(), root.join(&relative)),
                )
                .is_some()
            {
                return Err(error(root, "duplicate selected package"));
            }
            if dep
                .manifest
                .package
                .as_ref()
                .is_some_and(|p| p.has_explicit_source_policy())
            {
                let slot = project
                    .dir(&relative.split('/').collect::<Vec<_>>(), false)
                    .map_err(|e| error(&root.join(&relative), e))?;
                let binding = vibedeps::prepare_declared_slot_read(
                    &project,
                    &slot,
                    &relative,
                    dep,
                    input_format,
                )?
                .ok_or_else(|| {
                    error(
                        slot.path(),
                        "declared selected package lost its source policy",
                    )
                })?;
                bindings.insert(key, binding);
            }
        }
        Ok(Some(Self {
            project,
            root: root.to_owned(),
            selected,
            bindings,
            generated: Arc::new(BTreeMap::new()),
        }))
    }
    fn generated_observation(
        &self,
        group: &str,
        name: &str,
        relative: &str,
        overlay: Option<&[u8]>,
        self_coord: &SelfCoordinate,
    ) -> Result<(Vec<u8>, vibe_spec::source_policy::PackagePolicyCheckpoint), WorkspaceError> {
        let selected = self
            .selected
            .get(&(group.into(), name.into()))
            .ok_or_else(|| error(&self.root, "generated package is not selected"))?;
        let slot_relative = selected
            .root
            .strip_prefix(&self.root)
            .map_err(|_| error(&selected.root, "selected generated slot escapes workspace"))?;
        let spelling = crate::path_to_slash(slot_relative);
        let slot = self
            .project
            .dir(&spelling.split('/').collect::<Vec<_>>(), false)
            .map_err(|e| error(&selected.root, e))?;
        let policy = vibe_spec::source_policy::observe_package_policy_in(&self.project, &slot)
            .map_err(|e| error(&selected.root, e))?;
        let resolver = self.resolver(self_coord)?;
        if self.bindings.contains_key(&(group.into(), name.into())) {
            resolver
                .recheck_installed_package(group, name)
                .map_err(|e| error(&selected.root, e))?;
        }
        let bytes = match overlay {
            Some(bytes) => bytes.to_vec(),
            None => self
                .project
                .read_source_bytes_in(&slot, relative)
                .map_err(|e| error(&selected.root, e))?
                .ok_or_else(|| error(&selected.root, "generated artifact is absent"))?,
        };
        policy
            .recheck_unchanged()
            .map_err(|e| error(&selected.root, e))?;
        if self.bindings.contains_key(&(group.into(), name.into())) {
            resolver
                .recheck_installed_package(group, name)
                .map_err(|e| error(&selected.root, e))?;
        }
        Ok((bytes, policy))
    }
    pub(crate) fn resolver(
        &self,
        self_coord: &SelfCoordinate,
    ) -> Result<FileResolver, WorkspaceError> {
        FileResolver::with_selected_world(
            &self.root,
            &self.root,
            self.selected.clone(),
            self_coord.clone(),
        )
        .with_installed_source_bindings(
            self.project.clone(),
            self.bindings.values().cloned().collect(),
            None,
        )
        .map_err(|e| error(&self.root, e))
    }
    fn entry_row(
        &self,
        entry: &BootEntry,
    ) -> Result<Option<(&PreparedInstalledSource, &PreparedOutputRow)>, WorkspaceError> {
        if entry.unit_substituted {
            return Ok(None);
        }
        let BootProvenance::Dependency { group, name } = &entry.provenance else {
            return Ok(None);
        };
        let Some(binding) = self.bindings.get(&(group.to_string(), name.clone())) else {
            return Ok(None);
        };
        let relative = Path::new(&entry.path)
            .strip_prefix(binding.slot_relative())
            .map_err(|_| {
                error(
                    Path::new(&entry.path),
                    "boot input lies outside its selected slot",
                )
            })?;
        let output = crate::path_to_slash(relative);
        let row = binding
            .outputs()
            .iter()
            .find(|row| row.output_path == output)
            .ok_or_else(|| {
                error(
                    Path::new(&entry.path),
                    "boot input is not a recorded declared output",
                )
            })?;
        if row.interpretation != Some(SourceInterpretation::VibeSpec) {
            return Err(error(
                Path::new(&entry.path),
                "ordinary resource is not a native boot source",
            ));
        }
        Ok(Some((binding, row)))
    }
    pub(crate) fn snippet(
        &self,
        dep: &ResolvedDep,
        source: &Path,
    ) -> Result<Option<String>, WorkspaceError> {
        let Some(binding) = self
            .bindings
            .get(&(dep.group.to_string(), dep.name.clone()))
        else {
            return Ok(None);
        };
        let original = crate::path_to_slash(source);
        let row = binding.output_for_source(&original).ok_or_else(|| {
            error(
                source,
                "declared boot source has no selected recorded output",
            )
        })?;
        if row.interpretation != Some(SourceInterpretation::VibeSpec)
            || row.native_address.is_none()
        {
            return Err(error(
                source,
                "boot requires a selected native VibeSpec source",
            ));
        }
        Ok(Some(format!(
            "{}/{}",
            binding.slot_relative(),
            row.output_path
        )))
    }
    pub(crate) fn normal_seed(
        &self,
        entry: &BootEntry,
    ) -> Result<Option<SpecAddress>, WorkspaceError> {
        let Some((binding, row)) = self.entry_row(entry)? else {
            return Ok(None);
        };
        let expected = binding.expectation();
        let address = row.native_address.as_ref().ok_or_else(|| {
            error(
                Path::new(&entry.path),
                "native output has no original logical address",
            )
        })?;
        SpecAddress::parse(&format!(
            "spec://{}/{}/{}",
            expected.group, expected.name, address
        ))
        .map(Some)
        .map_err(|e| error(Path::new(&entry.path), e))
    }
    pub(crate) fn simple(
        &self,
        entry: &BootEntry,
        self_coord: &SelfCoordinate,
        overlay: Option<&[u8]>,
    ) -> Result<Option<String>, WorkspaceError> {
        let Some((binding, row)) = self.entry_row(entry)? else {
            return Ok(None);
        };
        let expected = binding.expectation();
        let bytes = self
            .resolver(self_coord)?
            .read_installed_source(
                expected.group.as_str(),
                &expected.name,
                &row.source_path,
                overlay,
            )
            .map_err(|e| error(Path::new(&entry.path), e))?;
        vibe_spec::project_declared_native(&bytes, &row.media, SourceInterpretation::VibeSpec)
            .map(Some)
            .map_err(|e| error(Path::new(&entry.path), e))
    }
}

pub(super) fn error(path: &Path, reason: impl std::fmt::Display) -> WorkspaceError {
    WorkspaceError::SpecMaterialization {
        path: path.to_owned(),
        reason: format!(
            "violates REQ spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-INSTALLED-READ-LAW: {reason}; fix surface: selected installed boot source binding"
        ),
    }
}

/// Standalone callers retain legacy reading but cannot guess a declared policy.
pub(super) fn refuse_unbound_declared(
    root: &Path,
    entry: &BootEntry,
) -> Result<(), WorkspaceError> {
    unbound_guards(root, entry).map(|_| ())
}
pub(super) fn unbound_guards(
    root: &Path,
    entry: &BootEntry,
) -> Result<Vec<vibe_spec::source_policy::PackagePolicyCheckpoint>, WorkspaceError> {
    let relative = Path::new(&entry.path);
    vibe_core::manifest::declarant_path(relative).map_err(|e| error(relative, e.reason()))?;
    let physical = root.canonicalize().map_err(|e| error(root, e))?;
    let project = Project::open(&physical).map_err(|e| error(root, e))?;
    let mut components = entry.path.split('/').collect::<Vec<_>>();
    components.pop();
    let mut guards = Vec::new();
    loop {
        let directory = project
            .dir(&components, false)
            .map_err(|e| error(relative, e))?;
        let policy = vibe_spec::source_policy::observe_package_policy_in(&project, &directory)
            .map_err(|e| error(relative, e))?;
        match policy.mode() {
            vibe_spec::source_policy::PackageSourceMode::Explicit
            | vibe_spec::source_policy::PackageSourceMode::ExplicitEmpty => {
                return Err(error(
                    relative,
                    "declared boot input has no independently selected binding",
                ));
            }
            vibe_spec::source_policy::PackageSourceMode::Legacy => {
                guards.push(policy);
                return Ok(guards);
            }
            vibe_spec::source_policy::PackageSourceMode::MissingManifest => {}
        }
        guards.push(policy);
        if components.pop().is_none() {
            return Ok(guards);
        }
    }
}
