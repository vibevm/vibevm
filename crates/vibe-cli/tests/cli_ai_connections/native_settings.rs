use super::*;
use std::os::unix::fs::PermissionsExt;

#[test]
fn explicit_native_pi_preview_apply_noop_and_stale_preserve_neighbors() {
    let f = F::new();
    let agent = f.0.path().join(".pi/agent");
    fs::create_dir_all(&agent).unwrap();
    for p in [f.0.path().join(".pi"), agent.clone()] {
        fs::set_permissions(p, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let models = agent.join("models.json");
    let initial =
        br#"{"unknown":{"keep":true},"providers":{"neighbor":{"apiKey":"synthetic-neighbor"}}}"#;
    fs::write(&models, initial).unwrap();
    fs::set_permissions(&models, fs::Permissions::from_mode(0o600)).unwrap();
    let secret = f.json(
        &[
            "secrets",
            "create",
            "--kind",
            "api-key",
            "--label",
            "Synthetic",
            "--value-stdin",
        ],
        Some(VALUE.as_bytes()),
    );
    let key = secret["items"][0]["id"].as_str().unwrap();
    let added = f.json(
        &[
            "ai",
            "connections",
            "add",
            "--label",
            "Synthetic",
            "--endpoint",
            "http://127.0.0.1:32123/v1",
            "--api-kind",
            "openai-chat-completions",
            "--default-model",
            "synthetic",
            "--secret-ref",
            key,
        ],
        None,
    );
    let connection = added["items"].as_object().unwrap().keys().next().unwrap();
    let request = [
        "ai",
        "setup",
        "--native-settings",
        "--agent",
        "pi",
        "--connection",
        connection,
    ];
    let preview = f.json(&request, None);
    assert_eq!(preview["applied"], false);
    assert_eq!(fs::read(&models).unwrap(), initial);
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../schemas/ai_native_settings.schema.json"
    ))
    .unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert!(validator.is_valid(&preview));
    let mut invalid = preview.clone();
    invalid["value"] = "not allowed".into();
    assert!(!validator.is_valid(&invalid));
    let mut apply = request.to_vec();
    apply.extend([
        "--apply-preview",
        preview["preview"]["preview_id"].as_str().unwrap(),
    ]);
    let applied = f.json(&apply, None);
    assert_eq!(applied["applied"], true);
    assert_eq!(applied["changed"], true);
    let after = fs::read(&models).unwrap();
    let native: serde_json::Value = serde_json::from_slice(&after).unwrap();
    assert_eq!(native["unknown"]["keep"], true);
    assert_eq!(
        native["providers"]["neighbor"]["apiKey"],
        "synthetic-neighbor"
    );
    assert!(
        native["providers"]["vibevm"]["apiKey"].as_str() == Some(VALUE),
        "native key mismatch"
    );
    assert!(!f.run(&apply, None).status.success());
    let fresh = f.json(&request, None);
    let mut noop = request.to_vec();
    noop.extend([
        "--apply-preview",
        fresh["preview"]["preview_id"].as_str().unwrap(),
    ]);
    assert_eq!(f.json(&noop, None)["changed"], false);
    assert_eq!(fs::read(&models).unwrap(), after);
    assert!(
        !f.run(
            &[
                "ai",
                "setup",
                "--native-settings",
                "--agent",
                "claude",
                "--connection",
                connection
            ],
            None
        )
        .status
        .success()
    );
    assert_eq!(fs::read(&models).unwrap(), after);
}

#[test]
fn explicit_native_qwen_preserves_comments_and_refuses_shadowing_or_trailing_comma() {
    let f = F::new();
    let dir = f.0.path().join(".qwen");
    fs::create_dir(&dir).unwrap();
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).unwrap();
    let path = dir.join("settings.json");
    let before = b"{// keep\n\"security\":{\"auth\":{/* auth */}},\"neighbor\":{\"keep\":true}}";
    fs::write(&path, before).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let key = f.json(
        &[
            "secrets",
            "create",
            "--kind",
            "api-key",
            "--label",
            "Synthetic",
            "--value-stdin",
        ],
        Some(VALUE.as_bytes()),
    );
    let key = key["items"][0]["id"].as_str().unwrap();
    let added = f.json(
        &[
            "ai",
            "connections",
            "add",
            "--label",
            "Synthetic",
            "--endpoint",
            "http://127.0.0.1:9/v1",
            "--api-kind",
            "openai-chat-completions",
            "--default-model",
            "synthetic",
            "--secret-ref",
            key,
        ],
        None,
    );
    let connection = added["items"].as_object().unwrap().keys().next().unwrap();
    let request = [
        "ai",
        "setup",
        "--native-settings",
        "--agent",
        "qwen",
        "--connection",
        connection,
    ];
    let preview = f.json(&request, None);
    assert_eq!(fs::read(&path).unwrap(), before);
    let mut apply = request.to_vec();
    apply.extend([
        "--apply-preview",
        preview["preview"]["preview_id"].as_str().unwrap(),
    ]);
    assert_eq!(f.json(&apply, None)["changed"], true);
    let written = fs::read(&path).unwrap();
    assert!(written.windows(7).any(|w| w == b"// keep"));
    assert!(written.windows(10).any(|w| w == b"/* auth */"));
    let fresh = f.json(&request, None);
    let mut noop = request.to_vec();
    noop.extend([
        "--apply-preview",
        fresh["preview"]["preview_id"].as_str().unwrap(),
    ]);
    assert_eq!(f.json(&noop, None)["changed"], false);
    for unsafe_input in [
        br#"{"modelProviders":{"openai":[{"id":"shadow"}]}}"#.as_slice(),
        b"{\"other\":true,}",
    ] {
        fs::write(&path, unsafe_input).unwrap();
        assert!(!f.run(&request, None).status.success());
        assert_eq!(fs::read(&path).unwrap(), unsafe_input);
    }
}

