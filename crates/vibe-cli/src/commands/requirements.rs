//! `vibe requirements` — the R7.5 read-only requirements surface
//! (PROP-054 `##FACT-QUERY-CONTRACT` / `##REF-REQUIREMENTS-SURFACES`; R7
//! architecture §6.1).
//!
//! A parser and a renderer, and deliberately nothing else. The ONE
//! constructor of report members is [`vibe_requirements::query`]: this
//! command builds the effective query, resolves the selected node,
//! injects the clock and — only when asked — the specmap relation
//! provider, then prints exactly what comes back. It assembles no report
//! member, joins no source, reads no authored document itself, and never
//! renders a fact's prose. `--json` is the generated root exactly; the
//! human and quiet text are both projections of the shared
//! [`vibe_requirements::text::render`], so the CLI and MCP cannot drift
//! into two different answers to the same question.
//!
//! **Read-only by construction.** Two reads happen here: one
//! `Workspace::discover_selected`, and one `LifecycleStateStore::peek`
//! for the optional evidence join key. Neither writes. Nothing begins,
//! adopts, leases or checkpoints a run, and no phase, sync or
//! materialisation is reachable from this file — a project that has
//! never run a lifecycle phase still has no `.vibe` after this command.
//!
//! **Algorithmic and credential-free.** There is no provider config, no
//! model, no token and no transport on this path; the only "provider"
//! it knows is the read-only relation adapter, injected as a value. The
//! fence cell beside this file pins both properties.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#FACT-QUERY-CONTRACT");

use anyhow::{Context as _, Result};
use vibe_wire::generated::lifecycle_state::LifecycleState;
use vibe_wire::generated::requirements_report::RequirementsReport;

use crate::cli::RequirementsArgs;
use crate::output::Context;

/// Answer one requirements query over the selected project and render it.
pub fn run(ctx: &Context, args: RequirementsArgs) -> Result<()> {
    // The grammar decides FIRST. An unacceptable question — a bare-id
    // prefix, a zero or an over-cap limit — is refused by the wire
    // owner's own validator before a single path is touched, so a
    // refused invocation leaves the project byte-identical, `.vibe`
    // included.
    let query = vibe_requirements::RequirementsQuery::try_new(
        args.address_prefix.as_deref(),
        args.limit,
        args.relations,
    )?;

    // ONE read-only discovery. Its whole job is the canonical selected
    // node root the query answers for — never the raw `--path`, which
    // may be relative, uncanonical, or a member of a larger workspace —
    // plus the workspace root and selected rel the optional lifecycle
    // join needs.
    let selected = vibe_workspace::Workspace::discover_selected(&args.path)
        .with_context(|| format!("resolving the selected node at `{}`", args.path.display()))?;

    let context = vibe_requirements::QueryContext {
        selected_root: selected.selected_root.clone(),
        // The clock is an INPUT: the library never clocks itself, so the
        // surface owns `observed_at` and the same report is reproducible
        // from a fixed one.
        observed_at: chrono::Utc::now(),
        lifecycle_run_id: current_run_id(&selected)?,
    };

    // The one enrichment injection. Without `--relations` the library
    // receives `None` and calls nothing — no config is read and no map
    // is loaded or built; with it, exactly the read-only specmap
    // adapter, which the library calls at most once.
    let specmap = vibe_trace::SpecmapRelationProvider;
    let provider: Option<&dyn vibe_requirements::RelationProvider> = if query.relations() {
        Some(&specmap)
    } else {
        None
    };

    let report = vibe_requirements::query(&query, &context, provider)?;
    emit(ctx, &report)
}

/// The current lifecycle run id, when durable state carries one THIS
/// node authored.
///
/// `peek` is the read-only half of the state store: it opens the
/// workspace root as a capability and bounded-reads one file. No lease
/// is taken, no directory is created and no state is begun or adopted —
/// which is why the join key can be offered without turning a read-only
/// query into a run.
///
/// **Absence and mismatch are answers; a broken read is not.** `Ok(None)`
/// (no state at all) and a header naming another node both legitimately
/// mean "no join key" — nothing was claimed and nothing was lost. `Err`
/// is a different fact: a state file IS present and could not be safely
/// decoded or proved. Degrading that to `None` would hand back a
/// generated report whose absent `lifecycle_run_id` is indistinguishable
/// from an honest absence, while the MCP surface — asking the identical
/// question — refuses. So it refuses here too: the exactness failure is
/// the user's to fix, and no report is emitted over an input the surface
/// could not read.
fn current_run_id(selected: &vibe_workspace::SelectedWorkspace) -> Result<Option<String>> {
    let state =
        vibe_lifecycle::LifecycleStateStore::peek(&selected.workspace.root).with_context(|| {
            format!(
                "reading the durable lifecycle state at `{}` for the read-only requirements \
                 run-id join; repair or remove `{}` and re-run",
                selected.workspace.root.display(),
                vibe_lifecycle::LifecycleStateStore::FILE,
            )
        })?;
    Ok(state.and_then(|state| joined_run_id(&state, selected.selected.as_str())))
}

/// The pure half of the join: the stored run id, carried only when the
/// state's own selected node is the node being answered for.
///
/// A run authored by a sibling member is a different node's evidence.
/// Attaching its id would invite an orchestrator to join two answers
/// that were never about the same thing — the exact confusion a typed
/// join key exists to prevent — so a mismatch, an absent `selected` and
/// an absent `run_id` all answer the same way: no key.
fn joined_run_id(state: &LifecycleState, selected: &str) -> Option<String> {
    (state.run.selected.as_deref() == Some(selected))
        .then(|| state.run.run_id.clone())
        .flatten()
}

/// The three output shapes, all derived from the SAME validated report.
fn emit(ctx: &Context, report: &RequirementsReport) -> Result<()> {
    if ctx.is_json() {
        // The generated root EXACTLY — printed directly rather than
        // wrapped in a vibe envelope (the shape `vibe explain --json`
        // and `vibe query --json` already established), so `--json`
        // output parses as one requirements report and nothing else.
        println!("{}", serde_json::to_string_pretty(report)?);
        return Ok(());
    }
    let text = vibe_requirements::text::render(report);
    if ctx.is_quiet() {
        // Quiet's contract is one line, and the shared projection's own
        // first line IS that summary. Taking it rather than minting one
        // here keeps quiet and human output from drifting.
        ctx.summary(text.lines().next().unwrap_or_default());
        return Ok(());
    }
    print!("{text}");
    Ok(())
}

// The grammar reds, the join's truth table and the surface fence live in
// their own cell: a fence that `include_str!`s this file would otherwise
// match its own needles, and this file keeps its line budget.
#[cfg(test)]
#[path = "requirements_tests.rs"]
mod tests;
