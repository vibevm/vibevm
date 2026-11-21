//! The provider the solver sees during one configured operation: source
//! observations pass through the snapshot, so each candidate is evaluated
//! once and every retry of the same operation reads the same answer.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#CANDIDATE-BOUNDARY");

use super::{ConfigurationSnapshot, project};
use crate::{DepProvider, DepProviderError, ResolvedGraph, VersionEnumerator};
use std::collections::BTreeSet;
use vibe_core::{
    Group, PackageRef,
    manifest::{Manifest, PackageConfigurationError},
};

/// The solver's view of one operation: every manifest it is handed is the
/// candidate's evaluated dependency view, captured once in the snapshot.
pub(super) struct ConfiguredProvider<'a, P> {
    pub(super) source: P,
    pub(super) snapshot: &'a ConfigurationSnapshot,
    pub(super) blocked: &'a BTreeSet<(String, String)>,
}
impl<P: VersionEnumerator> DepProvider for ConfiguredProvider<'_, P> {
    fn resolve_version(&self, package: &PackageRef) -> Result<semver::Version, DepProviderError> {
        self.source.resolve_version(package)
    }
    fn validate_selected(&self, graph: &ResolvedGraph) -> Result<(), DepProviderError> {
        for node in graph.iter() {
            let candidates = self.snapshot.candidates.borrow();
            let candidate = candidates
                .get(&(node.group.clone(), node.name.clone(), node.version.clone()))
                .ok_or_else(|| {
                    DepProviderError::Other("selected candidate was not captured".into())
                })?;
            if let Err(error) = &candidate.evaluated {
                return Err(DepProviderError::Configuration(error.clone()));
            }
        }
        self.snapshot.refuse_unmerged_incoming_edges(graph)?;
        self.source.validate_selected(graph)
    }
    fn fetch_manifest(
        &self,
        group: &Group,
        name: &str,
        version: &semver::Version,
    ) -> Result<Manifest, DepProviderError> {
        let key = (group.clone(), name.to_owned(), version.clone());
        if let Some(error) = self.snapshot.source_errors.borrow().get(&key) {
            return Err(error.clone());
        }
        if !self.snapshot.candidates.borrow().contains_key(&key) {
            let original = match self.source.fetch_manifest(group, name, version) {
                Ok(manifest) => manifest,
                Err(error) => {
                    self.snapshot
                        .source_errors
                        .borrow_mut()
                        .insert(key.clone(), error.clone());
                    return Err(error);
                }
            };
            let candidate = self
                .snapshot
                .evaluate_candidate(group, name, original, false);
            self.snapshot
                .candidates
                .borrow_mut()
                .insert(key.clone(), candidate);
        }
        let mut manifest = project(&self.snapshot.candidates.borrow()[&key])?;
        let owner = format!("{group}/{name}");
        let targets: BTreeSet<_> = self
            .blocked
            .iter()
            .filter(|(parent, _)| parent == &owner)
            .map(|(_, target)| target.clone())
            .collect();
        if !targets.is_empty()
            && matches!(
                self.snapshot.candidates.borrow()[&key].evaluated,
                Ok(Some(_))
            )
        {
            return Err(DepProviderError::Configuration(
                PackageConfigurationError::Unsupported {
                    package: owner,
                    reason: "visibility-masked configured declaration provenance is not connected"
                        .into(),
                },
            ));
        }
        crate::mask_manifest_edges(&mut manifest, &targets);
        Ok(manifest)
    }
}
impl<P: VersionEnumerator> VersionEnumerator for ConfiguredProvider<'_, P> {
    fn list_versions(
        &self,
        group: &Group,
        name: &str,
    ) -> Result<Vec<semver::Version>, DepProviderError> {
        let key = (group.clone(), name.to_owned());
        if let Some(versions) = self.snapshot.versions.borrow().get(&key) {
            return versions.clone();
        }
        let versions = self.source.list_versions(group, name);
        self.snapshot
            .versions
            .borrow_mut()
            .insert(key, versions.clone());
        versions
    }
}
