//! The CHILD install observation policy, and the lazy embedded-registry root.
//!
//! This is the install half of the two-observer split, and it is deliberately
//! NOT the phase observer. Its stream formula nulls on a SUPPRESSED context
//! (the phase one nulls on `--quiet`), and its machine-failure emission bit is
//! the child context's answer — which is why a composed phase verb's slot
//! failure stays silent while a direct `vibe install --json` narrates the same
//! failure. Merging the two flips that bit.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#H-SCRIPT");

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Context as _;
use vibe_lifecycle::RunMetadata;
use vibe_lifecycle::process::StreamMode;
use vibe_orchestrator::ports::{
    InstallNarration, InstallObserver, RegistryEnvironment, RegistryEnvironmentSnapshot,
};

use crate::output;

use super::events::CtxObserver;
use super::{emit_closure_diff, evidence, lane_sizes, report};

/// The CLI's install narration, over the two contexts it really owns.
pub(crate) struct CliInstallObserver<'a> {
    /// The install's own context — the CHILD one under a phase verb.
    ctx: &'a output::Context,
    /// Where slot-lifecycle narration goes when the install is a phase verb's
    /// prerequisite: the OUTER context, so its rows are still visible while
    /// the install's own summary stays suppressed.
    lifecycle_output: Option<&'a output::Context>,
    plan_events: CtxObserver<'a>,
}

impl<'a> CliInstallObserver<'a> {
    pub(crate) fn new(
        ctx: &'a output::Context,
        lifecycle_output: Option<&'a output::Context>,
    ) -> Self {
        Self {
            ctx,
            lifecycle_output,
            plan_events: CtxObserver::new(ctx),
        }
    }

    pub(crate) fn with_progress(mut self, progress: vibe_core::progress::Progress) -> Self {
        self.plan_events = CtxObserver::new(self.ctx).with_progress(progress);
        self
    }
}

impl InstallObserver for CliInstallObserver<'_> {
    fn stream_mode(&self) -> StreamMode {
        if self.ctx.is_json() {
            StreamMode::Capture
        } else if self.ctx.suppresses_output() {
            StreamMode::Null
        } else {
            StreamMode::Inherit
        }
    }

    fn emit_machine_failure(&self) -> bool {
        self.ctx.is_json() && !self.ctx.suppresses_output()
    }

    fn narrate(&self, event: InstallNarration<'_>) {
        match event {
            InstallNarration::EmptyWorld => self
                .ctx
                .heading("nothing declared — regenerating boot artifacts for the empty world"),
            InstallNarration::FreshLock => {
                self.ctx.heading("vibe.lock is fresh — skipping resolution");
            }
            InstallNarration::Resolution(planned) => {
                report::present_resolution(self.ctx, planned);
            }
            InstallNarration::ClosureDiff {
                old,
                new,
                lanes_before,
                lanes_after,
            } => emit_closure_diff(self.ctx, "install", old, new, lanes_before, lanes_after),
        }
    }

    fn lane_sizes(&self, root: &Path) -> Vec<(String, Option<u64>)> {
        lane_sizes(root)
    }

    fn plan_events(&self) -> &dyn vibe_install::PlanObserver {
        &self.plan_events
    }

    fn progress(&self) -> vibe_core::progress::Progress {
        self.plan_events.progress_handle()
    }

    fn slot_observer(
        &self,
        metadata: &RunMetadata,
    ) -> Arc<dyn vibe_install::SlotLifecycleObserver> {
        Arc::new(
            LifecycleSlotObserver::new(self.lifecycle_output.unwrap_or(self.ctx), metadata.clone())
                .with_panel(self.plan_events.progress_handle())
                .writing_evidence(),
        )
    }
}

