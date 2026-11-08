//! The runner (`PROP-065#execution`, `#failure`, `#resume`): steps one at a
//! time in `<cache>/app-work/<key>/`, `steps.json` rewritten after each.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#execution");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#failure");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#resume");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#display");

use std::fs;
use std::path::{Path, PathBuf};

use vibe_core::manifest::{StepKind, StepOrigin, StepPhase};

use vibe_core::progress::ProgressTask;

use crate::agents::AgentError;
use crate::error::{AppError, InstallerError, StepError};
use crate::index::{AuthoredCommands, StepOutcome};
use crate::installer::{ChildIo, io, source_build};
use crate::operations::{Command, OperationContext};
use crate::resolve::ResolvedPackage;

use super::env::StepPaths;
use super::kinds::{self, StepContext};
use super::plan::{Plan, PlannedStep, work_dir};
use super::resume::{StartPoint, start_point};
use super::runs::{RunOutcome, RunRecord, transfer_facts};
use super::state::{Start, StepState, StepsFile};
use super::template::Vars;
use super::work::{human_bytes, tree_bytes};

mod continuation;
mod environment;
mod projections;

pub(crate) use projections::step_label;
use projections::writes_log;

/// The flags one run applies to every step.
pub(crate) struct RunFlags {
    pub operation: Option<vibe_wire::generated::shared::ApplicationPendingTarget>,
    /// The command the run record names (`PROP-065#runs`).
    pub command: Command,
    pub start: Start,
    pub force: bool,
    /// An update re-running external steps in place: shell steps run their
    /// `update` command when they declare one.
    pub update_in_place: bool,
    /// The installation key an update replaces, named in the run record.
    pub replaces: Option<String>,
    /// What the requirements put first on each step's PATH
    /// (`PROP-065#app-dependencies`).
    pub paths: StepPaths,
}

/// One sequence of steps over one work directory.
pub(crate) struct Run<'a> {
    pub(super) ctx: &'a OperationContext<'a>,
    pub(super) resolved: &'a ResolvedPackage,
    pub(super) plan: &'a Plan,
    pub(super) work: PathBuf,
    pub(super) state: StepsFile,
    /// Steps before this index are kept from the resumed state.
    first: usize,
    /// A resume that starts in the finish phase keeps every prepare step
    /// (a parked finish step, `PROP-064#parking`).
    finish_resume: bool,
    fresh: bool,
    opened: bool,
    force: bool,
    update_in_place: bool,
    command: Command,
    pub(super) record: Option<RunRecord>,
    /// `VIBE_FEATURE_<ID>` and `VIBE_PARAM_<ID>` of every child.
    env: Vec<(String, String)>,
    /// Steps a resumed state recorded `delegated` (`PROP-064#parking`).
    pub(super) delegated: Vec<String>,
    replaces: Option<String>,
    paths: StepPaths,
}

impl<'a> Run<'a> {
    /// Check the resume point and the forced / update variants before
    /// anything changes on disk.
    pub fn new(
        ctx: &'a OperationContext<'a>,
        resolved: &'a ResolvedPackage,
        plan: &'a Plan,
        flags: RunFlags,
    ) -> Result<Self, AppError> {
        let env = environment::prepare(plan)?;
        let work = work_dir(ctx.settings, &plan.package);
        let key = plan.package.key();
        let StartPoint {
            state,
            first,
            finish_resume,
            delegated,
        } = start_point(plan, &work, &flags.start, flags.operation.as_ref())?;
        let kept = |index: usize, planned: &PlannedStep| {
            index < first || (finish_resume && planned.step.phase == StepPhase::Prepare)
        };
        let pending = || {
            plan.steps
                .iter()
                .enumerate()
                .filter(|(index, planned)| planned.run && !kept(*index, planned))
                .map(|(_, planned)| planned)
        };
        if flags.force
            && pending().any(|planned| {
                matches!(
                    planned.step.kind,
                    StepKind::Native {
                        force_arguments: None,
                        ..
                    }
                )
            })
        {
            return Err(AppError::ForceUnsupported {
                key,
                reason: "the native installer declares no force_arguments",
            });
        }
        let shells: Vec<_> = pending()
            .filter_map(|planned| match &planned.step.kind {
                StepKind::Shell { update, .. } if planned.step.origin == StepOrigin::Authored => {
                    Some(update.is_some())
                }
                _ => None,
            })
            .collect();
        if flags.update_in_place && !flags.force && !shells.is_empty() && !shells.contains(&true) {
            return Err(AppError::MissingUpdate { key });
        }
        Ok(Self {
            ctx,
            resolved,
            plan,
            work,
            state,
            first,
            finish_resume,
            fresh: flags.start == Start::Fresh && flags.operation.is_none(),
            opened: false,
            force: flags.force,
            update_in_place: flags.update_in_place,
            command: flags.command,
            record: None,
            env,
            delegated,
            replaces: flags.replaces,
            paths: flags.paths,
        })
    }

