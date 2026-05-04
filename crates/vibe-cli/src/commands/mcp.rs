//! `vibe mcp` — Model Context Protocol surface.
//!
//! Spec: PROP-004 §5.1 + ROADMAP §M1.7. Three subcommands today:
//!
//! - `vibe mcp serve` — run the JSON-RPC server over stdio.
//! - `vibe mcp install` — detect coding agents in the project tree
//!   and write per-agent MCP config so they pick up vibevm on next
//!   start (Claude Code's `.claude/settings.json`, Cursor's
//!   `.cursor/mcp.json`, etc.). Idempotent.
//! - `vibe mcp status` — show what `install` would write without
//!   touching disk.
//!
//! Library implementation lives in `vibe-mcp`; this module is just
//! the CLI dispatch + per-agent config writer.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Serialize;
use serde_json::{Map, Value};
use vibe_core::manifest::ProjectManifest;
use vibe_mcp::{Server, ServerContext};

use crate::cli::{McpArgs, McpInstallArgs, McpServeArgs, McpStatusArgs, McpSubcommand};
use crate::output;

pub fn run(ctx: &output::Context, args: McpArgs) -> Result<()> {
    match args.command {
        McpSubcommand::Serve(sub) => run_serve(sub),
        McpSubcommand::Install(sub) => run_install(ctx, sub),
        McpSubcommand::Status(sub) => run_status(ctx, sub),
    }
}

fn run_serve(args: McpServeArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let server_ctx = ServerContext::new(project_root);
    let mut server = Server::stdio(server_ctx);
    server.run().context("MCP server I/O error")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Agent detection + config writers
// ---------------------------------------------------------------------------

/// Coding agent supported by this slice. New variants land per slice
/// once the agent's MCP config conventions are nailed down.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum Agent {
    /// Claude Code — `.claude/settings.json` `mcpServers` block.
    ClaudeCode,
    /// Cursor — `.cursor/mcp.json` `mcpServers` block.
    Cursor,
}

impl Agent {
    pub fn as_str(self) -> &'static str {
        match self {
            Agent::ClaudeCode => "claude",
            Agent::Cursor => "cursor",
        }
    }

    pub fn parse_filter(filter: &str) -> Result<Vec<Agent>> {
        match filter {
            "all" => Ok(vec![Agent::ClaudeCode, Agent::Cursor]),
            "claude" | "claude-code" => Ok(vec![Agent::ClaudeCode]),
            "cursor" => Ok(vec![Agent::Cursor]),
            other => bail!(
                "unknown --agent value `{other}` (expected `all`, `claude`, or `cursor`)"
            ),
        }
    }

    /// File the config block lives in, relative to project root.
    pub fn config_relative_path(self) -> &'static str {
        match self {
            Agent::ClaudeCode => ".claude/settings.json",
            Agent::Cursor => ".cursor/mcp.json",
        }
    }

    /// Marker file/dir whose presence in the project tree signals
    /// that the agent is in active use. Walked by `detect_agents`.
    fn presence_markers(self) -> &'static [&'static str] {
        match self {
            Agent::ClaudeCode => &[".claude", "CLAUDE.md"],
            Agent::Cursor => &[".cursor", ".cursorrules"],
        }
    }
}

/// Detect which agents have any presence-marker in the project tree.
/// Empty result is legal — `--force` lets the operator install
/// against an absent agent, but the default is conservative.
pub fn detect_agents(project_root: &Path) -> Vec<Agent> {
    [Agent::ClaudeCode, Agent::Cursor]
        .into_iter()
        .filter(|a| {
            a.presence_markers()
                .iter()
                .any(|m| project_root.join(m).exists())
        })
        .collect()
}

/// Outcome of one agent's config-write attempt.
#[derive(Debug, Clone, Serialize)]
pub struct AgentInstallReport {
    pub agent: String,
    pub config_path: String,
    /// `created` / `updated` / `unchanged` / `dry-run` / `skipped`.
    pub status: &'static str,
    /// Human-readable explanation when status is `skipped` or
    /// `dry-run`.
    pub note: Option<String>,
}

#[derive(Debug, Serialize)]
struct InstallReport {
    ok: bool,
    command: &'static str,
    project: String,
    detected: Vec<String>,
    targeted: Vec<String>,
    results: Vec<AgentInstallReport>,
    dry_run: bool,
}

