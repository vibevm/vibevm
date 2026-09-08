//! Native build-provider adapter over the generated four-operation wire.

use vibe_core::manifest::{ArtifactBuildTarget, ArtifactInput, ArtifactKind};
use vibe_native_loader::NativeMechanism;
use vibe_wire::behaviour::native_build::RetainedBuild;
use vibe_wire::generated::artifact_record::ArtifactShape;
use vibe_wire::generated::native::e1::{build_reply as reply, build_request as request};
use vibe_wire::generated::shared::{
    NativeDeployArtifactKind as WireKind, NativeDeployArtifactShape as WireShape,
};

use crate::mechanism::contain::{
    checked_relative, digest_file, forward_slashed, join_relative, tree_digest, walk_tree,
};
use crate::mechanism::error::preview;
use crate::native::{NativeMechanismBinding, PreparedNativeMechanism};

use super::record::{
    NativeRecordInputs, VerifiedNativeOutput, config_fingerprint, record_native_outputs,
};
use super::{BuildError, BuildExecution, ProducedArtifact};

#[path = "rollback.rs"]
mod rollback;

pub(super) trait NativeBuildCalls {
    fn kinds(&self) -> &[WireKind];
    fn invoke(
        &self,
        target: &str,
        value: &request::BuildRequest,
        retained: RetainedBuild<'_>,
    ) -> Result<reply::BuildReply, String>;
}

impl NativeBuildCalls for NativeMechanism {
    fn kinds(&self) -> &[WireKind] {
        &self.descriptor().artifact_kinds
    }

    fn invoke(
        &self,
        target: &str,
        value: &request::BuildRequest,
        retained: RetainedBuild<'_>,
    ) -> Result<reply::BuildReply, String> {
        self.invoke_build(target, value, retained)
            .map_err(|error| error.to_string())
    }
}

pub(super) fn execute(
    execution: &BuildExecution<'_>,
    target: &ArtifactBuildTarget,
    entry: &PreparedNativeMechanism,
    binding: &NativeMechanismBinding,
) -> Result<Vec<ProducedArtifact>, BuildError> {
    let mechanism = entry
        .admit(binding)
        .map_err(|error| fault(target, binding, "admit", &error.to_string()))?;
    execute_with(execution, target, entry, binding, &mechanism)
}

