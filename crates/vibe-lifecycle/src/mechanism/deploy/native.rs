//! Native ABI-1 adapter for an already-prepared deploy mechanism.
//! Selection/build/record/image publication happened at the build fence. This
//! cell only admits that immutable image, translates the six generated wire
//! operations, and hands answers back to the incumbent deploy engine.

use std::sync::Mutex;

use vibe_core::manifest::{ArtifactKind, DeployTarget};
use vibe_native_loader::NativeMechanism;
use vibe_wire::behaviour::native_deploy::{ENVELOPE_EPOCH, PROTOCOL_EPOCH};
use vibe_wire::generated::native::e1::{deploy_reply as reply, deploy_request as request};

use super::protocol::{
    ApplyReport, DeployDescriptor, DeployFingerprint, DeployPlan, DeployProvider,
    DeployTargetRequest, ObservedResource, PlannedDeployResource, RemoveReport, artifact_kind,
    authority, effect, network, operation, prior_receipt, privilege, recovery_intent,
    reversibility, wire_artifact, wire_path, wire_plan,
};
use crate::mechanism::deploy::ledger::CheckpointLedger;
use crate::mechanism::error::deploy::{
    native_digest, native_lock_identity, native_owned_identity, native_scalar,
    native_transport as native_fault,
};
use crate::mechanism::{
    EffectClass, MechanismError, NetworkUse, PrivilegeNeed, ProviderDescriptor, ProviderOperation,
    Reversibility,
};
use crate::native::{NativeMechanismBinding, PreparedNativeMechanism};

pub(super) struct NativeDeployProvider {
    mechanism: NativeMechanism,
    provider: String,
    kinds: Vec<ArtifactKind>,
    operations: Vec<ProviderOperation>,
    effect: EffectClass,
    network: NetworkUse,
    privilege: PrivilegeNeed,
    reversibility: Reversibility,
    atomic_replacement: bool,
    reference_ownership: bool,
    planned: Mutex<Vec<PlannedDeployResource>>,
}

impl NativeDeployProvider {
    pub(super) fn new(
        prepared: &PreparedNativeMechanism,
        binding: &NativeMechanismBinding,
        target: &DeployTarget,
    ) -> Result<Self, MechanismError> {
        if binding.protocol != PROTOCOL_EPOCH {
            return Err(native_fault(
                &target.id,
                &binding.pin,
                "admit",
                "prepared binding protocol differs from native deploy protocol 1",
            ));
        }
        let mechanism = prepared
            .admit(binding)
            .map_err(|error| native_fault(&target.id, &binding.pin, "admit", &error.to_string()))?;
        let descriptor = mechanism.descriptor();
        if descriptor.protocol != binding.protocol || descriptor.id != binding.descriptor_id {
            return Err(native_fault(
                &target.id,
                &binding.pin,
                "admit",
                "admitted descriptor differs from prepared binding",
            ));
        }
        Ok(Self {
            kinds: descriptor
                .artifact_kinds
                .iter()
                .map(artifact_kind)
                .collect(),
            operations: descriptor.operations.iter().map(operation).collect(),
            effect: effect(&descriptor.effect),
            network: network(&descriptor.network),
            privilege: privilege(&descriptor.privilege),
            reversibility: reversibility(&descriptor.reversibility),
            atomic_replacement: descriptor.atomic_replacement,
            reference_ownership: descriptor.reference_ownership,
            mechanism,
            provider: binding.pin.clone(),
            planned: Mutex::new(Vec::new()),
        })
    }

    fn invoke(
        &self,
        target: &str,
        operation: &'static str,
        request: &request::DeployRequest,
    ) -> Result<reply::DeployReply, MechanismError> {
        self.mechanism
            .invoke(request)
            .map_err(|error| native_fault(target, &self.provider, operation, &error.to_string()))
    }

    fn identity(&self, request: &DeployTargetRequest<'_>) -> request::InvocationIdentity {
        request::InvocationIdentity {
            provider: self.provider.clone(),
            mechanism: self.mechanism.descriptor().id.clone(),
            target: request.target.id.clone(),
            profile: request.profile.to_owned(),
        }
    }

    fn artifact(
        &self,
        request: &DeployTargetRequest<'_>,
        operation: &'static str,
    ) -> Result<request::DeployArtifact, MechanismError> {
        let artifact = request.artifact.ok_or_else(|| {
            native_fault(
                &request.target.id,
                &self.provider,
                operation,
                "the engine supplied no resolved artifact",
            )
        })?;
        if !self.kinds.contains(&artifact.kind) {
            return Err(native_fault(
                &request.target.id,
                &self.provider,
                operation,
                "resolved artifact kind is absent from the admitted descriptor",
            ));
        }
        Ok(wire_artifact(artifact))
    }

