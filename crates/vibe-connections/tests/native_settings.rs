//! Native adapter interface smoke test; no ordinary user configuration.
use vibe_connections::{
    CodexSettingsAdapter, DefaultScope, Metadata, NativeSettingsAdapter, PiSettingsAdapter, Store,
};

#[test]
fn pi_native_adapter_admits_only_its_verified_protocol() {
    let adapter = PiSettingsAdapter;
    assert_eq!(adapter.id(), "pi");
    assert!(adapter.supports("openai-chat-completions"));
    assert!(!adapter.supports("anthropic-messages"));
    assert!(!adapter.supports("openai-responses"));
}

#[cfg(unix)]
#[test]
fn incompatible_responses_default_and_overrides_refuse_without_publication() {
    use std::os::unix::fs::PermissionsExt;
    let t = tempfile::tempdir().unwrap();
    std::fs::set_permissions(t.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let keys = vibe_secrets::Store::at_settings(t.path()).unwrap();
    let key = keys
        .create(
            "api-key",
            "synthetic",
            vibe_secrets::ProtectedValue::new(b"synthetic".to_vec()).unwrap(),
        )
        .unwrap();
    let store = Store::at_settings(t.path()).unwrap();
    let metadata = |kind: &str| Metadata {
        label: kind.into(),
        endpoint: "http://127.0.0.1:9/v1".into(),
        api_kind: kind.into(),
        default_model: "synthetic".into(),
    };
    let chat = store
        .add(metadata("openai-chat-completions"), &key)
        .unwrap();
    let responses = store.add(metadata("openai-responses"), &key).unwrap();
    let revision = store.list().unwrap().store_revision;
    let selected = store.set_default(&chat, revision).unwrap();
    let p = store
        .set_binding(
            "pi",
            Some(vibe_secrets::ReferenceBinding::InheritDefault),
            selected.store_revision,
        )
        .unwrap();
    store
        .set_binding(
            "qwen",
            Some(vibe_secrets::ReferenceBinding::Connection {
                connection: chat.clone(),
            }),
            p.store_revision,
        )
        .unwrap();
    let index = t.path().join("secrets/ai/index.toml");
    let before = std::fs::read(&index).unwrap();
    assert!(
        store
            .default_scope_preview(&responses, DefaultScope::DefaultOnly)
            .is_err()
    );
    assert!(
        store
            .default_scope_preview(&responses, DefaultScope::ManagedOverrides)
            .is_err()
    );
    assert!(
        store
            .set_default(&responses, store.list().unwrap().store_revision)
            .is_err()
    );
    assert!(
        store
            .set_binding(
                "qwen",
                Some(vibe_secrets::ReferenceBinding::Connection {
                    connection: responses.clone()
                }),
                store.list().unwrap().store_revision
            )
            .is_err()
    );
    // Locked incumbent writer also refuses even without the connection facade.
    assert!(
        keys.select_reference("ai", Some(&responses), store.list().unwrap().store_revision)
            .is_err()
    );
    assert!(
        keys.bind_reference(
            "ai",
            "pi",
            Some(vibe_secrets::ReferenceBinding::Connection {
                connection: responses.clone()
            }),
            store.list().unwrap().store_revision
        )
        .is_err()
    );
    assert_eq!(std::fs::read(index).unwrap(), before);
    assert!(CodexSettingsAdapter.supports("openai-responses"));
    assert!(!CodexSettingsAdapter.supports("openai-chat-completions"));
}
