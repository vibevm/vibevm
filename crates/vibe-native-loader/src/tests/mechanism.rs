use super::*;
use vibe_core::manifest::MechanismKey;

#[path = "mechanism_roles.rs"]
mod roles;

fn manifest(name: &str) -> Vec<u8> {
    role_manifest(
        "deploy",
        name,
        &[
            "plan",
            "fingerprint",
            "apply",
            "verify",
            "remove",
            "recover",
        ],
    )
}

fn role_manifest(role: &str, name: &str, operations: &[&str]) -> Vec<u8> {
    match serde_json::to_vec(&json!({"mechanisms": [{
        "id": "selected", "role": role, "name": name, "protocol": 1,
        "operations": operations,
        "artifact_kinds": ["executable"], "effect": "user", "network": "never",
        "privilege": "none", "reversibility": "reversible",
        "atomic_replacement": true, "reference_ownership": true
    }]})) {
        Ok(bytes) => bytes,
        Err(error) => panic!("mechanism manifest JSON: {error:?}"),
    }
}

fn key() -> MechanismKey {
    match "deploy:selected".parse() {
        Ok(key) => key,
        Err(error) => panic!("logical mechanism key: {error:?}"),
    }
}

fn request(mechanism: &str) -> DeployRequest {
    match serde_json::from_value(json!({
        "operation": "remove", "envelope": 1, "protocol": 1,
        "identity": {"provider": "org.example/plugin", "mechanism": mechanism,
            "target": "tool", "profile": "default"},
        "resources": []
    })) {
        Ok(request) => request,
        Err(error) => panic!("mechanism request: {error:?}"),
    }
}

fn reply() -> Vec<u8> {
    match serde_json::to_vec(&json!({
        "operation": "remove", "envelope": 1, "protocol": 1,
        "result": {"status": "ok", "removed": [], "expected_remaining": [],
            "evidence": "fixture"}
    })) {
        Ok(bytes) => bytes,
        Err(error) => panic!("mechanism reply: {error:?}"),
    }
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
    assert_eq!(
        selected.expected_role(),
        vibe_core::manifest::MechanismRole::Deploy
    );
    assert_eq!(selected.expected_name(), "selected");
    assert_eq!(selected.expected_protocol(), 1);
    selected.invoke(&request("selected")).unwrap();
    selected.invoke(&request("selected")).unwrap();
    assert_eq!(opener.open_count.load(Ordering::SeqCst), 1);
    assert_eq!(library.invoke_count.load(Ordering::SeqCst), 2);
    assert_eq!(library.free_count.load(Ordering::SeqCst), 2);
}

#[test]
fn build_and_package_handles_retain_role_and_refuse_deploy_invocation() {
    for role in ["build", "package"] {
        let (_directory, path) = fake_file();
        let (loader, library, _) = loader_for(
            FakeManifest::Bytes(role_manifest(
                role,
                "selected",
                &["plan", "fingerprint", "apply", "verify"],
            )),
            FakeCall::published(0, reply()),
        );
        let logical: MechanismKey = format!("{role}:selected").parse().unwrap();
        let selected = loader
            .admit_mechanism(&path, "org.example/plugin", "selected", &logical)
            .expect("matching producing role admits");
        assert_eq!(selected.expected_role().as_str(), role);
        assert_eq!(selected.expected_name(), "selected");
        assert_eq!(selected.expected_protocol(), 1);
        assert!(matches!(
            selected.invoke(&request("selected")),
            Err(NativeLoadError::MechanismRoleInvocation { .. })
        ));
        assert_eq!(library.invoke_count.load(Ordering::SeqCst), 0);
    }
}
