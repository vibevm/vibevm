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
    artifact: &'a BoundProjectArtifact,
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
            artifact,
        }
    }

    pub(crate) fn for_locked(
        store: &'a BoundVibeBinStore<'a>,
        artifact: &'a BoundProjectArtifact,
    ) -> Self {
        Self {
            view: BoundView::Locked(store),
            artifact,
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
        if !self
            .artifact
            .matches_resolved(&resolved.relative, &resolved.digest, resolved.bytes)
        {
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
            BoundView::Preplan(_) => Err(effect_not_ready(request, "verify")),
        }
    }

    fn classify_launcher(
        &self,
        request: &DeployTargetRequest<'_>,
        relative: &str,
    ) -> Result<launcher::Occupant, MechanismError> {
        match self.view {
            BoundView::Locked(store) => store
                .classify_launcher(relative)
                .map_err(|error| observation_error(request, relative, error).into()),
            BoundView::Preplan(settings) => settings
                .read_file_bounded(relative, 64 * 1024)
                .map(|bytes| {
                    bytes
                        .as_deref()
                        .map_or(launcher::Occupant::Vacant, launcher::classify_bytes)
                })
                .map_err(|error| {
                    DeployProviderError::Observe {
                        target: request.target.id.clone(),
                        resource: relative.into(),
                        reason: format!("{error:#}"),
                    }
                    .into()
                }),
        }
    }

    fn pointer_present(
        &self,
        request: &DeployTargetRequest<'_>,
        relative: &str,
    ) -> Result<bool, MechanismError> {
        match self.view {
            BoundView::Locked(store) => store
                .state(relative)
                .map(|state| state.is_some())
                .map_err(|error| observation_error(request, relative, error).into()),
            BoundView::Preplan(settings) => settings
                .read_file_bounded(relative, 128)
                .map(|bytes| bytes.is_some())
                .map_err(|error| {
                    DeployProviderError::Observe {
                        target: request.target.id.clone(),
                        resource: relative.into(),
                        reason: format!("{error:#}"),
                    }
                    .into()
                }),
        }
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
        if request.prior_receipt.is_some() || request.recovery_intent.is_some() {
            return Err(effect_not_ready(request, "update or recovery"));
        }
        let (destination, artifact) = self.admit(request)?;
        let occupant = self.classify_launcher(request, &destination.launcher)?;
        launcher::refuse_collision(&request.target.id, &destination.launcher, occupant)?;
        if occupant != launcher::Occupant::Vacant
            || self.pointer_present(request, &destination.pointer)?
        {
            return Err(effect_not_ready(
                request,
                "update of an unowned destination",
            ));
        }
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
            // The first bound route has no admitted remove or inverse replay.
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
        if request.prior_receipt.is_some() || request.recovery_intent.is_some() {
            return Err(effect_not_ready(request, "update or recovery"));
        }
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
        let occupant = store
            .classify_launcher(&destination.launcher)
            .map_err(|error| observation_error(request, &destination.launcher, error))?;
        launcher::refuse_collision(&request.target.id, &destination.launcher, occupant)?;
        if occupant != launcher::Occupant::Vacant
            || store
                .prior_pointer(&destination.pointer)
                .map_err(|error| observation_error(request, &destination.pointer, error))?
                .is_some()
        {
            return Err(effect_not_ready(
                request,
                "update of an unowned destination",
            ));
        }
        let payload = store::payload_relative(destination.flavour, &artifact.digest);
        let placement = store
            .place_payload(self.artifact, destination.flavour)
            .map_err(|error| write_error(request, &payload, error))?;
        checkpoint.completed(&payload)?;
        // Recheck the collision after the payload step. The physical lock
        // coordinates participating writers; this still detects other edits.
        let occupant = store
            .classify_launcher(&destination.launcher)
            .map_err(|error| observation_error(request, &destination.launcher, error))?;
        launcher::refuse_collision(&request.target.id, &destination.launcher, occupant)?;
        if occupant != launcher::Occupant::Vacant {
            return Err(effect_not_ready(
                request,
                "update of an unowned destination",
            ));
        }
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
        _resources: &[String],
        _prior_state_handle: Option<&str>,
    ) -> Result<RemoveReport, MechanismError> {
        Err(effect_not_ready(request, "remove"))
    }

    fn recover(
        &self,
        request: &DeployTargetRequest<'_>,
        _plan: &DeployPlan,
        _observed: &[ObservedResource],
        _checkpoint: &mut CheckpointLedger<'_>,
    ) -> Result<ApplyReport, MechanismError> {
        Err(effect_not_ready(request, "recover"))
    }
}

#[cfg(test)]
#[path = "bound_provider_tests.rs"]
mod tests;
