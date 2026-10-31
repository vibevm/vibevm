//! The `agent` step kind (`PROP-064`): consent before any step runs, then
//! per step the envelope, one spawned agent (or a parked task when `vibe`
//! runs inside a hosting agent), and its answer mapped to the step outcome.
//! Agents run one at a time; a prompt never passes through a shell.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-064#consent");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-064#modes");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-064#parking");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-064#never");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-064#envelope");

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use vibe_core::manifest::{AgentMode, StepDecl, StepKind, StepPhase};
use vibe_wire::generated::application::e1::agent_hook_result::Status;

use crate::agents::{
    AgentError, AgentHooks, AgentNotice, AgentPrompt, ParkedStep, Placeholders, RunMode,
    SelectedAgent,
};
use crate::error::{AppError, InstallerError, StepError};
use crate::installer::{ChildIo, run_attached, run_child};

use super::envelope::{self, ContextFacts, Envelope};
use super::plan::{Plan, PlannedStep};
use super::run::Run;
use super::runs::RunRecord;
use super::state::{AgentRun, AgentStatus};
use super::template::{self, Vars};

/// `inherit` is interactive in an attended terminal, unattended otherwise.
pub(crate) fn mode_of(mode: Option<AgentMode>, attended: bool) -> RunMode {
    match mode.unwrap_or_default() {
        AgentMode::Interactive => RunMode::Interactive,
        AgentMode::Unattended => RunMode::Unattended,
        AgentMode::Inherit if attended => RunMode::Interactive,
        AgentMode::Inherit => RunMode::Unattended,
    }
}

/// The prompt after template expansion (`PROP-065#templates`): inline, or
/// read from a package-relative `prompt_file`.
pub(crate) fn prompt_text(
    step: &StepDecl,
    package_root: &Path,
    vars: &Vars<'_>,
) -> Result<String, AppError> {
    let StepKind::Agent {
        prompt,
        prompt_file,
        ..
    } = &step.kind
    else {
        return Ok(String::new());
    };
    let text = match (prompt, prompt_file) {
        (Some(prompt), _) => prompt.clone(),
        (None, Some(file)) => {
            fs::read_to_string(package_root.join(file)).map_err(|error| StepError::Template {
                step: step.id.clone(),
                field: "prompt_file",
                reason: format!("`{}` cannot be read: {error}", file.display()),
            })?
        }
        (None, None) => String::new(),
    };
    template::expand(&text, vars, &step.id, "prompt")
}

/// Show every agent prompt that will run and obtain consent for it before
/// the first step (`PROP-064#consent`, `#never`): a trusted group or
/// `--assume-yes` proceeds, an attended terminal asks, anything else is
/// refused before any agent starts. Parked steps need no consent here: the
/// hosting agent is shown the task.
pub(crate) fn admit<'p>(
    hooks: &AgentHooks<'_>,
    plan: &Plan,
    pending: impl Iterator<Item = &'p PlannedStep>,
    answered: &dyn Fn(&str) -> bool,
    package_root: &Path,
    vars: &Vars<'_>,
    attended: bool,
) -> Result<(), AppError> {
    let Some(agent) = hooks.selected.filter(|_| !hooks.hosted) else {
        return Ok(());
    };
    for planned in pending {
        let step = &planned.step;
        if !planned.run || !matches!(step.kind, StepKind::Agent { .. }) || answered(&step.id) {
            continue;
        }
        let prompt =
            prompt_text(step, package_root, vars).or_else(|_| raw_prompt(step, package_root))?;
        let StepKind::Agent { mode, .. } = &step.kind else {
            continue;
        };
        let envelope = Envelope::at(&vars.work, &step.id);
        let preview = agent.invocation(
            &step.id,
            mode_of(*mode, attended),
            &Placeholders {
                prompt: "{prompt}",
                prompt_file: &envelope.prompt,
                context_file: &envelope.context,
                work: &vars.work,
            },
        );
        let mut command = vec![preview.command];
        command.extend(preview.args);
        let shown = AgentPrompt {
            step: &step.id,
            title: &step.title,
            agent: &agent.id,
            command: &command,
            prompt: &prompt,
        };
        if let Some(notify) = hooks.notify {
            notify(&AgentNotice::Prompt(shown));
        }
        if hooks.assume_yes || hooks.trusts(&plan.package.group) {
            continue;
        }
        let refused = |reason: String| -> AppError {
            AgentError::ConsentRequired {
                key: plan.package.key(),
                step: step.id.clone(),
                agent: agent.id.clone(),
                reason,
            }
            .into()
        };
        match hooks.confirm {
            Some(ask) => match ask(&shown) {
                Ok(true) => {}
                Ok(false) => return Err(refused("the prompt was declined".into())),
                Err(error) => return Err(refused(format!("the question failed: {error}"))),
            },
            None => {
                return Err(refused(
                    "an unattended run proceeds only with --assume-yes (or --unattended)".into(),
                ));
            }
        }
    }
    Ok(())
}

