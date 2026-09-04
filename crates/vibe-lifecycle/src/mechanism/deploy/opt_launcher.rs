//! The builtin `deploy:vibe-opt-launcher` provider.
//!
//! One recorded opaque file is reconciled to exactly
//! `<settings-root>/opt/bin/<artifact-id>`. All source, destination and
//! rollback I/O goes through retained capability-relative handles.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ZAI-GLM-LAUNCHER-DELIVERY");

use sha2::{Digest, Sha256};
use vibe_core::manifest::{ArtifactKind, ExtensionConfig};
use vibe_safefs::StableFileState;
use vibe_wire::generated::artifact_record::ArtifactShape;
use vibe_wire::generated::deploy_intent::DeployIntent;
use vibe_wire::generated::deploy_receipt::DeployReceipt;

use crate::mechanism::deploy::protocol::{
    ApplyReport, DeployDescriptor, DeployFingerprint, DeployPlan, ObservedResource,
    PlannedDeployResource, RemoveReport, ResolvedDeployArtifact,
};
use crate::mechanism::deploy::state::CheckpointLedger;
use crate::mechanism::error::{DeployProviderError, preview};
use crate::mechanism::package::static_file::portable_filename;
use crate::mechanism::vibebin::store;
use crate::mechanism::{
    BUILTIN_VIBE_OPT_LAUNCHER_PIN, DeployProvider, DeployTargetRequest, EffectClass,
    MechanismError, NetworkUse, PrivilegeNeed, ProviderDescriptor, ProviderOperation,
    Reversibility,
};

