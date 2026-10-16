//! Install-only selected slot witness carriage into the boot file resolver.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use vibe_core::manifest::SpecFormat;
use vibe_core::{ContentHash, Group};
use vibe_spec::{CompilePlans, FileResolver, SelectedPackage, SelfCoordinate};

use crate::boot::EffectiveBoot;
use crate::compile_trace::ScopeAcquisition;
use crate::{WorkspaceError, vibedeps};

use super::{compile_artifact, compile_static_artifact_with_plans};

/// Exact install resolution evidence for one selected dependency slot. A
/// boot-only reconstruction may lack `source_hash`; descriptor-owned slots
/// refuse rather than treating their own record as an independent pin.
#[derive(Debug, Clone)]
pub(crate) struct SelectedSlotWitness {
    /// Independently declared selected subskill content, never inferred from
    /// a directory or promoted into the package.sources catalogue.
    pub subskills: Vec<crate::install::effective_subskills::CapturedSubskill>,
    pub group: Group,
    pub name: String,
    pub version: semver::Version,
    pub root: PathBuf,
    pub source_hash: Option<ContentHash>,
    pub spec_format: SpecFormat,
}

pub(crate) fn selected_resolver(
    workspace_root: &Path,
    self_coord: &SelfCoordinate,
    witnesses: &[SelectedSlotWitness],
) -> Result<FileResolver, WorkspaceError> {
    let mut selected = BTreeMap::new();
    let mut descriptor_maps = Vec::new();
    for witness in witnesses {
        if !witness.root.is_absolute() || !witness.root.starts_with(workspace_root) {
            return Err(invalid("selected dependency slot is outside the workspace"));
        }
        let key = (witness.group.to_string(), witness.name.clone());
        if selected
            .insert(
                key,
                SelectedPackage::new(witness.version.to_string(), &witness.root),
            )
            .is_some()
        {
            return Err(invalid("selected dependency coordinate is repeated"));
        }
        let map = vibedeps::read_validated_descriptor_map_optional_hash(
            &witness.root,
            &witness.group,
            &witness.name,
            &witness.version,
            witness.source_hash.as_ref(),
            witness.spec_format,
        )
        .map_err(|reason| invalid(&format!("selected descriptor slot: {reason}")))?;
        if let Some(map) = map {
            descriptor_maps.push(map);
        }
    }
    // A world without descriptors retains the old constructor and its exact
    // suffix/slot behavior. Once any selected descriptor is present, all
    // coordinates are pinned to the same resolution that supplied its hash.
    let mut resolver = if descriptor_maps.is_empty() {
        FileResolver::new(workspace_root, self_coord.clone())
    } else {
        FileResolver::with_selected_world(
            workspace_root,
            workspace_root,
            selected,
            self_coord.clone(),
        )
    };
    for map in descriptor_maps {
        resolver = resolver
            .with_descriptor_source_map(map.spec_root().to_owned(), Some(map))
            .map_err(|error| invalid(&error.to_string()))?;
    }
    Ok(resolver)
}

#[allow(
    clippy::too_many_arguments,
    reason = "one install-only static render boundary"
)]
pub(crate) fn render_static_observed_selected(
    boot: &EffectiveBoot,
    workspace_root: &Path,
    self_coord: &SelfCoordinate,
    spec_format: SpecFormat,
    acquisition: Option<&ScopeAcquisition<'_>>,
    plans: CompilePlans,
    witnesses: &[SelectedSlotWitness],
) -> Result<Option<String>, WorkspaceError> {
    let compiled = compile_static_artifact_with_plans(
        boot,
        workspace_root,
        self_coord,
        spec_format,
        Some(witnesses),
        acquisition,
        plans,
        None,
        compile_artifact,
    )?;
    compiled
        .map(|compiled| {
            String::from_utf8(compiled.artifact.into_bytes()).map_err(|error| {
                invalid(&format!(
                    "the built-in STATIC backend returned non-UTF-8 bytes: {error}"
                ))
            })
        })
        .transpose()
}

fn invalid(reason: &str) -> WorkspaceError {
    WorkspaceError::InlineCompile {
        reason: reason.to_owned(),
    }
}

#[cfg(test)]
#[path = "descriptor_reader/tests.rs"]
mod tests;
