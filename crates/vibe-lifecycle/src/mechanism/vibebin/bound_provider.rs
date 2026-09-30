//! The pinned planning and observation half of the builtin provider.
//!
//! It implements the incumbent provider protocol, but its effect verbs
//! refuse until the engine binds the before-image, checkpoint and inverse
//! journals to the host admission. That keeps a future bound route on ONE
//! provider/transaction protocol without claiming a safe effect today.

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
        let occupant = self.classify_launcher(request, &destination.launcher)?;
        launcher::refuse_collision(&request.target.id, &destination.launcher, occupant)?;
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
            reversible: true,
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
        _plan: &DeployPlan,
        _checkpoint: &mut CheckpointLedger<'_>,
    ) -> Result<ApplyReport, MechanismError> {
        Err(effect_not_ready(request, "apply"))
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
