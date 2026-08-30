//! `deploy:{claude,codex,opencode}-plugin` through the six-verb protocol.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS");

use std::sync::{Arc, Mutex};

use vibe_core::manifest::ArtifactKind;

use self::artifact::AdmittedProjection;
pub(crate) use self::client::PluginClient;
use self::client::{CliIdentity, framed_hash};
use self::config::PluginDeployConfig;
use self::wire::{ClientVersion, InstalledPlugin};
use crate::mechanism::deploy::protocol::{
    ApplyReport, DeployDescriptor, DeployFingerprint, DeployPlan, DeployTargetRequest,
    ObservedResource, PlannedDeployResource, RemoveReport, ResolvedDeployArtifact,
};
use crate::mechanism::deploy::state::CheckpointLedger;
use crate::mechanism::error::DeployProviderError;
use crate::mechanism::{
    DeployProvider, EffectClass, MechanismError, NetworkUse, PrivilegeNeed, ProviderDescriptor,
    ProviderOperation, Reversibility,
};
use crate::process::{ProcessRunner, SystemProcessRunner};

mod artifact;
mod client;
mod config;
mod marketplace;
mod opencode;
mod wire;

const ADAPTER_EPOCH: u32 = 1;
const CONFIG_DOMAIN: &str = "client-plugin-deploy-config/1";
const FINGERPRINT_DOMAIN: &str = "client-plugin-deploy/1";
const RESOURCE_DOMAIN: &str = "client-plugin-resource/1";
const SUPPORTED_KINDS: [ArtifactKind; 1] = [ArtifactKind::Directory];
const OPERATIONS: [ProviderOperation; 6] = [
    ProviderOperation::Plan,
    ProviderOperation::Fingerprint,
    ProviderOperation::Apply,
    ProviderOperation::Verify,
    ProviderOperation::Remove,
    ProviderOperation::Recover,
];

pub(crate) struct ClientPluginProvider {
    client: PluginClient,
    runner: Arc<dyn ProcessRunner>,
    planned_version: Mutex<Option<ClientVersion>>,
}

struct Admitted<'a> {
    config: PluginDeployConfig,
    artifact: &'a ResolvedDeployArtifact,
    projection: AdmittedProjection,
}

impl ClientPluginProvider {
    pub(crate) fn new(client: PluginClient) -> Self {
        Self {
            client,
            runner: Arc::new(SystemProcessRunner),
            planned_version: Mutex::new(None),
        }
    }

