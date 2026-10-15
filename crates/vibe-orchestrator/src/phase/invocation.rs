//! Complete current-prefix admission before prerequisite installation/effects.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ADMITTED-INVOCATION-ORDER");
use crate::failure::Measurement;
use crate::values::LifecycleValues;
use crate::{RitualPlan, world};
use anyhow::{Context, Result, bail};
use specmark::spec;
use std::path::Path;
use vibe_core::lifecycle::{ExtensionPoint, PhasePoint};
use vibe_core::manifest::{ArtifactBuildTarget, DeclaredEffect, Manifest};
use vibe_lifecycle::{
    AgentBackend, HandlerExecution, InvocationIntent, LifecycleLease, Phase, RunMetadata,
    SelectorSubject,
};

#[derive(Debug, thiserror::Error)]
pub enum InvocationPreparationNeed {
    #[error("preparation-required: current selected world is unavailable ({0})")]
    CurrentWorld(String),
    #[error("preparation-required: selected prerequisite selector scope is unavailable for `{0}`")]
    SelectorScope(String),
    #[error("unsupported-effect-classification: selected mechanism `{0}` has no declared effects")]
    Mechanism(String),
    #[error("creation-required: selected mechanism `{0}` declares product-source effects")]
    MechanismProduction(String),
    #[error(
        "preparation-required: current artifact-target observation requires explicit admitted preparation for `{0}`"
    )]
    ArtifactObservation(String),
}