fn run_install(ctx: &output::Context, args: McpInstallArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let detected = detect_agents(&project_root);
    let filter = Agent::parse_filter(&args.agent)?;
    let targeted: Vec<Agent> = filter
        .iter()
        .copied()
        .filter(|a| args.force || detected.contains(a))
        .collect();

    let mut results: Vec<AgentInstallReport> = Vec::new();
    for agent in &targeted {
        let path = project_root.join(agent.config_relative_path());
        let entry = build_mcp_entry(&project_root);
        let outcome = if args.dry_run {
            preview_install(*agent, &path, &entry)?
        } else {
            apply_install(*agent, &path, &entry)?
        };
        results.push(outcome);
    }

    let report = InstallReport {
        ok: true,
        command: "mcp:install",
        project: project_root.display().to_string(),
        detected: detected.iter().map(|a| a.as_str().to_string()).collect(),
        targeted: targeted.iter().map(|a| a.as_str().to_string()).collect(),
        results: results.clone(),
        dry_run: args.dry_run,
    };

    if ctx.is_json() {
        ctx.emit_json(&report)?;
        return Ok(());
    }
    if ctx.is_quiet() {
        let written = results
            .iter()
            .filter(|r| matches!(r.status, "created" | "updated"))
            .count();
        ctx.summary(&format!(
            "vibe mcp install: {written} agent config{} {}",
            if written == 1 { "" } else { "s" },
            if args.dry_run { "previewed" } else { "written" }
        ));
        return Ok(());
    }

    if results.is_empty() {
        ctx.summary(&format!(
            "no supported agents detected in `{}` (Claude Code or Cursor). Use `--force` to provision regardless.",
            project_root.display()
        ));
        return Ok(());
    }
    for r in &results {
        let prefix = if args.dry_run { "would" } else { r.status };
        let note = r
            .note
            .as_deref()
            .map(|n| format!(" ({n})"))
            .unwrap_or_default();
        ctx.step(&format!(
            "{} update  {}  → {}{note}",
            prefix, r.agent, r.config_path
        ));
    }
    Ok(())
}

fn run_status(ctx: &output::Context, args: McpStatusArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let detected = detect_agents(&project_root);
    let mut results: Vec<AgentInstallReport> = Vec::new();
    for agent in [Agent::ClaudeCode, Agent::Cursor] {
        let path = project_root.join(agent.config_relative_path());
        let entry = build_mcp_entry(&project_root);
        let outcome = preview_install(agent, &path, &entry)?;
        results.push(outcome);
    }
    let report = InstallReport {
        ok: true,
        command: "mcp:status",
        project: project_root.display().to_string(),
        detected: detected.iter().map(|a| a.as_str().to_string()).collect(),
        targeted: vec![],
        results: results.clone(),
        dry_run: true,
    };
    if ctx.is_json() {
        ctx.emit_json(&report)?;
        return Ok(());
    }
    ctx.summary(&format!(
        "Detected agents: {}",
        if detected.is_empty() {
            "(none)".to_string()
        } else {
            detected
                .iter()
                .map(|a| a.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        }
    ));
    for r in &results {
        let note = r
            .note
            .as_deref()
            .map(|n| format!(" ({n})"))
            .unwrap_or_default();
        ctx.step(&format!("{}  {}  → {}{note}", r.status, r.agent, r.config_path));
    }
    Ok(())
}

/// Compose the MCP server entry the agent's config should reference.
/// Today: `vibe mcp serve --path <project>` — the binary name is
/// hard-coded as `vibe`; if the operator's installation puts the
/// binary at an unusual path, the json file can be hand-edited
/// post-install. Future: probe `which vibe` and use the resolved
/// path so a globally-installed `vibe` doesn't depend on PATH at
/// agent start time.
fn build_mcp_entry(project_root: &Path) -> Value {
    let project_str = project_root.display().to_string().replace('\\', "/");
    serde_json::json!({
        "command": "vibe",
        "args": ["mcp", "serve", "--path", project_str],
    })
}

fn preview_install(
    agent: Agent,
    config_path: &Path,
    new_entry: &Value,
) -> Result<AgentInstallReport> {
    let (status, note) = decide_action(config_path, new_entry)?;
    let dry_status = match status {
        "unchanged" => "unchanged",
        "created" => "would-create",
        "updated" => "would-update",
        other => other,
    };
    Ok(AgentInstallReport {
        agent: agent.as_str().to_string(),
        config_path: config_path.display().to_string().replace('\\', "/"),
        status: dry_status,
        note,
    })
}

fn apply_install(
    agent: Agent,
    config_path: &Path,
    new_entry: &Value,
) -> Result<AgentInstallReport> {
    let (status, note) = decide_action(config_path, new_entry)?;
    if status == "unchanged" {
        return Ok(AgentInstallReport {
            agent: agent.as_str().to_string(),
            config_path: config_path.display().to_string().replace('\\', "/"),
            status: "unchanged",
            note,
        });
    }
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating dir `{}`", parent.display()))?;
    }
    let merged = merge_mcp_block(config_path, new_entry)?;
    let serialized = serde_json::to_string_pretty(&merged)
        .with_context(|| "serializing merged config")?;
    fs::write(config_path, serialized + "\n")
        .with_context(|| format!("writing `{}`", config_path.display()))?;
    Ok(AgentInstallReport {
        agent: agent.as_str().to_string(),
        config_path: config_path.display().to_string().replace('\\', "/"),
        status,
        note,
    })
}

