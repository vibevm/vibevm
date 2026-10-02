//! TTY-only prompts for MCP installation.

use super::*;

// ---------------------------------------------------------------------------
// Interactive helpers — TTY-only paths
// ---------------------------------------------------------------------------

pub(super) fn interactive_ask_scope(path: &Path) -> Result<Scope> {
    if !stdin_is_tty() {
        bail!(
            "no --scope and stdin is not a TTY — pass `--scope project|user|both` or \
             `--auto` (auto-resolves scope from vibe.toml presence)"
        );
    }
    let has_toml = has_vibe_toml(path);
    let default_idx = if has_toml { 0 } else { 1 };
    let prompt = if has_toml {
        "Where to install? (vibe.toml found — defaulting to project-level)"
    } else {
        "Where to install? (vibe.toml not found — defaulting to user-level)"
    };
    let chosen = dialoguer::Select::new()
        .with_prompt(prompt)
        .items([
            "Project-level — per-project files committed to git",
            "User-level    — global home/config dirs, works everywhere",
            "Both          — project AND user simultaneously",
        ])
        .default(default_idx)
        .interact()?;
    Ok(match chosen {
        0 => Scope::Project,
        1 => Scope::User,
        2 => Scope::Both,
        _ => unreachable!(),
    })
}

pub(super) fn interactive_ask_what() -> Result<What> {
    if !stdin_is_tty() {
        return Ok(What::Both);
    }
    let chosen = dialoguer::Select::new()
        .with_prompt("What to install?")
        .items([
            "Both MCP server config and SKILL.md (recommended)",
            "MCP server only",
            "SKILL.md only",
        ])
        .default(0)
        .interact()?;
    Ok(match chosen {
        0 => What::Both,
        1 => What::Mcp,
        2 => What::Skill,
        _ => unreachable!(),
    })
}

pub(super) fn interactive_select_agents(detected: &[Agent], force: bool) -> Result<Vec<Agent>> {
    if !stdin_is_tty() {
        bail!(
            "no --agent and stdin is not a TTY — pass `--agent <name>` (one of \
             `all`, `claude`, `claude-desktop`, `cursor`, `opencode`, `codex`) or \
             `--auto` to detect every supported agent"
        );
    }
    // Slice 5: always show ALL agents, with checkbox preselected for
    // detected ones and a `(not detected)` badge on the rest. `--force`
    // toggles whether unchecked-not-detected agents will install
    // anyway when chosen — but visually they're always pickable.
    let _ = force; // currently informational only at the wizard layer
    let pool: Vec<Agent> = Agent::ALL.to_vec();
    let labels: Vec<String> = pool
        .iter()
        .map(|a| {
            let badge = if detected.contains(a) {
                ""
            } else {
                "  (not detected)"
            };
            format!("{}{}", a.as_str(), badge)
        })
        .collect();
    let defaults: Vec<bool> = pool.iter().map(|a| detected.contains(a)).collect();
    let chosen = dialoguer::MultiSelect::new()
        .with_prompt("Which agents? (space to toggle, enter to confirm)")
        .items(&labels)
        .defaults(&defaults)
        .interact()?;
    Ok(chosen.into_iter().map(|i| pool[i]).collect())
}
