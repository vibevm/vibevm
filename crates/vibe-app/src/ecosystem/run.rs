//! Running one adapter verb through the existing step engine (PROP-075
//! `#ECOSYSTEM-ADAPTER`): no new step kind, no second runner — the
//! per-step entry point of `PROP-065#kinds` with an ecosystem environment
//! and the configured installation's own PATH.
//!
//! The PATH front is built by the very function a step requirement and a
//! migration side use, so `npm` is **that version's** npm and no script
//! ever knows a version number.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#ECOSYSTEM-ADAPTER");

use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};

use vibe_core::manifest::{EcosystemDecl, EcosystemVerb, Need, StepDecl, StepPhase};
use vibe_wire::generated::application_ecosystem as wire;

use crate::error::{AppError, EcosystemFailure, StepError};
use crate::installer::{ChildIo, TransferFacts, io};
use crate::operations::OperationContext;
use crate::resolve::ResolvedPackage;
use crate::steps::{EcosystemEnv, StepContext, StepPaths, StepVars, run_step};

use super::Target;

/// Everything one adapter run needs that does not change between its
/// adapters.
pub(super) struct Session<'a> {
    pub ctx: &'a OperationContext<'a>,
    /// Which installation, which adapters and which verb; the item list is
    /// per adapter and is handed to [`Session::run_verb`] instead, because
    /// an apply computes a different one for each (`PROP-075#SLOT-EXTRAS`).
    pub target: &'a Target<'a>,
    /// The installation key the verb runs inside, and its version.
    pub key: &'a str,
    pub version: &'a str,
    /// The run's own work directory, always kept.
    pub work: &'a Path,
    /// That version's package tree, where an adapter's scripts live.
    pub package: &'a ResolvedPackage,
    /// That installation's application root.
    pub root: PathBuf,
    /// What goes first on the verb's PATH: that installation's proxy and
    /// payload directories (`PROP-065#app-dependencies`, reused verbatim).
    pub paths: StepPaths,
    pub platform: String,
    pub arch: String,
}