/// A prompt whose expansion needs a value known only later (`$app_root`
/// of a finish step) is shown as written.
fn raw_prompt(step: &StepDecl, package_root: &Path) -> Result<String, AppError> {
    let StepKind::Agent {
        prompt,
        prompt_file,
        ..
    } = &step.kind
    else {
        return Ok(String::new());
    };
    match (prompt, prompt_file) {
        (Some(prompt), _) => Ok(prompt.clone()),
        (None, Some(file)) => Ok(fs::read_to_string(package_root.join(file)).unwrap_or_default()),
        (None, None) => Ok(String::new()),
    }
}

/// Everything one agent step runs with.
pub(crate) struct AgentStep<'a> {
    pub hooks: &'a AgentHooks<'a>,
    pub plan: &'a Plan,
    pub package_root: &'a Path,
    pub work: &'a Path,
    /// `{work}` of the agent: the work directory, or `app_root` for a
    /// finish step.
    pub agent_work: &'a Path,
    pub app_root: Option<&'a Path>,
    pub vars: &'a Vars<'a>,
    pub attended: bool,
    pub earlier: Vec<(String, String, String)>,
    pub io: ChildIo<'a>,
    /// The PATH (and `VIBE_APP_USE`) the step's requirements put first,
    /// exactly as a recipe step of the same option receives them
    /// (`PROP-065#app-dependencies`); empty without requirements.
    pub requirements: &'a [(String, String)],
    /// The run record's `steps/<id>/`.
    pub record_step: Option<PathBuf>,
    /// `<home>/agentic/outbox/<run>/`.
    pub outbox: PathBuf,
    pub run_id: String,
    /// The step was parked before: an answer may be waiting.
    pub resumed_delegation: bool,
}

/// Run one agent step: the agent record for `steps.json` and either the
/// answer's message or the error (a parked step is `AgentError::Parked`).
pub(crate) fn run(
    cx: &AgentStep<'_>,
    step: &StepDecl,
) -> (Option<AgentRun>, Result<Option<String>, AppError>) {
    let StepKind::Agent { mode, .. } = &step.kind else {
        return (None, Ok(None));
    };
    let mode = mode_of(*mode, cx.attended);
    let envelope = Envelope::at(cx.work, &step.id);
    let mut record = AgentRun {
        agent: if cx.hooks.hosted {
            "hosting-agent".into()
        } else {
            cx.hooks
                .selected
                .map_or_else(String::new, |agent| agent.id.clone())
        },
        mode,
        envelope: envelope.run.display().to_string(),
        duration_ms: None,
        status: None,
        message: None,
    };
    let result = deliver(cx, step, mode, &envelope, &mut record);
    if let Some(record_step) = &cx.record_step {
        envelope.keep(record_step);
    }
    (Some(record), result)
}