/// The narration seam the slot lifecycle calls back into.
///
/// It observes and renders human progress and writes the step evidence of
/// PROP-065 `#ordinary-plan`; the machine document belongs to the outermost
/// command, which folds every row this observer saw into its single report.
///
/// **The panel and the line records are the same rule PROP-065 `#display`
/// states, and only one of them happens.** Where a terminal can draw, each
/// running contribution is a panel row of the shared renderer — the label
/// `<id> (<kind>) <title>`, a spinner and elapsed time while it runs, the
/// renderer's own record once it finishes, its reason once when it is
/// skipped. Where one cannot — redirected output, `--quiet`, `--json`, the
/// machine stream, the progress opt-out — the progress handle is inert and
/// the flat line record is emitted instead. Emitting both would print every
/// contribution twice in exactly the case a person is watching.
pub(crate) struct LifecycleSlotObserver {
    ctx: output::Context,
    metadata: RunMetadata,
    /// The panel this run draws its rows on; inert wherever the flat
    /// behaviour applies.
    progress: vibe_core::progress::Progress,
    /// The open panel row of each running contribution, by execution key.
    rows: std::sync::Mutex<std::collections::BTreeMap<String, vibe_core::progress::ProgressTask>>,
    /// The run's step evidence, absent when no settings home could be
    /// resolved to write it under.
    evidence: Option<evidence::StepEvidence>,
}

impl LifecycleSlotObserver {
    pub(crate) fn new(ctx: &output::Context, metadata: RunMetadata) -> Self {
        Self {
            ctx: ctx.clone(),
            metadata,
            progress: vibe_core::progress::Progress::default(),
            rows: std::sync::Mutex::new(std::collections::BTreeMap::new()),
            evidence: None,
        }
    }

    /// Draw this run's contributions on the given panel.
    pub(crate) fn with_panel(mut self, progress: vibe_core::progress::Progress) -> Self {
        self.progress = progress;
        self
    }

    /// Write the step evidence under the user's settings home.
    ///
    /// A home that cannot be resolved costs the evidence and nothing else:
    /// this is derived state, and an install must not fail over a cache.
    pub(crate) fn writing_evidence(mut self) -> Self {
        self.evidence = crate::commands::app::global::settings_root()
            .ok()
            .map(|settings| evidence::StepEvidence::new(settings.path()));
        self
    }

    /// The panel row's label: `<id> (<kind>) <title>`, which is the row
    /// `#display` prescribes for a step.
    fn label(entry: &vibe_install::SlotLifecyclePlanEntry) -> String {
        let id = entry.id.as_str();
        match entry.title.as_deref() {
            Some(title) => format!("{id} ({}) {title}", entry.handler),
            None => format!("{id} ({})", entry.handler),
        }
    }

    /// Close this contribution's panel row, and say whether there was one —
    /// which is also the answer to "may the flat line record be skipped".
    fn close_row(&self, report: &vibe_install::SlotLifecycleReport) -> bool {
        let Ok(mut rows) = self.rows.lock() else {
            return false;
        };
        let Some(row) = rows.remove(&report.key) else {
            return false;
        };
        match report.status.as_str() {
            "ok" | "fresh" => row.finish(),
            "skip" => row.skip(report.message.clone().unwrap_or_else(|| "skipped".into())),
            status => row.fail(format!(
                "{status}{}",
                report
                    .message
                    .as_deref()
                    .map(|message| format!(" — {message}"))
                    .unwrap_or_default()
            )),
        }
        true
    }
}

impl vibe_install::SlotLifecycleObserver for LifecycleSlotObserver {
    fn observe(&self, plan: &vibe_install::SlotLifecyclePlan) -> std::result::Result<(), String> {
        crate::commands::lifecycle::surface_slot_plan(&self.ctx, plan, &self.metadata)
            .map_err(|error| error.to_string())
    }

    fn started(
        &self,
        entry: &vibe_install::SlotLifecyclePlanEntry,
    ) -> std::result::Result<(), String> {
        if let Some(evidence) = &self.evidence {
            evidence.started(&entry.key);
        }
        let row = self.progress.task(Self::label(entry));
        // An inert handle hands back a task with no id: there is no panel to
        // put a row on, so the flat line records stay the whole narration.
        if row.id().get() != 0
            && let Ok(mut rows) = self.rows.lock()
        {
            rows.insert(entry.key.clone(), row);
        }
        Ok(())
    }