    pub(super) fn attended(&self) -> bool {
        self.ctx.chooser.is_some()
    }

    /// Whether a resume starts in the finish phase: every prepare step is
    /// done and the payload is already placed (a parked finish step).
    pub fn resumes_finish(&self) -> bool {
        self.finish_resume
    }

    /// Whether the step at `index` is kept from the resumed state.
    pub(super) fn kept(&self, index: usize) -> bool {
        index < self.first
            || (self.finish_resume && self.plan.steps[index].step.phase == StepPhase::Prepare)
    }

    pub(super) fn vars(&self, app_root: Option<&'a Path>) -> Vars<'a> {
        Vars {
            work: self.work.clone(),
            package: &self.resolved.root,
            vibe_home: self.ctx.settings.path(),
            app_root,
            features: &self.plan.features,
            params: &self.plan.parameters,
        }
    }

    pub fn work(&self) -> &Path {
        &self.work
    }

    pub fn out(&self) -> PathBuf {
        self.work.join("out")
    }

    /// Create (or reopen) the work directory and drop the state of every
    /// step that runs again.
    fn open(&mut self) -> Result<(), AppError> {
        if self.opened {
            return Ok(());
        }
        if self.fresh && self.work.exists() {
            if let Some(notice) = self.ctx.on_notice {
                notice(&format!(
                    "replacing kept work directory {} ({})",
                    self.work.display(),
                    human_bytes(tree_bytes(&self.work))
                ));
            }
            fs::remove_dir_all(&self.work).map_err(|source| io("clearing", &self.work, source))?;
        }
        for directory in ["sources", "out", "inputs"] {
            let path = self.work.join(directory);
            fs::create_dir_all(&path).map_err(|source| io("creating", &path, source))?;
        }
        let again: Vec<&str> = self
            .plan
            .steps
            .iter()
            .enumerate()
            .filter(|(index, _)| !self.kept(*index))
            .map(|(_, planned)| planned.step.id.as_str())
            .collect();
        self.state
            .steps
            .retain(|step| !again.contains(&step.id.as_str()) || self.delegated.contains(&step.id));
        if let Some(operation) = &mut self.state.operation {
            use vibe_wire::generated::shared::ApplicationPendingOperationPhase as P;
            if matches!(operation.phase, P::Prepared | P::Dependencies | P::Removed) {
                operation.phase = P::Running;
            }
        }
        self.opened = true;
        let mut record = RunRecord::open(
            self.ctx.settings,
            &self.plan.package,
            self.command,
            &self.plan.option,
            Some(self.plan),
        )?;
        if let Some(replaced) = &self.replaces {
            record.replaces(replaced);
        }
        self.record = Some(record);
        self.save()
    }

    /// Rewrite `steps.json` in the work directory and the run record.
    fn save(&self) -> Result<(), AppError> {
        if let Some(record) = &self.record {
            record.steps(&self.state);
        }
        self.state.save(&self.work)
    }

    /// Record how the run ended: done without an error, otherwise
    /// interrupted or failed.
    pub fn close(&mut self, error: Option<&AppError>) {
        let outcome = match error {
            None => RunOutcome::Done,
            Some(AppError::Step(StepError::Interrupted { .. }))
            | Some(AppError::Installer(InstallerError::Cancelled(_))) => RunOutcome::Interrupted,
            Some(AppError::Agent(AgentError::Parked(_))) => RunOutcome::Delegated,
            Some(_) => RunOutcome::Failed,
        };
        if let Some(record) = &mut self.record {
            record.close(outcome);
        }
    }

    /// Stop before the next step (or placement) when an interrupt arrived
    /// while no child was running; the step is recorded `interrupted`.
    fn check_interrupt(&mut self, next: Option<&PlannedStep>) -> Result<(), AppError> {
        self.ctx
            .progress
            .checkpoint()
            .map_err(InstallerError::Cancelled)?;
        if !self
            .ctx
            .interrupt
            .is_some_and(|interrupt| interrupt.poll(None))
        {
            return Ok(());
        }
        if let Some(planned) = next {
            self.state.record(StepState {
                id: planned.step.id.clone(),
                kind: planned.step.kind.name().into(),
                outcome: StepOutcome::Interrupted,
                reason: Some("interrupted before it started".into()),
                started: None,
                finished: None,
                uninstall: AuthoredCommands::default(),
                agent: None,
                env: Vec::new(),
                served: Vec::new(),
            });
            self.save()?;
        }
        Err(self.interrupted(next.map(|planned| planned.step.id.clone())))
    }

    fn interrupted(&self, step: Option<String>) -> AppError {
        StepError::Interrupted {
            key: self.plan.package.key(),
            during: match &step {
                Some(id) => format!("during step `{id}`"),
                None => "before the payload was placed".into(),
            },
            step,
            work: self.work.display().to_string(),
        }
        .into()
    }

    /// Run every `prepare` step; `app_root` is the root the installation is
    /// meant to get (`VIBE_APP_ROOT` of shell steps).
    pub fn prepare(&mut self, app_root: &Path) -> Result<(), AppError> {
        self.open()?;
        self.phase(StepPhase::Prepare, app_root)?;
        self.check_interrupt(None)
    }

    /// Move a non-empty `$VIBE_OUT` to `payload` (`PROP-065#result`).
    pub fn take_out(&self, payload: &Path) -> Result<(), AppError> {
        let out = self.out();
        let populated = fs::read_dir(&out).is_ok_and(|mut entries| entries.next().is_some());
        if !populated {
            return Err(StepError::EmptyResult {
                key: self.plan.package.key(),
                path: out.display().to_string(),
            }
            .into());
        }
        Ok(source_build::move_payload(&out, payload)?)
    }

    /// Run every `finish` step against the placed or registered root. A
    /// failing hook step is an error (the caller restores); a failing
    /// authored step is returned for the caller to report after the index
    /// is written.
    pub fn finish(&mut self, app_root: &Path) -> Result<Option<AppError>, AppError> {
        self.open()?;
        self.phase(StepPhase::Finish, app_root)
    }

    fn phase(&mut self, phase: StepPhase, app_root: &Path) -> Result<Option<AppError>, AppError> {
        let plan = self.plan;
        let progress = self.ctx.progress;
        for (index, planned) in plan.steps.iter().enumerate() {
            let step = &planned.step;
            if step.phase != phase {
                continue;
            }
            let task = progress.task(step_label(step));
            if self.kept(index) {
                task.skip("done before --resume");
                continue;
            }
            if let Err(error) = self.check_interrupt(Some(planned)) {
                task.fail("interrupted");
                return match phase {
                    StepPhase::Prepare => Err(error),
                    StepPhase::Finish => Ok(Some(error)),
                };
            }
            let Err(error) = self.step(planned, app_root, &task) else {
                continue;
            };
            let key = plan.package.key();
            let kind = step.kind.name();
            if matches!(
                error,
                AppError::Installer(InstallerError::Interrupted { .. })
            ) {
                task.fail("interrupted");
                let error = self.interrupted(Some(step.id.clone()));
                return match phase {
                    StepPhase::Prepare => Err(error),
                    StepPhase::Finish => Ok(Some(error)),
                };
            }
            if matches!(error, AppError::Agent(AgentError::Parked(_))) {
                task.skip("parked for the hosting agent");
                return match phase {
                    StepPhase::Prepare => Err(error),
                    StepPhase::Finish => Ok(Some(error)),
                };
            }
            // PROP-066 `#row`: a fetch refused before any connection is
            // not a failed step. Nothing ran, nothing was fetched, and
            // `--resume` cannot change the answer — only the row can — so
            // the refusal surfaces as itself, keeping its `remap_denied`
            // code and its refusal exit status instead of being wrapped as
            // `step_failed`.
            if matches!(
                error,
                AppError::Installer(
                    InstallerError::RemapDenied { .. }
                        | InstallerError::ContactRefused(_)
                        | InstallerError::SourcePolicy(_)
                        | InstallerError::Cancelled(_)
                )
            ) {
                task.fail(error.to_string());
                return match phase {
                    StepPhase::Prepare => Err(error),
                    StepPhase::Finish => Ok(Some(error)),
                };
            }
            task.fail(error.to_string());
            if step.optional {
                continue;
            }
            return match (step.origin, phase) {
                (StepOrigin::PreInstall | StepOrigin::PostInstall, _) => Err(AppError::Hook {
                    hook: if step.origin == StepOrigin::PreInstall {
                        "pre_install"
                    } else {
                        "post_install"
                    },
                    key,
                    reason: error.to_string(),
                }),
                (_, StepPhase::Prepare) => Err(StepError::Failed {
                    key,
                    step: step.id.clone(),
                    kind,
                    work: self.work.display().to_string(),
                    source: Box::new(error),
                }
                .into()),
                (_, StepPhase::Finish) => Ok(Some(
                    StepError::FinishFailed {
                        key,
                        step: step.id.clone(),
                        kind,
                        source: Box::new(error),
                    }
                    .into(),
                )),
            };
        }
        Ok(None)
    }

    /// Run or skip one step and record its outcome.
    fn step(
        &mut self,
        planned: &PlannedStep,
        app_root: &Path,
        task: &ProgressTask,
    ) -> Result<(), AppError> {
        let step = &planned.step;
        let now = || chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        let mut state = StepState {
            id: step.id.clone(),
            kind: step.kind.name().into(),
            outcome: StepOutcome::Skipped,
            reason: planned.reason.clone(),
            started: None,
            finished: None,
            uninstall: AuthoredCommands::default(),
            agent: None,
            env: Vec::new(),
            served: Vec::new(),
        };
        if !planned.run {
            task.skip(planned.reason.clone().unwrap_or_default());
            self.state.record(state);
            return self.save();
        }
        state.started = Some(now());
        let settings = self.ctx.settings;
        let cache_root = settings.app_sources_cache();
        let log = self.record.as_ref().map(|record| record.log(&step.id));
        let ctx = self.ctx;
        if let (Some(told), Some(path)) = (ctx.on_step_log, log.as_deref())
            && writes_log(&step.kind)
        {
            told.opening(&step.id, path);
        }
        let io = ChildIo {
            relay: ctx.relay(),
            log: log.as_deref(),
            stdout_log: None,
            task: Some(task),
            interrupt: ctx.interrupt,
            stdin_null: ctx.panel.is_some(),
            panel: ctx.panel,
        };
        if matches!(step.kind, StepKind::Env { .. }) {
            let result = self.env_step(step, app_root);
            if let Ok(applied) = &result {
                state.env = applied.clone();
            }
            return self.settle(
                state,
                result.map(|_| (AuthoredCommands::default(), None)),
                task,
            );
        }
        let env = self.paths.child_env(&self.env, &step.id, ctx.search_path);
        if matches!(step.kind, StepKind::Agent { .. }) {
            // The agent's child sees the PATH a recipe step of this option
            // sees, and nothing more of the step environment
            // (`PROP-065#app-dependencies`, `PROP-064#envelope`).
            let requirements = self.paths.child_env(&[], &step.id, ctx.search_path);
            let (agent, result) = self.agent_step(planned, app_root, io, &requirements);
            state.agent = agent;
            return self.settle(
                state,
                result.map(|message| (AuthoredCommands::default(), message)),
                task,
            );
        }
        let package_key = self.plan.package.key();
        let served = std::cell::RefCell::new(Vec::new());
        let transfer = transfer_facts(self.record.as_ref(), self.command, &package_key, &step.id);
        let cx = StepContext {
            http_client: ctx.http_client,
            work: &self.work,
            package_root: &self.resolved.root,
            cache_root: &cache_root,
            vars: Vars {
                work: self.work.clone(),
                package: &self.resolved.root,
                vibe_home: settings.path(),
                app_root: (step.phase == StepPhase::Finish).then_some(app_root),
                features: &self.plan.features,
                params: &self.plan.parameters,
            },
            platform: &self.plan.platform,
            arch: &self.plan.arch,
            app_root,
            phase: step.phase,
            offline: ctx.offline,
            force: self.force,
            update_in_place: self.update_in_place,
            io,
            env: &env,
            remap: vibe_core::remap::active(),
            package_key: &package_key,
            served: &served,
            transfer: &transfer,
        };
        let result = kinds::run(&cx, step);
        // The candidates that served this step's inputs are part of the
        // step's record whether it went on to succeed or fail
        // (`PROP-066#provenance`).
        state.served = served.borrow().clone();
        self.settle(state, result.map(|uninstall| (uninstall, None)), task)
    }

    /// Record a run step's outcome: done (with an agent's message as the
    /// reason), delegated, interrupted or failed.
    fn settle(
        &mut self,
        mut state: StepState,
        result: Result<(AuthoredCommands, Option<String>), AppError>,
        task: &ProgressTask,
    ) -> Result<(), AppError> {
        let result = match result {
            Err(error @ AppError::Step(StepError::ResumeMismatch { .. })) => return Err(error),
            result => result,
        };
        state.finished =
            Some(chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
        let outcome = match result {
            Ok((uninstall, message)) => {
                task.finish();
                state.outcome = StepOutcome::Done;
                state.uninstall = uninstall;
                state.reason = message;
                Ok(())
            }
            Err(error) => {
                state.outcome = match &error {
                    AppError::Installer(
                        InstallerError::Interrupted { .. } | InstallerError::Cancelled(_),
                    ) => StepOutcome::Interrupted,
                    AppError::Agent(AgentError::Parked(_)) => StepOutcome::Delegated,
                    _ => StepOutcome::Failed,
                };
                state.reason = Some(error.to_string());
                Err(error)
            }
        };
        self.state.record(state);
        self.save()?;
        outcome
    }
}
