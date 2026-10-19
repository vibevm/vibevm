//! The pinned builtin provider for the first host-admitted local effect.
//!
//! Its locked form uses only held project and destination capabilities.
//! The engine must supply the binding-aware durable intent before `apply`;
//! this adapter does not grant an effect from a pathname or wire value.

use super::bound::{BoundProjectArtifact, BoundVibeBinError, BoundVibeBinStore};
use super::launcher;
use super::{Destination, VibeBinProvider, digest_of, store};
use crate::mechanism::deploy::protocol::{
    ApplyReport, DeployDescriptor, DeployFingerprint, DeployPlan, ObservedResource,
    PlannedDeployResource, RemoveReport,
};
use crate::mechanism::deploy::state::CheckpointLedger;
use crate::mechanism::error::DeployProviderError;
use crate::mechanism::{DeployProvider, DeployTargetRequest, MechanismError};
use vibe_safefs::Project;

/// A host-owned, lock-scoped adapter. Only the host can make the store: it
/// borrows a destination lock tied to the same pinned settings project.
pub(crate) struct BoundVibeBinProvider<'a> {
    view: BoundView<'a>,
    artifact: Option<&'a BoundProjectArtifact>,
}

#[derive(Clone, Copy)]
enum BoundView<'a> {
    /// No-create preplan through the admitted held settings root.
    Preplan(&'a Project),
    /// Lock-scoped recheck/independent observation through pinned children.
    Locked(&'a BoundVibeBinStore<'a>),
}

impl<'a> BoundVibeBinProvider<'a> {
    pub(crate) fn for_preplan(settings: &'a Project, artifact: &'a BoundProjectArtifact) -> Self {
        Self {
            view: BoundView::Preplan(settings),
            artifact: Some(artifact),
        }
    }

    pub(crate) fn for_locked(
        store: &'a BoundVibeBinStore<'a>,
        artifact: &'a BoundProjectArtifact,
    ) -> Self {
        Self {
            view: BoundView::Locked(store),
            artifact: Some(artifact),
        }
    }

    /// Closed retained cleanup needs no artifact source or live provider selection.
    pub(crate) fn for_retained(store: &'a BoundVibeBinStore<'a>) -> Self {
        Self {
            view: BoundView::Locked(store),
            artifact: None,
        }
    }

    fn admit<'b>(
        &self,
        request: &DeployTargetRequest<'b>,
    ) -> Result<
        (
            Destination,
            &'b crate::mechanism::deploy::protocol::ResolvedDeployArtifact,
        ),
        MechanismError,
    > {
        let (destination, resolved) = VibeBinProvider.admit(request)?;
        if !self.artifact.is_some_and(|artifact| {
            artifact.matches_resolved(&resolved.relative, &resolved.digest, resolved.bytes)
        }) {
            return Err(DeployProviderError::BoundArtifactMismatch {
                target: request.target.id.clone(),
                artifact: resolved.id.clone(),
            }
            .into());
        }
        Ok((destination, resolved))
    }

    fn observe(
        &self,
        request: &DeployTargetRequest<'_>,
        resource: &str,
    ) -> Result<Option<String>, MechanismError> {
        let destination = Destination::resolve(
            &request.target.id,
            request.target.config.as_ref(),
            launcher::LauncherFlavour::NATIVE,
        )?;
        if resource != destination.launcher && resource != destination.pointer {
            return Err(DeployProviderError::Observe {
                target: request.target.id.clone(),
                resource: resource.into(),
                reason: "resource is outside this bound vibe-bin plan".into(),
            }
            .into());
        }
        match self.view {
            BoundView::Locked(store) => store
                .state(resource)
                .map(|state| state.map(|value| value.sha256))
                .map_err(|error| observation_error(request, resource, error).into()),
            BoundView::Preplan(settings) => settings
                .stable_file_state(resource)
                .map(|state| state.map(|value| value.sha256))
                .map_err(|error| {
                    DeployProviderError::Observe {
                        target: request.target.id.clone(),
                        resource: resource.into(),
                        reason: error.to_string(),
                    }
                    .into()
                }),
        }
    }

    fn check_destination(
        &self,
        request: &DeployTargetRequest<'_>,
        desired: &[PlannedDeployResource],
    ) -> Result<(), MechanismError> {
        if request.prior_receipt.is_none() && request.recovery_intent.is_none() {
            let resource = &desired[0].resource;
            let occupant = match self.view {
                BoundView::Locked(store) => store
                    .classify_launcher(resource)
                    .map_err(|e| observation_error(request, resource, e))?,
                BoundView::Preplan(settings) => settings
                    .read_file_bounded(resource, 64 * 1024)
                    .map_err(|e| DeployProviderError::Observe {
                        target: request.target.id.clone(),
                        resource: resource.clone(),
                        reason: e.to_string(),
                    })?
                    .as_deref()
                    .map_or(launcher::Occupant::Vacant, launcher::classify_bytes),
            };
            launcher::refuse_collision(&request.target.id, resource, occupant)?;
            if occupant != launcher::Occupant::Vacant {
                return Err(effect_not_ready(
                    request,
                    "update of an unowned destination",
                ));
            }
        }
        if let Some(prior) = request.prior_receipt {
            if prior.resources.len() != desired.len()
                || prior
                    .resources
                    .iter()
                    .any(|owned| !desired.iter().any(|row| row.resource == owned.resource))
            {
                return Err(effect_not_ready(
                    request,
                    "changing the owned destination set",
                ));
            }
        }
        for row in desired {
            let observed = self.observe(request, &row.resource)?;
            let prior = request.prior_receipt.and_then(|receipt| {
                receipt
                    .resources
                    .iter()
                    .find(|owned| owned.resource == row.resource)
                    .map(|owned| owned.post_digest.as_str())
            });
            let allowed = if let Some(intent) = request.recovery_intent {
                let expected = intent
                    .resources
                    .iter()
                    .find(|r| r.resource == row.resource)
                    .ok_or_else(|| effect_not_ready(request, "recovery resource differs"))?;
                if expected.desired_digest != row.desired_digest
                    || expected.prior_digest.as_deref() != prior
                {
                    return Err(effect_not_ready(request, "recovery before-image differs"));
                }
                observed.as_deref() == prior
                    || observed.as_deref() == Some(row.desired_digest.as_str())
            } else {
                observed.as_deref() == prior
            };
            if !allowed {
                if request.prior_receipt.is_none() && request.recovery_intent.is_none() {
                    return Err(effect_not_ready(
                        request,
                        "update of an unowned destination",
                    ));
                }
                return Err(DeployProviderError::Observe {
                    target: request.target.id.clone(),
                    resource: row.resource.clone(),
                    reason: "destination differs from exact prior/desired ownership".into(),
                }
                .into());
            }
        }
        Ok(())
    }
}