    fn outcome(
        &self,
        report: &vibe_install::SlotLifecycleReport,
    ) -> std::result::Result<(), String> {
        let log = self
            .evidence
            .as_ref()
            .and_then(|evidence| evidence.record(report));
        // `#ordinary-failure`: a failure's warning carries the contribution,
        // its captured output and **its log path**. The first two are
        // already in the row a client reads and in the error the run
        // refuses with; the path is only knowable here, where the record
        // was written.
        if let Some(log) = log
            && report.status != "ok"
            && !self.ctx.is_json()
            && !self.ctx.is_quiet()
        {
            self.ctx
                .step(&format!("log of `{}`: {}", report.key, log.display()));
        }
        if self.close_row(report) {
            return Ok(());
        }
        crate::commands::lifecycle::emit_slot_transition_outcome(&self.ctx, &self.metadata, report)
            .map_err(|error| error.to_string())
    }
}

/// The run ended — however it ended. Sealing the evidence here rather than
/// at a success path is deliberate: a refused, failed or parked install is
/// exactly the one whose rows a reader comes looking for.
impl Drop for LifecycleSlotObserver {
    fn drop(&mut self) {
        if let Some(evidence) = &self.evidence {
            evidence.close();
        }
    }
}

/// The registry environment: SEED, then LOAD, once.
///
/// The closure the command boundary already owned does BOTH halves of the seed
/// — it calls `ensure_default_global_registry` and then discovers a source
/// install's embedded `packages/` root — and only afterwards is the machine
/// global configuration read. That order is the whole point: the core used to
/// load the global config first and ask for the embedded root much later, so on
/// a fresh machine the very first `vibe build` resolved against a registry file
/// its own seed had not written yet, failed, and succeeded on the second run.
///
/// The laziness is still load-bearing on the embedded half: the lookup happens
/// at exactly the point the install phase needs it, so a chain that never
/// installs never pays for it. And it is one-shot by construction — the closure
/// is taken, so a second preparation is a typed internal refusal rather than a
/// second seed.
pub(crate) struct CliRegistryEnvironment<F> {
    prepare: std::cell::RefCell<Option<F>>,
}

impl<F: FnOnce() -> Option<PathBuf>> CliRegistryEnvironment<F> {
    pub(crate) const fn new(prepare: F) -> Self {
        Self {
            prepare: std::cell::RefCell::new(Some(prepare)),
        }
    }
}

impl<F: FnOnce() -> Option<PathBuf>> RegistryEnvironment for CliRegistryEnvironment<F> {
    fn prepare(&self) -> anyhow::Result<RegistryEnvironmentSnapshot> {
        let seed = self
            .prepare
            .borrow_mut()
            .take()
            .context("internal: the registry environment was prepared twice")?;
        // SEED first — this writes the default `~/.vibe/registry.toml` when a
        // fresh machine has none…
        let embedded_root = seed();
        // …and only THEN load, so the load sees whatever the seed just wrote.
        let global =
            vibe_package_source::deferred_user_registry(crate::output::seeds_default_registry());
        Ok(RegistryEnvironmentSnapshot {
            embedded_root,
            global,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(quiet: bool, json: bool) -> output::Context {
        output::Context::from_flags(quiet, json, None, true, crate::cli::AgentModeArg::Cli)
            .with_progress(true, output::ProgressMode::Plain)
    }

    #[test]
    fn ordinary_install_observer_enables_only_human_progress() {
        let human = context(false, false);
        let observer = CliInstallObserver::new(&human, None).with_progress(human.progress());
        assert_ne!(observer.progress().task("human install").id().get(), 0);

        for (quiet, json) in [(true, false), (false, true)] {
            let silent = context(quiet, json);
            let observer = CliInstallObserver::new(&silent, None).with_progress(silent.progress());
            assert_eq!(observer.progress().task("silent install").id().get(), 0);
        }
    }
}
