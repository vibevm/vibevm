//! Immediate authored instruction scope; neither addressed lookup nor inventory.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-HOST-BOOT",
    r = 1
);
use super::inventory::Membership;
use super::*;
use std::collections::BTreeSet;
use vibe_core::manifest::{
    SelectedSource, SourceInterpretation, SourceLoading, SourceSelector, select_source_rows,
};

/// Original selected instruction bytes; media never follows a filename suffix.
#[derive(Debug)]
pub struct CapturedBootResource {
    relative: String,
    media: String,
    observation: vibe_safefs::SourceObservation,
}
impl CapturedBootResource {
    pub fn relative(&self) -> &str {
        &self.relative
    }
    pub fn media(&self) -> &str {
        &self.media
    }
    pub fn bytes(&self) -> &[u8] {
        self.observation.bytes()
    }
}
/// Finite immediate scope retained with its original policy observation.
#[derive(Debug)]
pub struct CapturedAuthoredBoot {
    resources: Vec<CapturedBootResource>,
    membership: Membership,
    root: PathBuf,
    identity: FileIdentity,
    manifest: Option<Vec<u8>>,
}
impl CapturedAuthoredBoot {
    pub fn resources(&self) -> &[CapturedBootResource] {
        &self.resources
    }
}
impl PackagePolicyObservation {
    fn boot_membership(&self) -> Result<(Membership, Vec<(String, String)>), SourcePolicyError> {
        self.recheck_unchanged()?;
        if self.materialized_record_present()?
            || self
                .state
                .logical_root
                .ancestors()
                .any(crate::resolver::is_materialized_slot_root)
            || self
                .project
                .root_path()
                .ancestors()
                .any(crate::resolver::is_materialized_slot_root)
        {
            return Err(self.unsupported("original authored boot in a materialized root"));
        }
        let rows = self
            .source_rows()
            .ok_or_else(|| self.unsupported("authored boot capture without declared policy"))?;
        let scope = vibe_core::layout::current_boot_dir();
        let prefix = scope
            .to_str()
            .ok_or_else(|| self.inventory_error("nonportable boot scope"))?
            .replace('\\', "/");
        let mut membership = Membership::default();
        let directory = self.inventory_directory(&prefix, &mut membership)?;
        let mut candidates = BTreeSet::new();
        if let Some(directory) = &directory {
            for name in self
                .project
                .child_names(directory)
                .map_err(|e| self.inventory_error(e.to_string()))?
            {
                if vibe_core::manifest::declarant_path_component(&name).is_ok() {
                    candidates.insert(format!("{prefix}/{name}"));
                }
            }
        }
        let mut exact = BTreeSet::new();
        for row in rows {
            if let SourceSelector::File(path) = row.selector()
                && Path::new(path).parent() == Some(scope.as_path())
            {
                candidates.insert(path.clone());
                exact.insert(path.clone());
            }
        }
        let mut selected = Vec::new();
        let mut claims = BTreeSet::new();
        let mut identities = BTreeSet::new();
        for path in candidates {
            // The two generated system-prompt outputs arrive here for free,
            // as members of the layout owner's generated list. The authored
            // template needs its own arm: it is reserved but not generated,
            // so it is excluded from automatic enrolment without ever being
            // owned or removed (PROP-076 `#SYS-LAYOUT-RESERVATION`).
            if vibe_core::layout::is_generated_boot_output(Path::new(&path))
                || vibe_core::layout::is_reserved_boot_authored_input(Path::new(&path))
            {
                continue;
            }
            // The string is a diagnostic identity only, not a native coordinate.
            let descriptor = match select_source_rows(
                &self.state.logical_root.display().to_string(),
                rows,
                &path,
            )
            .map_err(|e| self.inventory_error(format!("invalid boot path: {e:?}")))?
            {
                SelectedSource::Descriptor { descriptor, .. } => descriptor,
                SelectedSource::NeedsInput {
                    declaration_indices,
                    ..
                } => {
                    if declaration_indices.iter().any(|index| {
                        rows[*index].interpretation() == SourceInterpretation::VibeSpec
                            && rows[*index].loading() == SourceLoading::Scan
                    }) {
                        return Err(
                            self.inventory_error(format!("ambiguous authored boot source: {path}"))
                        );
                    }
                    continue;
                }
                SelectedSource::Unclassified { .. } => continue,
            };
            if descriptor.interpretation() != SourceInterpretation::VibeSpec
                || descriptor.loading() != SourceLoading::Scan
            {
                continue;
            }
            let name = path
                .rsplit('/')
                .next()
                .ok_or_else(|| self.inventory_error("empty boot path"))?;
            let present = match &directory {
                Some(directory) => self
                    .project
                    .source_file_present_in(directory, name)
                    .map_err(|e| self.inventory_error(e.to_string()))?,
                None => false,
            };
            if !present {
                if exact.contains(&path) {
                    return Err(self.inventory_error(format!(
                        "exact authored boot source absent or not regular: {path}"
                    )));
                }
                continue;
            }
            if !claims.insert(crate::canonical_doc_path(&path))
                || !identities.insert(vibe_safefs::path_identity_key(&path))
            {
                return Err(
                    self.inventory_error(format!("ambiguous authored native boot claim: {path}"))
                );
            }
            membership.files.insert(path.clone());
            selected.push((path, descriptor.media().into()));
        }
        // With no selected file there is no source-parent witness to retain.
        // Generated boot publication may create the empty scope itself; a new
        // eligible authored member is still detected by the selected file set.
        if membership.files.is_empty() {
            membership.directories.clear();
        }
        self.recheck_unchanged()?;
        Ok((membership, selected))
    }
    /// Capture native scan instructions only within the incumbent immediate scope.
    pub fn capture_authored_boot(&self) -> Result<CapturedAuthoredBoot, SourcePolicyError> {
        let (membership, selected) = self.boot_membership()?;
        let mut resources = Vec::new();
        for (relative, media) in selected {
            let observation = self
                .read_source_observation(&relative)?
                .ok_or_else(|| self.inventory_error("selected authored boot disappeared"))?;
            resources.push(CapturedBootResource {
                relative,
                media,
                observation,
            });
        }
        let captured = CapturedAuthoredBoot {
            resources,
            membership,
            root: self.state.logical_root.clone(),
            identity: self.state.identity,
            manifest: self.state.bytes.clone(),
        };
        self.recheck_authored_boot(&captured)?;
        Ok(captured)
    }
    /// Recheck the original policy, selected membership, parents, identities and bytes.
    pub fn recheck_authored_boot(
        &self,
        captured: &CapturedAuthoredBoot,
    ) -> Result<(), SourcePolicyError> {
        if captured.root != self.state.logical_root
            || captured.identity != self.state.identity
            || captured.manifest != self.state.bytes
        {
            return Err(
                self.inventory_error("authored boot belongs to a different policy observation")
            );
        }
        if self.boot_membership()?.0 != captured.membership {
            return Err(self.inventory_error("authored boot membership or parent changed"));
        }
        for resource in &captured.resources {
            let current = self
                .read_source_observation(&resource.relative)?
                .ok_or_else(|| self.inventory_error("authored boot disappeared"))?;
            if current.identity() != resource.observation.identity()
                || current.bytes() != resource.bytes()
            {
                return Err(self.inventory_error("authored boot bytes or identity changed"));
            }
        }
        self.recheck_unchanged()
    }
}

#[cfg(test)]
mod tests;
