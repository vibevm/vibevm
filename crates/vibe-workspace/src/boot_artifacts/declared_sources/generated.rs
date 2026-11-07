//! Operation-local observations admitted by the existing unit freshness owner.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-INSTALLED-READ",
    r = 1
);
use super::*;
use crate::boot::EffectiveBoot;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GeneratedObservation {
    path: PathBuf,
    group: String,
    name: String,
    relative: String,
    bytes: Vec<u8>,
    index: Vec<u8>,
}
impl DeclaredBootSources {
    pub(crate) fn root(&self) -> &Path {
        &self.root
    }
    pub(crate) fn capture_generated(
        &self,
        boot: &EffectiveBoot,
        format: SpecFormat,
        self_coord: &SelfCoordinate,
    ) -> Result<Vec<GeneratedObservation>, WorkspaceError> {
        let mut observations = Vec::new();
        for entry in boot
            .static_entries()
            .filter(|entry| entry.unit_substituted && !entry.elided && !entry.use_ref)
        {
            let BootProvenance::Dependency { group, name } = &entry.provenance else {
                return Err(error(&self.root, "generated input lacks selected provider"));
            };
            let selected = self
                .selected
                .get(&(group.to_string(), name.clone()))
                .ok_or_else(|| error(&self.root, "generated input provider is not selected"))?;
            let relative =
                vibe_core::layout::current_boot_dir().join(super::super::static_file(format));
            let path = selected.root.join(&relative);
            if self.root.join(&entry.path) != path {
                return Err(error(
                    &path,
                    "generated input differs from selected render target",
                ));
            }
            let stale = vibe_core::layout::current_boot_dir()
                .join(super::super::publication::stale_static_file(format));
            // Read through the held selected slot: absence is required, and a
            // substituted symlink or special file is a read refusal.
            let slot_relative = selected
                .root
                .strip_prefix(&self.root)
                .map_err(|_| error(&path, "generated slot escapes workspace"))?;
            let spelling = crate::path_to_slash(slot_relative);
            let slot = self
                .project
                .dir(&spelling.split('/').collect::<Vec<_>>(), false)
                .map_err(|e| error(&path, e))?;
            if self
                .project
                .read_source_bytes_in(&slot, &crate::path_to_slash(&stale))
                .map_err(|e| error(&path, e))?
                .is_some()
            {
                return Err(error(
                    &path,
                    "generated artifact has a stale representation sibling",
                ));
            }
            let relative = crate::path_to_slash(&relative);
            let (bytes, _) =
                self.generated_observation(group.as_str(), name, &relative, None, self_coord)?;
            let (index, _) = self.generated_observation(
                group.as_str(),
                name,
                &crate::path_to_slash(&vibe_core::layout::current_boot_index()),
                None,
                self_coord,
            )?;
            observations.push(GeneratedObservation {
                path,
                group: group.to_string(),
                name: name.clone(),
                relative,
                bytes,
                index,
            });
        }
        Ok(observations)
    }
    pub(crate) fn retain_generated(&mut self, observations: Vec<GeneratedObservation>) {
        for observation in observations {
            Arc::make_mut(&mut self.generated).insert(observation.path.clone(), observation);
        }
    }
    pub(super) fn checked_generated_observation(
        &self,
        group: &str,
        name: &str,
        relative: &str,
        overlay: Option<&[u8]>,
        self_coord: &SelfCoordinate,
    ) -> Result<(Vec<u8>, vibe_spec::source_policy::PackagePolicyCheckpoint), WorkspaceError> {
        let captured = self.generated_observation(group, name, relative, overlay, self_coord)?;
        // This private overlay is populated only from prepared replay
        // publications in dependency order. It supersedes the old disk unit.
        if overlay.is_some() {
            return Ok(captured);
        }
        let selected = self
            .selected
            .get(&(group.into(), name.into()))
            .ok_or_else(|| error(&self.root, "generated provider is not selected"))?;
        let path = selected.root.join(relative);
        let expected = self.generated.get(&path).ok_or_else(|| {
            error(
                &path,
                "generated artifact lacks validated reuse observation",
            )
        })?;
        let (index, _) = self.generated_observation(
            group,
            name,
            &crate::path_to_slash(&vibe_core::layout::current_boot_index()),
            None,
            self_coord,
        )?;
        if captured.0 != expected.bytes || index != expected.index {
            return Err(error(
                &path,
                "generated artifact changed after reuse validation",
            ));
        }
        Ok(captured)
    }
}