pub(super) fn prepare(
    root: &Path,
    manifest: &Manifest,
    workspace: &vibe_workspace::Workspace,
    phases: &[Phase],
    metadata: &RunMetadata,
    lease: &LifecycleLease,
    agent: &dyn AgentBackend,
    targets: &[ArtifactBuildTarget],
) -> Result<Option<RitualPlan>> {
    let intent = InvocationIntent::from_requested(&metadata.requested)
        .context("unsupported public invocation spelling")?;
    // Reuse exact selected M01 constraint/continuity preparation. This is a
    // bounded observation at its existing grade, never source authentication.
    let checked = if manifest.requires.packages.is_empty() {
        None
    } else {
        Some(
            vibe_workspace::source_query::SourceQueryService::prepare_host(root)
                .map_err(|error| InvocationPreparationNeed::CurrentWorld(format!("{error:?}")))?,
        )
    };
    let loaded = if intent.admits_product_source() {
        world::capture_admission_metadata(root, workspace)
    } else {
        world::capture_default_prepared(root, workspace)
    }
    .map_err(|error| InvocationPreparationNeed::CurrentWorld(error.to_string()))?;
    if let Some(checked) = &checked {
        for package in &loaded.world.packages {
            // Minting performs the incumbent before/after continuity checks
            // around the retained selected inventory, AFTER collection. This
            // is finite same-host correlation at its original grade, not a
            // content-authentication or hostile-ABA guarantee.
            let inventory = checked
                .retain_source_inventory(&package.group, &package.name)
                .map_err(|error| InvocationPreparationNeed::CurrentWorld(format!("{error:?}")))?;
            let expected: vibe_core::PackageRef = format!(
                "{}:{}/{}@={}",
                package.kind, package.group, package.name, package.version
            )
            .parse()
            .map_err(|error| {
                InvocationPreparationNeed::CurrentWorld(format!(
                    "invalid selected package pin: {error}"
                ))
            })?;
            if inventory.pin() != &expected {
                return Err(InvocationPreparationNeed::CurrentWorld(
                    "selected package pin changed during collection".into(),
                )
                .into());
            }
        }
    }
    if intent.admits_product_source() {
        let candidates = vibe_lifecycle::native::enabled_native_candidates(&loaded.registry);
        if !candidates.is_empty() {
            let borrowed = candidates.iter().collect::<Vec<_>>();
            let groups = vibe_lifecycle::native::project_native_source_groups(
                &borrowed,
                vibe_lifecycle::native::NativePlatform::current()?,
            )?;
            if !groups.is_empty() {
                let key = "build:cargo".parse()?;
                let compiler = vibe_extension_registry::resolve_mechanism(
                    &loaded.mechanisms,
                    &key,
                    None,
                    &loaded.mechanism_routes,
                )?;
                compiler
                    .row()
                    .declaration()
                    .effects
                    .as_ref()
                    .ok_or_else(|| {
                        InvocationPreparationNeed::Mechanism(compiler.row().pin().to_string())
                    })?;
            }
        }
    }
    let mut prefix = Vec::new();
    for view in loaded.registry.exhaustive(SelectorSubject::unscoped()) {
        if !view.row.is_enabled() {
            continue;
        }
        let point = view.row.declaration().point;
        match point {
            ExtensionPoint::Phase(PhasePoint::Default(phase))
                if phases.contains(&phase) && view.selector_matches =>
            {
                prefix.push((phase.to_string(), HandlerExecution::from_row(view.row)));
            }
            ExtensionPoint::Compile(_) if phases.contains(&Phase::Install) => {
                // Do not silently drop a selected producer because a phase
                // projection cannot express its target/subject. Precision
                // remains unavailable rather than guessing a safe empty set.
                if view.row.declaration().applies_to.is_some() {
                    return Err(InvocationPreparationNeed::SelectorScope(
                        view.row.key().to_string(),
                    )
                    .into());
                }
                prefix.push(("install".into(), HandlerExecution::from_row(view.row)));
            }
            _ => {}
        }
    }
    if phases.contains(&Phase::Install) {
        prefix.extend(
            loaded
                .slot_plan
                .executions()
                .cloned()
                .map(|execution| ("install".into(), execution)),
        );
    }
    for target in targets.iter().filter(|_| phases.contains(&Phase::Build)) {
        let selected = vibe_extension_registry::resolve_mechanism(
            &loaded.mechanisms,
            &target.mechanism,
            target.provider.as_ref(),
            &loaded.mechanism_routes,
        )?;
        let row = selected.row();
        let effects = row
            .declaration()
            .effects
            .as_ref()
            .ok_or_else(|| InvocationPreparationNeed::Mechanism(row.pin().to_string()))?;
        if effects.contains(DeclaredEffect::ProductSource) && !intent.admits_product_source() {
            return Err(
                InvocationPreparationNeed::MechanismProduction(row.pin().to_string()).into(),
            );
        }
        if intent == InvocationIntent::Verify {
            // No execute_build_targets freshness/build branch can hide here.
            // The complete target-observation adapter is explicit unavailable
            // until the actual owner supplies a non-effectful current proof.
            return Err(InvocationPreparationNeed::ArtifactObservation(target.id.clone()).into());
        }
    }
    // Admission order follows actual invocation order, not storage order.
    prefix.sort_by_key(|(phase, _)| phases.iter().position(|p| p.as_str() == phase).unwrap_or(1));
    vibe_lifecycle::validate_invocation_prefix(
        lease,
        &loaded.project,
        &loaded.world,
        metadata,
        &prefix,
        agent,
    )?;
    if intent.admits_product_source() {
        return Ok(None);
    }
    Ok(Some(world::plan_from_loaded(loaded, phases)?))
}

/// What one phase run produced.
#[derive(Debug)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
pub enum PhaseOutcome {
    /// The chain ran to completion.
    Completed(LifecycleValues),
    /// The chain parked at a hosted agent row.
    Parked(LifecycleValues),
    /// The chain failed; the surface owns the report family and the exit code.
    Failed {
        /// The rows and phase this run had really measured.
        measurement: Measurement,
        /// The caller's error, unchanged.
        original: anyhow::Error,
        /// Whether the failing site emitted a machine document with tracing off.
        emit_machine_failure: bool,
    },
}