const OPT_BIN_DIR: &str = "opt/bin";
const ADAPTER_EPOCH: u32 = 1;
const SUPPORTED_KINDS: [ArtifactKind; 1] = [ArtifactKind::File];
const SUPPORTED_KINDS_LIST: &str = "file";
const DEPLOY_OPERATIONS: [ProviderOperation; 6] = [
    ProviderOperation::Plan,
    ProviderOperation::Fingerprint,
    ProviderOperation::Apply,
    ProviderOperation::Verify,
    ProviderOperation::Remove,
    ProviderOperation::Recover,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Occupancy {
    Absent,
    Owned,
    Interrupted,
    InterruptedUpdate,
}

struct Destination {
    resource: String,
    relative: String,
}

mod rollback;

use rollback::{PriorHandle, backup_relative, render_handle};

/// The receipt-owned opt-launcher provider.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct VibeOptLauncherProvider;

impl VibeOptLauncherProvider {
    fn admit<'a>(
        &self,
        request: &'a DeployTargetRequest<'_>,
    ) -> Result<(Destination, &'a ResolvedDeployArtifact), DeployProviderError> {
        empty_config(&request.target.id, request.target.config.as_ref())?;
        let artifact = request
            .artifact
            .ok_or_else(|| DeployProviderError::NoArtifact {
                target: request.target.id.clone(),
                provider: BUILTIN_VIBE_OPT_LAUNCHER_PIN,
            })?;
        if !SUPPORTED_KINDS.contains(&artifact.kind) {
            return Err(DeployProviderError::ArtifactKind {
                target: request.target.id.clone(),
                artifact: artifact.id.clone(),
                provider: BUILTIN_VIBE_OPT_LAUNCHER_PIN,
                kind: artifact.kind.as_str(),
                supported: SUPPORTED_KINDS_LIST,
            });
        }
        if artifact.shape != ArtifactShape::File {
            return Err(DeployProviderError::ArtifactShape {
                target: request.target.id.clone(),
                artifact: artifact.id.clone(),
                provider: BUILTIN_VIBE_OPT_LAUNCHER_PIN,
            });
        }
        if !portable_filename(&artifact.id) {
            return Err(DeployProviderError::Config {
                target: request.target.id.clone(),
                member: "artifact".to_owned(),
                reason: format!(
                    "artifact id `{}` is not one safe portable filename",
                    preview(&artifact.id),
                ),
            });
        }
        if request.target.artifact != artifact.id {
            return Err(DeployProviderError::Config {
                target: request.target.id.clone(),
                member: "artifact".to_owned(),
                reason:
                    "the resolved artifact identity differs from the target's declared artifact"
                        .to_owned(),
            });
        }
        Ok((destination(&artifact.id), artifact))
    }

    fn destination_without_artifact(
        &self,
        request: &DeployTargetRequest<'_>,
    ) -> Result<Destination, DeployProviderError> {
        empty_config(&request.target.id, request.target.config.as_ref())?;
        if !portable_filename(&request.target.artifact) {
            return Err(DeployProviderError::Config {
                target: request.target.id.clone(),
                member: "artifact".to_owned(),
                reason: format!(
                    "artifact id `{}` is not one safe portable filename",
                    preview(&request.target.artifact),
                ),
            });
        }
        Ok(destination(&request.target.artifact))
    }

    fn occupancy_under(
        &self,
        request: &DeployTargetRequest<'_>,
        destination: &Destination,
        intent: Option<&DeployIntent>,
    ) -> Result<Occupancy, DeployProviderError> {
        let Some(observed) = self.observe_digest(request, destination)? else {
            return Ok(Occupancy::Absent);
        };
        let owned = request.prior_receipt.and_then(|receipt| {
            receipt
                .resources
                .iter()
                .find(|owned| owned.resource == destination.resource)
                .map(|owned| (receipt, owned))
        });
        if let Some((_, owned)) = owned.as_ref()
            && owned.post_digest == observed
        {
            return Ok(Occupancy::Owned);
        }
        if let Some(occupancy) =
            interrupted_under(intent, request.prior_receipt, destination, &observed)
        {
            return Ok(occupancy);
        }
        if let Some((_, owned)) = owned {
            return Err(DeployProviderError::OccupantDrifted {
                target: request.target.id.clone(),
                resource: destination.resource.clone(),
                recorded: owned.post_digest.clone(),
                observed,
            });
        }
        Err(DeployProviderError::OccupantUnowned {
            target: request.target.id.clone(),
            resource: destination.resource.clone(),
            observed,
        })
    }

    fn observe_digest(
        &self,
        request: &DeployTargetRequest<'_>,
        destination: &Destination,
    ) -> Result<Option<String>, DeployProviderError> {
        store::resource_state(
            &request.target.id,
            request.settings_root,
            &destination.relative,
        )
        .map(|state| state.map(|state| resource_digest(&state)))
    }

    fn publish(
        &self,
        request: &DeployTargetRequest<'_>,
        destination: &Destination,
        artifact: &ResolvedDeployArtifact,
        checkpoint: &mut CheckpointLedger<'_>,
    ) -> Result<(), MechanismError> {
        let source = store::copy_resource_expected(
            &request.target.id,
            request.project_root,
            &artifact.relative,
            request.settings_root,
            &destination.relative,
            desired_mode(),
            &artifact.digest,
            artifact.bytes,
        )?;
        if source.sha256 != artifact.digest || source.bytes != artifact.bytes {
            return Err(MechanismError::Deploy(DeployProviderError::Observe {
                target: request.target.id.clone(),
                resource: artifact.relative.clone(),
                reason: format!(
                    "the held artifact changed since engine resolution (resolved {} bytes at {}, copied {} bytes at {})",
                    artifact.bytes, artifact.digest, source.bytes, source.sha256,
                ),
            }));
        }
        checkpoint.completed(&destination.resource)
    }
}

impl DeployProvider for VibeOptLauncherProvider {
    fn descriptor(&self) -> DeployDescriptor {
        DeployDescriptor {
            provider: ProviderDescriptor {
                key: BUILTIN_VIBE_OPT_LAUNCHER_PIN,
                kinds: &SUPPORTED_KINDS,
                effect: EffectClass::User,
                network: NetworkUse::Never,
                privilege: PrivilegeNeed::None,
                reversibility: Reversibility::Reversible,
                operations: &DEPLOY_OPERATIONS,
            },
            atomic_replacement: true,
            reference_ownership: false,
        }
    }