fn deliver(
    cx: &AgentStep<'_>,
    step: &StepDecl,
    mode: RunMode,
    envelope: &Envelope,
    record: &mut AgentRun,
) -> Result<Option<String>, AppError> {
    let prompt = prompt_text(step, cx.package_root, cx.vars)?;
    let facts = ContextFacts {
        package: &cx.plan.package,
        option: &cx.plan.option,
        step: &step.id,
        mode,
        work: cx.work,
        agent_work: cx.agent_work,
        package_root: cx.package_root,
        app_root: cx.app_root,
        features: &cx.plan.features,
        earlier: cx.earlier.clone(),
    };
    let context = envelope.write(&prompt, &envelope::context(envelope, &facts))?;
    if cx.resumed_delegation && envelope.result.is_file() {
        return answer(cx, step, envelope, None, record);
    }
    envelope.clear_answer()?;
    if cx.hooks.hosted {
        let resume = cx
            .hooks
            .resume_command
            .unwrap_or("the same command with --resume");
        let task = envelope::park(
            &cx.outbox,
            &step.id,
            &step.title,
            &prompt,
            &context,
            envelope,
            resume,
        )?;
        record.status = Some(AgentStatus::Parked);
        if let Some(task_progress) = cx.io.task {
            task_progress.detail(format!("agent_parked: task={}", task.display()));
        }
        return Err(AgentError::Parked(Box::new(ParkedStep {
            key: cx.plan.package.key(),
            step: step.id.clone(),
            run_id: cx.run_id.clone(),
            run: envelope.run.display().to_string(),
            task_files: vec![task.display().to_string()],
            resume: resume.into(),
            work: cx.work.display().to_string(),
        }))
        .into());
    }
    let Some(agent) = cx.hooks.selected else {
        return Err(StepError::AgentRequired {
            key: cx.plan.package.key(),
            step: step.id.clone(),
            reason: "no agent is selected for this run".into(),
        }
        .into());
    };
    let started = Instant::now();
    let exit = spawn(cx, step, agent, mode, &prompt, envelope);
    record.duration_ms = Some(u32::try_from(started.elapsed().as_millis()).unwrap_or(u32::MAX));
    let exit = match exit {
        Err(error @ InstallerError::Interrupted { .. }) => return Err(error.into()),
        other => other,
    };
    answer(cx, step, envelope, Some(exit), record)
}

/// Start the agent with the mode's argument vector and the hook
/// environment (`PROP-064#envelope`): attached to the terminal when
/// interactive, streamed and transcribed when unattended.
fn spawn(
    cx: &AgentStep<'_>,
    step: &StepDecl,
    agent: &SelectedAgent,
    mode: RunMode,
    prompt: &str,
    envelope: &Envelope,
) -> Result<(), InstallerError> {
    let invocation = agent.invocation(
        &step.id,
        mode,
        &Placeholders {
            prompt,
            prompt_file: &envelope.prompt,
            context_file: &envelope.context,
            work: cx.agent_work,
        },
    );
    let mut command = Command::new(&invocation.command);
    command
        .args(&invocation.args)
        .current_dir(&invocation.cwd)
        .env("VIBE_HOOK_STEP", &step.id)
        .env("VIBE_HOOK_RUN", &envelope.run)
        .env("VIBE_HOOK_PROMPT_FILE", &envelope.prompt)
        .env("VIBE_HOOK_CONTEXT_FILE", &envelope.context)
        .env("VIBE_WORK", cx.work)
        .env("VIBE_SOURCES", cx.work.join("sources"))
        .env("VIBE_OUT", cx.work.join("out"))
        .env("VIBE_PACKAGE", cx.package_root)
        .envs(cx.requirements.iter().map(|(name, value)| (name, value)))
        .envs(&invocation.env);
    if let Some(root) = cx.app_root {
        command.env("VIBE_APP_ROOT", root);
    }
    if let Some(task) = cx.io.task {
        task.detail(format!(
            "agent_started: agent={} mode={} envelope={}",
            agent.id,
            mode.as_str(),
            envelope.run.display()
        ));
    }
    let what = format!("agent `{}` of step `{}`", agent.id, step.id);
    match mode {
        RunMode::Interactive => run_attached(&mut command, &what, cx.io.interrupt),
        RunMode::Unattended => run_child(
            &mut command,
            &what,
            &ChildIo {
                stdout_log: Some(&envelope.transcript),
                ..cx.io
            },
        ),
    }
}