    fn admit<'a>(
        &self,
        request: &'a DeployTargetRequest<'_>,
    ) -> Result<Admitted<'a>, MechanismError> {
        let config = PluginDeployConfig::parse(&request.target.id, request.target.config.as_ref())?;
        let artifact = request
            .artifact
            .ok_or_else(|| DeployProviderError::NoArtifact {
                target: request.target.id.clone(),
                provider: self.client.pin(),
            })?;
        self.refuse_artifact_update(request, &artifact.digest)?;
        let projection =
            artifact::admit(&request.target.id, self.client.pin(), self.client, artifact)?;
        if let Some(identity) = &projection.identity
            && identity.name != config.name
        {
            return Err(DeployProviderError::PluginArtifact {
                target: request.target.id.clone(), artifact: artifact.id.clone(),
                provider: self.client.pin(),
                reason: format!(
                    "manifest name `{}` differs from strict config name `{}`; dots/underscores valid in a canonical plugin are deliberately not client install names",
                    identity.name, config.name
                ),
            }.into());
        }
        Ok(Admitted {
            config,
            artifact,
            projection,
        })
    }

    fn refuse_artifact_update(
        &self,
        request: &DeployTargetRequest<'_>,
        selected: &str,
    ) -> Result<(), DeployProviderError> {
        if let Some(receipt) = request.prior_receipt
            && receipt.artifact_digest != selected
        {
            return Err(DeployProviderError::PluginArtifactUpdate {
                target: request.target.id.clone(),
                recorded: receipt.artifact_digest.clone(),
                selected: selected.to_owned(),
            });
        }
        Ok(())
    }

    fn version(&self, request: &DeployTargetRequest<'_>) -> Result<ClientVersion, MechanismError> {
        let version = wire::probe_version(self.runner.as_ref(), self.client, request)?;
        if let Ok(mut slot) = self.planned_version.lock() {
            *slot = Some(version.clone());
        }
        Ok(version)
    }

    fn config_digest(&self, name: &str) -> String {
        framed_hash(
            CONFIG_DOMAIN,
            &[("client", self.client.as_str()), ("name", name)],
        )
    }

    fn cli_identity(
        &self,
        request: &DeployTargetRequest<'_>,
        admitted: &Admitted<'_>,
    ) -> Result<CliIdentity, MechanismError> {
        let identity = admitted.projection.identity.as_ref().ok_or_else(|| {
            DeployProviderError::PluginArtifact {
                target: request.target.id.clone(),
                artifact: admitted.artifact.id.clone(),
                provider: self.client.pin(),
                reason:
                    "a CLI projection reached coordinate planning without its required manifest"
                        .to_owned(),
            }
        })?;
        let marketplace =
            marketplace::marketplace_name(self.client, &request.target.id, &admitted.config.name);
        let coordinate = format!("{}@{marketplace}", admitted.config.name);
        let resource = format!("plugin:{}:{coordinate}", self.client.as_str());
        let desired_digest = framed_hash(
            RESOURCE_DOMAIN,
            &[
                ("client", self.client.as_str()),
                ("name", &admitted.config.name),
                ("marketplace", &marketplace),
                ("artifact", &admitted.artifact.digest),
                ("version", &identity.version),
            ],
        );
        Ok(CliIdentity {
            marketplace,
            coordinate,
            resource,
            version: identity.version.clone(),
            desired_digest,
        })
    }

    fn list_cli(
        &self,
        request: &DeployTargetRequest<'_>,
        admitted: &Admitted<'_>,
        identity: &CliIdentity,
    ) -> Result<Option<InstalledPlugin>, MechanismError> {
        Ok(wire::list(
            self.runner.as_ref(),
            self.client,
            request,
            &admitted.config.name,
            &identity.marketplace,
            &identity.coordinate,
        )?)
    }

    fn judge_cli(
        &self,
        request: &DeployTargetRequest<'_>,
        identity: &CliIdentity,
        installed: Option<&InstalledPlugin>,
        allow_intent: bool,
    ) -> Result<bool, MechanismError> {
        let present = installed.is_some();
        if let Some(installed) = installed
            && (installed.version != identity.version
                || !installed.enabled
                || !installed.user_scope)
        {
            return Err(DeployProviderError::PluginOccupancy {
                target: request.target.id.clone(), resource: identity.resource.clone(),
                reason: format!(
                    "matching coordinate has version `{}`, enabled={}, user-scope={}; desired is version `{}`, enabled user scope",
                    installed.version, installed.enabled, installed.user_scope, identity.version
                ),
            }.into());
        }
        let prior = request.prior_receipt.and_then(|receipt| {
            receipt
                .resources
                .iter()
                .find(|owned| owned.resource == identity.resource)
        });
        match (present, prior) {
            (false, None) => Ok(false),
            (true, Some(owned)) if owned.post_digest == identity.desired_digest => Ok(true),
            (true, None) if allow_intent && interrupted(request, identity) => Ok(true),
            (true, None) => Err(DeployProviderError::PluginOccupancy {
                target: request.target.id.clone(),
                resource: identity.resource.clone(),
                reason: "the installed coordinate has no owning prior receipt".to_owned(),
            }
            .into()),
            (false, Some(owned)) => Err(DeployProviderError::PluginOccupancy {
                target: request.target.id.clone(),
                resource: identity.resource.clone(),
                reason: format!(
                    "receipt records `{}`, but the client list reports absence",
                    owned.post_digest
                ),
            }
            .into()),
            (true, Some(owned)) => Err(DeployProviderError::PluginOccupancy {
                target: request.target.id.clone(),
                resource: identity.resource.clone(),
                reason: format!(
                    "receipt records `{}`, desired/observed witness is `{}`",
                    owned.post_digest, identity.desired_digest
                ),
            }
            .into()),
        }
    }

    fn reconcile_cli(
        &self,
        request: &DeployTargetRequest<'_>,
        admitted: &Admitted<'_>,
        identity: &CliIdentity,
        already: bool,
        checkpoint: &mut CheckpointLedger<'_>,
    ) -> Result<ApplyReport, MechanismError> {
        let support = marketplace::materialize(
            request,
            self.client,
            &admitted.projection,
            &identity.marketplace,
            &admitted.config.name,
            &identity.version,
            &admitted.artifact.digest,
        )?;
        wire::marketplace_add(self.runner.as_ref(), self.client, request, &support)?;
        if !already {
            wire::install(
                self.runner.as_ref(),
                self.client,
                request,
                &identity.coordinate,
            )?;
        }
        let after = self.list_cli(request, admitted, identity)?;
        if after.as_ref().is_none_or(|item| {
            item.version != identity.version || !item.enabled || !item.user_scope
        }) {
            return Err(DeployProviderError::PluginOccupancy {
                target: request.target.id.clone(),
                resource: identity.resource.clone(),
                reason: "post-install list did not prove the exact desired coordinate".to_owned(),
            }
            .into());
        }
        checkpoint.completed(&identity.resource)?;
        Ok(ApplyReport {
            prior_state_handle: None,
            evidence: format!(
                "{} plugin `{}`: immutable marketplace verified, exact coordinate {} {}, then re-listed",
                self.client.as_str(),
                admitted.config.name,
                identity.coordinate,
                if already {
                    "was already desired"
                } else {
                    "was installed"
                },
            ),
        })
    }

    fn cli_observed_digest(
        &self,
        request: &DeployTargetRequest<'_>,
        config: &PluginDeployConfig,
        receipt_artifact: &str,
    ) -> Result<(String, Option<String>), MechanismError> {
        let marketplace =
            marketplace::marketplace_name(self.client, &request.target.id, &config.name);
        let coordinate = format!("{}@{marketplace}", config.name);
        let resource = format!("plugin:{}:{coordinate}", self.client.as_str());
        let installed = wire::list(
            self.runner.as_ref(),
            self.client,
            request,
            &config.name,
            &marketplace,
            &coordinate,
        )?;
        if installed.as_ref().is_some_and(|item| !item.active_user()) {
            return Err(DeployProviderError::PluginOccupancy {
                target: request.target.id.clone(),
                resource,
                reason: "the matching client-list entry is not enabled in user scope".to_owned(),
            }
            .into());
        }
        let digest = installed.map(|item| {
            framed_hash(
                RESOURCE_DOMAIN,
                &[
                    ("client", self.client.as_str()),
                    ("name", &config.name),
                    ("marketplace", &marketplace),
                    ("artifact", receipt_artifact),
                    ("version", &item.version),
                ],
            )
        });
        Ok((resource, digest))
    }
}

