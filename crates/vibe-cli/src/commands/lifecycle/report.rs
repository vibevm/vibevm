//! How one lifecycle invocation reports itself — and how a hosted handoff
//! reaches every rendering from ONE typed value.
//!
//! Human, quiet and JSON all read the same `delegation`: JSON emits it as an
//! additive typed member of the single generated document, human and quiet
//! print exactly one fenced `vibe-agent-tasks` block. Nothing here smuggles a
//! machine fact into a prose notice.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-HANDSHAKE");

use vibe_wire::generated::lifecycle_report::LifecycleDelegation;

use crate::output;

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
