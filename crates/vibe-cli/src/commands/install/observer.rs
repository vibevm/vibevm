//! The CHILD install observation policy, and the lazy embedded-registry root.
//!
//! This is the install half of the two-observer split, and it is deliberately
//! NOT the phase observer. Its requested stream formula nulls on a SUPPRESSED
//! context; ordinary contribution observation captures full logs and suppresses
//! raw terminal output under `--quiet`/`--json`. Its machine-failure emission bit is
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

/// Human-only facts observed during this invocation, never guessed from disk.
#[derive(Clone, Debug, Default)]
pub(super) struct HumanTrailer {
    /// The resolved coordinate of the sole explicit request.
    pub root: Option<String>,
    /// The parent of logs/ from the first successfully recorded step.
    pub evidence_directory: Option<PathBuf>,
    /// Direct install emits this line only after its adapter has completed.
    pub defer: bool,
}

/// Shared only between this run's observer and final renderer.
pub(super) type SharedTrailer = Arc<std::sync::Mutex<HumanTrailer>>;

/// The CLI's install narration, over the two contexts it really owns.
pub(crate) struct CliInstallObserver<'a> {
    /// The install's own context — the CHILD one under a phase verb.
    ctx: &'a output::Context,
    /// Where slot-lifecycle narration goes when the install is a phase verb's
    /// prerequisite: the OUTER context, so its rows are still visible while
    /// the install's own summary stays suppressed.
    lifecycle_output: Option<&'a output::Context>,
    plan_events: CtxObserver<'a>,
    log_mode: Option<crate::commands::app::display::LogMode>,
    trailer: SharedTrailer,
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
            log_mode: None,
            trailer: SharedTrailer::default(),
        }
    }

    /// Share actual run observations with its final draft.
    pub(super) fn trailer(&self) -> SharedTrailer {
        self.trailer.clone()
    }

    pub(crate) fn with_log_mode(mut self, mode: crate::commands::app::display::LogMode) -> Self {
        self.log_mode = Some(mode);
        self
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
                if let Ok(mut trailer) = self.trailer.lock() {
                    trailer.root = planned.single_explicit_root().and_then(|root| {
                        planned
                            .resolution
                            .iter()
                            .find(|row| {
                                root.group.as_ref() == Some(&row.group) && row.name == root.name
                            })
                            .map(|row| format!("{}/{}@{}", row.group, row.name, row.version))
                    });
                }
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
                .with_log_mode(self.log_mode.unwrap_or_else(|| {
                    crate::commands::app::display::log_mode(
                        None,
                        output::env_app_log().as_deref(),
                        self.ctx.has_interactive_progress(),
                    )
                }))
                .with_panel(self.plan_events.progress_handle())
                .writing_evidence()
                .with_trailer(self.trailer.clone()),
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
    rows: std::sync::Mutex<
        std::collections::BTreeMap<String, Arc<vibe_core::progress::ProgressTask>>,
    >,
    /// The run's step evidence, absent when no settings home could be
    /// resolved to write it under.
    evidence: Option<evidence::StepEvidence>,
    /// Author labels by execution key, shared by planned skips and running rows.
    labels: std::sync::Mutex<std::collections::BTreeMap<String, (String, Option<String>)>>,
    log_mode: crate::commands::app::display::LogMode,
    trailer: SharedTrailer,
}

impl LifecycleSlotObserver {
    pub(crate) fn new(ctx: &output::Context, metadata: RunMetadata) -> Self {
        Self {
            ctx: ctx.clone(),
            metadata,
            progress: vibe_core::progress::Progress::default(),
            rows: std::sync::Mutex::new(std::collections::BTreeMap::new()),
            evidence: None,
            labels: std::sync::Mutex::new(std::collections::BTreeMap::new()),
            log_mode: crate::commands::app::display::LogMode::Live,
            trailer: SharedTrailer::default(),
        }
    }

