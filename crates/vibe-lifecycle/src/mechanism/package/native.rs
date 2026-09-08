//! Native package-provider adapter over the generated four-operation wire.

use vibe_core::manifest::ArtifactPackageTarget;
use vibe_native_loader::NativeMechanism;
use vibe_wire::behaviour::native_package::RetainedPackage;
use vibe_wire::generated::native::e1::{package_reply as reply, package_request as request};
use vibe_wire::generated::shared::{
    NativeDeployArtifactKind as WireKind, NativeDeployArtifactShape as WireShape,
};

use crate::mechanism::contain::{
    checked_relative, digest_file, forward_slashed, join_relative, tree_digest, walk_tree,
};
use crate::mechanism::error::preview;
use crate::native::{NativeMechanismBinding, PreparedNativeMechanism};

use super::protocol::InputOrigin;
use super::record::{
    NativeRecordInputs, VerifiedNativeOutput, artifact_shape, native_config_digest,
    native_input_digest, parse_kind, parse_shape, plan_path_fault, planned_owner_count,
    record_native_outputs, wire_kind, wire_shape,
};
use super::{
    PackageError, PackageExecution, PackageTargetRequest, PackagedArtifact, prepare_output_dir,
};

pub(super) trait NativePackageCalls {
    fn kinds(&self) -> &[WireKind];
    fn invoke(
        &self,
        target: &str,
        value: &request::PackageRequest,
        retained: RetainedPackage<'_>,
    ) -> Result<reply::PackageReply, String>;
}

impl NativePackageCalls for NativeMechanism {
    fn kinds(&self) -> &[WireKind] {
        &self.descriptor().artifact_kinds
    }

    fn invoke(
        &self,
        target: &str,
        value: &request::PackageRequest,
        retained: RetainedPackage<'_>,
    ) -> Result<reply::PackageReply, String> {
        self.invoke_package(target, value, retained)
            .map_err(|error| error.to_string())
    }
}

pub(super) fn execute(
    execution: &PackageExecution<'_>,
    request_value: &PackageTargetRequest<'_>,
    entry: &PreparedNativeMechanism,
    binding: &NativeMechanismBinding,
) -> Result<Vec<PackagedArtifact>, PackageError> {
    let mechanism = entry
        .admit(binding)
        .map_err(|error| fault(request_value.target, binding, "admit", &error.to_string()))?;
    execute_with(execution, request_value, entry, binding, &mechanism)
}

