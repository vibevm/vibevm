#![deny(unsafe_code)]

use vibe_ext::mechanism_reply::{DeployReplyRemove, RemoveResult, RemoveResultOk};
use vibe_ext::{
    DeployReply, DeployRequest, MechanismDescriptor, MechanismManifest, NativeDeployArtifactKind,
    NativeDeployEffect, NativeDeployNetwork, NativeDeployOperation, NativeDeployPrivilege,
    NativeDeployReversibility, NativeMechanismRole,
};

pub fn fixture_marker() -> &'static str {
    "vibe-native-loader-mechanism-fixture"
}

fn manifest() -> MechanismManifest {
    MechanismManifest {
        mechanisms: vec![MechanismDescriptor {
            artifact_kinds: vec![NativeDeployArtifactKind::Executable],
            atomic_replacement: true,
            effect: NativeDeployEffect::User,
            id: "fixture-deploy".to_owned(),
            name: "fixture".to_owned(),
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
            reference_ownership: true,
            reversibility: NativeDeployReversibility::Reversible,
            role: NativeMechanismRole::Deploy,
        }],
    }
}

fn handle(request: DeployRequest) -> DeployReply {
    let DeployRequest::Remove(request) = request else {
        panic!("fixture accepts remove in this atom")
    };
    DeployReply::Remove(Box::new(DeployReplyRemove {
        envelope: request.envelope,
        protocol: request.protocol,
        result: RemoveResult::Ok(Box::new(RemoveResultOk {
            evidence: "real mechanism fixture".to_owned(),
            expected_remaining: Vec::new(),
            removed: request.resources,
        })),
    }))
}

vibe_ext::vibe_mechanism_provider!(manifest = manifest(), handler = handle);