impl DeployProvider for ClientPluginProvider {
    fn descriptor(&self) -> DeployDescriptor {
        DeployDescriptor {
            provider: ProviderDescriptor {
                key: self.client.pin(),
                kinds: &SUPPORTED_KINDS,
                effect: EffectClass::User,
                network: NetworkUse::Never,
                privilege: PrivilegeNeed::None,
                reversibility: Reversibility::Reversible,
                operations: &OPERATIONS,
            },
            atomic_replacement: true,
            reference_ownership: true,
        }
    }

    fn plan(&self, request: &DeployTargetRequest<'_>) -> Result<DeployPlan, MechanismError> {
        let admitted = self.admit(request)?;
        self.version(request)?;
        if self.client == PluginClient::OpenCode {
            return opencode::plan(
                request,
                &admitted.projection,
                self.config_digest(&admitted.config.name),
            );
        }
        let identity = self.cli_identity(request, &admitted)?;
        let installed = self.list_cli(request, &admitted, &identity)?;
        let already = self.judge_cli(request, &identity, installed.as_ref(), true)?;
        Ok(DeployPlan {
            resources: vec![PlannedDeployResource {
                resource: identity.resource.clone(),
                desired_digest: identity.desired_digest,
            }],
            lock_resources: vec![self.client.logical_lock().to_owned()],
            config_digest: self.config_digest(&admitted.config.name),
            reversible: request.prior_receipt.is_none(),
            summary: format!(
                "{} plugin `{}` at `{}` ({})",
                self.client.as_str(),
                admitted.config.name,
                identity.coordinate,
                if already {
                    "already desired"
                } else {
                    "install"
                },
            ),
        })
    }

    fn fingerprint(
        &self,
        request: &DeployTargetRequest<'_>,
        _plan: &DeployPlan,
    ) -> Result<DeployFingerprint, MechanismError> {
        let version = self
            .planned_version
            .lock()
            .ok()
            .and_then(|slot| slot.clone())
            .map_or_else(|| self.version(request), Ok)?;
        Ok(DeployFingerprint {
            digest: framed_hash(
                FINGERPRINT_DOMAIN,
                &[
                    ("client", self.client.as_str()),
                    ("adapter-epoch", &ADAPTER_EPOCH.to_string()),
                    ("client-version", &version.rendered),
                ],
            ),
            summary: format!(
                "{} client plugin adapter epoch {ADAPTER_EPOCH}, tested client {}",
                self.client.as_str(),
                version.rendered,
            ),
        })
    }

    fn apply(
        &self,
        request: &DeployTargetRequest<'_>,
        _plan: &DeployPlan,
        checkpoint: &mut CheckpointLedger<'_>,
    ) -> Result<ApplyReport, MechanismError> {
        let admitted = self.admit(request)?;
        self.version(request)?;
        if self.client == PluginClient::OpenCode {
            return opencode::apply(request, &admitted.projection, checkpoint);
        }
        let identity = self.cli_identity(request, &admitted)?;
        let installed = self.list_cli(request, &admitted, &identity)?;
        let already = self.judge_cli(request, &identity, installed.as_ref(), false)?;
        self.reconcile_cli(request, &admitted, &identity, already, checkpoint)
    }