pub(super) fn execute_with(
    execution: &PackageExecution<'_>,
    domain: &PackageTargetRequest<'_>,
    entry: &PreparedNativeMechanism,
    binding: &NativeMechanismBinding,
    calls: &impl NativePackageCalls,
) -> Result<Vec<PackagedArtifact>, PackageError> {
    for output in &domain.target.outputs {
        if !calls.kinds().contains(&wire_kind(output.kind)) {
            return Err(fault(
                domain.target,
                binding,
                "admit",
                &format!(
                    "descriptor does not advertise output kind `{}`",
                    output.kind
                ),
            ));
        }
    }
    let target = wire_target(domain.target)
        .map_err(|reason| fault(domain.target, binding, "plan", &reason))?;
    let inputs = wire_inputs(domain);
    let authority = authority(domain, binding)?;
    let identity = identity(domain.target, binding);

    let plan_request = request::PackageRequest::Plan(Box::new(request::PackageRequestPlan {
        authority: authority.clone(),
        envelope: 1,
        identity: identity.clone(),
        inputs: inputs.clone(),
        protocol: binding.protocol,
        target: target.clone(),
    }));
    let plan_reply = calls
        .invoke(&domain.target.id, &plan_request, RetainedPackage::default())
        .map_err(|reason| fault(domain.target, binding, "plan", &reason))?;
    let plan = accepted_plan(domain.target, binding, plan_reply)?;
    validate_plan_ownership(domain.target, binding, &plan)?;

    let fingerprint_request =
        request::PackageRequest::Fingerprint(Box::new(request::PackageRequestFingerprint {
            authority: authority.clone(),
            envelope: 1,
            identity: identity.clone(),
            inputs: inputs.clone(),
            plan: plan.clone(),
            protocol: binding.protocol,
            target: target.clone(),
        }));
    let fingerprint_reply = calls
        .invoke(
            &domain.target.id,
            &fingerprint_request,
            RetainedPackage {
                target: Some(&target),
                inputs: Some(&inputs),
                authority: Some(&authority),
                plan: Some(&plan),
                ..RetainedPackage::default()
            },
        )
        .map_err(|reason| fault(domain.target, binding, "fingerprint", &reason))?;
    let fingerprint = accepted_fingerprint(domain.target, binding, fingerprint_reply)?;

    prepare_output_dir(domain)?;
    let staging = request::StagingAuthority {
        root_absolute: authority.output_root_absolute.clone(),
        root_relative: authority.output_root_relative.clone(),
    };
    let apply_request = request::PackageRequest::Apply(Box::new(request::PackageRequestApply {
        authority: authority.clone(),
        envelope: 1,
        fingerprint: fingerprint.clone(),
        identity: identity.clone(),
        inputs: inputs.clone(),
        plan: plan.clone(),
        protocol: binding.protocol,
        staging: staging.clone(),
        target: target.clone(),
    }));
    let apply_reply = calls
        .invoke(
            &domain.target.id,
            &apply_request,
            RetainedPackage {
                target: Some(&target),
                inputs: Some(&inputs),
                authority: Some(&authority),
                plan: Some(&plan),
                fingerprint: Some(&fingerprint),
                ..RetainedPackage::default()
            },
        )
        .map_err(|reason| fault(domain.target, binding, "apply", &reason))?;
    let (staged, apply_evidence) = accepted_staged(domain.target, binding, &plan, apply_reply)?;

    let verify_request = request::PackageRequest::Verify(Box::new(request::PackageRequestVerify {
        authority: authority.clone(),
        envelope: 1,
        fingerprint: fingerprint.clone(),
        identity,
        inputs: inputs.clone(),
        plan: plan.clone(),
        protocol: binding.protocol,
        staged: staged.clone(),
        staging: staging.clone(),
        target: target.clone(),
    }));
    let verify_reply = calls
        .invoke(
            &domain.target.id,
            &verify_request,
            RetainedPackage {
                target: Some(&target),
                inputs: Some(&inputs),
                authority: Some(&authority),
                plan: Some(&plan),
                fingerprint: Some(&fingerprint),
                staging: Some(&staging),
                staged: Some(&staged),
            },
        )
        .map_err(|reason| fault(domain.target, binding, "verify", &reason))?;
    let (verified, verify_evidence) = accepted_verified(domain.target, binding, verify_reply)?;
    let outputs = verify_outputs(domain, binding, &plan, &staged, &verified)?;
    let input_digest = native_input_digest(domain.inputs);
    let config_digest = native_config_digest(domain.target, &binding.pin)
        .map_err(|reason| fault(domain.target, binding, "record", &reason))?;
    let evidence = format!(
        "native plan={}; fingerprint={}; apply={apply_evidence}; verify={verify_evidence}",
        plan.summary, fingerprint.digest
    );
    record_native_outputs(
        execution,
        domain,
        &NativeRecordInputs {
            entry,
            binding,
            input_digest: &input_digest,
            config_digest: &config_digest,
            provider_fingerprint: &fingerprint.digest,
            evidence: &evidence,
        },
        outputs,
    )
}

fn wire_target(target: &ArtifactPackageTarget) -> Result<request::PackageTarget, String> {
    let config_toml = target
        .config
        .as_ref()
        .map(|value| toml::to_string(value.as_table()).map_err(|error| error.to_string()))
        .transpose()?;
    Ok(request::PackageTarget {
        id: target.id.clone(),
        outputs: target
            .outputs
            .iter()
            .map(|output| request::DeclaredOutput {
                id: output.id.clone(),
                kind: wire_kind(output.kind),
            })
            .collect(),
        config_toml,
    })
}

fn wire_inputs(domain: &PackageTargetRequest<'_>) -> Vec<request::ResolvedInput> {
    domain
        .inputs
        .iter()
        .map(|input| request::ResolvedInput {
            bytes: input.bytes.to_string(),
            digest: input.digest.clone(),
            name: input.name.clone(),
            origin: match input.origin {
                InputOrigin::ArtifactRecord { kind } => request::InputOrigin::ArtifactRecord(
                    Box::new(request::InputOriginArtifactRecord {
                        recorded_kind: wire_kind(kind),
                    }),
                ),
                InputOrigin::WorkspacePath => request::InputOrigin::WorkspacePath(Box::new(
                    request::InputOriginWorkspacePath {},
                )),
            },
            path_absolute: forward_slashed(&input.absolute),
            path_relative: input.relative.clone(),
            reference: input.reference.clone(),
            shape: wire_shape(&input.shape),
        })
        .collect()
}

