//! Bind package policy and declared original inputs before slot publication.
//! Captured content uses the existing prepared-file/reconcile owner; this is
//! finite admission, not a directory snapshot or source authentication scheme.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-ADMISSION-BOUNDARY");

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use vibe_core::manifest::{Manifest, SelectedSource, SourceSelector, SpecFormat};
use vibe_safefs::SourceObservation;
use vibe_spec::source_policy::{PackagePolicyObservation, observe_package_policy};

use super::slot_diff::PreparedSlotFile;
use crate::WorkspaceError;

pub(super) struct SourcePolicy {
    root: PathBuf,
    observed: PackagePolicyObservation,
    captured: BTreeMap<String, Arc<SourceObservation>>,
}

pub(super) fn refused(path: &Path, reason: impl std::fmt::Display) -> WorkspaceError {
    WorkspaceError::SpecMaterialization {
        path: path.to_owned(),
        reason: format!(
            "violates REQ spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-ADMISSION-BOUNDARY: {reason}; fix surface: package manifest and original declared source inputs"
        ),
    }
}

/// Reuse materializer admission before whole-resolution effects. Captures are
/// deliberately discarded; publication still takes and rechecks its own inputs.
pub(crate) fn validate_source_inputs(
    root: &Path,
    format: SpecFormat,
    known: &Manifest,
) -> Result<(), WorkspaceError> {
    SourcePolicy::observe(root, format, Some(known)).map(|_| ())
}

impl SourcePolicy {
    pub(super) fn root(&self) -> &Path {
        &self.root
    }
    pub(super) fn observe(
        root: &Path,
        format: SpecFormat,
        known: Option<&Manifest>,
    ) -> Result<Self, WorkspaceError> {
        if !root.is_dir() {
            return Err(WorkspaceError::Io {
                path: root.to_owned(),
                reason: "source content tree does not exist or is not a directory".to_owned(),
            });
        }
        if let Some(known) = known {
            crate::install::validate_declared_source_operation(
                known,
                root,
                format,
                known
                    .package
                    .as_ref()
                    .is_some_and(|package| package.materialization.is_in_place()),
            )?;
        }
        let observed = observe_package_policy(root).map_err(|error| refused(root, error))?;
        if let Some(manifest) = observed.manifest() {
            crate::install::validate_declared_source_operation(
                manifest,
                root,
                format,
                manifest
                    .package
                    .as_ref()
                    .is_some_and(|package| package.materialization.is_in_place()),
            )?;
        }
        let declared = |manifest: &Manifest| {
            manifest
                .package
                .as_ref()
                .is_some_and(|package| package.has_explicit_source_policy())
        };
        if let Some(known) = known
            && (declared(known) || observed.manifest().is_some_and(declared))
            && observed.manifest() != Some(known)
        {
            return Err(refused(
                &root.join(Manifest::FILENAME),
                "full observed manifest differs from supplied manifest, or declared metadata is missing",
            ));
        }
        let mut policy = Self {
            root: root.to_owned(),
            observed,
            captured: BTreeMap::new(),
        };
        if policy.observed.manifest_bytes().is_some() {
            policy.capture(Manifest::FILENAME)?;
            if policy.captured[Manifest::FILENAME].bytes()
                != policy.observed.manifest_bytes().unwrap_or_default()
            {
                return Err(refused(
                    &root.join(Manifest::FILENAME),
                    "manifest changed between policy and payload observations",
                ));
            }
        }
        let paths = policy.observed.package_sources().unwrap_or(&[]).iter().map(|descriptor| match descriptor.selector() {
            SourceSelector::File(path) => Ok(path.clone()),
            SourceSelector::Directory(path) => Err(refused(&root.join(path), "directory source declarations require a complete census; this materializer admits exact-file declarations only")),
        }).collect::<Result<Vec<_>, _>>()?;
        for path in paths {
            let package = policy
                .observed
                .package()
                .ok_or_else(|| refused(root, "declared package metadata unavailable"))?;
            match package.select_source(&path).map_err(|error| {
                refused(
                    &root.join(&path),
                    format!("source selection failed: {error:?}"),
                )
            })? {
                SelectedSource::Descriptor { .. } => {}
                SelectedSource::NeedsInput {
                    declaration_indices,
                    ..
                } => {
                    return Err(refused(
                        &root.join(&path),
                        format!("source declaration conflict at rows {declaration_indices:?}"),
                    ));
                }
                SelectedSource::Unclassified { .. } => {
                    return Err(refused(
                        &root.join(&path),
                        "declared source has no selected interpretation",
                    ));
                }
            }
            if !policy.captured.contains_key(&path) {
                policy.capture(&path)?;
            }
        }
        policy.recheck()?;
        Ok(policy)
    }

    fn capture(&mut self, relative: &str) -> Result<(), WorkspaceError> {
        let observed = self
            .observed
            .read_source_observation(relative)
            .map_err(|error| refused(&self.root.join(relative), error))?
            .ok_or_else(|| refused(&self.root.join(relative), "declared input is missing"))?;
        self.captured
            .insert(relative.to_owned(), Arc::new(observed));
        Ok(())
    }

    pub(super) fn captured(&self, relative: &Path) -> Option<Arc<SourceObservation>> {
        self.captured.get(&crate::path_to_slash(relative)).cloned()
    }

    pub(super) fn recheck(&self) -> Result<(), WorkspaceError> {
        self.observed
            .recheck_unchanged()
            .map_err(|error| refused(&self.root, error))?;
        for (relative, captured) in &self.captured {
            let current = self
                .observed
                .read_source_observation(relative)
                .map_err(|error| refused(&self.root.join(relative), error))?
                .ok_or_else(|| {
                    refused(
                        &self.root.join(relative),
                        "captured input disappeared before publication",
                    )
                })?;
            if current.identity() != captured.identity()
                || current.bytes() != captured.bytes()
                || current.permissions() != captured.permissions()
            {
                return Err(refused(
                    &self.root.join(relative),
                    "captured input identity, bytes or permissions changed before publication",
                ));
            }
        }
        Ok(())
    }

    pub(super) fn validate_prepared(
        &self,
        files: &[PreparedSlotFile],
    ) -> Result<(), WorkspaceError> {
        for relative in self.captured.keys() {
            if !files.iter().any(|file| file.path_wire() == *relative) {
                return Err(refused(
                    &self.root.join(relative),
                    "captured input is not part of the materializer's payload footprint",
                ));
            }
        }
        for file in files {
            let relative = file.path_wire();
            if self.observed.package_sources().is_some()
                && relative.eq_ignore_ascii_case(super::SLOT_RECORD_FILENAME)
            {
                return Err(refused(
                    &self.root.join(&relative),
                    format!(
                        "payload path `{relative}` collides with reserved destination `{}`",
                        super::SLOT_RECORD_FILENAME
                    ),
                ));
            }
        }
        self.recheck()
    }
}

#[cfg(test)]
mod tests;
