#![deny(unsafe_code)]

use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::json;
use vibe_ext::mechanism_reply::{DeployReplyRemove, RemoveResult, RemoveResultOk};
use vibe_ext::{
    DeployReply, DeployRequest, MechanismDescriptor, MechanismManifest, NativeDeployArtifactKind,
    NativeDeployEffect, NativeDeployNetwork, NativeDeployOperation, NativeDeployPrivilege,
    NativeDeployReversibility, NativeMechanismRole,
};

static HANDLER_CALLS: AtomicUsize = AtomicUsize::new(0);

fn handle(request: DeployRequest) -> DeployReply {
    HANDLER_CALLS.fetch_add(1, Ordering::SeqCst);
    let DeployRequest::Remove(request) = request else {
        panic!("author-surface test invokes only remove")
    };
    DeployReply::Remove(Box::new(DeployReplyRemove {
        envelope: request.envelope,
        protocol: request.protocol,
        result: RemoveResult::Ok(Box::new(RemoveResultOk {
            evidence: "safe typed provider".to_owned(),
            expected_remaining: Vec::new(),
            removed: request.resources,
        })),
    }))
}

vibe_ext::vibe_mechanism_provider!(
    manifest = MechanismManifest {
        mechanisms: vec![MechanismDescriptor {
            artifact_kinds: vec![NativeDeployArtifactKind::Executable],
            atomic_replacement: true,
            effect: NativeDeployEffect::User,
            id: "safe-deploy".to_owned(),
            name: "safe".to_owned(),
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
    },
    handler = handle,
);

#[test]
fn mechanism_author_uses_only_safe_generated_types_and_the_shared_abi() {
    assert_eq!(vibe_ext_abi(), 1);
    assert!(!vibe_ext_manifest().is_null());
}

#[test]
fn mechanism_macro_admits_before_the_typed_handler() {
    HANDLER_CALLS.store(0, Ordering::SeqCst);
    let request = |protocol, mechanism| {
        serde_json::to_vec(&json!({
            "operation": "remove", "envelope": 1, "protocol": protocol,
            "identity": {"provider": "org.example/provider", "mechanism": mechanism,
                "target": "tool", "profile": "default"},
            "resources": []
        }))
        .unwrap()
    };
    let invalid = request(2, "safe-deploy");
    let mut response = std::ptr::null_mut();
    let mut len = 0;
    assert_ne!(
        vibe_ext_invoke(invalid.as_ptr(), invalid.len(), &mut response, &mut len),
        0
    );
    assert!(response.is_null());
    assert_eq!(HANDLER_CALLS.load(Ordering::SeqCst), 0);

    let undeclared = request(1, "undeclared");
    assert_ne!(
        vibe_ext_invoke(
            undeclared.as_ptr(),
            undeclared.len(),
            &mut response,
            &mut len,
        ),
        0
    );
    assert!(response.is_null());
    assert_eq!(HANDLER_CALLS.load(Ordering::SeqCst), 0);

    let valid = request(1, "safe-deploy");
    assert_eq!(
        vibe_ext_invoke(valid.as_ptr(), valid.len(), &mut response, &mut len),
        0
    );
    assert!(!response.is_null());
    assert_ne!(len, 0);
    assert_eq!(HANDLER_CALLS.load(Ordering::SeqCst), 1);
    vibe_ext_free(response, len);
}