fn authority(
    domain: &PackageTargetRequest<'_>,
    binding: &NativeMechanismBinding,
) -> Result<request::PackageAuthority, PackageError> {
    let package_root = checked_relative(domain.package_root)
        .map_err(|error| fault(domain.target, binding, "plan", error.reason()))?;
    let output_root = checked_relative(&format!("{package_root}/{}", domain.target.id))
        .map_err(|error| fault(domain.target, binding, "plan", error.reason()))?;
    Ok(request::PackageAuthority {
        output_root_absolute: vibe_core::machine_json_path(&join_relative(
            domain.project_root,
            &output_root,
        )),
        output_root_relative: output_root,
        package_root_absolute: vibe_core::machine_json_path(&join_relative(
            domain.project_root,
            &package_root,
        )),
        package_root_relative: package_root,
        project_root_absolute: vibe_core::machine_json_path(domain.project_root),
    })
}

fn identity(
    target: &ArtifactPackageTarget,
    binding: &NativeMechanismBinding,
) -> request::InvocationIdentity {
    request::InvocationIdentity {
        mechanism: target.mechanism.to_string(),
        provider: binding.pin.clone(),
        target: target.id.clone(),
    }
}

fn accepted_plan(
    target: &ArtifactPackageTarget,
    binding: &NativeMechanismBinding,
    value: reply::PackageReply,
) -> Result<request::PackagePlan, PackageError> {
    let reply::PackageReply::Plan(value) = value else {
        unreachable!("loader admits exact operation")
    };
    let reply::PlanResult::Ok(result) = value.result else {
        let reply::PlanResult::Fail(fail) = value.result else {
            unreachable!()
        };
        return Err(fault(target, binding, "plan", &fail.message));
    };
    let outputs = result
        .plan
        .outputs
        .into_iter()
        .map(|output| {
            Ok(request::PlannedOutput {
                id: output.id,
                kind: parse_kind(&output.kind)?,
                path_relative: output.path_relative,
                shape: parse_shape(&output.shape)?,
                media_type: output.media_type,
            })
        })
        .collect::<Result<Vec<_>, String>>()
        .map_err(|reason| fault(target, binding, "plan", &reason))?;
    Ok(request::PackagePlan {
        outputs,
        summary: result.plan.summary,
    })
}

fn accepted_fingerprint(
    target: &ArtifactPackageTarget,
    binding: &NativeMechanismBinding,
    value: reply::PackageReply,
) -> Result<request::PackageFingerprint, PackageError> {
    let reply::PackageReply::Fingerprint(value) = value else {
        unreachable!("loader admits exact operation")
    };
    match value.result {
        reply::FingerprintResult::Ok(result) => Ok(request::PackageFingerprint {
            counted_inputs: result.fingerprint.counted_inputs,
            digest: result.fingerprint.digest,
        }),
        reply::FingerprintResult::Fail(fail) => {
            Err(fault(target, binding, "fingerprint", &fail.message))
        }
    }
}

fn accepted_staged(
    target: &ArtifactPackageTarget,
    binding: &NativeMechanismBinding,
    plan: &request::PackagePlan,
    value: reply::PackageReply,
) -> Result<(Vec<request::StagedOutput>, String), PackageError> {
    let reply::PackageReply::Apply(value) = value else {
        unreachable!("loader admits exact operation")
    };
    let reply::ApplyResult::Ok(result) = value.result else {
        let reply::ApplyResult::Fail(fail) = value.result else {
            unreachable!()
        };
        return Err(fault(target, binding, "apply", &fail.message));
    };
    let staged = result
        .staged
        .into_iter()
        .zip(&plan.outputs)
        .map(|(staged, planned)| request::StagedOutput {
            id: staged.id,
            kind: planned.kind.clone(),
            path_relative: staged.path_relative,
            shape: planned.shape.clone(),
            media_type: planned.media_type.clone(),
        })
        .collect();
    Ok((staged, result.evidence))
}

fn accepted_verified(
    target: &ArtifactPackageTarget,
    binding: &NativeMechanismBinding,
    value: reply::PackageReply,
) -> Result<(Vec<reply::VerifiedOutput>, String), PackageError> {
    let reply::PackageReply::Verify(value) = value else {
        unreachable!("loader admits exact operation")
    };
    match value.result {
        reply::VerifyResult::Ok(result) => Ok((result.verified, result.evidence)),
        reply::VerifyResult::Fail(fail) => Err(fault(target, binding, "verify", &fail.message)),
    }
}

