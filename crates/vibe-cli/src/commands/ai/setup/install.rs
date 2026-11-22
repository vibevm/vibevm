//! Optional agent package installation in attended `vibe ai setup`; each
//! selected package goes through the ordinary `vibe install -g` in a child.
use anyhow::Result;
use vibe_core::cancellation::CancellationToken;

/// Agents offered for installation and their `org.vibevm.agents` packages.
const AGENTS: [(&str, &str); 6] = [
    ("pi", "pi"),
    ("qwen", "qwen"),
    ("codex", "codex"),
    ("opencode", "opencode"),
    ("hermes", "hermes"),
    ("claude", "claude-code"),
];

/// Ask (default No), then install each selected package that is not yet
/// installed. One agent's outcome never stops the others.
pub(super) fn offer(cancel: &CancellationToken) -> Result<()> {
    if !crate::output::prompts::confirm("Install AI agent packages now?", false)? {
        return Ok(());
    }
    let names: Vec<&str> = AGENTS.iter().map(|(name, _)| *name).collect();
    let choices = crate::output::prompts::named(&names);
    let Some(selected) = crate::output::prompts::choose_many(
        "Select agent packages to install",
        &choices,
        &[false; 6],
    )?
    else {
        return Ok(());
    };
    let settings = crate::commands::app::global::settings_root()?;
    let exe = std::env::current_exe()?;
    for index in selected {
        let (agent, package) = AGENTS[index];
        let package = format!("org.vibevm.agents/{package}");
        let prefix = format!("{package}@");
        let present = || {
            crate::commands::app::global::installed_keys(&settings)
                .iter()
                .any(|key| key.starts_with(&prefix))
        };
        let outcome = if present() {
            "already-installed"
        } else {
            cancel.checkpoint()?;
            let ran = std::process::Command::new(&exe)
                .args(["install", "-g", package.as_str()])
                .status()
                .is_ok_and(|status| status.success());
            match (ran, present()) {
                (true, true) => "installed",
                (true, false) => "not-installed",
                (false, _) => "failed",
            }
        };
        println!("Agent package {agent}: {outcome}");
    }
    Ok(())
}
