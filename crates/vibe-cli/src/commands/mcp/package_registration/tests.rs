//! Registration ownership and rollback tests.

use super::*;
use vibe_core::manifest::McpServerDecl;

fn remote() -> DeclaredMcpServer {
    DeclaredMcpServer {
        decl: McpServerDecl {
            name: "fpf".into(),
            binary: None,
            url: Some("https://mcp.fpf.tools/mcp".into()),
            transport: Some("streamable-http".into()),
            description: None,
            args: Vec::new(),
        },
        binary: None,
        package: "ai.lev/fpf-mcp".into(),
        group: "ai.lev".into(),
        version: "1.0.0".into(),
    }
}

#[test]
fn selected_json_registration_refuses_edits_and_preserves_foreign_entries() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    fs::write(
        &path,
        r#"{"theme":"dark","mcpServers":{"other":{"command":"other"}}}"#,
    )
    .unwrap();
    let server = remote();
    let agent = Agent::QwenCode;
    let payload = server_payload(agent, dir.path(), &server, false).unwrap();
    let row = register_one(agent, Scope::Project, &path, &server, &payload, false).unwrap();
    assert_eq!(row.status, "created");
    let doc = read_json(&path).unwrap();
    assert_eq!(
        doc["mcpServers"]["fpf"]["httpUrl"],
        "https://mcp.fpf.tools/mcp"
    );
    assert_eq!(doc["mcpServers"]["other"]["command"], "other");
    assert_eq!(doc["theme"], "dark");
    assert_eq!(vibe_mcp::pkg_servers::managed_entries(&doc), ["fpf"]);
    assert_eq!(
        register_one(agent, Scope::Project, &path, &server, &payload, false)
            .unwrap()
            .status,
        "unchanged"
    );
    let mut edited = doc.clone();
    edited["mcpServers"]["fpf"]["httpUrl"] = "https://example.org/changed".into();
    atomic_write(
        &path,
        (serde_json::to_string_pretty(&edited).unwrap() + "\n").as_bytes(),
    )
    .unwrap();
    assert!(register_one(agent, Scope::Project, &path, &server, &payload, false).is_err());
    assert!(remove_selected(agent, Scope::Project, &path, "ai.lev/fpf-mcp", None, false).is_err());
    atomic_write(
        &path,
        (serde_json::to_string_pretty(&doc).unwrap() + "\n").as_bytes(),
    )
    .unwrap();
    remove_selected(
        agent,
        Scope::Project,
        &path,
        "mcp:ai.lev/fpf-mcp@=1.0.0",
        None,
        false,
    )
    .unwrap();
    let cleaned = read_json(&path).unwrap();
    assert!(cleaned["mcpServers"].get("fpf").is_none());
    assert_eq!(cleaned["mcpServers"]["other"]["command"], "other");
    assert!(cleaned.get("vibevm").is_none());
}

#[test]
fn codex_toml_uses_same_receipt_and_preserves_other_servers() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    fs::write(&path, "[mcp_servers.other]\ncommand = 'other'\n").unwrap();
    let server = remote();
    let payload = server_payload(Agent::Codex, dir.path(), &server, false).unwrap();
    register_one(Agent::Codex, Scope::User, &path, &server, &payload, false).unwrap();
    let config = read_toml(&path).unwrap();
    assert_eq!(
        config["mcp_servers"]["fpf"]["url"].as_str(),
        Some("https://mcp.fpf.tools/mcp")
    );
    assert_eq!(
        config["mcp_servers"]["other"]["command"].as_str(),
        Some("other")
    );
    assert_eq!(
        registration_status(Agent::Codex, Scope::User, &path, &server)
            .unwrap()
            .state,
        "registered"
    );
    remove_selected(
        Agent::Codex,
        Scope::User,
        &path,
        "ai.lev/fpf-mcp",
        None,
        false,
    )
    .unwrap();
    let config = read_toml(&path).unwrap();
    assert!(config["mcp_servers"].get("fpf").is_none());
    assert_eq!(
        config["mcp_servers"]["other"]["command"].as_str(),
        Some("other")
    );
}

