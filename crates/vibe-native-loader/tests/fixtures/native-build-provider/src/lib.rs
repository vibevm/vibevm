#![deny(unsafe_code)]

use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};
use vibe_ext::build_reply::{
    ApplyResult, ApplyResultOk, BuildFingerprint, BuildPlan, BuildReplyApply,
    BuildReplyFingerprint, BuildReplyPlan, BuildReplyVerify, FingerprintResult,
    FingerprintResultOk, PlanResult, PlanResultOk, PlannedOutput, StagedOutput, VerifiedOutput,
    VerifyResult, VerifyResultOk,
};
use vibe_ext::{
    BuildReply, BuildRequest, MechanismDescriptor, MechanismManifest, NativeDeployArtifactKind,
    NativeDeployEffect, NativeDeployNetwork, NativeDeployOperation, NativeDeployPrivilege,
    NativeDeployReversibility, NativeMechanismRole,
};

const OUTPUT: &str = "foreign.bin";
const CONTENTS: &[u8] = b"foreign native build output\n";

pub fn fixture_marker() -> &'static str {
    "vibe-native-loader-build-provider-fixture"
}

fn manifest() -> MechanismManifest {
    MechanismManifest {
        mechanisms: vec![MechanismDescriptor {
            artifact_kinds: vec![NativeDeployArtifactKind::Executable],
            atomic_replacement: true,
            effect: NativeDeployEffect::Workspace,
            id: "fixture-build".to_owned(),
            name: "cargo".to_owned(),
            network: NativeDeployNetwork::Never,
            operations: vec![
                NativeDeployOperation::Plan,
                NativeDeployOperation::Fingerprint,
                NativeDeployOperation::Apply,
                NativeDeployOperation::Verify,
            ],
            privilege: NativeDeployPrivilege::None,
            protocol: 1,
            reference_ownership: false,
            reversibility: NativeDeployReversibility::Reversible,
            role: NativeMechanismRole::Build,
        }],
    }
}

fn handle(request: BuildRequest) -> BuildReply {
    match request {
        BuildRequest::Plan(request) => {
            assert_eq!(
                request.target.config_toml.as_deref(),
                Some("foreign = true\n")
            );
            BuildReply::Plan(Box::new(BuildReplyPlan {
                envelope: request.envelope,
                protocol: request.protocol,
                result: PlanResult::Ok(Box::new(PlanResultOk {
                    plan: BuildPlan {
                        outputs: vec![PlannedOutput {
                            id: "foreign-bin".to_owned(),
                            kind: "executable".to_owned(),
                            path_relative: OUTPUT.to_owned(),
                            shape: "file".to_owned(),
                        }],
                        summary: "foreign native build plan".to_owned(),
                    },
                })),
            }))
        }
        BuildRequest::Fingerprint(request) => {
            BuildReply::Fingerprint(Box::new(BuildReplyFingerprint {
                envelope: request.envelope,
                protocol: request.protocol,
                result: FingerprintResult::Ok(Box::new(FingerprintResultOk {
                    fingerprint: BuildFingerprint {
                        digest: format!("{:x}", Sha256::digest(b"foreign-build-provider-v1")),
                        summary: "foreign native build fingerprint".to_owned(),
                    },
                })),
            }))
        }
        BuildRequest::Apply(request) => {
            let path = Path::new(&request.staging.root_absolute).join(OUTPUT);
            let fresh = fs::read(&path).ok().as_deref() == Some(CONTENTS);
            if !fresh {
                write(&path, CONTENTS);
            }
            BuildReply::Apply(Box::new(BuildReplyApply {
                envelope: request.envelope,
                protocol: request.protocol,
                result: ApplyResult::Ok(Box::new(ApplyResultOk {
                    evidence: "foreign native build apply".to_owned(),
                    staged: vec![StagedOutput {
                        fresh,
                        id: "foreign-bin".to_owned(),
                        path_relative: OUTPUT.to_owned(),
                    }],
                })),
            }))
        }
        BuildRequest::Verify(request) => {
            let path = Path::new(&request.staging.root_absolute).join(OUTPUT);
            let bytes = read(&path);
            BuildReply::Verify(Box::new(BuildReplyVerify {
                envelope: request.envelope,
                protocol: request.protocol,
                result: VerifyResult::Ok(Box::new(VerifyResultOk {
                    evidence: "foreign native build verify".to_owned(),
                    verified: vec![VerifiedOutput {
                        bytes: bytes.len().to_string(),
                        digest: format!("{:x}", Sha256::digest(&bytes)),
                        id: "foreign-bin".to_owned(),
                        path_relative: OUTPUT.to_owned(),
                    }],
                })),
            }))
        }
    }
}

fn write(path: &Path, bytes: &[u8]) {
    let Some(parent) = path.parent() else {
        panic!("fixture output has no parent")
    };
    if let Err(error) = fs::create_dir_all(parent) {
        panic!("fixture creates staging: {error:?}");
    }
    if let Err(error) = fs::write(path, bytes) {
        panic!("fixture writes output: {error:?}");
    }
}

fn read(path: &Path) -> Vec<u8> {
    match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => panic!("fixture reads output: {error:?}"),
    }
}

vibe_ext::vibe_build_provider!(manifest = manifest(), handler = handle);
