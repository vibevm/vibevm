#![deny(unsafe_code)]

use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};
use vibe_ext::package_reply::{
    ApplyResult, ApplyResultOk, FingerprintResult, FingerprintResultOk, PackageFingerprint,
    PackagePlan, PackageReplyApply, PackageReplyFingerprint, PackageReplyPlan, PackageReplyVerify,
    PlanResult, PlanResultOk, PlannedOutput, StagedOutput, VerifiedOutput, VerifyResult,
    VerifyResultOk,
};
use vibe_ext::package_request::InputOrigin;
use vibe_ext::{
    MechanismDescriptor, MechanismManifest, NativeDeployArtifactKind, NativeDeployEffect,
    NativeDeployNetwork, NativeDeployOperation, NativeDeployPrivilege, NativeDeployReversibility,
    NativeMechanismRole, PackageReply, PackageRequest,
};

const OUTPUT: &str = "foreign-package.bin";

/// Identify the real package-provider fixture linked into the commissioning E2E.
///
/// ```
/// assert_eq!(
///     vibe_native_loader_package_provider_fixture::fixture_marker(),
///     "vibe-native-loader-package-provider-fixture"
/// );
/// ```
pub fn fixture_marker() -> &'static str {
    "vibe-native-loader-package-provider-fixture"
}

fn manifest() -> MechanismManifest {
    MechanismManifest {
        mechanisms: vec![MechanismDescriptor {
            artifact_kinds: vec![NativeDeployArtifactKind::File],
            atomic_replacement: true,
            effect: NativeDeployEffect::Workspace,
            id: "fixture-package".to_owned(),
            name: "static-file".to_owned(),
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
            role: NativeMechanismRole::Package,
        }],
    }
}

fn handle(request: PackageRequest) -> PackageReply {
    match request {
        PackageRequest::Plan(request) => {
            assert_eq!(
                request.target.config_toml.as_deref(),
                Some("foreign = true\n")
            );
            assert_input(&request.inputs);
            PackageReply::Plan(Box::new(PackageReplyPlan {
                envelope: request.envelope,
                protocol: request.protocol,
                result: PlanResult::Ok(Box::new(PlanResultOk {
                    plan: PackagePlan {
                        outputs: vec![PlannedOutput {
                            id: "foreign-package".to_owned(),
                            kind: "file".to_owned(),
                            path_relative: OUTPUT.to_owned(),
                            shape: "file".to_owned(),
                            media_type: Some("application/octet-stream".to_owned()),
                        }],
                        summary: "foreign native package plan".to_owned(),
                    },
                })),
            }))
        }
        PackageRequest::Fingerprint(request) => {
            assert_input(&request.inputs);
            PackageReply::Fingerprint(Box::new(PackageReplyFingerprint {
                envelope: request.envelope,
                protocol: request.protocol,
                result: FingerprintResult::Ok(Box::new(FingerprintResultOk {
                    fingerprint: PackageFingerprint {
                        counted_inputs: "1".to_owned(),
                        digest: format!("{:x}", Sha256::digest(b"foreign-package-provider-v1")),
                    },
                })),
            }))
        }
        PackageRequest::Apply(request) => {
            let input = assert_input(&request.inputs);
            let destination = Path::new(&request.staging.root_absolute).join(OUTPUT);
            copy(Path::new(&input.path_absolute), &destination);
            PackageReply::Apply(Box::new(PackageReplyApply {
                envelope: request.envelope,
                protocol: request.protocol,
                result: ApplyResult::Ok(Box::new(ApplyResultOk {
                    evidence: "foreign native package apply".to_owned(),
                    staged: vec![StagedOutput {
                        id: "foreign-package".to_owned(),
                        path_relative: OUTPUT.to_owned(),
                    }],
                })),
            }))
        }
        PackageRequest::Verify(request) => {
            let path = Path::new(&request.staging.root_absolute).join(OUTPUT);
            let bytes = read(&path);
            PackageReply::Verify(Box::new(PackageReplyVerify {
                envelope: request.envelope,
                protocol: request.protocol,
                result: VerifyResult::Ok(Box::new(VerifyResultOk {
                    evidence: "foreign native package verify".to_owned(),
                    verified: vec![VerifiedOutput {
                        bytes: bytes.len().to_string(),
                        digest: format!("{:x}", Sha256::digest(&bytes)),
                        files: "1".to_owned(),
                        id: "foreign-package".to_owned(),
                        path_relative: OUTPUT.to_owned(),
                    }],
                })),
            }))
        }
    }
}

fn assert_input(
    inputs: &[vibe_ext::package_request::ResolvedInput],
) -> &vibe_ext::package_request::ResolvedInput {
    let [input] = inputs else {
        panic!("fixture requires exactly one resolved input")
    };
    assert_eq!(input.name, "foreign-bin");
    assert_eq!(input.reference, "artifact:foreign-bin");
    assert_eq!(
        input.path_relative,
        "target/vibe-native/foreign-build/foreign.bin"
    );
    assert!(matches!(
        &input.origin,
        InputOrigin::ArtifactRecord(origin)
            if origin.recorded_kind == NativeDeployArtifactKind::Executable
    ));
    let bytes = read(Path::new(&input.path_absolute));
    assert_eq!(input.bytes, bytes.len().to_string());
    assert_eq!(input.digest, format!("{:x}", Sha256::digest(&bytes)));
    input
}

fn copy(source: &Path, destination: &Path) {
    let Some(parent) = destination.parent() else {
        panic!("fixture output has no parent")
    };
    if let Err(error) = fs::create_dir_all(parent) {
        panic!("fixture creates staging: {error:?}");
    }
    if let Err(error) = fs::copy(source, destination) {
        panic!("fixture copies input: {error:?}");
    }
}

fn read(path: &Path) -> Vec<u8> {
    match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => panic!("fixture reads input: {error:?}"),
    }
}

vibe_ext::vibe_package_provider!(manifest = manifest(), handler = handle);
