//! The hosted ports one `lifecycle_run` execution goes through — the
//! stdout-safe, credential-free adapters of every surface capability the
//! shared command cell asks for (R7.4 A15c3).
//!
//! Load-bearing invariants, each structural:
//!
//! * **Capture/quiet everywhere.** This surface's stdout IS the JSON-RPC
//!   channel: an `Inherit` stream or an unquiet binary build would splice
//!   handler and `cargo` bytes into the transport and corrupt every later
//!   frame. Every observation adapter answers `StreamMode::Capture` and
//!   suppresses machine-failure emission; rows travel as structured
//!   values, never as printed bytes.
//! * **No payment, by construction.** The package source is the A15a
//!   shared composition under the hosted default options — REAL manifest
//!   dependencies, registry auth inside `vibe-registry` as part of real
//!   resolution — while qualification is the named refusing canary (the
//!   MCP grammar carries no package inputs, so there is nothing to
//!   qualify). The agent backend is A15c1's no-spend hosted backend.
//! * **No environment reads.** Seeding and the embedded root arrive as
//!   decided facts on the [`ServerContext`](crate::ServerContext); this
//!   crate reads no ambient variable and loads no user configuration.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#lifecycle");

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use vibe_lifecycle::RunMetadata;
use vibe_lifecycle::process::StreamMode;
use vibe_orchestrator::RitualPlan;
use vibe_orchestrator::ports::PackageSource;
use vibe_orchestrator::ports::PackageSourceBuild;
use vibe_orchestrator::ports::{
    ConfirmGate, InstallObserver, PackageSourceFactory, RegistryEnvironment,
    RegistryEnvironmentSnapshot, RunObserver,
};
use vibe_wire::generated::lifecycle_report::LifecycleContributionReport;

use crate::ServerContext;

/// The OUTER phase observation policy: capture streams, silence machine
/// failures, and observe nothing — this surface's presentation IS the
/// structured report it returns.
pub(crate) struct HostedRunObserver;

impl RunObserver for HostedRunObserver {
    fn stream_mode(&self) -> StreamMode {
        StreamMode::Capture
    }

    fn binary_quiet(&self) -> bool {
        true
    }

    fn emit_machine_failure(&self) -> bool {
        false
    }

    fn observe_plan(
        &self,
        _plan: &RitualPlan,
        _metadata: &RunMetadata,
        _emit_empty: bool,
    ) -> Result<()> {
        Ok(())
    }

    fn observe_contribution(&self, _report: &LifecycleContributionReport) {}

    /// Unreachable by construction: this surface never composes the clean
    /// epoch (the tool grammar refuses `clean` before anything executes),
    /// so the untracked-clean failure arm has no caller. Refuse as the
    /// internal break it is rather than guessing.
    fn observe_untracked_failure(
        &self,
        _metadata: &RunMetadata,
        _phase: &str,
        _contributions: &[LifecycleContributionReport],
    ) -> Result<()> {
        anyhow::bail!(
            "internal: the hosted lifecycle surface never composes a clean epoch, so an \
             untracked clean failure cannot be observed"
        )
    }
}

/// The CHILD install observation policy: capture, silence, no narration,
/// no plan events, no slot observation — the install's outcome reaches
/// the caller through the same structured report.
pub(crate) struct HostedInstallObserver;

impl InstallObserver for HostedInstallObserver {
    fn stream_mode(&self) -> StreamMode {
        StreamMode::Capture
    }

    fn emit_machine_failure(&self) -> bool {
        false
    }

    fn narrate(&self, _event: vibe_orchestrator::ports::InstallNarration<'_>) {}

    fn lane_sizes(&self, _root: &Path) -> Vec<(String, Option<u64>)> {
        Vec::new()
    }

    fn plan_events(&self) -> &dyn vibe_install::PlanObserver {
        &vibe_install::NullObserver
    }

    fn slot_observer(
        &self,
        _metadata: &RunMetadata,
    ) -> Arc<dyn vibe_install::SlotLifecycleObserver> {
        Arc::new(vibe_install::NoSlotLifecycleObserver)
    }
}

/// Confirmation is pinned, never asked: the tool executes with
/// `assume_yes = true`, and no MCP argument can change that.
pub(crate) struct HostedConfirmGate;

impl ConfirmGate for HostedConfirmGate {
    fn confirm_install(&self, _packages: usize) -> Result<()> {
        Ok(())
    }
}

/// The hosted package-source factory: the A15a shared composition under
/// the DEFAULT hosted options — real resolution for real manifest
/// dependencies — wrapped as the orchestrator source with the named
/// refusing qualifier. The MCP grammar admits no package input, so
/// qualification has nothing to qualify; the canary makes that
/// unreachability loud instead of silent.
pub(crate) struct HostedPackageSourceFactory;

impl PackageSourceFactory for HostedPackageSourceFactory {
    fn build(&self, input: PackageSourceBuild<'_>) -> Result<Box<dyn PackageSource>> {
        let resolver = vibe_package_source::build_install_resolver(
            &vibe_package_source::PackageSourceOptions::default(),
            input.manifest,
            input.embedded_root,
            input.project_root,
            input.global,
            input.offline,
            input.locked,
        )?;
        Ok(Box::new(vibe_package_source::RegistryPackageSource::new(
            resolver,
            Box::new(vibe_package_source::RefusesQualification),
        )))
    }
}

/// The hosted registry environment: seed-then-load, once, entirely from
/// the context's decided facts. When the context permits seeding, the
/// default global registry is attempted BEFORE the machine-global config
/// load (the same order the CLI's closure owns); no ambient environment
/// is read. Seeding is best-effort just like the CLI startup path: an
/// unwritable optional default must not veto a project-local/declared source.
/// The subsequent config load remains fallible inside the executed install
/// phase and therefore becomes a structured lifecycle failure.
pub(crate) struct HostedRegistryEnvironment {
    embedded_root: Option<PathBuf>,
    seed: bool,
}

impl HostedRegistryEnvironment {
    pub(crate) fn new(ctx: &ServerContext) -> Self {
        Self {
            embedded_root: ctx.embedded_registry_root.clone(),
            seed: ctx.seed_default_registry,
        }
    }
}

impl RegistryEnvironment for HostedRegistryEnvironment {
    fn prepare(&self) -> Result<RegistryEnvironmentSnapshot> {
        if self.seed {
            let _ = vibe_core::ensure_default_global_registry();
        }
        Ok(RegistryEnvironmentSnapshot {
            embedded_root: self.embedded_root.clone(),
            global: vibe_core::GlobalRegistryConfig::load()?,
        })
    }
}
