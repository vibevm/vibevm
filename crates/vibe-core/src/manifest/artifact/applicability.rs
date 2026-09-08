//! Pure host-OS projection for `[[artifacts.package]]` targets.

use std::collections::{BTreeMap, BTreeSet};

use super::{ArtifactInput, ArtifactPackageTarget, ArtifactsError, ArtifactsSection};
use crate::manifest::{TargetApplicability, TargetOs};

/// The active package target set and the complete authored decision trail.
#[derive(Debug)]
pub struct PackageTargetProjection<'a> {
    active: Vec<&'a ArtifactPackageTarget>,
    decisions: Vec<TargetApplicability>,
    active_outputs: BTreeSet<String>,
    package_producers: BTreeMap<String, (&'a str, bool)>,
    os: TargetOs,
}

impl<'a> PackageTargetProjection<'a> {
    #[must_use]
    pub fn active(&self) -> &[&'a ArtifactPackageTarget] {
        &self.active
    }

    #[must_use]
    pub fn decisions(&self) -> &[TargetApplicability] {
        &self.decisions
    }

    /// Build outputs plus outputs of active package targets.
    #[must_use]
    pub fn active_output_ids(&self) -> &BTreeSet<String> {
        &self.active_outputs
    }

    /// Refuse an active consumer that names an inactive package producer.
    pub fn require_active_artifact(
        &self,
        consumer: &str,
        artifact: &str,
    ) -> Result<(), ArtifactsError> {
        match self.package_producers.get(artifact) {
            Some((producer, false)) => Err(ArtifactsError::ActiveConsumerInactiveProducer {
                consumer: consumer.to_owned(),
                producer: (*producer).to_owned(),
                artifact: artifact.to_owned(),
                os: self.os.to_string(),
            }),
            _ => Ok(()),
        }
    }
}

impl ArtifactsSection {
    /// Project package targets against one injected OS without probing the host.
    pub fn project_package_targets(
        &self,
        os: TargetOs,
    ) -> Result<PackageTargetProjection<'_>, ArtifactsError> {
        let decisions = self
            .package
            .iter()
            .map(|target| TargetApplicability::decide(&target.id, target.when.as_ref(), os))
            .collect::<Vec<_>>();
        let mut package_producers = BTreeMap::new();
        let mut active_outputs = self
            .build
            .iter()
            .flat_map(|target| target.outputs.iter().map(|output| output.id.clone()))
            .collect::<BTreeSet<_>>();
        for (target, decision) in self.package.iter().zip(&decisions) {
            for output in &target.outputs {
                package_producers.insert(
                    output.id.clone(),
                    (target.id.as_str(), decision.is_active()),
                );
                if decision.is_active() {
                    active_outputs.insert(output.id.clone());
                }
            }
        }
        let active = self
            .package
            .iter()
            .zip(&decisions)
            .filter_map(|(target, decision)| decision.is_active().then_some(target))
            .collect::<Vec<_>>();
        let projection = PackageTargetProjection {
            active,
            decisions,
            active_outputs,
            package_producers,
            os,
        };
        for target in &projection.active {
            for artifact in target
                .inputs
                .iter()
                .flatten()
                .filter_map(ArtifactInput::artifact_ref)
            {
                projection.require_active_artifact(&target.id, artifact)?;
            }
        }
        Ok(projection)
    }
}

#[cfg(test)]
mod tests {
    use crate::manifest::{Manifest, TargetApplicability, TargetOs};

    fn parse(rows: &str) -> Manifest {
        Manifest::parse_str(&format!("[project]\nname='demo'\nversion='0.1.0'\n{rows}")).unwrap()
    }

    #[test]
    fn projection_is_ordered_and_keeps_build_outputs_active() {
        let manifest = parse(
            "[[artifacts.build]]\nid='build'\nmechanism='build:cargo'\noutputs=[{id='app',kind='file'}]\n\
             [[artifacts.package]]\nid='win'\nmechanism='package:static-file'\nwhen={os=['windows']}\noutputs=[{id='win-file',kind='file'}]\n\
             [[artifacts.package]]\nid='posix'\nmechanism='package:static-file'\nwhen={os=['linux','macos']}\noutputs=[{id='posix-file',kind='file'}]\n",
        );
        let projected = manifest
            .artifacts
            .as_ref()
            .unwrap()
            .project_package_targets(TargetOs::Linux)
            .unwrap();
        assert_eq!(
            projected
                .active()
                .iter()
                .map(|target| target.id.as_str())
                .collect::<Vec<_>>(),
            ["posix"]
        );
        assert_eq!(
            projected
                .decisions()
                .iter()
                .map(TargetApplicability::status)
                .collect::<Vec<_>>(),
            ["skipped", "active"]
        );
        assert!(projected.active_output_ids().contains("app"));
        assert!(!projected.active_output_ids().contains("win-file"));
    }

    #[test]
    fn active_consumer_of_inactive_package_producer_refuses_by_both_ids() {
        let manifest = parse(
            "[[artifacts.package]]\nid='win'\nmechanism='package:static-file'\nwhen={os=['windows']}\noutputs=[{id='win-file',kind='file'}]\n\
             [[artifacts.package]]\nid='consumer'\nmechanism='package:static-file'\ninputs=[{artifact='win-file'}]\noutputs=[{id='bundle',kind='file'}]\n",
        );
        let error = manifest
            .artifacts
            .as_ref()
            .unwrap()
            .project_package_targets(TargetOs::Linux)
            .unwrap_err()
            .to_string();
        for expected in ["consumer", "win", "win-file", "linux"] {
            assert!(error.contains(expected), "{error}");
        }
    }
}