impl Session<'_> {
    /// Run the request's verb on one adapter. A failure is recorded, never
    /// propagated: one adapter's failure does not stop another's, and
    /// nothing of the runtime installation is ever rolled back.
    pub(super) fn run_verb(
        &self,
        adapter: &EcosystemDecl,
        items: &[String],
    ) -> Result<wire::Adapter, AppError> {
        let verb = self.target.verb;
        let mut row = wire::Adapter {
            id: adapter.id.clone(),
            name: adapter.name.clone(),
            outcome: wire::Outcome::Done,
            items: items.to_vec(),
            observed: Vec::new(),
            step: None,
            log: None,
            reason: None,
            retry: None,
            started: Some(now()),
            finished: None,
        };
        let Some(step) = adapter.verb(verb) else {
            return Ok(skipped(
                row,
                format!(
                    "the adapter `{}` declares no `{}` verb",
                    adapter.id,
                    verb.as_str()
                ),
            ));
        };
        if verb.takes_items() && items.is_empty() {
            // "With an empty final list, the adapter is not run at all"
            // (`PROP-075#SLOT-EXTRAS`).
            return Ok(skipped(
                row,
                format!(
                    "no items to {} in the ecosystem `{}`",
                    verb.as_str(),
                    adapter.id
                ),
            ));
        }
        let dir = self.adapter_dir(&adapter.id)?;
        let list = dir.join("items");
        let log = dir.join(format!("{}.log", verb.as_str()));
        row.step = Some(step.id.clone());
        row.log = Some(log.display().to_string());
        match self.run_step(adapter, verb, step, items, &list, &log) {
            Ok(()) => {
                row.outcome = wire::Outcome::Done;
                if verb == EcosystemVerb::List {
                    // The answer is the file, never stdout: so no
                    // ecosystem's listing format is ever parsed and the
                    // answer survives a manager that writes notices.
                    row.observed = crate::migrate::observed(&list);
                }
            }
            Err(failure) => {
                row.outcome = wire::Outcome::Failed;
                row.reason = Some(failure.to_string());
                row.retry = Some(self.target.retry(&adapter.id, items));
            }
        }
        row.finished = Some(now());
        Ok(row)
    }

    /// Where this adapter's log and answer file go:
    /// `<work>/<adapter id>/`. They are kept one level down deliberately —
    /// the **top** of the work directory is the verb's own working
    /// directory and belongs to the package's scripts.
    fn adapter_dir(&self, id: &str) -> Result<PathBuf, AppError> {
        let dir = self.work.join(id);
        fs::create_dir_all(&dir).map_err(|source| io("creating", &dir, source))?;
        Ok(dir)
    }

    /// Whether every `needs` row of an adapter verb is met on this host
    /// (`PROP-065#needs`). An adapter verb fetches nothing of vibe's own,
    /// so `network` is met whenever the invocation is not offline, and
    /// `agent` is never met, because an `agent` step is refused at
    /// validation and nothing here delegates.
    fn unmet_need(&self, step: &StepDecl) -> Option<String> {
        step.needs
            .iter()
            .find(|need| match need {
                Need::Command(name) => !(self.ctx.facts.command_present)(name),
                Need::Agent => true,
                Need::Terminal => self.ctx.chooser.is_none(),
                Need::Network => self.ctx.offline,
            })
            .map(Need::to_string)
    }

    /// One verb, through the ordinary step runner.
    fn run_step(
        &self,
        adapter: &EcosystemDecl,
        verb: EcosystemVerb,
        step: &StepDecl,
        items: &[String],
        list: &Path,
        log: &Path,
    ) -> Result<(), AppError> {
        let ctx = self.ctx;
        if let Some(need) = self.unmet_need(step) {
            // Refused before the child starts, so a verb that cannot run
            // has changed nothing (`PROP-065#needs`).
            return Err(StepError::UnmetNeed {
                key: format!("{}@{}", self.target.slot, self.version),
                step: step.id.clone(),
                need,
            }
            .into());
        }
        if let Some(told) = ctx.on_step_log {
            told.opening(&step.id, log);
        }
        let task = ctx.progress.task(format!(
            "{} · {} `{}`",
            adapter.name,
            verb.as_str(),
            step.id
        ));
        let ecosystem = EcosystemEnv {
            id: &adapter.id,
            items,
            list,
        };
        let served = RefCell::new(Vec::new());
        let transfer = TransferFacts {
            package: self.key.to_owned(),
            step: step.id.clone(),
            batch: None,
            resume: None,
        };
        let env = self.paths.child_env(&[], &step.id, ctx.search_path);
        let cx = StepContext {
            http_client: ctx.http_client,
            work: self.work,
            package_root: &self.package.root,
            cache_root: &ctx.settings.app_sources_cache(),
            vars: StepVars {
                work: self.work.to_path_buf(),
                package: &self.package.root,
                vibe_home: ctx.settings.path(),
                app_root: Some(&self.root),
                features: &[],
                params: &[],
                // An adapter verb expands no `$out` and no `$sources`,
                // exactly as a migration side does not: it produces no
                // payload.
                migration: true,
            },
            platform: &self.platform,
            arch: &self.arch,
            app_root: &self.root,
            phase: StepPhase::Prepare,
            offline: ctx.offline,
            force: false,
            update_in_place: false,
            io: ChildIo {
                relay: ctx.relay(),
                log: Some(log),
                stdout_log: None,
                task: Some(&task),
                interrupt: ctx.interrupt,
                stdin_null: ctx.panel.is_some(),
                panel: ctx.panel,
            },
            env: &env,
            remap: vibe_core::remap::active(),
            package_key: self.key,
            served: &served,
            transfer: &transfer,
            migration: None,
            ecosystem: Some(&ecosystem),
        };
        match run_step(&cx, step) {
            Ok(_) => {
                task.finish();
                Ok(())
            }
            Err(error) => {
                task.fail(error.to_string());
                Err(StepError::EcosystemFailed(Box::new(EcosystemFailure {
                    slot: self.target.slot.to_owned(),
                    ecosystem: adapter.id.clone(),
                    verb: verb.as_str(),
                    step: step.id.clone(),
                    work: self.work.display().to_string(),
                    log: log.display().to_string(),
                    retry: self.target.retry(&adapter.id, items),
                    source: Box::new(error),
                }))
                .into())
            }
        }
    }
}

/// An adapter this run did not execute: it declares no such verb, or the
/// list it would have been handed was empty. Never an error.
fn skipped(mut row: wire::Adapter, reason: String) -> wire::Adapter {
    row.outcome = wire::Outcome::Skipped;
    row.reason = Some(reason);
    row.finished = Some(now());
    row
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}