    fn desired(
        &self,
        request: &DeployTargetRequest<'_>,
        resources: &[String],
    ) -> Result<Vec<request::PlannedResource>, MechanismError> {
        let planned = self.planned.lock().map_err(|_| {
            native_fault(
                &request.target.id,
                &self.provider,
                "verify",
                "native plan cache is unavailable",
            )
        })?;
        resources
            .iter()
            .map(|resource| {
                let digest = planned
                    .iter()
                    .find(|row| row.resource == *resource)
                    .map(|row| row.desired_digest.clone())
                    .or_else(|| {
                        request.prior_receipt.and_then(|receipt| {
                            receipt
                                .resources
                                .iter()
                                .find(|row| row.resource == *resource)
                                .map(|row| row.post_digest.clone())
                        })
                    })
                    .ok_or_else(|| {
                        native_fault(
                            &request.target.id,
                            &self.provider,
                            "verify",
                            "requested resource has no retained desired digest",
                        )
                    })?;
                Ok(request::PlannedResource {
                    resource: resource.clone(),
                    desired_digest: digest,
                })
            })
            .collect()
    }
}

impl DeployProvider for NativeDeployProvider {
    fn descriptor(&self) -> DeployDescriptor<'_> {
        DeployDescriptor {
            provider: ProviderDescriptor {
                key: &self.provider,
                kinds: &self.kinds,
                effect: self.effect,
                network: self.network,
                privilege: self.privilege,
                reversibility: self.reversibility,
                operations: &self.operations,
            },
            atomic_replacement: self.atomic_replacement,
            reference_ownership: self.reference_ownership,
        }
    }

    fn plan(&self, call: &DeployTargetRequest<'_>) -> Result<DeployPlan, MechanismError> {
        let request = request::DeployRequest::Plan(Box::new(request::DeployRequestPlan {
            envelope: ENVELOPE_EPOCH,
            protocol: PROTOCOL_EPOCH,
            identity: self.identity(call),
            artifact: self.artifact(call, "plan")?,
            authority: authority(call),
            config_toml: config(call, &self.provider)?,
            prior_receipt: call.prior_receipt.map(prior_receipt),
            recovery_intent: call.recovery_intent.map(recovery_intent),
        }));
        let reply::DeployReply::Plan(reply) = self.invoke(&call.target.id, "plan", &request)?
        else {
            unreachable!("loader admits exact reply operation")
        };
        let reply::PlanResult::Ok(result) = reply.result else {
            let reply::PlanResult::Fail(fail) = reply.result else {
                unreachable!()
            };
            return Err(native_fault(
                &call.target.id,
                &self.provider,
                "plan",
                &fail.message,
            ));
        };
        scalar(&result.summary, call, &self.provider, "plan", "summary")?;
        digest(&result.config_digest, call, &self.provider, "plan")?;
        let resources = result
            .resources
            .into_iter()
            .map(|row| {
                digest(&row.desired_digest, call, &self.provider, "plan")?;
                Ok(PlannedDeployResource {
                    resource: owned_resource(&row.resource, call, &self.provider, "plan")?,
                    desired_digest: row.desired_digest,
                })
            })
            .collect::<Result<Vec<_>, MechanismError>>()?;
        let lock_resources = result
            .lock_resources
            .iter()
            .map(|resource| lock_resource(resource, call, &self.provider, "plan"))
            .collect::<Result<Vec<_>, _>>()?;
        *self.planned.lock().map_err(|_| {
            native_fault(
                &call.target.id,
                &self.provider,
                "plan",
                "native plan cache is unavailable",
            )
        })? = resources.clone();
        Ok(DeployPlan {
            resources,
            lock_resources,
            config_digest: result.config_digest,
            reversible: result.reversible,
            summary: result.summary,
        })
    }

    fn fingerprint(
        &self,
        call: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
    ) -> Result<DeployFingerprint, MechanismError> {
        let request =
            request::DeployRequest::Fingerprint(Box::new(request::DeployRequestFingerprint {
                envelope: ENVELOPE_EPOCH,
                protocol: PROTOCOL_EPOCH,
                identity: self.identity(call),
                artifact: self.artifact(call, "fingerprint")?,
                plan: wire_plan(plan),
                config_toml: config(call, &self.provider)?,
            }));
        let reply::DeployReply::Fingerprint(reply) =
            self.invoke(&call.target.id, "fingerprint", &request)?
        else {
            unreachable!("loader admits exact reply operation")
        };
        match reply.result {
            reply::FingerprintResult::Ok(result) => {
                digest(&result.digest, call, &self.provider, "fingerprint")?;
                scalar(
                    &result.summary,
                    call,
                    &self.provider,
                    "fingerprint",
                    "summary",
                )?;
                Ok(DeployFingerprint {
                    digest: result.digest,
                    summary: result.summary,
                })
            }
            reply::FingerprintResult::Fail(fail) => Err(native_fault(
                &call.target.id,
                &self.provider,
                "fingerprint",
                &fail.message,
            )),
        }
    }

    fn apply(
        &self,
        call: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
        checkpoint: &mut CheckpointLedger<'_>,
    ) -> Result<ApplyReport, MechanismError> {
        let request = request::DeployRequest::Apply(Box::new(request::DeployRequestApply {
            envelope: ENVELOPE_EPOCH,
            protocol: PROTOCOL_EPOCH,
            identity: self.identity(call),
            artifact: self.artifact(call, "apply")?,
            plan: wire_plan(plan),
            staging: call.staging.map(wire_path),
            prior_receipt: call.prior_receipt.map(prior_receipt),
        }));
        let reply::DeployReply::Apply(reply) = self.invoke(&call.target.id, "apply", &request)?
        else {
            unreachable!("loader admits exact reply operation")
        };
        apply_result(
            reply.result,
            call,
            &self.provider,
            "apply",
            plan,
            checkpoint,
        )
    }

    fn verify(
        &self,
        call: &DeployTargetRequest<'_>,
        resources: &[String],
    ) -> Result<Vec<ObservedResource>, MechanismError> {
        let desired = self.desired(call, resources)?;
        let request = request::DeployRequest::Verify(Box::new(request::DeployRequestVerify {
            envelope: ENVELOPE_EPOCH,
            protocol: PROTOCOL_EPOCH,
            identity: self.identity(call),
            resources: desired,
        }));
        let reply::DeployReply::Verify(reply) = self.invoke(&call.target.id, "verify", &request)?
        else {
            unreachable!("loader admits exact reply operation")
        };
        match reply.result {
            reply::VerifyResult::Ok(result) => {
                let observed = result
                    .observed
                    .into_iter()
                    .map(|row| ObservedResource {
                        resource: row.resource,
                        digest: row.digest,
                    })
                    .collect::<Vec<_>>();
                if observed
                    .iter()
                    .map(|row| &row.resource)
                    .ne(resources.iter())
                {
                    return Err(native_fault(
                        &call.target.id,
                        &self.provider,
                        "verify",
                        "reply resources differ from the exact requested order",
                    ));
                }
                for row in &observed {
                    if let Some(value) = &row.digest {
                        digest(value, call, &self.provider, "verify")?;
                    }
                }
                Ok(observed)
            }
            reply::VerifyResult::Fail(fail) => Err(native_fault(
                &call.target.id,
                &self.provider,
                "verify",
                &fail.message,
            )),
        }
    }

    fn remove(
        &self,
        call: &DeployTargetRequest<'_>,
        resources: &[String],
        prior_state_handle: Option<&str>,
    ) -> Result<RemoveReport, MechanismError> {
        let request = request::DeployRequest::Remove(Box::new(request::DeployRequestRemove {
            envelope: ENVELOPE_EPOCH,
            protocol: PROTOCOL_EPOCH,
            identity: self.identity(call),
            resources: resources.to_vec(),
            prior_state_handle: prior_state_handle.map(str::to_owned),
        }));
        let reply::DeployReply::Remove(reply) = self.invoke(&call.target.id, "remove", &request)?
        else {
            unreachable!("loader admits exact reply operation")
        };
        match reply.result {
            reply::RemoveResult::Ok(result)
                if result.expected_remaining.is_empty() && result.removed == resources =>
            {
                scalar(&result.evidence, call, &self.provider, "remove", "evidence")?;
                Ok(RemoveReport {
                    removed: result.removed,
                    expected_remaining: Vec::new(),
                    evidence: result.evidence,
                })
            }
            reply::RemoveResult::Ok(_) => Err(native_fault(
                &call.target.id,
                &self.provider,
                "remove",
                "remove reply must acknowledge the exact requested resources and no expected_remaining rows",
            )),
            reply::RemoveResult::Fail(fail) => Err(native_fault(
                &call.target.id,
                &self.provider,
                "remove",
                &fail.message,
            )),
        }
    }

    fn recover(
        &self,
        call: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
        observed: &[ObservedResource],
        checkpoint: &mut CheckpointLedger<'_>,
    ) -> Result<ApplyReport, MechanismError> {
        let request = request::DeployRequest::Recover(Box::new(request::DeployRequestRecover {
            envelope: ENVELOPE_EPOCH,
            protocol: PROTOCOL_EPOCH,
            identity: self.identity(call),
            artifact: self.artifact(call, "recover")?,
            plan: wire_plan(plan),
            observed: observed
                .iter()
                .map(|row| request::ObservedResource {
                    resource: row.resource.clone(),
                    digest: row.digest.clone(),
                })
                .collect(),
            staging: call.staging.map(wire_path),
            prior_receipt: call.prior_receipt.map(prior_receipt),
            recovery_intent: call.recovery_intent.map(recovery_intent),
        }));
        let reply::DeployReply::Recover(reply) =
            self.invoke(&call.target.id, "recover", &request)?
        else {
            unreachable!("loader admits exact reply operation")
        };
        apply_result(
            reply.result,
            call,
            &self.provider,
            "recover",
            plan,
            checkpoint,
        )
    }
}

