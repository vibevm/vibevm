//! Agent-profile + detection tests (PROP-015 §2.4, §2.5), exercising the
//! public `vibe_mcp::agents` surface: Scope/What parsing, agent
//! detection, per-agent config/skill path resolution, and the
//! scope-aware MCP entry shape. These relocated from vibe-cli's
//! commands/mcp.rs when the domain moved here (CONVERT-PLAN v0.1 §7.3).

use std::path::Path;

use vibe_mcp::agents::{Agent, ConfigPayload, Scope, What, detect_agents};

fn json_payload(agent: Agent, scope: Scope, project: Option<&Path>) -> serde_json::Value {
    match agent.build_mcp_entry(scope, project) {
        ConfigPayload::Json(v) => v,
        ConfigPayload::Toml(_) => panic!("expected JSON for {}", agent.as_str()),
    }
}

fn toml_payload(agent: Agent, scope: Scope, project: Option<&Path>) -> toml::Value {
    match agent.build_mcp_entry(scope, project) {
        ConfigPayload::Toml(v) => v,
        ConfigPayload::Json(_) => panic!("expected TOML for {}", agent.as_str()),
    }
}

// ---- Scope / What ----

#[test]
fn scope_parse_known_values() {
    assert_eq!(Scope::parse("project").unwrap(), Scope::Project);
    assert_eq!(Scope::parse("user").unwrap(), Scope::User);
    assert_eq!(Scope::parse("both").unwrap(), Scope::Both);
    assert!(Scope::parse("global").is_err());
}

#[test]
fn scope_expand_both_yields_two_concrete() {
    assert_eq!(Scope::Both.expand(), vec![Scope::Project, Scope::User]);
    assert_eq!(Scope::Project.expand(), vec![Scope::Project]);
    assert_eq!(Scope::User.expand(), vec![Scope::User]);
}

#[test]
fn scope_requires_vibe_toml_only_for_project() {
    assert!(Scope::Project.requires_vibe_toml());
    assert!(!Scope::Both.requires_vibe_toml());
    assert!(!Scope::User.requires_vibe_toml());
}

#[test]
fn what_parse_known_values() {
    assert_eq!(What::parse("mcp").unwrap(), What::Mcp);
    assert_eq!(What::parse("skill").unwrap(), What::Skill);
    assert_eq!(What::parse("both").unwrap(), What::Both);
    assert!(What::parse("nope").is_err());
}

#[test]
fn what_includes_axes() {
    assert!(What::Mcp.includes_mcp() && !What::Mcp.includes_skill());
    assert!(!What::Skill.includes_mcp() && What::Skill.includes_skill());
    assert!(What::Both.includes_mcp() && What::Both.includes_skill());
}

// ---- detection ----

#[test]
fn detect_finds_claude_via_marker_dir() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".claude")).unwrap();
    let agents = detect_agents(Some(dir.path()));
    assert!(agents.contains(&Agent::ClaudeCode));
}

#[test]
fn detect_finds_opencode_via_agents_md() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("AGENTS.md"), "x").unwrap();
    let agents = detect_agents(Some(dir.path()));
    assert!(agents.contains(&Agent::OpenCode));
}

#[test]
fn detect_with_no_project_root_falls_back_to_host_probe() {
    // Without a project root, only host-presence agents can show up.
    // The set is non-deterministic per machine, but the call must not
    // panic and must return at most all-five.
    let agents = detect_agents(None);
    for a in &agents {
        assert!(Agent::ALL.contains(a));
    }
}

// ---- parse_filter ----

#[test]
fn parse_filter_known_values() {
    assert_eq!(Agent::parse_filter("all").unwrap(), Agent::ALL.to_vec());
    assert_eq!(
        Agent::parse_filter("claude").unwrap(),
        vec![Agent::ClaudeCode]
    );
    assert_eq!(
        Agent::parse_filter("claude-desktop").unwrap(),
        vec![Agent::ClaudeCodeDesktop]
    );
    assert_eq!(Agent::parse_filter("cursor").unwrap(), vec![Agent::Cursor]);
    assert_eq!(
        Agent::parse_filter("opencode").unwrap(),
        vec![Agent::OpenCode]
    );
    assert_eq!(Agent::parse_filter("codex").unwrap(), vec![Agent::Codex]);
    assert!(Agent::parse_filter("nope").is_err());
}

// ---- per-agent profile ----

#[test]
fn supports_project_scope_only_for_three_agents() {
    assert!(Agent::ClaudeCode.supports_project_scope());
    assert!(Agent::Cursor.supports_project_scope());
    assert!(Agent::OpenCode.supports_project_scope());
    assert!(!Agent::ClaudeCodeDesktop.supports_project_scope());
    assert!(!Agent::Codex.supports_project_scope());
}

#[test]
fn supports_skill_only_for_three_agents() {
    assert!(Agent::ClaudeCode.supports_skill());
    assert!(Agent::OpenCode.supports_skill());
    assert!(Agent::Codex.supports_skill());
    assert!(!Agent::ClaudeCodeDesktop.supports_skill());
    assert!(!Agent::Cursor.supports_skill());
}

// ---- config_path ----

