//! How one lifecycle invocation reports itself — and how a hosted handoff
//! reaches every rendering from ONE typed value.
//!
//! Human, quiet and JSON all read the same `delegation`: JSON emits it as an
//! additive typed member of the single generated document, human and quiet
//! print exactly one fenced `vibe-agent-tasks` block. Nothing here smuggles a
//! machine fact into a prose notice.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-HANDSHAKE");

use anyhow::Result;
use vibe_lifecycle::RunMetadata;
use vibe_wire::generated::lifecycle_report::{
    LifecycleContributionReport, LifecycleDelegation, LifecycleReport, LifecycleStepReport,
};

use crate::output;

pub(super) fn emit_report(
    ctx: &output::Context,
    requested: &str,
    chain: Vec<String>,
    steps: Vec<LifecycleStepReport>,
    contributions: Vec<LifecycleContributionReport>,
    notices: Vec<String>,
    delegation: Option<vibe_lifecycle::Delegation>,
) -> Result<()> {
    let report = LifecycleReport {
        chain,
        command: "lifecycle".to_string(),
        contributions,
        notices,
        ok: true,
        requested: requested.to_string(),
        steps,
        delegation: delegation.map(delegation_member).transpose()?,
        // R3.4: the shared trace member. Construction from a live recorder
        // lands with the command-owner atom; disabled omits it byte-for-byte.
        trace: None,
    };
    if ctx.is_json() {
        // Exactly ONE generated document, carrying the handoff as a typed
        // member — never a second object appended after it, and never a fence.
        // A parked run drops its buffered plan preview so the handoff document
        // stands alone; a completed run flushes the preview FIRST, keeping the
        // report last exactly as before.
        if report.delegation.is_some() {
            ctx.discard_json_plans();
        } else {
            ctx.flush_json_plans()?;
        }
        return ctx.emit_json(&report);
    }
    let fresh = report
        .contributions
        .iter()
        .filter(|row| row.status == "fresh")
        .count();
    let executed = report.contributions.len() - fresh;
    let ok = report
        .contributions
        .iter()
        .filter(|row| row.status == "ok")
        .count();
    let contribution_summary = format!(
        "{} contribution(s) selected, {executed} executed, {ok} ok, {fresh} fresh",
        report.contributions.len(),
    );
    // Quiet still prints the required contract: the handoff is the whole
    // point of the invocation, and a caller that suppressed narration did not
    // ask to lose the tasks it must now perform.
    if ctx.is_quiet() {
        render_handoff(ctx, report.delegation.as_ref());
        ctx.summary(&format!(
            "vibe lifecycle: {requested} {} ({} phases, {contribution_summary}, {} notice(s))",
            completion(report.delegation.is_some()),
            report.steps.len(),
            report.notices.len(),
        ));
        return Ok(());
    }
    ctx.heading(&format!("lifecycle `{requested}`:"));
    for step in &report.steps {
        ctx.step(&format!("{}: {}", step.phase, step.status));
    }
    render_handoff(ctx, report.delegation.as_ref());
    ctx.summary(&format!(
        "vibe lifecycle: {requested} {} ({} phases, {contribution_summary}, {} notice(s))",
        completion(report.delegation.is_some()),
        report.steps.len(),
        report.notices.len(),
    ));
    Ok(())
}

const fn completion(parked: bool) -> &'static str {
    if parked { "parked" } else { "completed" }
}

/// The typed handoff is the SOURCE for every rendering. The engine adapter
/// validates it here, once: a non-empty task list, and every task the exact
/// deterministic path the reported run owns. A machine fact this load-bearing
/// is never smuggled into a prose notice.
pub(super) fn delegation_member(
    delegation: vibe_lifecycle::Delegation,
) -> Result<LifecycleDelegation> {
    if delegation.tasks.is_empty() {
        anyhow::bail!(
            "internal: run `{}` reported a hosted handoff with no task file",
            delegation.run_id
        );
    }
    let home = format!("{}/{}/", vibe_lifecycle::OUTBOX_RELATIVE, delegation.run_id);
    for task in &delegation.tasks {
        let owned = task
            .strip_prefix(&home)
            .is_some_and(|name| !name.contains('/') && name.ends_with(".md"));
        anyhow::ensure!(
            owned,
            "internal: task `{task}` does not live directly under run `{}`",
            delegation.run_id,
        );
    }
    Ok(LifecycleDelegation {
        resume: delegation.resume,
        run_id: delegation.run_id,
        tasks: delegation.tasks,
    })
}

/// Exactly one fenced `vibe-agent-tasks` block, in human AND quiet mode, read
/// from the same typed value the JSON document carries.
pub(super) fn render_handoff(ctx: &output::Context, delegation: Option<&LifecycleDelegation>) {
    let Some(delegation) = delegation else {
        return;
    };
    render_agent_task_fence(
        ctx,
        &delegation.run_id,
        &delegation.tasks,
        &delegation.resume,
    );
}

/// The one place the human/quiet contract is spelled. Both report families —
/// `cli-lifecycle-report` and `cli-install-report` — render through here, so
/// the fence a hosting agent parses cannot drift between them.
pub(crate) fn render_agent_task_fence(
    ctx: &output::Context,
    run_id: &str,
    tasks: &[String],
    resume: &str,
) {
    let mut block = String::from("```vibe-agent-tasks\n");
    block.push_str(&format!("run: {run_id}\n"));
    block.push_str("tasks:\n");
    for task in tasks {
        block.push_str(&format!("  - {task}\n"));
    }
    block.push_str(&format!("resume: {resume}\n"));
    block.push_str("```");
    ctx.summary(&block);
}

/// Validate a typed handoff at the engine adapter, wherever it is rendered:
/// a non-empty task list, and every task the exact deterministic path the
/// reported run owns.
pub(crate) fn check_delegation(delegation: &vibe_lifecycle::Delegation) -> Result<()> {
    delegation_member(delegation.clone()).map(|_| ())
}

/// A failed contribution's machine document. It never carries a handoff:
/// a park is not a failure, and a failure never parked.
pub(crate) fn emit_failure_outcome(
    ctx: &output::Context,
    metadata: &RunMetadata,
    phase: &str,
    contributions: &[LifecycleContributionReport],
) -> Result<()> {
    if !ctx.is_json() {
        return Ok(());
    }
    // A failing run still shows the plan it was executing: the deferral only
    // ever holds documents back until the outcome is known, and a failure is
    // an outcome.
    ctx.flush_json_plans()?;
    ctx.emit_json(&LifecycleReport {
        chain: metadata.chain.clone(),
        command: "lifecycle".into(),
        contributions: contributions.to_vec(),
        notices: Vec::new(),
        ok: false,
        requested: metadata.requested.clone(),
        steps: vec![LifecycleStepReport {
            phase: phase.into(),
            status: "fail".into(),
        }],
        delegation: None,
        trace: None,
    })
}
