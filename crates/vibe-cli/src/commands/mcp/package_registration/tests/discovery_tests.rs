//! Read-only native registration discovery with isolated injected roots.
use super::*;
use vibe_agent_projection::agents::AgentUserDirectories;

fn dirs(root: &Path) -> AgentUserDirectories {
    AgentUserDirectories {
        home: Some(root.join("home")),
        config: Some(root.join("config")),
    }
}

#[test]
#[specmark::verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#REG-GLOBAL-UNINSTALL-DEFAULTS"
)]
fn native_discovery_distinguishes_owned_present_missing_changed_pending_and_foreign() {
    let temp = tempfile::tempdir().unwrap();
    let dirs = dirs(temp.path());
    let server = remote();
    let agents = [
        Agent::ClaudeCode,
        Agent::Cursor,
        Agent::Codex,
        Agent::QwenCode,
    ];
    let mut before = Vec::new();
    for agent in agents {
        let config = agent
            .config_path_with_dirs(Scope::User, None, &dirs)
            .unwrap()
            .unwrap();
        let payload = server_payload(agent, temp.path(), &server, false).unwrap();
        register_one(agent, Scope::User, &config, &server, &payload, false).unwrap();
        match agent {
            Agent::Cursor => {
                fs::remove_file(&config).unwrap();
            }
            Agent::Codex => {
                fs::write(
                    &config,
                    "[mcp_servers.fpf]\nurl='https://changed.example'\n",
                )
                .unwrap();
            }
            Agent::QwenCode => {
                let mut receipt = read_receipt(&config).unwrap();
                let entry = receipt.entries.get_mut("fpf").unwrap();
                entry.pending_previous = Some(entry.payload.clone());
                entry.payload = serde_json::json!({"httpUrl":"https://next.example"});
                write_receipt(&config, &receipt).unwrap();
            }
            _ => {}
        }
        // Foreign collisions never count, even alongside an owned entry.
        if agent == Agent::ClaudeCode {
            let mut json = read_json(&config).unwrap();
            json["mcpServers"]["foreign"] = serde_json::json!({"command":"external"});
            fs::write(&config, serde_json::to_vec(&json).unwrap()).unwrap();
        }
        before.push(snapshot(config.clone()).unwrap());
        before.push(snapshot(receipt_path(&config).unwrap()).unwrap());
    }
    let foreign = Agent::OpenCode
        .config_path_with_dirs(Scope::User, None, &dirs)
        .unwrap()
        .unwrap();
    fs::create_dir_all(foreign.parent().unwrap()).unwrap();
    fs::write(
        &foreign,
        r#"{"mcp":{"fpf":{"url":"https://foreign.example"}}}"#,
    )
    .unwrap();
    before.push(snapshot(foreign).unwrap());
    // A receipt for another package is not a candidate for this package.
    let other = Agent::ClaudeCodeDesktop
        .config_path_with_dirs(Scope::User, None, &dirs)
        .unwrap()
        .unwrap();
    let mut receipt = Receipt::default();
    receipt.entries.insert(
        "fpf".into(),
        OwnedEntry {
            package: "other/package".into(),
            version: None,
            payload: serde_json::json!({}),
            pending_previous: None,
        },
    );
    write_receipt(&other, &receipt).unwrap();
    before.push(snapshot(receipt_path(&other).unwrap()).unwrap());
    let statuses = lifecycle::user_registration_statuses("mcp:ai.lev/fpf-mcp", &dirs).unwrap();
    assert_eq!(statuses.iter().map(|s| s.agent).collect::<Vec<_>>(), agents);
    let states: Vec<_> = statuses
        .iter()
        .map(|s| (s.registered, s.missing, s.changed, s.pending))
        .collect();
    assert_eq!(
        states,
        [(1, 0, 0, 0), (0, 1, 0, 0), (0, 0, 1, 0), (0, 0, 0, 1)]
    );
    assert_eq!(
        user_agent_configs_for_package(&server.package, &dirs)
            .unwrap()
            .len(),
        4
    );
    for saved in before {
        assert_eq!(snapshot(saved.path).unwrap().before, saved.before);
    }
}

#[test]
#[specmark::verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#REG-GLOBAL-UNINSTALL-DEFAULTS"
)]
fn native_discovery_partial_presence_preserves_drift_refusal_and_dry_graph() {
    let temp = tempfile::tempdir().unwrap();
    let dirs = dirs(temp.path());
    let config = Agent::Codex
        .config_path_with_dirs(Scope::User, None, &dirs)
        .unwrap()
        .unwrap();
    let server = remote();
    let payload = server_payload(Agent::Codex, temp.path(), &server, false).unwrap();
    register_one(Agent::Codex, Scope::User, &config, &server, &payload, false).unwrap();
    let config_before = fs::read(&config).unwrap();
    let receipt_before = fs::read(receipt_path(&config).unwrap()).unwrap();
    let verified = lifecycle::user_registration_statuses(&server.package, &dirs).unwrap();
    let defaults: Vec<_> = verified
        .iter()
        .filter(|s| s.registered > 0)
        .map(|s| s.agent)
        .collect();
    assert_eq!(defaults, [Agent::Codex]);
    let dry_plan = plan_global_removal(&server.package, &defaults, &dirs).unwrap();
    assert!(dry_plan.package_removed);
    assert_eq!(fs::read(&config).unwrap(), config_before);
    assert_eq!(
        fs::read(receipt_path(&config).unwrap()).unwrap(),
        receipt_before
    );
    let mut second = server.clone();
    second.decl.name = "second".into();
    register_one(Agent::Codex, Scope::User, &config, &second, &payload, false).unwrap();
    let mut doc = read_toml(&config).unwrap();
    doc["mcp_servers"]["second"]["url"] = toml::Value::String("https://changed.example".into());
    fs::write(&config, toml::to_string(&doc).unwrap()).unwrap();
    let before = fs::read(&config).unwrap();
    let receipt_before = fs::read(receipt_path(&config).unwrap()).unwrap();
    let statuses = lifecycle::user_registration_statuses(&server.package, &dirs).unwrap();
    assert_eq!((statuses[0].registered, statuses[0].changed), (1, 1));
    assert!(plan_global_removal(&server.package, &[Agent::Codex], &dirs).is_err());
    assert_eq!(fs::read(&config).unwrap(), before);
    assert_eq!(
        fs::read(receipt_path(&config).unwrap()).unwrap(),
        receipt_before
    );
}

#[test]
#[specmark::verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#REG-GLOBAL-UNINSTALL-DEFAULTS"
)]
fn native_discovery_sanitizes_toml_parser_errors_without_writes() {
    let temp = tempfile::tempdir().unwrap();
    let dirs = dirs(temp.path());
    let config = Agent::Codex
        .config_path_with_dirs(Scope::User, None, &dirs)
        .unwrap()
        .unwrap();
    let server = remote();
    let payload = server_payload(Agent::Codex, temp.path(), &server, false).unwrap();
    register_one(Agent::Codex, Scope::User, &config, &server, &payload, false).unwrap();
    let broken = "credential = 'TOP_SECRET_DO_NOT_REPORT' invalid";
    fs::write(&config, broken).unwrap();
    let error = lifecycle::user_registration_statuses(&server.package, &dirs)
        .err()
        .unwrap();
    let message = format!("{error:#}");
    assert!(message.contains("codex"));
    assert!(!message.contains("TOP_SECRET"));
    assert!(!message.contains("credential"));
    assert_eq!(fs::read_to_string(config).unwrap(), broken);
}