#[test]
fn codex_explicit_native_responses_preserves_toml_and_refuses_chat() {
    let f = F::new();
    let dir = f.0.path().join(".codex");
    fs::create_dir(&dir).unwrap();
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).unwrap();
    let path = dir.join("config.toml");
    fs::write(
        &path,
        b"# keep\nmodel='old'\n[model_providers.neighbor]\nname='keep'\n",
    )
    .unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let secret = f.json(
        &[
            "secrets",
            "create",
            "--kind",
            "api-key",
            "--label",
            "Synthetic",
            "--value-stdin",
        ],
        Some(VALUE.as_bytes()),
    );
    let key = secret["items"][0]["id"].as_str().unwrap();
    let add = |kind: &str| {
        f.json(
            &[
                "ai",
                "connections",
                "add",
                "--label",
                kind,
                "--endpoint",
                "http://127.0.0.1:9/v1",
                "--api-kind",
                kind,
                "--default-model",
                "sentinel",
                "--secret-ref",
                key,
            ],
            None,
        )
    };
    let responses = add("openai-responses");
    let rid = responses["items"]
        .as_object()
        .unwrap()
        .keys()
        .next()
        .unwrap();
    let chat = add("openai-chat-completions");
    let cid = chat["items"]
        .as_object()
        .unwrap()
        .iter()
        .find(|(_, v)| v["metadata"]["api_kind"] == "openai-chat-completions")
        .unwrap()
        .0;
    let before = fs::read(&path).unwrap();
    assert!(
        !f.run(
            &[
                "ai",
                "setup",
                "--native-settings",
                "--agent",
                "codex",
                "--connection",
                cid
            ],
            None
        )
        .status
        .success()
    );
    assert_eq!(fs::read(&path).unwrap(), before);
    let args = [
        "ai",
        "setup",
        "--native-settings",
        "--agent",
        "codex",
        "--connection",
        rid,
    ];
    let p = f.json(&args, None);
    assert_eq!(fs::read(&path).unwrap(), before);
    let mut apply = args.to_vec();
    apply.extend([
        "--apply-preview",
        p["preview"]["preview_id"].as_str().unwrap(),
    ]);
    assert_eq!(f.json(&apply, None)["changed"], true);
    let after = fs::read(&path).unwrap();
    assert!(after.starts_with(b"# keep"));
    let p = f.json(&args, None);
    let mut noop = args.to_vec();
    noop.extend([
        "--apply-preview",
        p["preview"]["preview_id"].as_str().unwrap(),
    ]);
    assert_eq!(f.json(&noop, None)["changed"], false);
    assert_eq!(fs::read(&path).unwrap(), after);
}

#[test]
fn opencode_public_native_keeps_model_comments_and_refuses_file_substitution() {
    let f = F::new();
    let config = f.0.path().join(".config");
    let dir = config.join("opencode");
    fs::create_dir_all(&dir).unwrap();
    for p in [&config, &dir] {
        fs::set_permissions(p, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let path = dir.join("opencode.json");
    let before=b"{// keep\n\"model\":{/* model */\"providerID\":\"old\",\"model\":\"old\"},\"providers\":{\"neighbor\":{\"name\":\"keep\"}}}";
    fs::write(&path, before).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let key = f.json(
        &[
            "secrets",
            "create",
            "--kind",
            "api-key",
            "--label",
            "Synthetic",
            "--value-stdin",
        ],
        Some(VALUE.as_bytes()),
    );
    let id = key["items"][0]["id"].as_str().unwrap();
    let added = f.json(
        &[
            "ai",
            "connections",
            "add",
            "--label",
            "Synthetic",
            "--endpoint",
            "http://127.0.0.1:9/v1",
            "--api-kind",
            "openai-chat-completions",
            "--default-model",
            "synthetic",
            "--secret-ref",
            id,
        ],
        None,
    );
    let connection = added["items"].as_object().unwrap().keys().next().unwrap();
    let args = [
        "ai",
        "setup",
        "--native-settings",
        "--agent",
        "opencode",
        "--connection",
        connection,
    ];
    let p = f.json(&args, None);
    assert_eq!(fs::read(&path).unwrap(), before);
    let mut apply = args.to_vec();
    apply.extend([
        "--apply-preview",
        p["preview"]["preview_id"].as_str().unwrap(),
    ]);
    assert_eq!(f.json(&apply, None)["changed"], true);
    let after = fs::read(&path).unwrap();
    assert!(after.windows(11).any(|x| x == b"/* model */"));
    assert!(after.windows(7).any(|x| x == b"// keep"));
    let p = f.json(&args, None);
    let mut noop = args.to_vec();
    noop.extend([
        "--apply-preview",
        p["preview"]["preview_id"].as_str().unwrap(),
    ]);
    assert_eq!(f.json(&noop, None)["changed"], false);
    let current = f.json(&["secrets", "details", id], None);
    let rev = current["items"][0]["current_revision"].as_str().unwrap();
    f.json(
        &[
            "secrets",
            "rotate",
            id,
            "--expect-current",
            rev,
            "--value-stdin",
        ],
        Some(b"{file:synthetic}"),
    );
    let p = f.json(&args, None);
    let mut refused = args.to_vec();
    refused.extend([
        "--apply-preview",
        p["preview"]["preview_id"].as_str().unwrap(),
    ]);
    assert!(!f.run(&refused, None).status.success());
    assert_eq!(fs::read(path).unwrap(), after);
}