fn apply_result(
    result: impl Into<ApplyOrRecover>,
    call: &DeployTargetRequest<'_>,
    provider: &str,
    operation: &'static str,
    plan: &DeployPlan,
    checkpoint: &mut CheckpointLedger<'_>,
) -> Result<ApplyReport, MechanismError> {
    match result.into() {
        ApplyOrRecover::Ok(completed, evidence, prior_state_handle) => {
            scalar(&evidence, call, provider, operation, "evidence")?;
            for resource in completed {
                if !plan.resources.iter().any(|row| row.resource == resource) {
                    return Err(native_fault(
                        &call.target.id,
                        provider,
                        operation,
                        "completed resource is absent from the accepted plan",
                    ));
                }
                checkpoint.completed(&resource)?;
            }
            Ok(ApplyReport {
                prior_state_handle,
                evidence,
            })
        }
        ApplyOrRecover::Fail(message) => {
            Err(native_fault(&call.target.id, provider, operation, &message))
        }
    }
}

enum ApplyOrRecover {
    Ok(Vec<String>, String, Option<String>),
    Fail(String),
}
impl From<reply::ApplyResult> for ApplyOrRecover {
    fn from(value: reply::ApplyResult) -> Self {
        match value {
            reply::ApplyResult::Ok(v) => Self::Ok(v.completed, v.evidence, v.prior_state_handle),
            reply::ApplyResult::Fail(v) => Self::Fail(v.message),
        }
    }
}
impl From<reply::RecoverResult> for ApplyOrRecover {
    fn from(value: reply::RecoverResult) -> Self {
        match value {
            reply::RecoverResult::Ok(v) => Self::Ok(v.completed, v.evidence, v.prior_state_handle),
            reply::RecoverResult::Fail(v) => Self::Fail(v.message),
        }
    }
}

