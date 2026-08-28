//! The post-durability callback `vibe install` runs, and its failure family.
//!
//! Split from the adapter because it answers a different question: the adapter
//! decides which phases a verb runs, this one is the single LIFECYCLE stage a
//! direct install performs after its world is already durable.

use std::path::Path;

use anyhow::Result;
use vibe_lifecycle::Phase;

use crate::commands::compile_trace;
use crate::commands::install::{
    InstallDisposition, InstallRunContext, WorldCallbackOutcome, WorldCallbackSummary,
};
use crate::output;

use super::plan::surface_plan;
use super::{CliRunObserver, LifecycleDraft, dispatch, slot, world};

#[cfg(test)]
mod tests;

/// Direct install callback after durability and before its final document.
///
/// This is a LIFECYCLE stage, and its failures have to look like one. The
/// install has already made its world durable by the time this runs; what
/// happens here is world planning, plan surfacing and phase dispatch, and a
/// failure in any of them is a failed phase run — not a failed install, and
/// certainly not a run that did nothing.
///
/// So the slot rows the install already produced (its own, and any an older
/// continuation's resume just serviced) are converted to lifecycle rows BEFORE
/// the first fallible call, and every way out carries them:
///
/// * a dispatch failure already carries its own Lifecycle draft and gains this
///   prefix, once, through [`compile_trace::prepend_lifecycle_rows`];
/// * a planning, surfacing or otherwise generic failure arrives bare and
///   becomes a carried Lifecycle draft here, with the historical silence of a
///   stage that never emitted a document of its own.
///
/// In both cases the original error object, its context chain and its emission
/// policy travel unchanged — only the rows are added.
pub(crate) fn after_direct_install(
    ctx: &output::Context,
    path: &Path,
    disposition: InstallDisposition,
    run: InstallRunContext,
    workspace: &vibe_workspace::Workspace,
) -> Result<WorldCallbackOutcome> {
    // Measured first, from a value already in hand: nothing between here and
    // the failure can make these rows unavailable.
    let prefix: Vec<vibe_wire::generated::lifecycle_report::LifecycleContributionReport> = run
        .lifecycle_reports
        .iter()
        .cloned()
        .map(slot::contribution_report)
        .collect();
    let requested = run.metadata.requested.clone();
    let chain = run.metadata.chain.clone();
    after_direct_install_stage(ctx, path, disposition, run, workspace).map_err(|error| {
        if compile_trace::is_carried(&error) {
            // Already this command's own family, measured at its own site —
            // it only lacked the rows that preceded it.
            return compile_trace::prepend_lifecycle_rows(error, prefix);
        }
        compile_trace::carry(
            compile_trace::RegisteredReportDraft::Lifecycle(Box::new(LifecycleDraft::failed(
                &requested,
                chain,
                Phase::Install.as_str(),
                prefix,
            ))),
            error,
            // The historical policy of these stages exactly: they never
            // emitted a document of their own when tracing was off.
            false,
        )
    })
}

/// The stage body. Every `?` here is what the wrapper above classifies.
fn after_direct_install_stage(
    ctx: &output::Context,
    path: &Path,
    disposition: InstallDisposition,
    run: InstallRunContext,
    workspace: &vibe_workspace::Workspace,
) -> Result<WorldCallbackOutcome> {
    let observer = CliRunObserver::new(ctx);
    let _ = disposition;
    let phases = [Phase::Validate, Phase::Install];
    // The install's OWN workspace — including a `--git` delta it just recorded
    // in memory. Planning from a rediscovery here would collect a world the
    // command did not produce.
    let ritual = world::plan_default_prepared(path, workspace, &phases)?;
    let metadata = run.metadata.clone();
    surface_plan(&observer, &ritual, &metadata, false)?;
    let state_chain = metadata.chain.clone();
    let slot_reports = run.lifecycle_reports;
    // The callback's dispatch reuses the command's ONE lease — shared into
    // the context by Arc, never reacquired here.
    let lease = run.lease.clone();
    let outcome = if let Some(shared) = run.lifecycle_run {
        dispatch::dispatch_plan_with_run(&observer, &ritual, &shared, &metadata)?
    } else {
        dispatch::dispatch_plan(&observer, &ritual, lease, metadata, state_chain)?
    };
    let parked = outcome.parked.map(|(_, delegation)| delegation);
    let contributions = outcome.reports;
    // NOTHING is rendered here. `vibe install` is the outermost command on
    // this path, so its one `cli-install-report` carries these rows and any
    // handoff; emitting a lifecycle report beside it was the second document.
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
