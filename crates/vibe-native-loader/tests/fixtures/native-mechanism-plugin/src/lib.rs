#![deny(unsafe_code)]

use std::fs;
use std::path::Path;

use vibe_ext::mechanism_reply::{
    ApplyResult, ApplyResultFail, ApplyResultOk, DeployReplyApply, DeployReplyFingerprint,
    DeployReplyPlan, DeployReplyRecover, DeployReplyRemove, DeployReplyVerify, FingerprintResult,
    FingerprintResultOk, ObservedResource, PlanResult, PlanResultOk, PlannedResource,
    RecoverResult, RecoverResultOk, RemoveResult, RemoveResultOk, VerifyResult, VerifyResultOk,
};
use vibe_ext::{
    DeployReply, DeployRequest, MechanismDescriptor, MechanismManifest, NativeDeployArtifactKind,
    NativeDeployEffect, NativeDeployNetwork, NativeDeployOperation, NativeDeployPrivilege,
    NativeDeployReversibility, NativeMechanismRole,
};

/// Identify the mechanism fixture linked through the loader's dev graph.
///
/// ```
/// assert_eq!(
///     vibe_native_loader_mechanism_fixture::fixture_marker(),
///     "vibe-native-loader-mechanism-fixture"
/// );
/// ```
pub fn fixture_marker() -> &'static str {
    "vibe-native-loader-mechanism-fixture"
}

fn descriptor(id: &str, name: &str, reference_ownership: bool) -> MechanismDescriptor {
    MechanismDescriptor {
        artifact_kinds: vec![
            NativeDeployArtifactKind::Executable,
            NativeDeployArtifactKind::File,
        ],
        atomic_replacement: true,
        effect: NativeDeployEffect::User,
        id: id.to_owned(),
        name: name.to_owned(),
        network: NativeDeployNetwork::Never,
        operations: vec![
            NativeDeployOperation::Plan,
            NativeDeployOperation::Fingerprint,
            NativeDeployOperation::Apply,
            NativeDeployOperation::Verify,
            NativeDeployOperation::Remove,
            NativeDeployOperation::Recover,
        ],
        privilege: NativeDeployPrivilege::None,
        protocol: 1,
        reference_ownership,
        reversibility: NativeDeployReversibility::Reversible,
        role: NativeMechanismRole::Deploy,
    }
}

fn manifest() -> MechanismManifest {
    MechanismManifest {
        mechanisms: vec![
            descriptor("fixture-deploy", "fixture", false),
            descriptor("fixture-vibe-bin", "vibe-bin", true),
        ],
    }
}

fn handle(request: DeployRequest) -> DeployReply {
    match request {
        DeployRequest::Plan(request) => {
            let leaf = if request.identity.target.starts_with("pure-") {
                "pure"
            } else {
                &request.identity.target
            };
            let destination = format!(
                "{}/native-provider/{leaf}",
                request.authority.settings_root.trim_end_matches('/'),
            );
            let resource = if request.identity.target == "logical" {
                format!("home:{destination}")
            } else if request.identity.target == "backslash" {
                format!("home:{}", destination.replace('/', "\\"))
            } else {
                destination.clone()
            };
            let lock = if request.identity.target == "bad-lock" {
                destination.replace('/', "\\")
            } else {
                destination
            };
            let desired = read_digest(&request.artifact.path_absolute);
            DeployReply::Plan(Box::new(DeployReplyPlan {
                envelope: request.envelope,
                protocol: request.protocol,
                result: PlanResult::Ok(Box::new(PlanResultOk {
                    resources: vec![PlannedResource {
                        resource,
                        desired_digest: desired,
                    }],
                    lock_resources: vec![lock],
                    config_digest: request.artifact.digest,
                    reversible: true,
                    summary: "fixture native provider plan".to_owned(),
                })),
            }))
        }
        DeployRequest::Fingerprint(request) => {
            DeployReply::Fingerprint(Box::new(DeployReplyFingerprint {
                envelope: request.envelope,
                protocol: request.protocol,
                result: FingerprintResult::Ok(Box::new(FingerprintResultOk {
                    digest: request.artifact.digest,
                    summary: "fixture native provider fingerprint".to_owned(),
                })),
            }))
        }
        DeployRequest::Apply(request) => {
            let resource = &request.plan.resources[0].resource;
            copy(
                &request.artifact.path_absolute,
                &request.plan.lock_resources[0],
            );
            let result = if request.identity.target.contains("interrupt") {
                ApplyResult::Fail(Box::new(ApplyResultFail {
                    message: "fixture interruption after destination write".to_owned(),
                }))
            } else {
                ApplyResult::Ok(Box::new(ApplyResultOk {
                    completed: vec![resource.clone()],
                    evidence: "fixture native apply".to_owned(),
                    prior_state_handle: None,
                }))
            };
            DeployReply::Apply(Box::new(DeployReplyApply {
                envelope: request.envelope,
                protocol: request.protocol,
                result,
            }))
        }
        DeployRequest::Verify(request) => {
            let observed = request
                .resources
                .into_iter()
                .map(|resource| ObservedResource {
                    digest: fs::read_to_string(destination(&resource.resource))
                        .ok()
                        .map(|value| value.trim().to_owned()),
                    resource: resource.resource,
                })
                .collect();
            DeployReply::Verify(Box::new(DeployReplyVerify {
                envelope: request.envelope,
                protocol: request.protocol,
                result: VerifyResult::Ok(Box::new(VerifyResultOk { observed })),
            }))
        }
        DeployRequest::Remove(request) => {
            let mut removed = Vec::new();
            for resource in request.resources {
                let destination = destination(&resource);
                if Path::new(&destination).exists()
                    && let Err(error) = fs::remove_file(destination)
                {
                    panic!("fixture removes owned resource: {error:?}");
                }
                removed.push(resource);
            }
            DeployReply::Remove(Box::new(DeployReplyRemove {
                envelope: request.envelope,
                protocol: request.protocol,
                result: RemoveResult::Ok(Box::new(RemoveResultOk {
                    evidence: "real mechanism fixture".to_owned(),
                    expected_remaining: Vec::new(),
                    removed,
                })),
            }))
        }
        DeployRequest::Recover(request) => {
            let resource = &request.plan.resources[0].resource;
            copy(
                &request.artifact.path_absolute,
                &request.plan.lock_resources[0],
            );
            DeployReply::Recover(Box::new(DeployReplyRecover {
                envelope: request.envelope,
                protocol: request.protocol,
                result: RecoverResult::Ok(Box::new(RecoverResultOk {
                    completed: vec![resource.clone()],
                    evidence: "fixture native recover".to_owned(),
                    prior_state_handle: None,
                })),
            }))
        }
    }
}

fn copy(source: &str, destination: &str) {
    let destination = Path::new(destination);
    let Some(parent) = destination.parent() else {
        panic!("destination parent");
    };
    if let Err(error) = fs::create_dir_all(parent) {
        panic!("fixture creates destination parent: {error:?}");
    }
    if let Err(error) = fs::copy(source, destination) {
        panic!("fixture copies artifact: {error:?}");
    }
}

fn destination(resource: &str) -> String {
    resource
        .strip_prefix("home:")
        .unwrap_or(resource)
        .replace('\\', "/")
}

fn read_digest(path: &str) -> String {
    match fs::read_to_string(path) {
        Ok(digest) => digest.trim().to_owned(),
        Err(error) => panic!("fixture reads resolved artifact: {error:?}"),
    }
}

vibe_ext::vibe_mechanism_provider!(manifest = manifest(), handler = handle);