/// Determine the post-merge status without mutating the file. Returns
/// `("created" | "updated" | "unchanged", note)`.
fn decide_action(
    config_path: &Path,
    new_entry: &Value,
) -> Result<(&'static str, Option<String>)> {
    if !config_path.exists() {
        return Ok(("created", Some("file does not exist yet".into())));
    }
    let existing = read_json(config_path)?;
    let existing_entry = existing
        .get("mcpServers")
        .and_then(|v| v.get("vibevm"));
    match existing_entry {
        Some(e) if e == new_entry => Ok(("unchanged", None)),
        Some(_) => Ok(("updated", Some("vibevm entry differs".into()))),
        None => Ok(("updated", Some("mcpServers/vibevm absent".into()))),
    }
}

fn read_json(path: &Path) -> Result<Value> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("reading `{}`", path.display()))?;
    if text.trim().is_empty() {
        return Ok(Value::Object(Map::new()));
    }
    let v: Value = serde_json::from_str(&text)
        .with_context(|| format!("parsing `{}`", path.display()))?;
    Ok(v)
}

fn merge_mcp_block(config_path: &Path, new_entry: &Value) -> Result<Value> {
    let mut existing = if config_path.exists() {
        read_json(config_path)?
    } else {
        Value::Object(Map::new())
    };
    let obj = existing
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("`{}` is not a JSON object", config_path.display()))?;
    let servers = obj
        .entry("mcpServers".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let servers_obj = servers
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("`mcpServers` is not an object"))?;
    servers_obj.insert("vibevm".to_string(), new_entry.clone());
    Ok(existing)
}

fn resolve_project_root(path: &Path) -> Result<PathBuf> {
    let canonical = path
        .canonicalize()
        .with_context(|| format!("canonicalizing `{}`", path.display()))?;
    let stripped = super::init::strip_unc_public(canonical);
    if !stripped.join(ProjectManifest::FILENAME).exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first or pass `--path <dir>`",
            stripped.display()
        );
    }
    Ok(stripped)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_finds_claude_via_marker_dir() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".claude")).unwrap();
        let agents = detect_agents(dir.path());
        assert_eq!(agents, vec![Agent::ClaudeCode]);
    }

    #[test]
    fn detect_finds_claude_via_claude_md() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("CLAUDE.md"), "x").unwrap();
        let agents = detect_agents(dir.path());
        assert!(agents.contains(&Agent::ClaudeCode));
    }

    #[test]
    fn detect_finds_cursor_via_marker_dir() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".cursor")).unwrap();
        let agents = detect_agents(dir.path());
        assert!(agents.contains(&Agent::Cursor));
    }

    #[test]
    fn parse_filter_known_values() {
        assert_eq!(Agent::parse_filter("all").unwrap().len(), 2);
        assert_eq!(
            Agent::parse_filter("claude").unwrap(),
            vec![Agent::ClaudeCode]
        );
        assert_eq!(
            Agent::parse_filter("claude-code").unwrap(),
            vec![Agent::ClaudeCode]
        );
        assert_eq!(
            Agent::parse_filter("cursor").unwrap(),
            vec![Agent::Cursor]
        );
        assert!(Agent::parse_filter("nope").is_err());
    }

    #[test]
    fn merge_inserts_into_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let entry = build_mcp_entry(dir.path());
        let merged = merge_mcp_block(&path, &entry).unwrap();
        assert_eq!(
            merged["mcpServers"]["vibevm"]["command"],
            "vibe"
        );
    }

    #[test]
    fn merge_preserves_existing_keys() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(
            &path,
            r#"{
              "preexisting": "value",
              "mcpServers": { "other-server": { "command": "x" } }
            }"#,
        )
        .unwrap();
        let entry = build_mcp_entry(dir.path());
        let merged = merge_mcp_block(&path, &entry).unwrap();
        assert_eq!(merged["preexisting"], "value");
        assert_eq!(merged["mcpServers"]["other-server"]["command"], "x");
        assert_eq!(merged["mcpServers"]["vibevm"]["command"], "vibe");
    }

    #[test]
    fn decide_action_reports_created_for_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nope.json");
        let entry = build_mcp_entry(dir.path());
        let (status, _) = decide_action(&path, &entry).unwrap();
        assert_eq!(status, "created");
    }

    #[test]
    fn decide_action_reports_unchanged_when_block_matches() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let entry = build_mcp_entry(dir.path());
        let merged = merge_mcp_block(&path, &entry).unwrap();
        std::fs::write(&path, serde_json::to_string_pretty(&merged).unwrap()).unwrap();
        let (status, _) = decide_action(&path, &entry).unwrap();
        assert_eq!(status, "unchanged");
    }

    #[test]
    fn decide_action_reports_updated_when_block_differs() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(
            &path,
            r#"{ "mcpServers": { "vibevm": { "command": "old" } } }"#,
        )
        .unwrap();
        let new_entry = build_mcp_entry(dir.path());
        let (status, _) = decide_action(&path, &new_entry).unwrap();
        assert_eq!(status, "updated");
    }
}