    fn plan(&self, request: &DeployTargetRequest<'_>) -> Result<DeployPlan, MechanismError> {
        let (destination, artifact) = self.admit(request)?;
        self.occupancy_under(request, &destination, request.recovery_intent)?;
        Ok(DeployPlan {
            resources: vec![PlannedDeployResource {
                resource: destination.resource.clone(),
                desired_digest: desired_resource_digest(&artifact.digest),
            }],
            lock_resources: vec![destination.resource.clone()],
            config_digest: config_digest(),
            reversible: true,
            summary: format!(
                "vibe-opt-launcher publishes the opaque file `{}` as {}",
                artifact.id, destination.resource,
            ),
        })
    }

    fn fingerprint(
        &self,
        _request: &DeployTargetRequest<'_>,
        _plan: &DeployPlan,
    ) -> Result<DeployFingerprint, MechanismError> {
        Ok(DeployFingerprint {
            digest: digest_of(format!("vibe-opt-launcher/{ADAPTER_EPOCH}").as_bytes()),
            summary: format!("the vibe-opt-launcher adapter epoch {ADAPTER_EPOCH}"),
        })
    }

    fn apply(
        &self,
        request: &DeployTargetRequest<'_>,
        _plan: &DeployPlan,
        checkpoint: &mut CheckpointLedger<'_>,
    ) -> Result<ApplyReport, MechanismError> {
        let (destination, artifact) = self.admit(request)?;
        let occupancy = self.occupancy_under(request, &destination, None)?;
        let prior_state_handle = match occupancy {
            Occupancy::Owned => {
                let expected = request
                    .prior_receipt
                    .and_then(|receipt| {
                        receipt
                            .resources
                            .iter()
                            .find(|owned| owned.resource == destination.resource)
                    })
                    .map(|owned| owned.post_digest.as_str())
                    .ok_or_else(|| DeployProviderError::RemoveNotOwned {
                        target: request.target.id.clone(),
                        resource: destination.resource.clone(),
                    })?;
                Some(self.save_prior(request, &destination, expected)?)
            }
            Occupancy::Absent => None,
            Occupancy::Interrupted | Occupancy::InterruptedUpdate => {
                return Err(MechanismError::Deploy(DeployProviderError::Write {
                    target: request.target.id.clone(),
                    path: destination.relative,
                    reason: "apply received recovery occupancy without an injected intent"
                        .to_owned(),
                }));
            }
        };
        self.publish(request, &destination, artifact, checkpoint)?;
        Ok(ApplyReport {
            prior_state_handle,
            evidence: format!(
                "vibe-opt-launcher atomically published the held opaque artifact `{}` at {}",
                artifact.id, destination.resource,
            ),
        })
    }

