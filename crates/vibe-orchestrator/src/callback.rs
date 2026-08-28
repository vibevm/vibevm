//! The post-durability stage a direct install runs after its world is durable.
//!
//! Split from the phase adapter because it answers a different question: the
//! adapter decides which phases a verb runs, this one is the single LIFECYCLE
//! stage a direct install performs once its world already exists.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM");

use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use vibe_lifecycle::{AgentBackend, Phase};

use crate::failure::{MeasuredFailure, Measurement, carry, is_carried, prepend_rows};
use crate::install::{InstallRunContext, WorldCallbackOutcome, WorldCallbackSummary};
use crate::plan::surface_plan;
use crate::ports::RunObserver;
use crate::values::contribution_report;
use crate::{dispatch, world};

#[cfg(test)]
#[path = "callback/tests.rs"]
mod tests;

/// The post-durability world stage, and its failure family.
///
/// This is a LIFECYCLE stage, and its failures have to look like one. The
/// install has already made its world durable by the time this runs; what
/// happens here is world planning, plan surfacing and phase dispatch, and a
/// failure in any of them is a failed phase run — not a failed install, and
/// certainly not a run that did nothing.
///
/// So the slot rows the install already produced (its own, and any an older
/// continuation's resume just serviced) are converted to lifecycle rows BEFORE
/// the first fallible call, and every way out carries them: a dispatch failure
/// already carries its own lifecycle measurement and gains this prefix once,
/// while a planning, surfacing or otherwise generic failure arrives bare and
/// becomes a measured lifecycle failure here, with the historical silence of a
/// stage that never emitted a document of its own.
///
/// In both cases the original error object, its context chain and its emission
/// policy travel unchanged — only the rows are added.
///
/// ```no_run
/// use vibe_orchestrator::after_durable_world_stage;
/// # fn call(
/// #     observer: &dyn vibe_orchestrator::ports::RunObserver,
/// #     path: &std::path::Path,
/// #     run: vibe_orchestrator::InstallRunContext,
/// #     workspace: &vibe_workspace::Workspace,
/// #     agent: std::sync::Arc<dyn vibe_lifecycle::AgentBackend>,
/// # ) -> anyhow::Result<()> {
/// let outcome = after_durable_world_stage(observer, path, run, workspace, &agent)?;
/// let _ = outcome.summary.selected_contributions;
/// # Ok(())
/// # }
/// ```
pub fn after_durable_world_stage(
    observer: &dyn RunObserver,
    path: &Path,
    run: InstallRunContext,
    workspace: &vibe_workspace::Workspace,
    agent: &Arc<dyn AgentBackend>,
) -> Result<WorldCallbackOutcome> {
    // Measured first, from a value already in hand: nothing between here and
    // the failure can make these rows unavailable.
    let prefix: Vec<vibe_wire::generated::lifecycle_report::LifecycleContributionReport> = run
        .lifecycle_reports
        .iter()
        .cloned()
        .map(contribution_report)
        .collect();
    let requested = run.metadata.requested.clone();
    let chain = run.metadata.chain.clone();
    stage(observer, path, run, workspace, agent).map_err(|error| {
        if is_carried::<Measurement>(&error) {
            // Already a measured failure, frozen at its own site — it only
            // lacked the rows that preceded it.
            return prepend_rows(error, prefix);
        }
        carry(MeasuredFailure {
            original: error,
            evidence: Measurement::Lifecycle {
                rows: prefix,
                stopped_phase: Phase::Install.as_str().to_string(),
                requested,
                chain,
            },
            // The historical policy of these stages exactly: they never
            // emitted a document of their own when tracing was off.
            emit_machine_failure: false,
        })
    })
}

/// The stage body. Every `?` here is what the wrapper above measures.
fn stage(
    observer: &dyn RunObserver,
    path: &Path,
    run: InstallRunContext,
    workspace: &vibe_workspace::Workspace,
    agent: &Arc<dyn AgentBackend>,
) -> Result<WorldCallbackOutcome> {
    // ---- the agreement gate, before planning and before any rebind --------
    //
    // This is a PUBLIC entry point too: `path`, `workspace`, `run.lease` and
    // `run.metadata.selected` arrive as four independent values, and everything
    // below writes — the plan is collected over the tree, the dispatch begins
    // (or continues) a run whose state store is rooted at the LEASE, and the
    // package-skill pass rebinds under it. A surface that called this with the
    // wrong pair would plan one world and record it against another.
    //
    // The install core proves the same two facts before IT mutates; this stage
    // runs after that mutation, over a workspace the caller may have replaced,
    // so the proof is owed again here rather than inherited.
    run.lease
        .ensure_root(&workspace.root, "at the post-durability world stage")?;
    let observed_selected = workspace
        .node_rel_of(path)
        .map(|rel| rel.as_str().to_string());
    run.lease.ensure_selected(
        &run.metadata.selected,
        observed_selected.as_deref(),
        "at the post-durability world stage",
    )?;
    let phases = [Phase::Validate, Phase::Install];
    // The install's OWN workspace — including a `--git` delta it just recorded
    // in memory. Planning from a rediscovery here would collect a world the
    // command did not produce.
    let ritual = world::plan_default_prepared(path, workspace, &phases)?;
    let metadata = run.metadata.clone();
    surface_plan(observer, &ritual, &metadata, false)?;
    let state_chain = metadata.chain.clone();
    let slot_reports = run.lifecycle_reports;
    // The callback's dispatch reuses the command's ONE lease — shared into
    // the context by Arc, never reacquired here.
    let lease = run.lease.clone();
    let outcome = if let Some(shared) = run.lifecycle_run {
        dispatch::dispatch_plan_with_run(observer, &ritual, &shared, agent, &metadata)?
    } else {
        dispatch::dispatch_plan(observer, &ritual, lease, agent, metadata, state_chain)?
    };
    let parked = outcome.parked.map(|(_, delegation)| delegation);
    let contributions = outcome.reports;
    // NOTHING is rendered here. The outermost command is the one that emits a
    // document, and its report carries these rows and any handoff.
    Ok(WorldCallbackOutcome {
        summary: WorldCallbackSummary {
            selected_contributions: ritual.executions.len() + slot_reports.len(),
            executed_contributions: contributions
                .iter()
                .filter(|row| row.status != "fresh")
                .count()
                + slot_reports.len(),
            successful_contributions: contributions
                .iter()
                .filter(|row| row.status == "ok")
                .count()
                + slot_reports.iter().filter(|row| row.status == "ok").count(),
            fresh_contributions: contributions
                .iter()
                .filter(|row| row.status == "fresh")
                .count(),
            notices: ritual.notices.len(),
        },
        contributions,
        notices: ritual.notices.clone(),
        parked,
    })
}