fn verify_outputs(
    domain: &PackageTargetRequest<'_>,
    binding: &NativeMechanismBinding,
    plan: &request::PackagePlan,
    staged: &[request::StagedOutput],
    verified: &[reply::VerifiedOutput],
) -> Result<Vec<VerifiedNativeOutput>, PackageError> {
    let root = domain.output_dir();
    let census = walk_tree(&root).map_err(|error| {
        output_fault(
            domain.target,
            binding,
            "staging",
            &error.path,
            &error.reason,
        )
    })?;
    for (path, _) in &census {
        let owners = planned_owner_count(plan, path);
        if owners != 1 {
            return Err(output_fault(
                domain.target,
                binding,
                "staging",
                path,
                if owners == 0 {
                    "staging contains a file no planned output owns"
                } else {
                    "staging file is claimed by more than one planned output"
                },
            ));
        }
    }
    let mut outputs = Vec::with_capacity(plan.outputs.len());
    for (index, planned) in plan.outputs.iter().enumerate() {
        let Some(staged) = staged.get(index) else {
            return Err(fault(
                domain.target,
                binding,
                "verify",
                "staged output count changed",
            ));
        };
        let Some(claimed) = verified.get(index) else {
            return Err(fault(
                domain.target,
                binding,
                "verify",
                "verified output count changed",
            ));
        };
        let absolute = if planned.path_relative == "." {
            root.clone()
        } else {
            join_relative(&root, &planned.path_relative)
        };
        let (digest, bytes, files) = witness(domain.target, binding, planned, &absolute)?;
        let claimed_bytes = decimal(domain.target, binding, &planned.id, &claimed.bytes)?;
        let claimed_files = decimal(domain.target, binding, &planned.id, &claimed.files)?;
        if claimed.digest != digest || claimed_bytes != bytes || claimed_files != files as u64 {
            return Err(output_fault(
                domain.target,
                binding,
                &planned.id,
                &planned.path_relative,
                "provider digest/byte/file claim differs from current staged output",
            ));
        }
        let path_relative = if planned.path_relative == "." {
            domain.output_dir_relative()
        } else {
            format!("{}/{}", domain.output_dir_relative(), planned.path_relative)
        };
        outputs.push(VerifiedNativeOutput {
            id: planned.id.clone(),
            kind: domain.target.outputs[index].kind,
            shape: artifact_shape(&planned.shape),
            path_absolute: forward_slashed(&absolute),
            path_relative,
            digest,
            bytes,
            files,
            media_type: staged.media_type.clone(),
        });
    }
    Ok(outputs)
}

fn witness(
    target: &ArtifactPackageTarget,
    binding: &NativeMechanismBinding,
    planned: &request::PlannedOutput,
    absolute: &std::path::Path,
) -> Result<(String, u64, usize), PackageError> {
    match planned.shape {
        WireShape::File => digest_file(absolute)
            .map(|(digest, bytes)| (digest, bytes, 1))
            .map_err(|error| {
                output_fault(
                    target,
                    binding,
                    &planned.id,
                    &planned.path_relative,
                    &error.reason(),
                )
            }),
        WireShape::Directory => tree_digest(absolute)
            .map(|tree| (tree.digest, tree.bytes, tree.files))
            .map_err(|error| {
                output_fault(target, binding, &planned.id, &error.path, &error.reason)
            }),
    }
}

fn validate_plan_ownership(
    target: &ArtifactPackageTarget,
    binding: &NativeMechanismBinding,
    plan: &request::PackagePlan,
) -> Result<(), PackageError> {
    if let Some((id, reason)) = plan_path_fault(plan) {
        let path = plan
            .outputs
            .iter()
            .find(|output| output.id == id)
            .map_or("provider-plan", |output| output.path_relative.as_str());
        return Err(output_fault(target, binding, id, path, reason));
    }
    Ok(())
}

fn decimal(
    target: &ArtifactPackageTarget,
    binding: &NativeMechanismBinding,
    output: &str,
    value: &str,
) -> Result<u64, PackageError> {
    value.parse::<u64>().map_err(|_| {
        output_fault(
            target,
            binding,
            output,
            "provider-claim",
            "provider count is not canonical u64",
        )
    })
}

fn fault(
    target: &ArtifactPackageTarget,
    binding: &NativeMechanismBinding,
    operation: &'static str,
    reason: &str,
) -> PackageError {
    PackageError::NativeTransport {
        target: target.id.clone(),
        pin: binding.pin.clone(),
        operation,
        reason: preview(reason),
    }
}

fn output_fault(
    target: &ArtifactPackageTarget,
    binding: &NativeMechanismBinding,
    output: &str,
    path: &str,
    reason: &str,
) -> PackageError {
    PackageError::NativeOutput {
        target: target.id.clone(),
        pin: binding.pin.clone(),
        output: preview(output),
        path: preview(path),
        reason: preview(reason),
    }
}