fn observation_error(
    request: &DeployTargetRequest<'_>,
    resource: &str,
    error: BoundVibeBinError,
) -> DeployProviderError {
    DeployProviderError::Observe {
        target: request.target.id.clone(),
        resource: resource.into(),
        reason: error.to_string(),
    }
}

fn write_error(
    request: &DeployTargetRequest<'_>,
    resource: &str,
    error: BoundVibeBinError,
) -> DeployProviderError {
    DeployProviderError::BoundWrite {
        target: request.target.id.clone(),
        path: resource.into(),
        reason: error.to_string(),
    }
}

fn effect_not_ready(request: &DeployTargetRequest<'_>, operation: &'static str) -> MechanismError {
    DeployProviderError::BoundEffectNotReady {
        target: request.target.id.clone(),
        operation,
    }
    .into()
}

impl DeployProvider for BoundVibeBinProvider<'_> {
    fn descriptor(&self) -> DeployDescriptor {
        VibeBinProvider.descriptor()
    }

    fn plan(&self, request: &DeployTargetRequest<'_>) -> Result<DeployPlan, MechanismError> {
        let (destination, artifact) = self.admit(request)?;
        let resources = vec![
            PlannedDeployResource {
                desired_digest: digest_of(&destination.body),
                resource: destination.launcher.clone(),
            },
            PlannedDeployResource {
                desired_digest: digest_of(&launcher::pointer_body(&artifact.digest)),
                resource: destination.pointer.clone(),
            },
        ];
        self.check_destination(request, &resources)?;
        let lock_resources = resources
            .iter()
            .map(|planned| planned.resource.clone())
            .collect();
        Ok(DeployPlan {
            summary: format!(
                "vibe-bin would install `{}` as {} with the active-payload pointer {}, resolving \
                 the payload {}",
                destination.command,
                destination.launcher,
                destination.pointer,
                store::payload_relative(destination.flavour, &artifact.digest),
            ),
            resources,
            lock_resources,
            config_digest: destination.config_digest(),
            reversible: false,
        })
    }

    fn fingerprint(
        &self,
        request: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
    ) -> Result<DeployFingerprint, MechanismError> {
        VibeBinProvider.fingerprint(request, plan)
    }

    fn apply(
        &self,
        request: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
        checkpoint: &mut CheckpointLedger<'_>,
    ) -> Result<ApplyReport, MechanismError> {
        let BoundView::Locked(store) = self.view else {
            return Err(effect_not_ready(request, "apply"));
        };
        let (destination, artifact) = self.admit(request)?;
        let expected = [
            PlannedDeployResource {
                desired_digest: digest_of(&destination.body),
                resource: destination.launcher.clone(),
            },
            PlannedDeployResource {
                desired_digest: digest_of(&launcher::pointer_body(&artifact.digest)),
                resource: destination.pointer.clone(),
            },
        ];
        if plan.resources.as_slice() != expected
            || plan.lock_resources != [destination.launcher.clone(), destination.pointer.clone()]
            || plan.config_digest != destination.config_digest()
            || plan.reversible
        {
            return Err(DeployProviderError::Observe {
                target: request.target.id.clone(),
                resource: destination.launcher,
                reason: "the admitted bound plan differs from the effect's exact resources".into(),
            }
            .into());
        }
        self.check_destination(request, &expected)?;
        let payload = store::payload_relative(destination.flavour, &artifact.digest);
        let placement = store
            .place_payload(
                self.artifact
                    .ok_or_else(|| effect_not_ready(request, "missing forward artifact"))?,
                destination.flavour,
            )
            .map_err(|error| write_error(request, &payload, error))?;
        checkpoint.completed(&payload)?;
        // Reobserve the exact owned states after immutable payload publication.
        self.check_destination(request, &expected)?;
        store
            .write_resource(
                &destination.launcher,
                &destination.body,
                destination.flavour.needs_executable_bit(),
            )
            .map_err(|error| write_error(request, &destination.launcher, error))?;
        checkpoint.completed(&destination.launcher)?;
        // Publish the pointer last, after the launcher is durable.
        let pointer = launcher::pointer_body(&artifact.digest);
        store
            .write_resource(&destination.pointer, &pointer, false)
            .map_err(|error| write_error(request, &destination.pointer, error))?;
        checkpoint.completed(&destination.pointer)?;
        Ok(ApplyReport {
            prior_state_handle: None,
            evidence: format!(
                "bound vibe-bin installed `{}` ({} flavour): payload {} at {payload}, launcher {}, pointer {}",
                destination.command,
                destination.flavour.as_str(),
                placement.as_str(),
                destination.launcher,
                destination.pointer,
            ),
        })
    }

    fn verify(
        &self,
        request: &DeployTargetRequest<'_>,
        resources: &[String],
    ) -> Result<Vec<ObservedResource>, MechanismError> {
        resources
            .iter()
            .map(|resource| {
                Ok(ObservedResource {
                    digest: self.observe(request, resource)?,
                    resource: resource.clone(),
                })
            })
            .collect()
    }

    fn remove(
        &self,
        request: &DeployTargetRequest<'_>,
        resources: &[String],
        prior_state_handle: Option<&str>,
    ) -> Result<RemoveReport, MechanismError> {
        let BoundView::Locked(store) = self.view else {
            return Err(effect_not_ready(request, "remove"));
        };
        if self.artifact.is_some() || prior_state_handle.is_some() {
            return Err(effect_not_ready(request, "remove"));
        }
        let receipt = request
            .prior_receipt
            .ok_or_else(|| effect_not_ready(request, "cleanup receipt absent"))?;
        let destination = Destination::resolve(
            &request.target.id,
            request.target.config.as_ref(),
            launcher::LauncherFlavour::NATIVE,
        )?;
        let expected = [
            (destination.launcher.as_str(), digest_of(&destination.body)),
            (
                destination.pointer.as_str(),
                digest_of(&launcher::pointer_body(&receipt.artifact_digest)),
            ),
        ];
        if receipt.desired_config_digest != destination.config_digest()
            || receipt.resources.len() != expected.len()
            || receipt.resources.iter().any(|r| {
                !expected
                    .iter()
                    .any(|(name, digest)| r.resource == *name && r.post_digest == *digest)
            })
            || resources
                .iter()
                .any(|r| !expected.iter().any(|(name, _)| r == name))
        {
            return Err(effect_not_ready(
                request,
                "unknown retained cleanup semantics",
            ));
        }
        // Reobserve the complete set before the first removal. The engine's
        // durable inverse marker is what permits already-absent members.
        for (name, digest) in &expected {
            let observed = self.observe(request, name)?;
            if observed.as_ref().is_some_and(|found| found != digest) {
                return Err(effect_not_ready(request, "cleanup third digest"));
            }
        }
        let mut removed = Vec::new();
        for resource in resources {
            if store
                .remove_resource(resource)
                .map_err(|e| write_error(request, resource, e))?
            {
                removed.push(resource.clone());
            }
        }
        Ok(RemoveReport {
            removed,
            expected_remaining: expected
                .iter()
                .map(|(name, _)| ObservedResource {
                    resource: (*name).into(),
                    digest: None,
                })
                .collect(),
            evidence: "removed retained bound launcher and pointer; immutable CAS payload retained"
                .into(),
        })
    }

    fn recover(
        &self,
        request: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
        _observed: &[ObservedResource],
        checkpoint: &mut CheckpointLedger<'_>,
    ) -> Result<ApplyReport, MechanismError> {
        if request.recovery_intent.is_none() {
            return Err(effect_not_ready(
                request,
                "recovery without retained intent",
            ));
        }
        self.apply(request, plan, checkpoint)
    }
}

#[cfg(test)]
#[path = "bound_provider_tests.rs"]
mod tests;
