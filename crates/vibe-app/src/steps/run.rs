//! The runner (`PROP-065#execution`, `#failure`, `#resume`): steps one at a
//! time in `<cache>/app-work/<key>/`, `steps.json` rewritten after each.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#execution");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#failure");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#resume");

use std::fs;
use std::path::{Path, PathBuf};

use vibe_core::manifest::{StepKind, StepOrigin, StepPhase};

use crate::error::{AppError, StepError};
use crate::index::{AuthoredCommands, StepOutcome, StepRecord};
use crate::installer::{io, source_build};
use crate::operations::OperationContext;
use crate::resolve::ResolvedPackage;

use super::kinds::{self, StepContext};
use super::plan::{Plan, PlannedStep, work_dir};
use super::state::{Start, StepState, StepsFile};
use super::template::Vars;

/// The flags one run applies to every step.
pub(crate) struct RunFlags {
    pub start: Start,
    pub force: bool,
    /// An update re-running external steps in place: shell steps run their
    /// `update` command when they declare one.
    pub update_in_place: bool,
}

/// One sequence of steps over one work directory.
pub(crate) struct Run<'a> {
    ctx: &'a OperationContext<'a>,
    resolved: &'a ResolvedPackage,
    plan: &'a Plan,
    work: PathBuf,
    state: StepsFile,
    /// Steps before this index are kept from the resumed state.
    first: usize,
    fresh: bool,
    opened: bool,
    force: bool,
    update_in_place: bool,
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
        let work = work_dir(ctx.settings, &plan.package);
        let key = plan.package.key();
        let fresh_state = || StepsFile {
            package: plan.package.clone(),
            option: plan.option.clone(),
            features: plan.features.clone(),
            steps: Vec::new(),
        };
        let (state, first) = match &flags.start {
            Start::Fresh => (fresh_state(), 0),
            start => {
                let state = StepsFile::load(&work)?.ok_or_else(|| StepError::NothingToResume {
                    key: key.clone(),
                    work: work.display().to_string(),
                })?;
                let mismatch = if state.package != plan.package {
                    Some(format!("it belongs to {}", state.package.key()))
                } else if state.option != plan.option {
                    Some(format!(
                        "it ran installer `{}`, not `{}`",
                        state.option, plan.option
                    ))
                } else if state.features != plan.features {
                    Some(format!(
                        "it ran with features [{}], not [{}]",
                        state.features.join(", "),
                        plan.features.join(", ")
                    ))
                } else {
                    None
                };
                if let Some(reason) = mismatch {
                    return Err(StepError::ResumeMismatch {
                        work: work.display().to_string(),
                        reason,
                    }
                    .into());
                }
                let first = match start {
                    Start::FromStep(id) => plan
                        .steps
                        .iter()
                        .position(|planned| &planned.step.id == id)
                        .ok_or_else(|| StepError::UnknownStep {
                            option: plan.option.clone(),
                            step: id.clone(),
                            steps: plan
                                .steps
                                .iter()
                                .map(|planned| planned.step.id.as_str())
                                .collect::<Vec<_>>()
                                .join(", "),
                        })?,
                    _ => plan
                        .steps
                        .iter()
                        .position(|planned| {
                            state.get(&planned.step.id).map(|step| step.outcome)
                                != Some(StepOutcome::Done)
                        })
                        .unwrap_or(plan.steps.len()),
                };
                (state, first)
            }
        };
        let pending = || plan.steps.iter().skip(first).filter(|planned| planned.run);
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
            fresh: flags.start == Start::Fresh,
            opened: false,
            force: flags.force,
            update_in_place: flags.update_in_place,
        })
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
            fs::remove_dir_all(&self.work).map_err(|source| io("clearing", &self.work, source))?;
        }
        for directory in ["sources", "out", "inputs"] {
            let path = self.work.join(directory);
            fs::create_dir_all(&path).map_err(|source| io("creating", &path, source))?;
        }
        let again: Vec<&str> = self.plan.steps[self.first..]
            .iter()
            .map(|planned| planned.step.id.as_str())
            .collect();
        self.state
            .steps
            .retain(|step| !again.contains(&step.id.as_str()));
        self.opened = true;
        self.state.save(&self.work)
    }

    /// Run every `prepare` step; `app_root` is the root the installation is
    /// meant to get (`VIBE_APP_ROOT` of shell steps).
    pub fn prepare(&mut self, app_root: &Path) -> Result<(), AppError> {
        self.open()?;
        self.phase(StepPhase::Prepare, app_root).map(|_| ())
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
        for (index, planned) in plan.steps.iter().enumerate() {
            if planned.step.phase != phase || index < self.first {
                continue;
            }
            if let Err(error) = self.step(planned, app_root) {
                let step = &planned.step;
                if step.optional {
                    continue;
                }
                let key = plan.package.key();
                let kind = step.kind.name();
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
        }
        Ok(None)
    }

    /// Run or skip one step and record its outcome.
    fn step(&mut self, planned: &PlannedStep, app_root: &Path) -> Result<(), AppError> {
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
        };
        if !planned.run {
            self.state.record(state);
            return self.state.save(&self.work);
        }
        state.started = Some(now());
        let settings = self.ctx.settings;
        let cache_root = settings.app_sources_cache();
        let cx = StepContext {
            work: &self.work,
            package_root: &self.resolved.root,
            cache_root: &cache_root,
            vars: Vars {
                work: self.work.clone(),
                package: &self.resolved.root,
                vibe_home: settings.path(),
                app_root: (step.phase == StepPhase::Finish).then_some(app_root),
                features: &self.plan.features,
            },
            platform: &self.plan.platform,
            arch: &self.plan.arch,
            app_root,
            phase: step.phase,
            offline: self.ctx.offline,
            json: self.ctx.json,
            force: self.force,
            update_in_place: self.update_in_place,
        };
        let result = kinds::run(&cx, step);
        state.finished = Some(now());
        let outcome = match result {
            Ok(uninstall) => {
                state.outcome = StepOutcome::Done;
                state.uninstall = uninstall;
                Ok(())
            }
            Err(error) => {
                state.outcome = StepOutcome::Failed;
                state.reason = Some(error.to_string());
                Err(error)
            }
        };
        self.state.record(state);
        self.state.save(&self.work)?;
        outcome
    }

    /// The step outcomes the index keeps.
    pub fn records(&self) -> Vec<StepRecord> {
        self.plan
            .steps
            .iter()
            .filter_map(|planned| self.state.get(&planned.step.id))
            .map(|state| StepRecord {
                id: state.id.clone(),
                kind: state.kind.clone(),
                outcome: state.outcome,
                uninstall: state.uninstall.clone(),
            })
            .collect()
    }
}