#[test]
fn config_path_project_lands_under_project_root() {
    let dir = tempfile::tempdir().unwrap();
    let p = Agent::ClaudeCode
        .config_path(Scope::Project, Some(dir.path()))
        .unwrap()
        .unwrap();
    let s = p.display().to_string().replace('\\', "/");
    assert!(s.ends_with("/.claude/settings.json"), "got {s}");

    let p = Agent::OpenCode
        .config_path(Scope::Project, Some(dir.path()))
        .unwrap()
        .unwrap();
    let s = p.display().to_string().replace('\\', "/");
    assert!(s.ends_with("/opencode.json"), "got {s}");

    let p = Agent::Cursor
        .config_path(Scope::Project, Some(dir.path()))
        .unwrap()
        .unwrap();
    let s = p.display().to_string().replace('\\', "/");
    assert!(s.ends_with("/.cursor/mcp.json"), "got {s}");
}

#[test]
fn config_path_user_only_agents_have_no_project_surface() {
    let dir = tempfile::tempdir().unwrap();
    assert!(
        Agent::ClaudeCodeDesktop
            .config_path(Scope::Project, Some(dir.path()))
            .unwrap()
            .is_none()
    );
    assert!(
        Agent::Codex
            .config_path(Scope::Project, Some(dir.path()))
            .unwrap()
            .is_none()
    );
}

#[test]
fn config_path_user_resolves_for_all_agents() {
    for &a in Agent::ALL {
        let p = a.config_path(Scope::User, None).unwrap();
        assert!(p.is_some(), "user-scope path missing for {}", a.as_str());
    }
}

#[test]
fn opencode_user_paths_use_xdg_style_on_every_os() {
    // OpenCode is documented to read `~/.config/opencode/` on every
    // platform — XDG-style, NOT %APPDATA% on Windows.
    let cfg = Agent::OpenCode
        .config_path(Scope::User, None)
        .unwrap()
        .unwrap();
    let cfg_s = cfg.display().to_string().replace('\\', "/");
    assert!(
        cfg_s.contains("/.config/opencode/opencode.json"),
        "expected XDG-style ~/.config/opencode/opencode.json; got `{cfg_s}`"
    );
    let skill = Agent::OpenCode
        .skill_path(Scope::User, None)
        .unwrap()
        .unwrap();
    let skill_s = skill.display().to_string().replace('\\', "/");
    assert!(
        skill_s.contains("/.config/opencode/skills/vibevm/SKILL.md"),
        "expected XDG-style ~/.config/opencode/skills/vibevm/SKILL.md; got `{skill_s}`"
    );
}

#[test]
fn config_path_both_is_internal_error() {
    let dir = tempfile::tempdir().unwrap();
    assert!(
        Agent::ClaudeCode
            .config_path(Scope::Both, Some(dir.path()))
            .is_err()
    );
}

// ---- build_mcp_entry scope-awareness ----

#[test]
fn project_scope_mcp_entry_carries_path_arg() {
    let dir = tempfile::tempdir().unwrap();
    let v = json_payload(Agent::ClaudeCode, Scope::Project, Some(dir.path()));
    let args: Vec<&str> = v["args"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a.as_str().unwrap())
        .collect();
    assert_eq!(args[0], "mcp");
    assert_eq!(args[1], "serve");
    assert_eq!(args[2], "--path");
    assert!(args.len() == 4, "expected 4 args, got {args:?}");
}

#[test]
fn user_scope_mcp_entry_omits_path_arg() {
    let v = json_payload(Agent::ClaudeCode, Scope::User, None);
    let args: Vec<&str> = v["args"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a.as_str().unwrap())
        .collect();
    assert_eq!(args, vec!["mcp", "serve"], "user-scope must omit --path");
}

#[test]
fn opencode_user_scope_entry_uses_command_array_without_path() {
    let v = json_payload(Agent::OpenCode, Scope::User, None);
    let cmd: Vec<&str> = v["command"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a.as_str().unwrap())
        .collect();
    assert_eq!(cmd, vec!["vibe", "mcp", "serve"]);
    assert_eq!(v["type"], "local");
    assert_eq!(v["enabled"], true);
}

#[test]
fn codex_user_scope_entry_returns_toml_table_without_path() {
    let v = toml_payload(Agent::Codex, Scope::User, None);
    let tbl = v.as_table().unwrap();
    assert_eq!(tbl.get("command").and_then(|x| x.as_str()), Some("vibe"));
    let args = tbl.get("args").and_then(|x| x.as_array()).unwrap();
    let strs: Vec<&str> = args.iter().filter_map(|a| a.as_str()).collect();
    assert_eq!(strs, vec!["mcp", "serve"]);
}

// ---- skill_path ----

#[test]
fn skill_path_user_works_without_project_root() {
    let p = Agent::ClaudeCode
        .skill_path(Scope::User, None)
        .unwrap()
        .unwrap();
    let s = p.display().to_string().replace('\\', "/");
    assert!(s.ends_with("/.claude/skills/vibevm/SKILL.md"), "got {s}");
}

#[test]
fn skill_path_project_lands_under_project_root() {
    let dir = tempfile::tempdir().unwrap();
    let p = Agent::OpenCode
        .skill_path(Scope::Project, Some(dir.path()))
        .unwrap()
        .unwrap();
    let s = p.display().to_string().replace('\\', "/");
    assert!(s.ends_with("/.opencode/skills/vibevm/SKILL.md"), "got {s}");
}