pub(super) fn execute_with(
    execution: &BuildExecution<'_>,
    target: &ArtifactBuildTarget,
    entry: &PreparedNativeMechanism,
    binding: &NativeMechanismBinding,
    calls: &impl NativeBuildCalls,
) -> Result<Vec<ProducedArtifact>, BuildError> {
    for output in &target.outputs {
        let kind = wire_kind(output.kind);
        if !calls.kinds().contains(&kind) {
            return Err(fault(
                target,
                binding,
                "admit",
                &format!(
                    "descriptor does not advertise output kind `{}`",
                    output.kind
                ),
            ));
        }
    }
    let target_wire =
        wire_target(target).map_err(|reason| fault(target, binding, "plan", &reason))?;
    let authority = authority(execution, target, binding)?;
    let identity = identity(target, binding);

    let plan_request = request::BuildRequest::Plan(Box::new(request::BuildRequestPlan {
        envelope: 1,
        protocol: binding.protocol,
        identity: identity.clone(),
        target: target_wire.clone(),
        authority: authority.clone(),
    }));
    let plan_reply = calls
        .invoke(&target.id, &plan_request, RetainedBuild::default())
        .map_err(|reason| fault(target, binding, "plan", &reason))?;
    let plan = accepted_plan(target, binding, plan_reply)?;
    validate_plan_ownership(target, binding, &plan)?;

    let fingerprint_request =
        request::BuildRequest::Fingerprint(Box::new(request::BuildRequestFingerprint {
            envelope: 1,
            protocol: binding.protocol,
            identity: identity.clone(),
            target: target_wire.clone(),
            authority: authority.clone(),
            plan: plan.clone(),
        }));
    let fingerprint_reply = calls
        .invoke(
            &target.id,
            &fingerprint_request,
            RetainedBuild {
                target: Some(&target_wire),
                authority: Some(&authority),
                plan: Some(&plan),
                ..RetainedBuild::default()
            },
        )
        .map_err(|reason| fault(target, binding, "fingerprint", &reason))?;
    let fingerprint = accepted_fingerprint(target, binding, fingerprint_reply)?;

    let (staging, staging_absolute) =
        rollback::staging(execution.project_root, execution.build_root, &target.id)
            .map_err(|reason| fault(target, binding, "apply", &reason))?;
    let rollback = rollback::Snapshot::capture(
        execution.project_root,
        execution.build_root,
        &staging.root_relative,
        &target.id,
        target.outputs.iter().map(|output| output.id.as_str()),
    )
    .map_err(|reason| fault(target, binding, "snapshot", &reason))?;
    let outcome = (|| {
        let apply_request = request::BuildRequest::Apply(Box::new(request::BuildRequestApply {
            envelope: 1,
            protocol: binding.protocol,
            identity: identity.clone(),
            target: target_wire.clone(),
            authority: authority.clone(),
            plan: plan.clone(),
            fingerprint: fingerprint.clone(),
            staging: staging.clone(),
        }));
        let apply_reply = calls
            .invoke(
                &target.id,
                &apply_request,
                RetainedBuild {
                    target: Some(&target_wire),
                    authority: Some(&authority),
                    plan: Some(&plan),
                    fingerprint: Some(&fingerprint),
                    ..RetainedBuild::default()
                },
            )
            .map_err(|reason| fault(target, binding, "apply", &reason))?;
        let (staged, apply_evidence) = accepted_staged(target, binding, &plan, apply_reply)?;

        let verify_request = request::BuildRequest::Verify(Box::new(request::BuildRequestVerify {
            envelope: 1,
            protocol: binding.protocol,
            identity,
            target: target_wire.clone(),
            authority: authority.clone(),
            plan: plan.clone(),
            fingerprint: fingerprint.clone(),
            staging: staging.clone(),
            staged: staged.clone(),
        }));
        let verify_reply = calls
            .invoke(
                &target.id,
                &verify_request,
                RetainedBuild {
                    target: Some(&target_wire),
                    authority: Some(&authority),
                    plan: Some(&plan),
                    fingerprint: Some(&fingerprint),
                    staging: Some(&staging),
                    staged: Some(&staged),
                },
            )
            .map_err(|reason| fault(target, binding, "verify", &reason))?;
        let (verified, verify_evidence) = accepted_verified(target, binding, verify_reply)?;
        let outputs = verify_outputs(
            target,
            binding,
            &staging.root_relative,
            &staging_absolute,
            &plan,
            &staged,
            &verified,
        )?;
        let config = config_fingerprint(target, &binding.pin)
            .map_err(|reason| fault(target, binding, "record", &reason))?;
        let evidence = format!(
            "native plan={}; fingerprint={}; apply={apply_evidence}; verify={verify_evidence}",
            plan.summary, fingerprint.summary
        );
        record_native_outputs(
            execution,
            target,
            &NativeRecordInputs {
                entry,
                binding,
                config: &config,
                fingerprint: &fingerprint.digest,
                evidence: &evidence,
            },
            outputs,
        )
    })();
    match outcome {
        Ok(produced) => {
            rollback.commit();
            Ok(produced)
        }
        Err(error) => {
            let original = error.to_string();
            rollback.restore().map_err(|reason| {
                fault(
                    target,
                    binding,
                    "rollback",
                    &format!("{reason}; original failure: {original}"),
                )
            })?;
            Err(error)
        }
    }
}

