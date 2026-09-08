use super::*;
use vibe_core::manifest::MechanismKey;

fn manifest(name: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({"mechanisms": [{
        "id": "selected", "role": "deploy", "name": name, "protocol": 1,
        "operations": ["plan", "fingerprint", "apply", "verify", "remove", "recover"],
        "artifact_kinds": ["executable"], "effect": "user", "network": "never",
        "privilege": "none", "reversibility": "reversible",
        "atomic_replacement": true, "reference_ownership": true
    }]}))
    .expect("mechanism manifest JSON")
}

fn key() -> MechanismKey {
    "deploy:selected".parse().expect("logical mechanism key")
}

fn request(mechanism: &str) -> DeployRequest {
    serde_json::from_value(json!({
        "operation": "remove", "envelope": 1, "protocol": 1,
        "identity": {"provider": "org.example/plugin", "mechanism": mechanism,
            "target": "tool", "profile": "default"},
        "resources": []
    }))
    .expect("mechanism request")
}

fn reply() -> Vec<u8> {
    serde_json::to_vec(&json!({
        "operation": "remove", "envelope": 1, "protocol": 1,
        "result": {"status": "ok", "removed": [], "expected_remaining": [],
            "evidence": "fixture"}
    }))
    .expect("mechanism reply")
}

#[test]
fn id_name_and_request_mismatches_refuse_before_invoke() {
    let (_directory, path) = fake_file();
    let (loader, library, _) = loader_for(
        FakeManifest::Bytes(manifest("selected")),
        FakeCall::published(0, reply()),
    );
    assert!(matches!(
        loader.admit_mechanism(&path, "org.example/plugin", "missing", &key()),
        Err(NativeLoadError::MissingMechanismId { .. })
    ));
    let wrong_role: MechanismKey = "build:selected".parse().expect("wrong-role key");
    assert!(matches!(
        loader.admit_mechanism(&path, "org.example/plugin", "selected", &wrong_role),
        Err(NativeLoadError::MechanismManifestAdmission { .. })
    ));
    let selected = loader
        .admit_mechanism(&path, "org.example/plugin", "selected", &key())
        .expect("selected descriptor");
    assert!(matches!(
        selected.invoke(&request("other")),
        Err(NativeLoadError::MechanismRequestAdmission { .. })
    ));
    assert_eq!(library.invoke_count.load(Ordering::SeqCst), 0);

    let (loader, library, _) = loader_for(
        FakeManifest::Bytes(manifest("other")),
        FakeCall::published(0, reply()),
    );
    assert!(matches!(
        loader.admit_mechanism(&path, "org.example/plugin", "selected", &key()),
        Err(NativeLoadError::MechanismManifestAdmission { .. })
    ));
    assert_eq!(library.invoke_count.load(Ordering::SeqCst), 0);
}

#[test]
fn malformed_and_oversize_replies_free_once() {
    for (call, oversized) in [
        (FakeCall::published(0, b"{".to_vec()), false),
        (
            FakeCall {
                status: 0,
                bytes: Some(vec![1]),
                len: REPLY_CAP + 1,
            },
            true,
        ),
    ] {
        let (_directory, path) = fake_file();
        let (loader, library, _) = loader_for(FakeManifest::Bytes(manifest("selected")), call);
        let selected = loader
            .admit_mechanism(&path, "org.example/plugin", "selected", &key())
            .expect("selected descriptor");
        let error = selected
            .invoke(&request("selected"))
            .expect_err("reply refuses");
        assert!(matches!(
            (oversized, error),
            (true, NativeLoadError::ReplyTooLarge { .. })
                | (false, NativeLoadError::MechanismReplyJson { .. })
        ));
        assert_eq!(library.free_count.load(Ordering::SeqCst), 1);
    }
}

#[test]
fn descriptor_is_owned_and_cached_handle_is_reused() {
    let (_directory, path) = fake_file();
    let (loader, library, opener) = loader_for(
        FakeManifest::Bytes(manifest("selected")),
        FakeCall::published(0, reply()),
    );
    let selected = loader
        .admit_mechanism(&path, "org.example/plugin", "selected", &key())
        .expect("selected descriptor");
    assert_eq!(selected.descriptor().name, "selected");
    assert_eq!(selected.logical_key(), &key());
    selected.invoke(&request("selected")).unwrap();
    selected.invoke(&request("selected")).unwrap();
    assert_eq!(opener.open_count.load(Ordering::SeqCst), 1);
    assert_eq!(library.invoke_count.load(Ordering::SeqCst), 2);
    assert_eq!(library.free_count.load(Ordering::SeqCst), 2);
}