fn config(
    call: &DeployTargetRequest<'_>,
    provider: &str,
) -> Result<Option<String>, MechanismError> {
    call.target
        .config
        .as_ref()
        .map(|value| {
            toml::to_string(value.as_table()).map_err(|error| {
                native_fault(
                    &call.target.id,
                    provider,
                    "plan",
                    &format!("canonical config encoding failed: {error}"),
                )
            })
        })
        .transpose()
}
fn owned_resource(
    value: &str,
    call: &DeployTargetRequest<'_>,
    provider: &str,
    operation: &'static str,
) -> Result<String, MechanismError> {
    native_owned_identity(value, &call.target.id, provider, operation)
}
fn lock_resource(
    value: &str,
    call: &DeployTargetRequest<'_>,
    provider: &str,
    operation: &'static str,
) -> Result<String, MechanismError> {
    native_lock_identity(value, &call.target.id, provider, operation)
}
fn digest(
    value: &str,
    call: &DeployTargetRequest<'_>,
    provider: &str,
    operation: &'static str,
) -> Result<(), MechanismError> {
    native_digest(value, &call.target.id, provider, operation)
}
fn scalar(
    value: &str,
    call: &DeployTargetRequest<'_>,
    provider: &str,
    operation: &'static str,
    name: &str,
) -> Result<(), MechanismError> {
    native_scalar(value, &call.target.id, provider, operation, name)
}