/// Map the exit status and `result.json` to the step outcome
/// (`PROP-064#answer`); `exit` is absent when a parked step's answer is
/// validated on `--resume`.
fn answer(
    cx: &AgentStep<'_>,
    step: &StepDecl,
    envelope: &Envelope,
    exit: Option<Result<(), InstallerError>>,
    record: &mut AgentRun,
) -> Result<Option<String>, AppError> {
    let mut problems = Vec::new();
    if let Some(Err(error)) = &exit {
        problems.push(error.to_string());
    }
    let message = match envelope.answer() {
        None if exit.is_none() => {
            problems.push("no result.json was written".into());
            None
        }
        None => None,
        Some(Err(reason)) => {
            problems.push(reason);
            None
        }
        Some(Ok(result)) => {
            if result.status == Status::Fail {
                problems.push(match &result.message {
                    Some(message) => format!("it answered fail: {message}"),
                    None => "it answered fail".into(),
                });
            }
            result.message
        }
    };
    let ok = problems.is_empty();
    record.status = Some(if ok {
        AgentStatus::Ok
    } else {
        AgentStatus::Fail
    });
    record.message.clone_from(&message);
    if let Some(task) = cx.io.task {
        task.detail(format!(
            "agent_finished: agent={} status={} duration_ms={}",
            record.agent,
            if ok { "ok" } else { "fail" },
            record.duration_ms.unwrap_or(0)
        ));
    }
    if let Some(notify) = cx.hooks.notify {
        notify(&AgentNotice::Answer {
            step: &step.id,
            ok,
            message: message.as_deref(),
        });
    }
    if ok {
        return Ok(message);
    }
    Err(AgentError::Failed {
        step: step.id.clone(),
        agent: record.agent.clone(),
        reason: problems.join("; "),
        run: envelope.run.display().to_string(),
    }
    .into())
}

/// The runner's side of agent steps: consent before the first step, and
/// the context of one delivery.
impl<'a> Run<'a> {
    /// Show every pending agent prompt and obtain consent before anything
    /// changes on disk (`PROP-064#consent`).
    pub fn admit_agents(&self) -> Result<(), AppError> {
        let vars = self.vars(None);
        let answered = |id: &str| {
            self.delegated.iter().any(|step| step == id)
                && self
                    .work
                    .join("steps")
                    .join(id)
                    .join("result.json")
                    .is_file()
        };
        admit(
            &self.ctx.agent,
            self.plan,
            self.plan
                .steps
                .iter()
                .enumerate()
                .filter(|(index, _)| !self.kept(*index))
                .map(|(_, planned)| planned),
            &answered,
            &self.resolved.root,
            &vars,
            self.attended(),
        )
    }

    /// Deliver one agent step (`PROP-064`): `{work}` is the work directory,
    /// or the placed root for a finish step; `requirements` is the PATH its
    /// required installations put first.
    pub(super) fn agent_step(
        &self,
        planned: &PlannedStep,
        app_root: &Path,
        io: ChildIo<'_>,
        requirements: &[(String, String)],
    ) -> (Option<AgentRun>, Result<Option<String>, AppError>) {
        let step = &planned.step;
        let finish = step.phase == StepPhase::Finish;
        let vars = self.vars(finish.then_some(app_root));
        // Reached steps in the order they ran (a finish step runs after
        // every prepare step, whatever its place in the file).
        let earlier = self
            .state
            .steps
            .iter()
            .filter(|state| state.id != step.id)
            .map(|state| {
                (
                    state.id.clone(),
                    state.kind.clone(),
                    state.outcome.as_str().to_owned(),
                )
            })
            .collect();
        let run_id = self
            .record
            .as_ref()
            .map_or_else(|| self.plan.package.directory_name(), RunRecord::id);
        let cx = AgentStep {
            hooks: &self.ctx.agent,
            plan: self.plan,
            package_root: &self.resolved.root,
            work: &self.work,
            agent_work: if finish { app_root } else { &self.work },
            app_root: finish.then_some(app_root),
            vars: &vars,
            attended: self.attended(),
            earlier,
            io,
            requirements,
            record_step: self.record.as_ref().map(|record| record.step_dir(&step.id)),
            outbox: self
                .ctx
                .settings
                .path()
                .join("agentic")
                .join("outbox")
                .join(&run_id),
            run_id,
            resumed_delegation: self.delegated.contains(&step.id),
        };
        run(&cx, step)
    }
}