#[test]
fn exact_legacy_entry_can_be_adopted_but_drifted_entry_cannot() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mcp.json");
    let server = remote();
    let payload = server_payload(Agent::QwenCode, dir.path(), &server, false).unwrap();
    let ConfigPayload::Json(entry) = &payload else {
        panic!("Qwen JSON");
    };
    let mut doc = serde_json::json!({"mcpServers": {"fpf": entry}});
    vibe_mcp::pkg_servers::mark_managed(&mut doc, "fpf").unwrap();
    fs::write(&path, serde_json::to_string_pretty(&doc).unwrap()).unwrap();
    assert_eq!(
        register_one(
            Agent::QwenCode,
            Scope::Project,
            &path,
            &server,
            &payload,
            false
        )
        .unwrap()
        .status,
        "adopted"
    );
    assert!(receipt_path(&path).unwrap().exists());
    remove_selected(
        Agent::QwenCode,
        Scope::Project,
        &path,
        &server.package,
        None,
        false,
    )
    .unwrap();
    assert!(read_json(&path).unwrap()["mcpServers"].get("fpf").is_none());
    fs::write(&path, serde_json::to_string_pretty(&doc).unwrap()).unwrap();
    doc["mcpServers"]["fpf"]["httpUrl"] = "https://example.org/changed".into();
    fs::write(&path, serde_json::to_string_pretty(&doc).unwrap()).unwrap();
    assert!(
        register_one(
            Agent::QwenCode,
            Scope::Project,
            &path,
            &server,
            &payload,
            false
        )
        .is_err()
    );
}

#[test]
fn atomic_write_replaces_complete_contents() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    fs::write(&path, "old content").unwrap();
    atomic_write(&path, b"new complete content").unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"new complete content");
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[cfg(unix)]
#[test]
fn atomic_write_uses_private_new_files_and_preserves_existing_mode() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    atomic_write(&path, b"first").unwrap();
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
    atomic_write(&path, b"second").unwrap();
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o640
    );
}

#[test]
fn pending_receipt_recovers_interrupted_url_replacement() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    fs::write(
        &path,
        "[mcp_servers.fpf]\nurl = 'https://old.example/mcp'\n",
    )
    .unwrap();
    let server = remote();
    let payload = server_payload(Agent::Codex, dir.path(), &server, false).unwrap();
    let old = serde_json::json!({"url": "https://old.example/mcp"});
    let mut receipt = Receipt::default();
    receipt.entries.insert(
        "fpf".into(),
        OwnedEntry {
            package: server.package.clone(),
            version: Some(server.version.clone()),
            payload: payload_json(&payload).unwrap(),
            pending_previous: Some(old),
        },
    );
    write_receipt(&path, &receipt).unwrap();
    assert_eq!(
        registration_status(Agent::Codex, Scope::User, &path, &server)
            .unwrap()
            .state,
        "pending"
    );
    register_one(Agent::Codex, Scope::User, &path, &server, &payload, false).unwrap();
    assert_eq!(
        registration_status(Agent::Codex, Scope::User, &path, &server)
            .unwrap()
            .state,
        "registered"
    );
    assert!(
        read_receipt(&path).unwrap().entries["fpf"]
            .pending_previous
            .is_none()
    );
}

#[test]
fn remote_desktop_is_skipped_for_all_but_refused_when_explicit() {
    let server = remote();
    let root = Path::new("/unused");
    assert!(
        planned_payload(Agent::ClaudeCodeDesktop, root, &server, false, true)
            .unwrap()
            .is_none()
    );
    assert!(planned_payload(Agent::ClaudeCodeDesktop, root, &server, false, false).is_err());
    assert!(
        planned_payload(Agent::QwenCode, root, &server, false, true)
            .unwrap()
            .is_some()
    );
}

