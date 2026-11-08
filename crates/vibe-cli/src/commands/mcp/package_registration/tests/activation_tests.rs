//! Temp-config tests; no native home, process launch or network is consulted.
use super::*;
use vibe_agent_projection::activation::ActivationIntent;

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
fn disable_survives_reinstall_version_update_and_restart_then_reenables() {
    for agent in [Agent::ClaudeCode, Agent::OpenCode, Agent::Codex] {
        let temp = tempfile::tempdir().unwrap();
        let config = temp.path().join("config");
        let mut server = remote();
        let payload = server_payload(agent, temp.path(), &server, true).unwrap();
        register_one(agent, Scope::Project, &config, &server, &payload, false).unwrap();
        let off = ActivationIntent {
            enabled: false,
            actor: Some("test-operator".into()),
            provenance: "test:disable".into(),
        };
        activation::set_activation(
            agent,
            Scope::Project,
            &config,
            &server,
            &payload,
            &off,
            false,
        )
        .unwrap();
        server.version = "2.0.0".into();
        server.decl.url = Some("https://example.test/v2".into());
        let next = server_payload(agent, temp.path(), &server, true).unwrap();
        register_one(agent, Scope::Project, &config, &server, &next, false).unwrap();
        let status = registration_status(agent, Scope::Project, &config, &server).unwrap();
        assert_eq!(status.state, "disabled");
        assert!(!status.desired_enabled);
        assert_eq!(
            status.activation.unwrap().actor.as_deref(),
            Some("test-operator")
        );
        assert_eq!(
            read_receipt(&config).unwrap().entries[&server.decl.name]
                .version
                .as_deref(),
            Some("2.0.0")
        );
        if matches!(agent, Agent::OpenCode | Agent::Codex) {
            assert_eq!(
                current_entry(agent, &config, &server.decl.name)
                    .unwrap()
                    .unwrap()["enabled"],
                false
            );
        } else {
            assert!(
                current_entry(agent, &config, &server.decl.name)
                    .unwrap()
                    .is_none()
            );
        }
        let on = ActivationIntent {
            enabled: true,
            actor: None,
            provenance: "test:enable".into(),
        };
        activation::set_activation(agent, Scope::Project, &config, &server, &next, &on, false)
            .unwrap();
        assert_eq!(
            registration_status(agent, Scope::Project, &config, &server)
                .unwrap()
                .state,
            "registered"
        );
    }
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
fn activation_is_scope_specific_and_foreign_drift_preserves_every_byte() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("project.json");
    let user = temp.path().join("user.json");
    let server = remote();
    let agent = Agent::ClaudeCode;
    let payload = server_payload(agent, temp.path(), &server, true).unwrap();
    for (scope, path) in [(Scope::Project, &project), (Scope::User, &user)] {
        register_one(agent, scope, path, &server, &payload, false).unwrap();
    }
    let off = ActivationIntent {
        enabled: false,
        actor: None,
        provenance: "test:disable".into(),
    };
    activation::set_activation(
        agent,
        Scope::Project,
        &project,
        &server,
        &payload,
        &off,
        false,
    )
    .unwrap();
    assert_eq!(
        registration_status(agent, Scope::User, &user, &server)
            .unwrap()
            .state,
        "registered"
    );
    let foreign = br#"{"mcpServers":{"fpf":{"command":"foreign"}}}"#;
    fs::write(&project, foreign).unwrap();
    let receipt = fs::read(receipt_path(&project).unwrap()).unwrap();
    assert!(
        activation::set_activation(
            agent,
            Scope::Project,
            &project,
            &server,
            &payload,
            &off,
            false
        )
        .is_err()
    );
    assert_eq!(fs::read(&project).unwrap(), foreign);
    assert_eq!(fs::read(receipt_path(&project).unwrap()).unwrap(), receipt);
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
fn failed_activation_dry_run_preserves_receipt_and_config_and_unknown_actor() {
    let root = tempfile::tempdir().unwrap();
    let config = root.path().join("config.json");
    let server = remote();
    let payload = server_payload(Agent::OpenCode, root.path(), &server, true).unwrap();
    register_one(
        Agent::OpenCode,
        Scope::Project,
        &config,
        &server,
        &payload,
        false,
    )
    .unwrap();
    assert!(
        registration_status(Agent::OpenCode, Scope::Project, &config, &server)
            .unwrap()
            .activation
            .is_none()
    );
    let oldconfig = fs::read(&config).unwrap();
    let oldreceipt = fs::read(receipt_path(&config).unwrap()).unwrap();
    let bad = ActivationIntent {
        enabled: false,
        actor: Some("\n".into()),
        provenance: "test".into(),
    };
    assert!(
        activation::set_activation(
            Agent::OpenCode,
            Scope::Project,
            &config,
            &server,
            &payload,
            &bad,
            false
        )
        .is_err()
    );
    assert_eq!(fs::read(&config).unwrap(), oldconfig);
    assert_eq!(
        fs::read(receipt_path(&config).unwrap()).unwrap(),
        oldreceipt
    );
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
fn builtin_identical_content_needs_explicit_registration_not_toggle_or_refresh() {
    let root = tempfile::tempdir().unwrap();
    let config = root.path().join("config.json");
    let agent = Agent::ClaudeCode;
    let payload = agent.build_mcp_entry();
    let native = payload_json(&payload).unwrap();
    fs::write(
        &config,
        serde_json::to_vec(&serde_json::json!({"mcpServers":{"vibevm":native}})).unwrap(),
    )
    .unwrap();
    let before = fs::read(&config).unwrap();
    let server = activation::builtin_server();
    let off = ActivationIntent {
        enabled: false,
        actor: None,
        provenance: "test:disable".into(),
    };
    assert!(
        activation::set_activation(
            agent,
            Scope::Project,
            &config,
            &server,
            &payload,
            &off,
            false
        )
        .is_err()
    );
    assert!(activation::register_builtin(agent, Scope::Project, &config, &payload, false).is_err());
    assert!(!receipt_path(&config).unwrap().exists());
    assert_eq!(fs::read(&config).unwrap(), before);
    assert_eq!(
        activation::register_builtin_explicit(
            agent,
            Scope::Project,
            &config,
            &payload,
            vibe_agent_projection::activation::BuiltinAdoption::explicit("vibe mcp install"),
            true
        )
        .unwrap()
        .status,
        "would-adopt"
    );
    assert!(!receipt_path(&config).unwrap().exists());
    activation::register_builtin_explicit(
        agent,
        Scope::Project,
        &config,
        &payload,
        vibe_agent_projection::activation::BuiltinAdoption::explicit("vibe mcp install"),
        false,
    )
    .unwrap();
    let owned = read_receipt(&config).unwrap();
    let intent = owned.entries["vibevm"].activation.as_ref().unwrap();
    assert!(intent.actor.is_none());
    assert_eq!(intent.provenance, "vibe mcp install:explicit-adoption");
    activation::set_activation(
        agent,
        Scope::Project,
        &config,
        &server,
        &payload,
        &off,
        false,
    )
    .unwrap();
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
fn declaration_churn_retains_mcp_disabled_intent_but_uninstall_forgets_it() {
    let root = tempfile::tempdir().unwrap();
    let config = root.path().join("config.json");
    let mut server = remote();
    let agent = Agent::ClaudeCode;
    let payload = server_payload(agent, root.path(), &server, true).unwrap();
    register_one(agent, Scope::User, &config, &server, &payload, false).unwrap();
    let off = ActivationIntent {
        enabled: false,
        actor: Some("operator".into()),
        provenance: "test:disable".into(),
    };
    activation::set_activation(agent, Scope::User, &config, &server, &payload, &off, false)
        .unwrap();
    let mut batch = PublicationBatch::default();
    remove::retire_vanished_in(
        agent,
        Scope::User,
        &config,
        &server.package,
        &server.decl.name,
        Mutation {
            dry_run: false,
            batch: &mut batch,
        },
    )
    .unwrap();
    assert_eq!(
        read_receipt(&config).unwrap().entries[&server.decl.name].activation,
        Some(off.clone())
    );
    server.version = "3.0.0".into();
    register_one(agent, Scope::User, &config, &server, &payload, false).unwrap();
    assert_eq!(
        registration_status(agent, Scope::User, &config, &server)
            .unwrap()
            .state,
        "disabled"
    );
    remove_selected(agent, Scope::User, &config, &server.package, None, false).unwrap();
    assert!(read_receipt(&config).unwrap().entries.is_empty());
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
fn missing_or_null_native_payload_cannot_grant_ownership() {
    let root = tempfile::tempdir().unwrap();
    let config = root.path().join("config.json");
    let path = receipt_path(&config).unwrap();
    for explicit_null in [false, true] {
        let mut input =
            serde_json::json!({"schema":2,"entries":{"fpf":{"package":"ai.lev/fpf-mcp"}}});
        if explicit_null {
            input["entries"]["fpf"]["payload"] = serde_json::Value::Null;
        }
        let decoded = serde_json::from_value::<Receipt>(input.clone());
        assert_eq!(
            decoded.is_ok(),
            explicit_null,
            "required presence is wire-owned; null refusal is domain-owned"
        );
        let bytes = serde_json::to_vec(&input).unwrap();
        fs::write(&path, &bytes).unwrap();
        assert!(read_receipt(&config).is_err());
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert!(!config.exists());
    }
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
fn known_mcp_owner_retains_intent_when_projection_is_foreign_or_unparseable() {
    for foreign in [
        r#"{"mcpServers":{"fpf":{"command":"foreign"}}}"#,
        "not JSON",
    ] {
        let root = tempfile::tempdir().unwrap();
        let config = root.path().join("config.json");
        let server = remote();
        let agent = Agent::ClaudeCode;
        let payload = server_payload(agent, root.path(), &server, true).unwrap();
        register_one(agent, Scope::Project, &config, &server, &payload, false).unwrap();
        fs::write(&config, foreign).unwrap();
        let before = fs::read(receipt_path(&config).unwrap()).unwrap();
        let off = ActivationIntent {
            enabled: false,
            actor: None,
            provenance: "test:disable".into(),
        };
        assert!(
            activation::set_activation(
                agent,
                Scope::Project,
                &config,
                &server,
                &payload,
                &off,
                true
            )
            .is_err()
        );
        assert_eq!(fs::read(receipt_path(&config).unwrap()).unwrap(), before);
        let error = activation::set_activation(
            agent,
            Scope::Project,
            &config,
            &server,
            &payload,
            &off,
            false,
        )
        .unwrap_err();
        assert!(error.to_string().contains("intent retained"));
        assert_eq!(fs::read_to_string(&config).unwrap(), foreign);
        assert_eq!(
            read_receipt(&config).unwrap().entries[&server.decl.name].activation,
            Some(off)
        );
    }
}

/// A stale unreceipted builtin entry (an older vibe's launch shape) is a typed
/// refusal for explicit registration and is replaced — only that key — by a
/// forced adoption, which leaves no unfinished `pending_previous` journal.
#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
fn stale_unreceipted_builtin_is_replaced_only_by_forced_adoption() {
    use vibe_agent_projection::activation::BuiltinAdoption;
    for agent in [Agent::ClaudeCode, Agent::Codex] {
        let root = tempfile::tempdir().unwrap();
        let config = root.path().join("config");
        let payload = agent.build_mcp_entry();
        let stale = match agent.config_format() {
            ConfigFormat::Json => {
                r#"{"keep":1,"mcpServers":{"vibevm":{"command":"old-binary","args":[]},"other":{"command":"o"}}}"#
            }
            ConfigFormat::Toml => {
                "keep = 1\n[mcp_servers.vibevm]\ncommand = \"old-binary\"\nargs = []\n[mcp_servers.other]\ncommand = \"o\"\n"
            }
        };
        fs::write(&config, stale).unwrap();
        let error = activation::register_builtin_explicit(
            agent,
            Scope::Project,
            &config,
            &payload,
            BuiltinAdoption::explicit("vibe mcp install"),
            false,
        )
        .unwrap_err();
        assert!(
            error
                .downcast_ref::<activation::BuiltinOwnershipError>()
                .is_some(),
            "{error:#}"
        );
        assert!(error.to_string().contains("vibe mcp install --force"));
        assert_eq!(fs::read_to_string(&config).unwrap(), stale);
        assert!(!receipt_path(&config).unwrap().exists());

        let forced = BuiltinAdoption::forced("vibe mcp install");
        let preview = activation::register_builtin_explicit(
            agent,
            Scope::Project,
            &config,
            &payload,
            forced,
            true,
        )
        .unwrap();
        assert_eq!(preview.status, "would-adopt");
        assert_eq!(fs::read_to_string(&config).unwrap(), stale);
        assert!(!receipt_path(&config).unwrap().exists());

        let applied = activation::register_builtin_explicit(
            agent,
            Scope::Project,
            &config,
            &payload,
            forced,
            false,
        )
        .unwrap();
        assert_eq!(applied.status, "adopted");
        assert!(applied.note.unwrap().contains("old-binary"));
        assert_eq!(
            current_entry(agent, &config, SERVER_NAME).unwrap(),
            Some(payload_json(&payload).unwrap())
        );
        assert!(current_entry(agent, &config, "other").unwrap().is_some());
        let owned = read_receipt(&config).unwrap();
        let row = &owned.entries[SERVER_NAME];
        assert!(row.pending_previous.is_none());
        assert_eq!(
            row.activation.as_ref().unwrap().provenance,
            "vibe mcp install:forced-adoption"
        );
        assert!(!owned.entries.contains_key("other"));
        assert_eq!(
            registration_status(
                agent,
                Scope::Project,
                &config,
                &activation::builtin_server()
            )
            .unwrap()
            .state,
            "registered"
        );
    }
}