use crate::install::{InstallInputs, InstallPolicy, PreparedSelection};
use crate::ports::{
    ConfirmGate, InstallManifestMutation, InstallObserver, PackageSourceFactory,
    RegistryEnvironment, RunObserver,
};
use std::sync::Arc;
use vibe_core::manifest::TargetOs;
use vibe_wire::generated::lifecycle_report::{LifecycleContributionReport, LifecycleStepReport};
use vibe_wire::generated::shared::Timestamp;
use vibe_workspace::compile_trace::TraceRun;

/// The prepared inputs one phase run owns, plus whatever the clean epoch
/// already contributed.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
pub struct PhaseRun<'a> {
    /// The requested phase.
    pub requested: Phase,
    /// The default phases this run executes, in chain order.
    pub phases: Vec<Phase>,
    /// The complete requested chain, including a clean prefix.
    pub chain: Vec<String>,
    /// The invocation's durable identity and effective posture.
    pub metadata: RunMetadata,
    /// The surface-neutral install inputs of the prerequisite install.
    pub install_args: InstallInputs,
    /// The narrow execution policy of the prerequisite install.
    pub policy: InstallPolicy,
    /// The command's mutation lease, from the prelude epoch. The prerequisite
    /// install and the phase dispatch both consume this one acquisition; the
    /// command boundary holds the owner through the finalised report.
    pub lease: Arc<LifecycleLease>,
    /// The command's ONE selected-world provenance bundle, from the prelude
    /// epoch (or the single post-clean reload): the canonical root, the manifest
    /// snapshot taken at it, and the tree built from THAT snapshot. Proven at
    /// the TOP of the executed region below — including on a validate-only
    /// chain, where dropping it would let a stored parse error be replaced by
    /// whatever a later workspace read happened to notice.
    pub selection: PreparedSelection,
    /// Step rows the clean epoch already produced.
    pub steps: Vec<LifecycleStepReport>,
    /// Contribution rows the clean epoch already produced.
    pub contributions: Vec<LifecycleContributionReport>,
    /// Notices the clean epoch already produced.
    pub notices: Vec<String>,
    /// The OUTER phase observation policy.
    pub observer: &'a dyn RunObserver,
    /// The CHILD install observation policy — never the same object.
    pub install_observer: &'a dyn InstallObserver,
    /// The surface's confirmation policy for the prerequisite install.
    pub confirm_gate: &'a dyn ConfirmGate,
    /// The surface's package-source composition root.
    pub sources: &'a dyn PackageSourceFactory,
    /// Where this run's registry environment is seeded and loaded — once.
    pub environment: &'a dyn RegistryEnvironment,
    /// The surface's own manifest mutation for the prerequisite install. A
    /// phase verb admits no manifest-mutating flag, so it injects the named
    /// no-op — stated in the call graph rather than assumed.
    pub manifest_mutation: &'a dyn InstallManifestMutation,
    /// The ONE agent backend every agent row of this run is served by.
    pub agent: Arc<dyn AgentBackend>,
    /// The surface's compile-trace recorder, borrowed.
    pub trace: Option<&'a TraceRun>,
    /// Host OS observed once by the outer lifecycle command.
    pub target_os: TargetOs,
    /// Everything the COMMAND LAYER resolved for this run's deploy half,
    /// when it resolved any (§7.0.5: "Profile resolution happens ONCE, in
    /// the command layer that owns flags, and travels as data"; §6.3.0.6:
    /// "Home and executable authority are injected"). `None` is a run that
    /// carries no deploy half — a chain that never reaches deploy, or a
    /// project that declares no deploy section — and the deploy fence then
    /// arms nothing at all.
    pub deploy: Option<crate::DeployAuthority>,
    /// The surface's injected instant for the verify comparison.
    ///
    /// This entry point owns the COMPLETE derived chain, so handing it down is
    /// also the permission for the engine-owned verify boundary to fire at all
    /// — the partial install callback deliberately withholds it. Nothing below
    /// a surface reads a clock, so the instant a member records is the one the
    /// command was given.
    pub observed_at: Timestamp,
}
