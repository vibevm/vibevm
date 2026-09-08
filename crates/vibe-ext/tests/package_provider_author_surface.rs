#![deny(unsafe_code)]

use std::sync::Mutex;
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};

use serde_json::json;
use vibe_ext::package_reply::{
    PackagePlan, PackageReplyFingerprint, PackageReplyPlan, PlanResult, PlanResultOk, PlannedOutput,
};
use vibe_ext::{
    MechanismDescriptor, MechanismManifest, NativeDeployArtifactKind, NativeDeployEffect,
    NativeDeployNetwork, NativeDeployOperation, NativeDeployPrivilege, NativeDeployReversibility,
    NativeMechanismRole, PackageReply, PackageRequest,
};

static HANDLER_CALLS: AtomicUsize = AtomicUsize::new(0);
static REPLY_MODE: AtomicU8 = AtomicU8::new(0);
static TEST_LOCK: Mutex<()> = Mutex::new(());

fn descriptor(role: NativeMechanismRole, id: &str, name: &str) -> MechanismDescriptor {
    MechanismDescriptor {
        artifact_kinds: vec![NativeDeployArtifactKind::Archive],
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
        mechanisms: vec![descriptor(
            NativeMechanismRole::Package,
            "packager",
            "archive",
        )],
    }
}

fn plan(id: &str) -> PackageReply {
    PackageReply::Plan(Box::new(PackageReplyPlan {
        envelope: 1,
        protocol: 1,
        result: PlanResult::Ok(Box::new(PlanResultOk {
            plan: PackagePlan {
                outputs: vec![PlannedOutput {
                    id: id.to_owned(),
                    kind: "archive".to_owned(),
                    path_relative: "bundle.tar".to_owned(),
                    shape: "file".to_owned(),
                    media_type: Some("application/x-tar".to_owned()),
                }],
                summary: "package bundle".to_owned(),
            },
        })),
    }))
}

fn handle(_request: PackageRequest) -> PackageReply {
    HANDLER_CALLS.fetch_add(1, Ordering::SeqCst);
    match REPLY_MODE.load(Ordering::SeqCst) {
        0 => plan("bundle"),
        1 => PackageReply::Fingerprint(Box::new(PackageReplyFingerprint {
            envelope: 1,
            protocol: 1,
            result: vibe_ext::package_reply::FingerprintResult::Fail(Box::new(
                vibe_ext::package_reply::FingerprintResultFail {
                    message: "wrong operation".to_owned(),
                },
            )),
        })),
        _ => plan("other"),
    }
}

vibe_ext::vibe_package_provider!(manifest = manifest(), handler = handle);

fn request(provider: &str, mechanism: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "operation":"plan", "envelope":1, "protocol":1,
        "identity":{"provider":provider,"mechanism":mechanism,"target":"bundle"},
        "target":{"id":"bundle","outputs":[{"id":"bundle","kind":"archive"}]},
        "inputs":[],
        "authority":{"project_root_absolute":"C:/work","package_root_absolute":"C:/work/target/packages","package_root_relative":"target/packages","output_root_absolute":"C:/work/target/packages/bundle","output_root_relative":"target/packages/bundle"}
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
    let raw = request("org.example/tools#packager", "package:archive");
    let encoded = __vibe_ext_dispatch(&raw).expect("safe typed dispatch");
    assert!(matches!(
        serde_json::from_slice::<PackageReply>(&encoded).unwrap(),
        PackageReply::Plan(_)
    ));
    HANDLER_CALLS.store(0, Ordering::SeqCst);
    let (status, response, len) = invoke(&raw);
    assert_eq!(status, 0);
    assert!(!response.is_null());
    assert_ne!(len, 0);
    vibe_ext_free(response, len);
    assert_eq!(HANDLER_CALLS.load(Ordering::SeqCst), 1);
}

#[test]
fn manifest_and_request_faults_refuse_before_handler() {
    let _guard = TEST_LOCK.lock().unwrap();
    HANDLER_CALLS.store(0, Ordering::SeqCst);
    let wrong = MechanismManifest {
        mechanisms: vec![descriptor(
            NativeMechanismRole::Build,
            "packager",
            "archive",
        )],
    };
    let mixed = MechanismManifest {
        mechanisms: vec![
            descriptor(NativeMechanismRole::Package, "packager", "archive"),
            descriptor(NativeMechanismRole::Build, "builder", "cargo"),
        ],
    };
    assert!(!vibe_ext::native_provider::validate_package_manifest(
        &wrong
    ));
    assert!(!vibe_ext::native_provider::validate_package_manifest(
        &mixed
    ));
    assert_eq!(HANDLER_CALLS.load(Ordering::SeqCst), 0);

    for raw in [
        b"not-json".to_vec(),
        request("org.example/tools#other", "package:archive"),
        request("org.example/tools#packager", "package:other"),
        request("org.example/tools#packager", "build:archive"),
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
    let raw = request("org.example/tools#packager", "package:archive");
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