    /// Carry actual evidence writes to this run's final human renderer.
    fn with_trailer(mut self, trailer: SharedTrailer) -> Self {
        self.trailer = trailer;
        self
    }

    fn with_log_mode(mut self, mode: crate::commands::app::display::LogMode) -> Self {
        self.log_mode = mode;
        self
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
            Some(title) => format!("{id} ({})  {title}", entry.handler),
            None => format!("{id} ({})", entry.handler),
        }
    }

    /// Render the post-record outcome only when no panel row owned it.
    #[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#children")]
    fn flat_outcome(
        &self,
        report: &vibe_install::SlotLifecycleReport,
        log: Option<&Path>,
    ) -> std::result::Result<(), String> {
        let labels = self.labels.lock().ok();
        let label = labels
            .as_ref()
            .and_then(|labels| labels.get(&report.key))
            .map(|(id, title)| (id.as_str(), title.as_deref()));
        crate::commands::lifecycle::emit_slot_transition_outcome(
            &self.ctx,
            &self.metadata,
            report,
            label,
            log,
        )
        .map_err(|error| error.to_string())
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
        if let Ok(mut labels) = self.labels.lock() {
            labels.extend(
                plan.entries
                    .iter()
                    .map(|entry| (entry.key.clone(), (entry.id.clone(), entry.title.clone()))),
            );
        }
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
            rows.insert(entry.key.clone(), Arc::new(row));
        }
        Ok(())
    }

    fn output_observer(
        &self,
        entry: &vibe_install::SlotLifecyclePlanEntry,
    ) -> Option<Arc<dyn vibe_lifecycle::process::ProcessOutputObserver>> {
        // These are the handlers executed by this process runner. Agent/native
        // backends retain their own protocol and observation owners.
        if !matches!(entry.handler.as_str(), "script" | "binary") {
            return None;
        }
        let task = self
            .rows
            .lock()
            .ok()
            .and_then(|rows| rows.get(&entry.key).cloned())
            .unwrap_or_else(|| Arc::new(vibe_core::progress::Progress::default().task("output")));
        let evidence = self.evidence.as_ref()?;
        let sink = evidence.stream(entry, task.clone(), self.ctx.clone(), self.log_mode);
        if let Some(path) = evidence.stream_path(&entry.key) {
            self.ctx.note_task_log(task.id(), &path);
        }
        sink
    }

    fn outcome(
        &self,
        report: &vibe_install::SlotLifecycleReport,
    ) -> std::result::Result<(), String> {
        let log = self
            .evidence
            .as_ref()
            .and_then(|evidence| evidence.record(report));
        if let Some(log) = &log
            && let Some(directory) = log.parent().and_then(Path::parent)
            && let Ok(mut trailer) = self.trailer.lock()
            && trailer.evidence_directory.is_none()
        {
            trailer.evidence_directory = Some(directory.to_path_buf());
        }
        if self.close_row(report) {
            return Ok(());
        }
        self.flat_outcome(report, log.as_deref())
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

    /// Capture the post-record flat boundary in a child harness, not a full install.
    #[test]
    fn slot_outcome_flat_record_names_actual_log_once() {
        use std::fs;
        const CHILD: &str = "VIBE_TEST_FLAT_SLOT_EVIDENCE";
        if std::env::var_os(CHILD).is_some() {
            let settings = tempfile::tempdir().unwrap();
            let ctx = context(false, false);
            let metadata = RunMetadata {
                requested: "install".into(),
                chain: vec!["install".into()],
                offline: true,
                assume_yes: true,
                agent_mode: vibe_wire::generated::lifecycle::e1::context::RunAgentMode::Cli,
                force: false,
                trace_compile: false,
                run_id: "flat-test".into(),
                started: "2026-11-24T00:00:00Z".into(),
                selected: ".".into(),
            };
            let mut observer = LifecycleSlotObserver::new(&ctx, metadata);
            // Inject only the scratch evidence root, without mutating process settings.
            observer.evidence = Some(evidence::StepEvidence::new(settings.path()));
            let report = vibe_install::SlotLifecycleReport {
                key: "org.example/hooked#must@slot(org.example/hooked@0.1.0)".into(),
                reference: "fixture".into(),
                slot_target: Some(vibe_wire::generated::lifecycle::e1::context::SlotTarget {
                    group: "org.example".into(),
                    name: "hooked".into(),
                    version: "0.1.0".into(),
                    kind: "tool".into(),
                    root: "/slot".into(),
                }),
                point: "slot:post-install".into(),
                provider: "org.example/hooked".into(),
                handler: "script".into(),
                tier: "dependency".into(),
                version: Some("0.1.0".into()),
                status: "fail".into(),
                flagged: true,
                message: Some("exit 17".into()),
                stdout: Some("synthetic captured output".into()),
                stderr: None,
                stdout_truncated: false,
                stderr_truncated: false,
            };
            observer
                .labels
                .lock()
                .unwrap()
                .insert(report.key.clone(), ("must".into(), Some("Must run".into())));
            assert!(
                !observer.close_row(&report),
                "new observer has no panel row"
            );
            let log = observer.evidence.as_ref().unwrap().record(&report).unwrap();
            assert!(log.is_file());
            observer.flat_outcome(&report, Some(&log)).unwrap();
            let _ = settings.keep(); // Parent-owned temporary HOME removes this child scratch.
            return;
        }
        let scratch = tempfile::tempdir().unwrap();
        let temporary = scratch.path().join("tmp");
        fs::create_dir(&temporary).unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "commands::install::observer::tests::slot_outcome_flat_record_names_actual_log_once", "--nocapture"])
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", scratch.path())
            .env("VIBE_SETTINGS", scratch.path().join("settings"))
            .env("TMPDIR", &temporary)
            .env("TMP", &temporary)
            .env("TEMP", &temporary)
            .env(CHILD, "1")
            .output().unwrap();
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.status.success(), "{combined}");
        let children: Vec<_> = fs::read_dir(&temporary)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.join("cache/install-steps").is_dir())
            .collect();
        assert_eq!(children.len(), 1);
        let versions = children[0].join("cache/install-steps/org.example/hooked/0.1.0");
        let runs: Vec<_> = fs::read_dir(versions)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        assert_eq!(runs.len(), 1);
        let document: vibe_wire::generated::install_steps::InstallSteps =
            serde_json::from_slice(&fs::read(runs[0].join("steps.json")).unwrap()).unwrap();
        assert_eq!(document.contributions.len(), 1);
        let failed = &document.contributions[0];
        assert_eq!(
            failed.outcome,
            vibe_wire::generated::install_steps::Outcome::Fail
        );
        let log = runs[0].join(failed.log.as_ref().unwrap());
        assert!(log.is_file());
        let line = combined
            .lines()
            .find(|line| line.contains("must  Must run · exit 17 · log "))
            .expect(&combined);
        assert!(
            line.starts_with("✗ └ ") || line.starts_with("x ` "),
            "{line}"
        );
        let displayed = line.split_once(" · log ").unwrap().1;
        let actual = log.to_string_lossy();
        let normalized = displayed.strip_prefix("~/").map_or_else(
            || displayed.to_owned(),
            |rest| scratch.path().join(rest).display().to_string(),
        );
        if let Some((head, tail)) = normalized.split_once('…') {
            assert!(
                actual.starts_with(head) && actual.ends_with(tail),
                "{line}: {actual}"
            );
        } else {
            assert_eq!(normalized, actual);
        }
        assert_eq!(combined.matches(displayed).count(), 1, "{combined}");
        assert_eq!(
            combined
                .matches(log.file_name().unwrap().to_str().unwrap())
                .count(),
            1,
            "{combined}"
        );
        assert!(!combined.contains("log of `"), "{combined}");
    }
}
