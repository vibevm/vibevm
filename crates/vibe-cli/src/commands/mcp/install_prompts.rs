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
    let chosen = crate::output::prompts::choose(
        prompt,
        &[
            described("Project-level", "per-project files committed to git"),
            described("User-level", "global home/config dirs, works everywhere"),
            described("Both", "project AND user simultaneously"),
        ],
        default_idx,
    )?;
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
    let chosen = crate::output::prompts::choose(
        "What to install?",
        &crate::output::prompts::named(&[
            "Both MCP server config and SKILL.md (recommended)",
            "MCP server only",
            "SKILL.md only",
        ]),
        0,
    )?;
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
    // A detected agent carries the list's own `[detected]` column; the rest
    // stay pickable without a badge of their own.
    let choices: Vec<vibe_app::Choice> = pool
        .iter()
        .map(|a| vibe_app::Choice {
            name: a.as_str().to_owned(),
            detected: detected.contains(a),
            ..vibe_app::Choice::default()
        })
        .collect();
    let defaults: Vec<bool> = pool.iter().map(|a| detected.contains(a)).collect();
    // The key legend above the list names space and enter, so the prompt
    // no longer repeats them.
    let chosen = crate::output::prompts::choose_many("Which agents?", &choices, &defaults)?
        .unwrap_or_default();
    Ok(chosen.into_iter().map(|i| pool[i]).collect())
}

/// A row with a name and the sentence after it.
fn described(name: &str, description: &str) -> vibe_app::Choice {
    vibe_app::Choice {
        name: name.to_owned(),
        description: Some(description.to_owned()),
        ..vibe_app::Choice::default()
    }
}
