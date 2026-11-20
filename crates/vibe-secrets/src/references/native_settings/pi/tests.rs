use super::*;
use std::os::unix::fs::PermissionsExt;

#[test]
fn confirmed_pi_sink_preserves_neighbors_and_refuses_stale_or_unsafe_subject() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    let agent = home.join(".pi/agent");
    std::fs::create_dir_all(&agent).unwrap();
    for path in [&home, &home.join(".pi"), &agent] {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let settings = temp.path().join("settings");
    std::fs::create_dir(&settings).unwrap();
    std::fs::set_permissions(&settings, std::fs::Permissions::from_mode(0o700)).unwrap();
    let store = Store::at_settings(&settings).unwrap();
    let marker = "!synthetic-$HOME-${MISSING}-$$-$!";
    let key = store
        .create(
            "api-key",
            "synthetic",
            ProtectedValue::new(marker.as_bytes().to_vec()).unwrap(),
        )
        .unwrap();
    let connection = store.create_reference("ai", &key, b"label = 'synthetic'\nendpoint = 'http://127.0.0.1:9/v1'\napi_kind = 'openai-chat-completions'\ndefault_model = 'synthetic'\n").unwrap();
    let models = agent.join("models.json");
    let auth = agent.join("auth.json");
    let before = br#"{"foreign":{"preserve":true},"providers":{"adjacent":{"apiKey":"synthetic-neighbor"},"vibevm":{"foreign":23,"models":[{"id":"other","foreign":true},{"id":"synthetic","baseUrl":"http://127.0.0.1:8/old","api":"anthropic-messages","foreign":42}]}}}"#;
    std::fs::write(&models, before).unwrap();
    std::fs::set_permissions(&models, std::fs::Permissions::from_mode(0o600)).unwrap();
    let oauth = br#"{"adjacent-oauth":{"type":"oauth","access":"synthetic","refresh":"synthetic","expires":9999999999999}}"#;
    std::fs::write(&auth, oauth).unwrap();
    std::fs::set_permissions(&auth, std::fs::Permissions::from_mode(0o600)).unwrap();
    let preview = store
        .preview_pi_native_settings(&home, &connection)
        .unwrap();
    assert_eq!(std::fs::read(&models).unwrap(), before);
    assert!(matches!(
        store.apply_pi_native_settings(&preview, "not-confirmed"),
        Err(Error::Conflict)
    ));
    assert_eq!(std::fs::read(&models).unwrap(), before);
    assert!(
        store
            .apply_pi_native_settings(&preview, preview.preview_id())
            .unwrap()
    );
    let after = std::fs::read(&models).unwrap();
    let parsed: Value = serde_json::from_slice(&after).unwrap();
    assert_eq!(parsed["foreign"]["preserve"], true);
    assert_eq!(
        parsed["providers"]["adjacent"]["apiKey"],
        "synthetic-neighbor"
    );
    assert_eq!(parsed["providers"][PROVIDER]["foreign"], 23);
    assert_eq!(parsed["providers"][PROVIDER]["models"][0]["id"], "other");
    assert_eq!(
        parsed["providers"][PROVIDER]["models"][1]["baseUrl"],
        "http://127.0.0.1:9/v1"
    );
    assert_eq!(
        parsed["providers"][PROVIDER]["models"][1]["api"],
        "openai-completions"
    );
    assert_eq!(parsed["providers"][PROVIDER]["models"][1]["foreign"], 42);
    assert_eq!(
        parsed["providers"][PROVIDER]["apiKey"],
        "$!synthetic-$$HOME-$${MISSING}-$$$$-$$$!"
    );
    assert_eq!(std::fs::read(&auth).unwrap(), oauth);
    assert!(matches!(
        store.apply_pi_native_settings(&preview, preview.preview_id()),
        Err(Error::Conflict)
    ));
    let noop = store
        .preview_pi_native_settings(&home, &connection)
        .unwrap();
    assert!(
        !store
            .apply_pi_native_settings(&noop, noop.preview_id())
            .unwrap()
    );
    assert_eq!(std::fs::read(&models).unwrap(), after);
    let native_stale = store
        .preview_pi_native_settings(&home, &connection)
        .unwrap();
    let changed = br#"{"foreign":"changed after preview"}"#;
    std::fs::write(&models, changed).unwrap();
    assert!(matches!(
        store.apply_pi_native_settings(&native_stale, native_stale.preview_id()),
        Err(Error::Conflict)
    ));
    assert_eq!(std::fs::read(&models).unwrap(), changed);
    std::fs::write(&models, &after).unwrap();
    let auth_stale = store.preview_pi_native_settings(&home, &connection).unwrap();
    std::fs::write(&auth, br#"{"vibevm":{"type":"api_key","key":"synthetic-shadow"}}"#).unwrap();
    assert!(matches!(store.preview_pi_native_settings(&home, &connection), Err(Error::Invalid)));
    assert!(store.apply_pi_native_settings(&auth_stale, auth_stale.preview_id()).is_err());
    assert_eq!(std::fs::read(&models).unwrap(), after);
    std::fs::write(&auth, oauth).unwrap();
    let old_target = store
        .preview_pi_native_settings(&home, &connection)
        .unwrap();
    std::fs::rename(&agent, home.join(".pi/retained-agent")).unwrap();
    std::fs::create_dir(&agent).unwrap();
    std::fs::set_permissions(&agent, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::write(&models, &after).unwrap();
    std::fs::set_permissions(&models, std::fs::Permissions::from_mode(0o600)).unwrap();
    let new_target = store
        .preview_pi_native_settings(&home, &connection)
        .unwrap();
    assert_ne!(old_target.preview_id(), new_target.preview_id());
    assert!(matches!(
        store.apply_pi_native_settings(&new_target, old_target.preview_id()),
        Err(Error::Conflict)
    ));
    assert!(matches!(
        store.apply_pi_native_settings(&old_target, old_target.preview_id()),
        Err(Error::Conflict)
    ));
    let stale = store
        .preview_pi_native_settings(&home, &connection)
        .unwrap();
    let snapshot = store.snapshot().unwrap();
    store
        .rotate(
            &key,
            &snapshot.items[&key].current_revision,
            ProtectedValue::new(b"rotated-synthetic".to_vec()).unwrap(),
        )
        .unwrap();
    assert!(matches!(
        store.apply_pi_native_settings(&stale, stale.preview_id()),
        Err(Error::Conflict)
    ));
    assert_eq!(std::fs::read(&models).unwrap(), after);
    std::fs::set_permissions(&models, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert!(matches!(
        store.preview_pi_native_settings(&home, &connection),
        Err(Error::Unavailable)
    ));
}
