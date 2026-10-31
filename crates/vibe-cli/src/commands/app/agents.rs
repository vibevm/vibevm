//! The agent posture of one application command (PROP-064): the agent
//! chosen from `--agent`, the external registration and `agents.toml` of the
//! user home, whether vibe runs inside a hosting agent, the consent
//! question, and the rendering of a parked step.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-064#selection");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-064#consent");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-064#parking");

use anyhow::Result;
use dialoguer::Confirm;
use vibe_app::{
    AgentError, AgentHooks, AgentNotice, AgentPrompt, AgentsConfig, AppError, SelectedAgent,
};
use vibe_wire::generated::lifecycle::e1::context::RunAgentMode;

use super::global;
use crate::output;

/// What the agent hooks of one command borrow.
pub(crate) struct AgentSetup {
    config: AgentsConfig,
    selected: Option<SelectedAgent>,
    hosted: bool,
    assume_yes: bool,
    show: bool,
    attended: bool,
    resume: String,
}

impl AgentSetup {
    /// Resolve the agent of this command (`--agent NAME` > the external
    /// registration > `default`), always from the user's `agents.toml`, so a
    /// project home inherits the user's agents.
    pub(crate) fn new(ctx: &output::Context, flag: Option<&str>, assume_yes: bool) -> Result<Self> {
        let config = AgentsConfig::load(&global::settings_root()?).map_err(AppError::from)?;
        let external = super::facts::external_agent().map_err(AppError::from)?;
        let selected = vibe_app::select_agent(&config, flag, external).map_err(AppError::from)?;
        Ok(Self {
            config,
            selected,
            hosted: ctx.agent_mode() == RunAgentMode::Agent,
            assume_yes: assume_yes || ctx.is_unattended(),
            show: !ctx.is_machine_progress(),
            attended: global::attended(ctx),
            resume: resume_command(std::env::args().skip(1)),
        })
    }

    /// The hooks an operation context carries.
    pub(crate) fn hooks(&self) -> AgentHooks<'_> {
        AgentHooks {
            selected: self.selected.as_ref(),
            hosted: self.hosted,
            assume_yes: self.assume_yes,
            trusted_groups: &self.config.trusted_groups,
            notify: self.show.then_some(&notify as &dyn Fn(&AgentNotice<'_>)),
            confirm: self
                .attended
                .then_some(&confirm as vibe_app::ConsentPrompt<'_>),
            resume_command: Some(&self.resume),
        }
    }
}

/// The external registration of this process, for `vibe agent list`.
pub(crate) fn external_registration() -> Result<Option<vibe_app::ExternalAgent>> {
    Ok(super::facts::external_agent().map_err(AppError::from)?)
}

/// The same command again with `--resume` (and without `--from-step`).
fn resume_command(args: impl Iterator<Item = String>) -> String {
    let mut words = vec!["vibe".to_owned()];
    let mut skip_value = false;
    for arg in args {
        if std::mem::take(&mut skip_value) {
            continue;
        }
        if arg == "--from-step" {
            skip_value = true;
            continue;
        }
        if arg.starts_with("--from-step=") || arg == "--resume" {
            continue;
        }
        words.push(arg);
    }
    words.push("--resume".into());
    words
        .iter()
        .map(|word| {
            if !word.is_empty()
                && word
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || "-_./:=@,+".contains(ch))
            {
                word.clone()
            } else {
                format!("'{}'", word.replace('\'', "'\\''"))
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Print the step, the agent command and the full prompt before any agent
/// starts, and an agent's answer when it ends (stderr; `--json` keeps
/// stdout to its envelope).
fn notify(notice: &AgentNotice<'_>) {
    match notice {
        AgentNotice::Prompt(prompt) => {
            eprintln!(
                "agent step `{}` ({}) will run agent `{}`:\n  command: {}\n  prompt:",
                prompt.step,
                prompt.title,
                prompt.agent,
                prompt.command.join(" ")
            );
            for line in prompt.prompt.lines() {
                eprintln!("    {line}");
            }
        }
        AgentNotice::Answer { step, ok, message } => {
            let verdict = if *ok { "ok" } else { "fail" };
            match message {
                Some(message) => eprintln!("agent step `{step}`: {verdict} — {message}"),
                None => eprintln!("agent step `{step}`: {verdict}"),
            }
        }
    }
}

/// The attended consent question (PROP-064 `#consent`).
fn confirm(prompt: &AgentPrompt<'_>) -> std::result::Result<bool, String> {
    Confirm::new()
        .with_prompt(format!(
            "Run agent `{}` with the prompt of step `{}`?",
            prompt.agent, prompt.step
        ))
        .default(false)
        .interact()
        .map_err(|error| error.to_string())
}

/// A parked step prints the fenced `vibe-agent-tasks` block a hosting agent
/// reads (PROP-054 `#AGENT-HANDSHAKE`) in human and quiet output; `--json`
/// carries the same facts in the error envelope.
pub(crate) fn report_parked<T>(
    ctx: &output::Context,
    result: std::result::Result<T, vibe_app::AppError>,
) -> std::result::Result<T, vibe_app::AppError> {
    if let Err(vibe_app::AppError::Agent(AgentError::Parked(parked))) = &result
        && !ctx.is_json()
    {
        crate::commands::lifecycle::render_agent_task_fence(
            ctx,
            &parked.run_id,
            &parked.task_files,
            &parked.resume,
        );
    }
    result
}

#[cfg(test)]
mod tests {
    use super::resume_command;

    #[test]
    fn the_resume_command_repeats_the_invocation_with_resume() {
        let args = [
            "install",
            "-g",
            "org.example/tool",
            "--from-step",
            "fix",
            "--set",
            "a=b c",
        ];
        assert_eq!(
            resume_command(args.iter().map(|arg| (*arg).to_owned())),
            "vibe install -g org.example/tool --set 'a=b c' --resume"
        );
        let resumed = ["update", "-g", "x/y", "--resume"];
        assert_eq!(
            resume_command(resumed.iter().map(|arg| (*arg).to_owned())),
            "vibe update -g x/y --resume"
        );
    }
}