    fn verify(
        &self,
        request: &DeployTargetRequest<'_>,
        resources: &[String],
    ) -> Result<Vec<ObservedResource>, MechanismError> {
        let destination = self.destination_without_artifact(request)?;
        resources
            .iter()
            .map(|resource| {
                if resource != &destination.resource {
                    return Err(MechanismError::Deploy(DeployProviderError::Observe {
                        target: request.target.id.clone(),
                        resource: resource.clone(),
                        reason: format!("this deployment owns only `{}`", destination.resource),
                    }));
                }
                Ok(ObservedResource {
                    resource: resource.clone(),
                    digest: self.observe_digest(request, &destination)?,
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
        let destination = self.destination_without_artifact(request)?;
        let receipt = request.prior_receipt.ok_or_else(|| {
            MechanismError::Deploy(DeployProviderError::RemoveNotOwned {
                target: request.target.id.clone(),
                resource: destination.resource.clone(),
            })
        })?;
        if resources.len() > 1
            || resources
                .iter()
                .any(|resource| resource != &destination.resource)
            || !receipt
                .resources
                .iter()
                .any(|owned| owned.resource == destination.resource)
        {
            return Err(MechanismError::Deploy(
                DeployProviderError::RemoveNotOwned {
                    target: request.target.id.clone(),
                    resource: resources
                        .first()
                        .cloned()
                        .unwrap_or_else(|| destination.resource.clone()),
                },
            ));
        }
        if let Some(encoded) = prior_state_handle {
            let (handle, backup) = self.load_prior(request, encoded)?;
            let prior_digest = resource_digest(&StableFileState {
                sha256: handle.sha256.clone(),
                bytes: handle.bytes,
                unix_mode: handle.unix_mode,
            });
            let current = self.observe_digest(request, &destination)?;
            let recorded = receipt
                .resources
                .iter()
                .find(|owned| owned.resource == destination.resource)
                .map(|owned| owned.post_digest.as_str());
            if current.as_deref() != Some(&prior_digest) {
                if current.as_deref() != recorded {
                    return Err(MechanismError::Deploy(
                        DeployProviderError::OccupantDrifted {
                            target: request.target.id.clone(),
                            resource: destination.resource,
                            recorded: recorded.unwrap_or("absent").to_owned(),
                            observed: current.unwrap_or_else(|| "absent".to_owned()),
                        },
                    ));
                }
                let restored = self.restore_prior(request, &destination, &handle, &backup)?;
                if restored.sha256 != handle.sha256 || restored.unix_mode != handle.unix_mode {
                    return Err(MechanismError::Deploy(DeployProviderError::Write {
                        target: request.target.id.clone(),
                        path: destination.relative,
                        reason: "the restored launcher differs from its validated rollback handle"
                            .to_owned(),
                    }));
                }
            }
            return Ok(RemoveReport {
                removed: Vec::new(),
                expected_remaining: vec![ObservedResource {
                    resource: destination.resource.clone(),
                    digest: Some(prior_digest),
                }],
                evidence: format!(
                    "vibe-opt-launcher restored the exact prior bytes and mode at {}",
                    destination.resource,
                ),
            });
        }
        let mut removed = Vec::with_capacity(1);
        if !resources.is_empty()
            && store::remove_resource(
                &request.target.id,
                request.settings_root,
                &destination.relative,
            )?
        {
            removed.push(destination.resource.clone());
        }
        Ok(RemoveReport {
            removed,
            expected_remaining: Vec::new(),
            evidence: format!(
                "vibe-opt-launcher removed only the receipt-owned file {}",
                destination.resource,
            ),
        })
    }

    fn recover(
        &self,
        request: &DeployTargetRequest<'_>,
        _plan: &DeployPlan,
        observed: &[ObservedResource],
        checkpoint: &mut CheckpointLedger<'_>,
    ) -> Result<ApplyReport, MechanismError> {
        let (destination, artifact) = self.admit(request)?;
        let desired_digest = desired_resource_digest(&artifact.digest);
        let current = observed
            .iter()
            .find(|seen| seen.resource == destination.resource)
            .and_then(|seen| seen.digest.as_deref());
        let prior_state_handle = if let Some(receipt) = request.prior_receipt {
            let owned = receipt
                .resources
                .iter()
                .find(|owned| owned.resource == destination.resource)
                .ok_or_else(|| DeployProviderError::RemoveNotOwned {
                    target: request.target.id.clone(),
                    resource: destination.resource.clone(),
                })?;
            if current == Some(desired_digest.as_str()) {
                let backup = backup_relative(request)?;
                let state =
                    store::resource_state(&request.target.id, request.settings_root, &backup)?
                        .ok_or_else(|| DeployProviderError::Observe {
                            target: request.target.id.clone(),
                            resource: backup.clone(),
                            reason: "the interrupted update has no retained prior launcher"
                                .to_owned(),
                        })?;
                if resource_digest(&state) != owned.post_digest {
                    return Err(MechanismError::Deploy(DeployProviderError::Observe {
                        target: request.target.id.clone(),
                        resource: backup.clone(),
                        reason: "the retained rollback state does not match the prior receipt"
                            .to_owned(),
                    }));
                }
                Ok::<_, DeployProviderError>(render_handle(&PriorHandle {
                    path: backup,
                    sha256: state.sha256,
                    bytes: state.bytes,
                    unix_mode: state.unix_mode,
                }))
            } else if current == Some(owned.post_digest.as_str()) {
                Ok(self.save_prior(request, &destination, &owned.post_digest)?)
            } else {
                Err(DeployProviderError::OccupantDrifted {
                    target: request.target.id.clone(),
                    resource: destination.resource.clone(),
                    recorded: owned.post_digest.clone(),
                    observed: current.unwrap_or("absent").to_owned(),
                })
            }
            .map(Some)?
        } else {
            None
        };
        let desired = current == Some(desired_digest.as_str());
        if desired {
            checkpoint.completed(&destination.resource)?;
        } else {
            self.publish(request, &destination, artifact, checkpoint)?;
        }
        Ok(ApplyReport {
            prior_state_handle,
            evidence: format!(
                "vibe-opt-launcher recovered {} to its full desired state",
                destination.resource,
            ),
        })
    }
}

fn empty_config(target: &str, config: Option<&ExtensionConfig>) -> Result<(), DeployProviderError> {
    if let Some((member, _)) = config.and_then(|value| value.as_table().iter().next()) {
        return Err(DeployProviderError::Config {
            target: target.to_owned(),
            member: preview(member),
            reason: "the vibe-opt-launcher config is empty; the artifact id is the fixed destination filename"
                .to_owned(),
        });
    }
    Ok(())
}

fn destination(filename: &str) -> Destination {
    let relative = format!("{OPT_BIN_DIR}/{filename}");
    Destination {
        resource: relative.clone(),
        relative,
    }
}

fn config_digest() -> String {
    digest_of(format!("vibe-opt-launcher-config/1\0adapter-epoch\0{ADAPTER_EPOCH}").as_bytes())
}

#[cfg(unix)]
const fn desired_mode() -> Option<u32> {
    Some(0o755)
}

#[cfg(not(unix))]
const fn desired_mode() -> Option<u32> {
    None
}

#[cfg(unix)]
fn resource_digest(state: &StableFileState) -> String {
    digest_of(
        format!(
            "vibe-opt-launcher-resource/1\0content\0{}\0mode\0{:04o}",
            state.sha256,
            state.unix_mode.unwrap_or(0),
        )
        .as_bytes(),
    )
}

#[cfg(not(unix))]
fn resource_digest(state: &StableFileState) -> String {
    state.sha256.clone()
}

fn desired_resource_digest(content: &str) -> String {
    resource_digest(&StableFileState {
        sha256: content.to_owned(),
        bytes: 0,
        unix_mode: desired_mode(),
    })
}

fn interrupted_under(
    intent: Option<&DeployIntent>,
    prior_receipt: Option<&DeployReceipt>,
    destination: &Destination,
    observed: &str,
) -> Option<Occupancy> {
    let intent = intent?;
    let planned = intent
        .resources
        .iter()
        .find(|planned| planned.resource == destination.resource)?;
    if planned.desired_digest != observed
        || intent.prior_generation != prior_receipt.map(|receipt| receipt.generation)
    {
        return None;
    }
    Some(if prior_receipt.is_some() {
        Occupancy::InterruptedUpdate
    } else {
        Occupancy::Interrupted
    })
}

fn digest_of(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
#[path = "opt_launcher/tests.rs"]
mod tests;
