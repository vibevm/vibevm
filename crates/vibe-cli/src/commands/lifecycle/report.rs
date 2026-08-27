//! How one lifecycle invocation reports itself — and how a hosted handoff
//! reaches every rendering from ONE typed value.
//!
//! Human, quiet and JSON all read the same `delegation`: JSON emits it as an
//! additive typed member of the single generated document, human and quiet
//! print exactly one fenced `vibe-agent-tasks` block. Nothing here smuggles a
//! machine fact into a prose notice.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-HANDSHAKE");

use anyhow::Result;
use vibe_wire::generated::lifecycle_report::LifecycleDelegation;

use crate::output;

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
