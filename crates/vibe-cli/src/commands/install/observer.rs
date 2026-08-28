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
use super::{emit_closure_diff, lane_sizes, report};

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
    pub(crate) const fn new(
        ctx: &'a output::Context,
        lifecycle_output: Option<&'a output::Context>,
    ) -> Self {
        Self {
            ctx,
            lifecycle_output,
            plan_events: CtxObserver(ctx),
        }
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
            InstallNarration::Resolution(resolution) => {
                report::present_resolution(self.ctx, resolution);
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

    fn slot_observer(
        &self,
        metadata: &RunMetadata,
    ) -> Arc<dyn vibe_install::SlotLifecycleObserver> {
        Arc::new(LifecycleSlotObserver::new(
            self.lifecycle_output.unwrap_or(self.ctx),
            metadata.clone(),
        ))
    }
}

/// The narration seam the slot lifecycle calls back into.
///
/// It observes and renders human progress only: the machine document belongs
/// to the outermost command, which folds every row this observer saw into its
/// single report.
pub(crate) struct LifecycleSlotObserver {
    ctx: output::Context,
    metadata: RunMetadata,
}

impl LifecycleSlotObserver {
    pub(crate) fn new(ctx: &output::Context, metadata: RunMetadata) -> Self {
        Self {
            ctx: ctx.clone(),
            metadata,
        }
    }
}

impl vibe_install::SlotLifecycleObserver for LifecycleSlotObserver {
    fn observe(&self, plan: &vibe_install::SlotLifecyclePlan) -> std::result::Result<(), String> {
        crate::commands::lifecycle::surface_slot_plan(&self.ctx, plan, &self.metadata)
            .map_err(|error| error.to_string())
    }

    fn outcome(
        &self,
        report: &vibe_install::SlotLifecycleReport,
    ) -> std::result::Result<(), String> {
        crate::commands::lifecycle::emit_slot_transition_outcome(&self.ctx, &self.metadata, report)
            .map_err(|error| error.to_string())
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
        let global = vibe_core::GlobalRegistryConfig::load()?;
        Ok(RegistryEnvironmentSnapshot {
            embedded_root,
            global,
        })
    }
}
