#![deny(unsafe_code)]

use std::sync::Mutex;
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};

use serde_json::json;
use vibe_ext::build_reply::{
    BuildPlan, BuildReplyFingerprint, BuildReplyPlan, FingerprintResult, FingerprintResultFail,
    PlanResult, PlanResultOk, PlannedOutput,
};
use vibe_ext::{
    BuildReply, BuildRequest, MechanismDescriptor, MechanismManifest, NativeDeployArtifactKind,
    NativeDeployEffect, NativeDeployNetwork, NativeDeployOperation, NativeDeployPrivilege,
    NativeDeployReversibility, NativeMechanismRole,
};

static HANDLER_CALLS: AtomicUsize = AtomicUsize::new(0);
static REPLY_MODE: AtomicU8 = AtomicU8::new(0);
static TEST_LOCK: Mutex<()> = Mutex::new(());

fn descriptor(role: NativeMechanismRole, id: &str, name: &str) -> MechanismDescriptor {
    MechanismDescriptor {
        artifact_kinds: vec![NativeDeployArtifactKind::Executable],
        atomic_replacement: true,
        effect: NativeDeployEffect::Workspace,
        id: id.to_owned(),
        name: name.to_owned(),
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
        role,
    }
}

fn manifest() -> MechanismManifest {
    MechanismManifest {
        mechanisms: vec![descriptor(NativeMechanismRole::Build, "builder", "cargo")],
    }
}

fn plan(id: &str) -> BuildReply {
    BuildReply::Plan(Box::new(BuildReplyPlan {
        envelope: 1,
        protocol: 1,
        result: PlanResult::Ok(Box::new(PlanResultOk {
            plan: BuildPlan {
                outputs: vec![PlannedOutput {
                    id: id.to_owned(),
                    kind: "executable".to_owned(),
                    path_relative: "demo".to_owned(),
                    shape: "file".to_owned(),
                }],
                summary: "build demo".to_owned(),
            },
        })),
    }))
}

fn handle(_request: BuildRequest) -> BuildReply {
    HANDLER_CALLS.fetch_add(1, Ordering::SeqCst);
    match REPLY_MODE.load(Ordering::SeqCst) {
        0 => plan("demo"),
        1 => BuildReply::Fingerprint(Box::new(BuildReplyFingerprint {
            envelope: 1,
            protocol: 1,
            result: FingerprintResult::Fail(Box::new(FingerprintResultFail {
                message: "wrong operation".to_owned(),
            })),
        })),
        _ => plan("other"),
    }
}

vibe_ext::vibe_build_provider!(manifest = manifest(), handler = handle);

fn request(provider: &str, mechanism: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "operation":"plan", "envelope":1, "protocol":1,
        "identity":{"provider":provider,"mechanism":mechanism,"target":"demo-build"},
        "target":{"id":"demo-build","workdir":".","outputs":[{"id":"demo","kind":"executable"}]},
        "authority":{"project_root_absolute":"C:/work","build_root_absolute":"C:/work/target/build","build_root_relative":"target/build","offline":true}
    }))
    .unwrap()
}

fn invoke(raw: &[u8]) -> (i32, *mut u8, usize) {
    let mut response = std::ptr::null_mut();
    let mut len = 0;
    let status = vibe_ext_invoke(raw.as_ptr(), raw.len(), &mut response, &mut len);
    (status, response, len)
}

#[test]
fn exact_symbols_stable_manifest_and_typed_reply_are_safe() {
    let _guard = TEST_LOCK.lock().unwrap();
    HANDLER_CALLS.store(0, Ordering::SeqCst);
    REPLY_MODE.store(0, Ordering::SeqCst);
    assert_eq!(vibe_ext_abi(), 1);
    assert_eq!(vibe_ext_manifest(), vibe_ext_manifest());

    let raw = request("org.example/tools#builder", "build:cargo");
    let encoded = __vibe_ext_dispatch(&raw).expect("safe typed dispatch");
    assert!(matches!(
        serde_json::from_slice::<BuildReply>(&encoded).unwrap(),
        BuildReply::Plan(_)
    ));
    HANDLER_CALLS.store(0, Ordering::SeqCst);
    let (status, response, len) = invoke(&raw);
    assert_eq!(status, 0);
    assert!(!response.is_null());
    assert_ne!(len, 0);
    // SAFETY is denied in this author crate: copy through the ABI-free owner is
    // tested by the host loader; here symbol publication and free are enough.
    vibe_ext_free(response, len);
    assert_eq!(HANDLER_CALLS.load(Ordering::SeqCst), 1);
}

#[test]
fn manifest_and_request_faults_refuse_before_handler() {
    let _guard = TEST_LOCK.lock().unwrap();
    HANDLER_CALLS.store(0, Ordering::SeqCst);
    let wrong = MechanismManifest {
        mechanisms: vec![descriptor(NativeMechanismRole::Package, "builder", "cargo")],
    };
    let mixed = MechanismManifest {
        mechanisms: vec![
            descriptor(NativeMechanismRole::Build, "builder", "cargo"),
            descriptor(NativeMechanismRole::Package, "packager", "archive"),
        ],
    };
    assert!(!vibe_ext::native_provider::validate_build_manifest(&wrong));
    assert!(!vibe_ext::native_provider::validate_build_manifest(&mixed));
    assert_eq!(HANDLER_CALLS.load(Ordering::SeqCst), 0);

    for raw in [
        b"not-json".to_vec(),
        request("org.example/tools#other", "build:cargo"),
        request("org.example/tools#builder", "build:other"),
        request("org.example/tools#builder", "package:cargo"),
    ] {
        HANDLER_CALLS.store(0, Ordering::SeqCst);
        let (status, response, len) = invoke(&raw);
        assert_ne!(status, 0);
        assert!(response.is_null());
        assert_eq!(len, 0);
        assert_eq!(HANDLER_CALLS.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn wrong_operation_and_descriptor_reply_refuse_after_handler() {
    let _guard = TEST_LOCK.lock().unwrap();
    let raw = request("org.example/tools#builder", "build:cargo");
    for mode in [1, 2] {
        HANDLER_CALLS.store(0, Ordering::SeqCst);
        REPLY_MODE.store(mode, Ordering::SeqCst);
        let (status, response, len) = invoke(&raw);
        assert_ne!(status, 0);
        assert!(response.is_null());
        assert_eq!(len, 0);
        assert_eq!(HANDLER_CALLS.load(Ordering::SeqCst), 1);
    }
}