    fn verify(
        &self,
        request: &DeployTargetRequest<'_>,
        resources: &[String],
    ) -> Result<Vec<ObservedResource>, MechanismError> {
        if self.client == PluginClient::OpenCode {
            return opencode::verify_contained(request, resources);
        }
        let config = PluginDeployConfig::parse(&request.target.id, request.target.config.as_ref())?;
        let receipt_artifact = request
            .prior_receipt
            .map(|receipt| receipt.artifact_digest.as_str())
            .or_else(|| request.artifact.map(|artifact| artifact.digest.as_str()))
            .unwrap_or("");
        let (resource, digest) = self.cli_observed_digest(request, &config, receipt_artifact)?;
        if resources != std::slice::from_ref(&resource) {
            return Err(DeployProviderError::RemoveNotOwned {
                target: request.target.id.clone(),
                resource: resources
                    .iter()
                    .find(|item| **item != resource)
                    .cloned()
                    .unwrap_or(resource),
            }
            .into());
        }
        Ok(vec![ObservedResource { resource, digest }])
    }

    fn remove(
        &self,
        request: &DeployTargetRequest<'_>,
        resources: &[String],
        _prior_state_handle: Option<&str>,
    ) -> Result<RemoveReport, MechanismError> {
        if self.client == PluginClient::OpenCode {
            return opencode::remove(request, resources);
        }
        let config = PluginDeployConfig::parse(&request.target.id, request.target.config.as_ref())?;
        let marketplace =
            marketplace::marketplace_name(self.client, &request.target.id, &config.name);
        let coordinate = format!("{}@{marketplace}", config.name);
        let resource = format!("plugin:{}:{coordinate}", self.client.as_str());
        let receipt = request
            .prior_receipt
            .ok_or_else(|| DeployProviderError::RemoveNotOwned {
                target: request.target.id.clone(),
                resource: resource.clone(),
            })?;
        if resources != std::slice::from_ref(&resource)
            || !receipt
                .resources
                .iter()
                .any(|owned| owned.resource == resource)
        {
            return Err(DeployProviderError::RemoveNotOwned {
                target: request.target.id.clone(),
                resource,
            }
            .into());
        }
        self.version(request)?;
        let installed = wire::list(
            self.runner.as_ref(),
            self.client,
            request,
            &config.name,
            &marketplace,
            &coordinate,
        )?;
        if installed.is_some() {
            wire::remove(self.runner.as_ref(), self.client, request, &coordinate)?;
        }
        if wire::list(
            self.runner.as_ref(),
            self.client,
            request,
            &config.name,
            &marketplace,
            &coordinate,
        )?
        .is_some()
        {
            return Err(DeployProviderError::PluginOccupancy {
                target: request.target.id.clone(),
                resource: resource.clone(),
                reason: "post-remove list still reports the coordinate".to_owned(),
            }
            .into());
        }
        Ok(RemoveReport {
            removed: installed.map(|_| vec![resource]).unwrap_or_default(),
            evidence: format!(
                "{}: exact receipt-owned coordinate removed or already absent; marketplace support retained",
                self.client.as_str()
            ),
        })
    }

    fn recover(
        &self,
        request: &DeployTargetRequest<'_>,
        _plan: &DeployPlan,
        _observed: &[ObservedResource],
        checkpoint: &mut CheckpointLedger<'_>,
    ) -> Result<ApplyReport, MechanismError> {
        let admitted = self.admit(request)?;
        self.version(request)?;
        if self.client == PluginClient::OpenCode {
            return opencode::recover(request, &admitted.projection, checkpoint);
        }
        let identity = self.cli_identity(request, &admitted)?;
        let installed = self.list_cli(request, &admitted, &identity)?;
        if let Some(item) = &installed
            && (item.version != identity.version || !item.enabled || !item.user_scope)
        {
            return Err(DeployProviderError::PluginOccupancy {
                target: request.target.id.clone(),
                resource: identity.resource.clone(),
                reason: "interrupted coordinate is present but not at the desired list witness"
                    .to_owned(),
            }
            .into());
        }
        self.reconcile_cli(
            request,
            &admitted,
            &identity,
            installed.is_some(),
            checkpoint,
        )
    }
}

fn interrupted(request: &DeployTargetRequest<'_>, identity: &CliIdentity) -> bool {
    request.recovery_intent.is_some_and(|intent| {
        intent.prior_generation == request.prior_receipt.map(|receipt| receipt.generation)
            && intent.resources.iter().any(|planned| {
                planned.resource == identity.resource
                    && planned.desired_digest == identity.desired_digest
            })
    })
}

#[cfg(test)]
#[path = "plugin/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "plugin/fixture.rs"]
pub(crate) mod fixture;

#[cfg(test)]
#[path = "plugin/lifecycle_tests.rs"]
mod lifecycle_tests;

#[cfg(test)]
#[path = "plugin/correction_tests.rs"]
mod correction_tests;
