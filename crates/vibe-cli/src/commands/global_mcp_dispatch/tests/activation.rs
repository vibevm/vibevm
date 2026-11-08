//! Managed activation and guarded rollback through the real global CLI owner.
use super::*;

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
fn managed_activation_survives_real_global_install_and_update_without_removing_package() {
    let harness = Harness::new();
    harness.seed("fpf", "1.0.0");
    harness.install("claude");
    harness
        .run(&[
            "mcp",
            "disable",
            "a",
            "--package",
            "ai.lev/fpf",
            "--agent",
            "claude",
            "--scope",
            "user",
            "--actor",
            "operator",
            "--json",
        ])
        .unwrap();
    assert_eq!(harness.names(Agent::ClaudeCode), ["b"]);
    assert!(harness.slot("1.0.0").exists());
    harness.install("claude");
    assert_eq!(harness.names(Agent::ClaudeCode), ["b"]);
    harness.seed("fpf", "1.0.1");
    harness
        .run(&["update", "-g", "mcp:ai.lev/fpf", "--assume-yes", "--json"])
        .unwrap();
    assert_eq!(harness.names(Agent::ClaudeCode), ["b"]);
    assert!(harness.slot("1.0.1").exists());
    let receipt: serde_json::Value =
        serde_json::from_slice(&fs::read(harness.receipt(Agent::ClaudeCode)).unwrap()).unwrap();
    assert_eq!(receipt["entries"]["a"]["activation"]["enabled"], false);
    assert_eq!(receipt["entries"]["a"]["activation"]["actor"], "operator");
    harness
        .run(&[
            "mcp",
            "enable",
            "a",
            "--package",
            "ai.lev/fpf",
            "--agent",
            "claude",
            "--scope",
            "user",
            "--json",
        ])
        .unwrap();
    assert_eq!(harness.names(Agent::ClaudeCode), ["b", "a"]);
    assert!(
        fs::read_to_string(harness.config(Agent::ClaudeCode))
            .unwrap()
            .contains("/1.0.1")
    );
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
fn actual_removal_inverse_refuses_foreign_replacement_without_restoring_sibling() {
    for replace_receipt in [false, true] {
        let harness = Harness::new();
        harness.seed("fpf", "1.0.0");
        harness.install("codex");
        let mut ctx = output::Context::from_flags(false, true, None, false, AgentModeArg::Cli);
        ctx.agent_user_dirs = harness.directories.clone();
        let mut removal =
            mcp::preflight_unregister_global_package(&ctx, "mcp:ai.lev/fpf", &[Agent::Codex])
                .unwrap();
        mcp::unregister_global_package(&mut removal).unwrap();
        let config = harness.config(Agent::Codex);
        let receipt = harness.receipt(Agent::Codex);
        let after = fs::read(&config).unwrap();
        let foreign = if replace_receipt { &receipt } else { &config };
        let bytes = if replace_receipt {
            b"foreign receipt".to_vec()
        } else {
            after.clone()
        };
        let replacement = foreign.with_extension("foreign");
        fs::write(&replacement, &bytes).unwrap();
        if !replace_receipt {
            fs::set_permissions(&replacement, fs::metadata(foreign).unwrap().permissions())
                .unwrap();
        }
        fs::rename(replacement, foreign).unwrap();
        let error = mcp::restore_global_package(&mut removal).unwrap_err();
        assert!(format!("{error:#}").contains("recovery required"));
        assert_eq!(fs::read(foreign).unwrap(), bytes);
        assert_eq!(fs::read(&config).unwrap(), after);
        assert!(harness.slot("1.0.0").exists());
    }
}

#[cfg(test)]
fn arm_refresh_foreign_cut(
    first: PathBuf,
    trigger: PathBuf,
    planted: Arc<std::sync::Mutex<Option<vibe_safefs::StableFileSnapshot>>>,
) {
    vibe_safefs::arm_before_publish_verify(Some(Box::new(move |parent, name| {
        if parent.join(name) == trigger {
            let bytes = fs::read(&first).unwrap();
            let replacement = first.with_extension("foreign");
            fs::write(&replacement, bytes).unwrap();
            fs::set_permissions(&replacement, fs::metadata(&first).unwrap().permissions()).unwrap();
            fs::rename(replacement, &first).unwrap();
            let observed = vibe_safefs::Project::open(first.parent().unwrap())
                .unwrap()
                .read_file_snapshot_bounded(first.file_name().unwrap().to_str().unwrap(), 1 << 20)
                .unwrap()
                .unwrap();
            *planted.lock().unwrap() = Some(observed);
            vibe_safefs::fail_after_publish(Some(name));
        } else {
            arm_refresh_foreign_cut(first.clone(), trigger.clone(), planted.clone());
        }
    })));
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
fn actual_global_update_keeps_foreign_same_byte_object_after_later_agent_failure() {
    let harness = Harness::new();
    harness.seed("fpf", "1.0.0");
    harness.install("claude");
    harness.install("cursor");
    harness.seed("fpf", "1.0.1");
    let config = harness.config(Agent::ClaudeCode);
    let planted = Arc::new(std::sync::Mutex::new(None));
    arm_refresh_foreign_cut(
        config.clone(),
        harness.receipt(Agent::Cursor),
        planted.clone(),
    );
    let result = harness.run(&["update", "-g", "mcp:ai.lev/fpf", "--assume-yes", "--json"]);
    vibe_safefs::arm_before_publish_verify(None);
    vibe_safefs::fail_after_publish(None);
    let error = result.unwrap_err();
    assert!(format!("{error:#}").contains("recovery required"));
    let foreign = planted
        .lock()
        .unwrap()
        .clone()
        .expect("later-agent publication cut fired");
    let actual = vibe_safefs::Project::open(config.parent().unwrap())
        .unwrap()
        .read_file_snapshot_bounded(config.file_name().unwrap().to_str().unwrap(), 1 << 20)
        .unwrap()
        .unwrap();
    assert_eq!(actual, foreign);
    assert!(
        actual
            .bytes
            .windows(b"/1.0.1".len())
            .any(|s| s == b"/1.0.1")
    );
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
fn actual_refresh_keeps_disabled_intent_when_all_old_declarations_vanish() {
    let harness = Harness::new();
    harness.seed("fpf", "1.0.0");
    harness.install("claude");
    harness
        .run(&[
            "mcp",
            "disable",
            "a",
            "--package",
            "ai.lev/fpf",
            "--agent",
            "claude",
            "--scope",
            "user",
            "--actor",
            "operator",
            "--json",
        ])
        .unwrap();
    harness.seed("fpf", "1.0.1");
    fs::write(harness.registry().join("ai.lev/fpf/v1.0.1/vibe.toml"),"[package]\nname='fpf'\ngroup='ai.lev'\nkind='mcp'\nversion='1.0.1'\n[[mcp_server]]\nname='c'\ntransport='streamable-http'\nurl='https://example.org/c'\n").unwrap();
    harness
        .run(&["update", "-g", "mcp:ai.lev/fpf", "--assume-yes", "--json"])
        .unwrap();
    assert!(harness.names(Agent::ClaudeCode).is_empty());
    let saved: serde_json::Value =
        serde_json::from_slice(&fs::read(harness.receipt(Agent::ClaudeCode)).unwrap()).unwrap();
    assert_eq!(saved["entries"]["a"]["activation"]["enabled"], false);
    assert!(saved["entries"].get("b").is_none());
    harness.seed("fpf", "1.0.2");
    harness
        .run(&["update", "-g", "mcp:ai.lev/fpf", "--assume-yes", "--json"])
        .unwrap();
    assert!(harness.names(Agent::ClaudeCode).is_empty());
    assert!(harness.slot("1.0.2").exists());
    harness
        .run(&[
            "mcp",
            "enable",
            "a",
            "--package",
            "ai.lev/fpf",
            "--agent",
            "claude",
            "--scope",
            "user",
            "--json",
        ])
        .unwrap();
    assert_eq!(harness.names(Agent::ClaudeCode), ["a"]);
    assert!(
        fs::read_to_string(harness.config(Agent::ClaudeCode))
            .unwrap()
            .contains("/1.0.2")
    );
}