fn wire_target(target: &ArtifactBuildTarget) -> Result<request::BuildTarget, String> {
    let inputs = target
        .inputs
        .iter()
        .flatten()
        .map(|input| match input {
            ArtifactInput::Path { path } => {
                request::DeclaredInput::Path(Box::new(request::DeclaredInputPath {
                    path_relative: path.display().to_string().replace('\\', "/"),
                }))
            }
            ArtifactInput::Artifact { artifact } => {
                request::DeclaredInput::Artifact(Box::new(request::DeclaredInputArtifact {
                    artifact: artifact.clone(),
                }))
            }
        })
        .collect();
    let outputs = target
        .outputs
        .iter()
        .map(|output| {
            let select_toml = output
                .select
                .as_ref()
                .map(|value| toml::to_string(value.as_table()).map_err(|error| error.to_string()))
                .transpose()?;
            Ok(request::DeclaredOutput {
                id: output.id.clone(),
                kind: wire_kind(output.kind),
                select_toml,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let config_toml = target
        .config
        .as_ref()
        .map(|value| toml::to_string(value.as_table()).map_err(|error| error.to_string()))
        .transpose()?;
    Ok(request::BuildTarget {
        id: target.id.clone(),
        outputs,
        workdir: target.workdir.clone(),
        config_toml,
        inputs,
    })
}

fn identity(
    target: &ArtifactBuildTarget,
    binding: &NativeMechanismBinding,
) -> request::InvocationIdentity {
    request::InvocationIdentity {
        provider: binding.pin.clone(),
        mechanism: target.mechanism.to_string(),
        target: target.id.clone(),
    }
}

fn authority(
    execution: &BuildExecution<'_>,
    target: &ArtifactBuildTarget,
    binding: &NativeMechanismBinding,
) -> Result<request::BuildAuthority, BuildError> {
    let build_root = checked_relative(execution.build_root)
        .map_err(|error| fault(target, binding, "plan", error.reason()))?;
    Ok(request::BuildAuthority {
        project_root_absolute: vibe_core::machine_json_path(execution.project_root),
        build_root_absolute: vibe_core::machine_json_path(&join_relative(
            execution.project_root,
            &build_root,
        )),
        build_root_relative: build_root,
        offline: execution.offline,
    })
}

fn accepted_plan(
    target: &ArtifactBuildTarget,
    binding: &NativeMechanismBinding,
    value: reply::BuildReply,
) -> Result<request::BuildPlan, BuildError> {
    let reply::BuildReply::Plan(value) = value else {
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
            })
        })
        .collect::<Result<Vec<_>, String>>()
        .map_err(|reason| fault(target, binding, "plan", &reason))?;
    Ok(request::BuildPlan {
        outputs,
        summary: result.plan.summary,
    })
}

fn accepted_fingerprint(
    target: &ArtifactBuildTarget,
    binding: &NativeMechanismBinding,
    value: reply::BuildReply,
) -> Result<request::BuildFingerprint, BuildError> {
    let reply::BuildReply::Fingerprint(value) = value else {
        unreachable!("loader admits exact operation")
    };
    match value.result {
        reply::FingerprintResult::Ok(result) => Ok(request::BuildFingerprint {
            digest: result.fingerprint.digest,
            summary: result.fingerprint.summary,
        }),
        reply::FingerprintResult::Fail(fail) => {
            Err(fault(target, binding, "fingerprint", &fail.message))
        }
    }
}

fn accepted_staged(
    target: &ArtifactBuildTarget,
    binding: &NativeMechanismBinding,
    plan: &request::BuildPlan,
    value: reply::BuildReply,
) -> Result<(Vec<request::StagedOutput>, String), BuildError> {
    let reply::BuildReply::Apply(value) = value else {
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
            fresh: staged.fresh,
            id: staged.id,
            kind: planned.kind.clone(),
            path_relative: staged.path_relative,
            shape: planned.shape.clone(),
        })
        .collect();
    Ok((staged, result.evidence))
}

fn accepted_verified(
    target: &ArtifactBuildTarget,
    binding: &NativeMechanismBinding,
    value: reply::BuildReply,
) -> Result<(Vec<reply::VerifiedOutput>, String), BuildError> {
    let reply::BuildReply::Verify(value) = value else {
        unreachable!("loader admits exact operation")
    };
    match value.result {
        reply::VerifyResult::Ok(result) => Ok((result.verified, result.evidence)),
        reply::VerifyResult::Fail(fail) => Err(fault(target, binding, "verify", &fail.message)),
    }
}