#[test]
fn multi_agent_uninstall_preflight_refuses_drift_without_partial_removal() {
    let dir = tempfile::tempdir().unwrap();
    let qwen = dir.path().join("qwen.json");
    let cursor = dir.path().join("cursor.json");
    let server = remote();
    for (agent, path) in [(Agent::QwenCode, &qwen), (Agent::Cursor, &cursor)] {
        let payload = server_payload(agent, dir.path(), &server, false).unwrap();
        register_one(agent, Scope::Project, path, &server, &payload, false).unwrap();
    }
    let qwen_before = fs::read(&qwen).unwrap();
    let mut drifted = read_json(&cursor).unwrap();
    drifted["mcpServers"]["fpf"]["url"] = "https://changed.example/mcp".into();
    atomic_write(
        &cursor,
        (serde_json::to_string_pretty(&drifted).unwrap() + "\n").as_bytes(),
    )
    .unwrap();
    let destinations = [
        (Agent::QwenCode, Scope::Project, qwen.clone()),
        (Agent::Cursor, Scope::Project, cursor.clone()),
    ];
    assert!(remove_from_configs(&destinations, &server.package, None, false).is_err());
    assert_eq!(fs::read(&qwen).unwrap(), qwen_before);
}

#[test]
fn global_refresh_preflight_refuses_edited_owned_entry() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let server = remote();
    let payload = server_payload(Agent::Codex, dir.path(), &server, false).unwrap();
    register_one(Agent::Codex, Scope::User, &path, &server, &payload, false).unwrap();
    preflight_owned_entries(Agent::Codex, &path, &server.package).unwrap();
    atomic_write(
        &path,
        b"[mcp_servers.fpf]\nurl = 'https://operator.example/mcp'\n",
    )
    .unwrap();
    assert!(preflight_owned_entries(Agent::Codex, &path, &server.package).is_err());
}

#[test]
fn user_scope_rejects_local_project_root_substitution() {
    let mut server = remote();
    server.decl.url = None;
    server.decl.binary = Some("local-fpf".into());
    server.decl.args = vec!["--path".into(), "{project_root}".into()];
    assert!(ensure_scope_supported(Scope::User, &server).is_err());
    ensure_scope_supported(Scope::Project, &server).unwrap();
}

#[test]
fn selector_kind_and_exact_version_are_honored() {
    assert!(normalized_coordinate("skill:ai.lev/fpf-mcp").is_err());
    assert_eq!(
        normalized_coordinate("mcp:ai.lev/fpf-mcp@=1.0.0").unwrap(),
        "ai.lev/fpf-mcp"
    );
    ensure_requested_version("mcp:ai.lev/fpf-mcp@=1.0.0", "1.0.0").unwrap();
    assert!(ensure_requested_version("mcp:ai.lev/fpf-mcp@=9.9.9", "1.0.0").is_err());
    assert!(ensure_requested_version("mcp:ai.lev/fpf-mcp@^1.0", "1.0.0").is_err());
}

#[test]
fn unchanged_endpoint_refreshes_the_owned_version_for_exact_removal() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.toml");
    let mut server = remote();
    let payload = server_payload(Agent::Codex, dir.path(), &server, false).unwrap();
    register_one(Agent::Codex, Scope::User, &config, &server, &payload, false).unwrap();
    let before = fs::read(&config).unwrap();
    server.version = "1.0.1".into();
    register_one(Agent::Codex, Scope::User, &config, &server, &payload, true).unwrap();
    assert_eq!(
        read_receipt(&config).unwrap().entries["fpf"]
            .version
            .as_deref(),
        Some("1.0.0")
    );
    register_one(Agent::Codex, Scope::User, &config, &server, &payload, false).unwrap();
    assert_eq!(fs::read(&config).unwrap(), before);
    assert!(
        remove_selected(
            Agent::Codex,
            Scope::User,
            &config,
            "mcp:ai.lev/fpf-mcp@=1.0.0",
            None,
            false
        )
        .is_err()
    );
    remove_selected(
        Agent::Codex,
        Scope::User,
        &config,
        "mcp:ai.lev/fpf-mcp@=1.0.1",
        None,
        false,
    )
    .unwrap();
    assert!(!receipt_path(&config).unwrap().exists());
}
