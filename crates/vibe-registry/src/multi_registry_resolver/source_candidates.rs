//! Source-aware discovery through the existing selection and acquisition owners.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY");
use super::*;
use crate::source_candidates::*;
use specmark::spec;
use vibe_core::manifest::{CanonicalSourceBinding as Binding, RegistryPin, SourcePolicyError};

impl MultiRegistryResolver {
    /// Pure edge admission against held identity first, otherwise the unfiltered catalog.
    pub fn validate_registry_pin(
        &self,
        group: &Group,
        name: &str,
        pin: &RegistryPin,
    ) -> Result<(), RegistryError> {
        let package = format!("{group}/{name}");
        if let Some(held) = self.locked.get(&package) {
            return vibe_core::manifest::validate_locked_registry_pin(&package, pin, held)
                .map_err(Into::into);
        }
        let kind = if self.overrides()?.contains_key(&package) {
            Some(vibe_core::manifest::SourceKind::Override)
        } else if self.path_packages.contains_key(&package) {
            Some(vibe_core::manifest::SourceKind::Path)
        } else if self.git_packages.contains_key(&package) {
            Some(vibe_core::manifest::SourceKind::Git)
        } else {
            None
        };
        if let Some(kind) = kind {
            return Err(SourcePolicyError::DirectSourceConflict { package, kind }.into());
        }
        vibe_core::manifest::validate_new_registry_pin(
            &package,
            pin,
            &self.declared_registry_rows()?,
        )
        .map_err(Into::into)
    }

    /// Retain the exact default lookup for the legacy lazy metadata callback.
    pub fn retain_default_candidate(
        &self,
        group: &Group,
        name: &str,
        version: &semver::Version,
    ) -> Result<RetainedCandidateObservation, RegistryError> {
        let reference = exact_reference(group, name, version)?;
        let resolution = self.resolve(&reference)?;
        self.retain_resolution(resolution, None)
    }

    /// Capture member identities only from this operation's actual fresh local registry.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY")]
    pub fn capture_subskill_source(
        &self,
        candidate: &RetainedSourceCandidate,
        paths: &std::collections::BTreeSet<String>,
    ) -> Result<crate::subskill_witness::LocalSubskillAdmission, RegistryError> {
        let resolution = candidate.resolution_for(&self.candidate_owner)?;
        if resolution.locked.is_some()
            || resolution.direct.is_some()
            || resolution.from_store
            || resolution.overridden
            || resolution.is_git_source
            || resolution.is_path_source
            || resolution.via_redirect.is_some()
        {
            return Err(invalid_selection(
                resolution,
                "subskill witness requires fresh local registry source admission",
            ));
        }
        let name = resolution.registry_name.as_deref().ok_or_else(|| {
            invalid_selection(resolution, "selected local registry alias is missing")
        })?;
        let registry = self
            .sources()?
            .iter()
            .find(|source| source.name() == name)
            .ok_or_else(|| {
                invalid_selection(resolution, "selected local registry is unavailable")
            })?;
        if !matches!(registry, RegistrySource::Local(_)) {
            return Err(invalid_selection(
                resolution,
                "subskill source mode is not connected",
            ));
        }
        crate::subskill_witness::LocalSubskillAdmission::capture(
            &resolution.resolved.source_dir,
            candidate.manifest(),
            paths,
        )
        .map_err(|error| invalid_selection(resolution, &format!("{error:#}")))
    }

    /// Consume only this owner's retained route; never solve a URL/version again.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY")]
    pub fn fetch_candidate_for_install(
        &self,
        candidate: &RetainedSourceCandidate,
        store: &Path,
        expected: Option<&str>,
        slot: &Path,
        progress: &vibe_core::progress::Progress,
    ) -> Result<crate::PackageAcquisition, RegistryError> {
        let resolution = candidate.resolution_for(&self.candidate_owner)?;
        let result = if matches!(candidate.binding(), Binding::Registry { .. })
            && resolution.locked.is_none()
            && resolution.direct.is_none()
            && !resolution.from_store
        {
            let task = progress.task(format!(
                "Acquiring retained {}/{}@{}",
                candidate.group(),
                candidate.name(),
                candidate.version()
            ));
            let package = format!("{}/{}", candidate.group(), candidate.name());
            let result = crate::git_backend::transfers::observed(
                Some(&task.progress()),
                Some(&package),
                Some(vibe_core::progress::transfer::Purpose::PackageClone),
                || {
                    self.fetch_retained_registry(
                        resolution,
                        candidate.manifest(),
                        store,
                        expected,
                        Some(slot),
                    )
                },
            );
            if result.is_ok() {
                task.finish();
            } else {
                task.fail("retained source acquisition failed");
            }
            result?
        } else {
            // Held/direct/path owners already retain their source authority.
            self.fetch_for_install(resolution, store, expected, slot, progress)?
        };
        let manifest = match &result {
            crate::PackageAcquisition::Cached(cached) => &cached.manifest,
            crate::PackageAcquisition::InPlace(prepared) => prepared.manifest(),
        };
        candidate.validate_manifest(manifest)?;
        Ok(result)
    }

    /// Existing owner grant, including explicit same-source updates, not catalog inference.
    pub fn respects_locked_selection(
        &self,
        candidate: &RetainedSourceCandidate,
        row: &vibe_core::manifest::LockedPackage,
    ) -> Result<bool, RegistryError> {
        let resolution = candidate.resolution_for(&self.candidate_owner)?;
        let key = format!("{}/{}", candidate.group(), candidate.name());
        if self.locked.get(&key) != Some(row) {
            return Ok(false);
        }
        let Some(selection) = &resolution.locked else {
            return Ok(false);
        };
        // A mutable in-workspace directory refresh keeps the held source and
        // version; only its content hash may move (PROP-011 §2.6).
        let mutable_refresh = self.mutable_locked.contains(&key)
            && selection.source_row().source_url == row.source_url
            && selection.source_row().version == row.version;
        Ok(selection.original_row() == row
            && (self.updates_locked(&key)
                || mutable_refresh
                || selection.replayed_row() == Some(row)))
    }
}

fn exact_reference(
    group: &Group,
    name: &str,
    version: &semver::Version,
) -> Result<PackageRef, RegistryError> {
    PackageRef::parse(&format!("{group}/{name}@={version}")).map_err(RegistryError::from)
}
fn invalid_selection(resolution: &MultiResolution, reason: &str) -> RegistryError {
    RegistryError::MalformedMeta {
        path: resolution.resolved.source_dir.clone(),
        reason: reason.into(),
    }
}

mod capture;
mod inventory;