fn verify_outputs(
    target: &ArtifactBuildTarget,
    binding: &NativeMechanismBinding,
    staging_relative: &str,
    staging_absolute: &std::path::Path,
    plan: &request::BuildPlan,
    staged: &[request::StagedOutput],
    verified: &[reply::VerifiedOutput],
) -> Result<Vec<VerifiedNativeOutput>, BuildError> {
    let census = walk_tree(staging_absolute)
        .map_err(|error| output_fault(target, binding, "staging", &error.path, &error.reason))?;
    for (path, _) in &census {
        let owners = plan
            .outputs
            .iter()
            .filter(|output| match output.shape {
                WireShape::File => path == &output.path_relative,
                WireShape::Directory => descendant(&output.path_relative, path),
            })
            .count();
        if owners != 1 {
            return Err(output_fault(
                target,
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
        let staged = &staged[index];
        let claimed = &verified[index];
        let absolute = join_relative(staging_absolute, &planned.path_relative);
        let (digest, bytes, files) = match planned.shape {
            WireShape::File => {
                let (digest, bytes) = digest_file(&absolute).map_err(|error| {
                    output_fault(
                        target,
                        binding,
                        &planned.id,
                        &planned.path_relative,
                        &error.reason(),
                    )
                })?;
                (digest, bytes, 1)
            }
            WireShape::Directory => {
                let tree = tree_digest(&absolute).map_err(|error| {
                    output_fault(target, binding, &planned.id, &error.path, &error.reason)
                })?;
                (tree.digest, tree.bytes, tree.files)
            }
        };
        let claimed_bytes = claimed.bytes.parse::<u64>().map_err(|_| {
            output_fault(
                target,
                binding,
                &planned.id,
                &planned.path_relative,
                "provider byte count is not canonical u64",
            )
        })?;
        if claimed.digest != digest || claimed_bytes != bytes {
            return Err(output_fault(
                target,
                binding,
                &planned.id,
                &planned.path_relative,
                "provider digest/byte claim differs from current staged output",
            ));
        }
        outputs.push(VerifiedNativeOutput {
            id: planned.id.clone(),
            kind: target.outputs[index].kind,
            shape: match planned.shape {
                WireShape::File => ArtifactShape::File,
                WireShape::Directory => ArtifactShape::Directory,
            },
            path_absolute: forward_slashed(&absolute),
            path_relative: format!("{staging_relative}/{}", planned.path_relative),
            digest,
            bytes,
            files,
            fresh: staged.fresh,
        });
    }
    Ok(outputs)
}

fn validate_plan_ownership(
    target: &ArtifactBuildTarget,
    binding: &NativeMechanismBinding,
    plan: &request::BuildPlan,
) -> Result<(), BuildError> {
    for (index, output) in plan.outputs.iter().enumerate() {
        for other in &plan.outputs[index + 1..] {
            if output.path_relative == other.path_relative
                || descendant(&output.path_relative, &other.path_relative)
                || descendant(&other.path_relative, &output.path_relative)
            {
                return Err(output_fault(
                    target,
                    binding,
                    &output.id,
                    &output.path_relative,
                    "planned output roots overlap by path segment",
                ));
            }
        }
    }
    Ok(())
}

fn descendant(root: &str, candidate: &str) -> bool {
    candidate
        .strip_prefix(root)
        .is_some_and(|suffix| suffix.starts_with('/'))
}

fn wire_kind(kind: ArtifactKind) -> WireKind {
    match kind {
        ArtifactKind::Executable => WireKind::Executable,
        ArtifactKind::Archive => WireKind::Archive,
        ArtifactKind::File => WireKind::File,
        ArtifactKind::Directory => WireKind::Directory,
        ArtifactKind::Skill => WireKind::Skill,
        ArtifactKind::AgentPlugin => WireKind::AgentPlugin,
    }
}

fn parse_kind(value: &str) -> Result<WireKind, String> {
    match value {
        "executable" => Ok(WireKind::Executable),
        "archive" => Ok(WireKind::Archive),
        "file" => Ok(WireKind::File),
        "directory" => Ok(WireKind::Directory),
        "skill" => Ok(WireKind::Skill),
        "agent-plugin" => Ok(WireKind::AgentPlugin),
        _ => Err("provider plan returned an unknown artifact kind".to_owned()),
    }
}

fn parse_shape(value: &str) -> Result<WireShape, String> {
    match value {
        "file" => Ok(WireShape::File),
        "directory" => Ok(WireShape::Directory),
        _ => Err("provider plan returned an unknown artifact shape".to_owned()),
    }
}

fn fault(
    target: &ArtifactBuildTarget,
    binding: &NativeMechanismBinding,
    operation: &'static str,
    reason: &str,
) -> BuildError {
    BuildError::NativeTransport {
        target: target.id.clone(),
        pin: binding.pin.clone(),
        operation,
        reason: preview(reason),
    }
}

fn output_fault(
    target: &ArtifactBuildTarget,
    binding: &NativeMechanismBinding,
    output: &str,
    path: &str,
    reason: &str,
) -> BuildError {
    BuildError::NativeOutput {
        target: target.id.clone(),
        pin: binding.pin.clone(),
        output: preview(output),
        path: preview(path),
        reason: preview(reason),
    }
}
