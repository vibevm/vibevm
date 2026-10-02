//! Shared terminal selection for global MCP package registrations.

use std::io::{self, IsTerminal};

use anyhow::Result;
use vibe_agent_projection::agents::Agent;

use crate::output;

specmark::scope!(
    "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#REG-GLOBAL-AGENT-SELECTION"
);

pub(crate) fn interactive(ctx: &output::Context) -> bool {
    !ctx.is_json() && !ctx.is_unattended() && io::stdin().is_terminal()
}

pub(crate) fn prompt(
    ctx: &output::Context,
    agents: &[Agent],
    defaults: &[Agent],
) -> Result<Vec<Agent>> {
    let (labels, defaults) = choices(agents, defaults);
    let indices = ctx.suspend_progress(|| {
        super::global_mcp_choices::choose(
            "Select agents:",
            "All detected agents (default)",
            &labels,
            &defaults,
            "select an agent with Space, or pass --agent <name> (or --agent all)",
        )
    })?;
    indices
        .into_iter()
        .map(|index| {
            agents
                .get(index)
                .copied()
                .ok_or_else(|| anyhow::anyhow!("agent selection returned an invalid choice"))
        })
        .collect()
}

pub(crate) fn prompt_uninstall(
    ctx: &output::Context,
    statuses: &[super::mcp::GlobalAgentRegistration],
) -> Result<Vec<Agent>> {
    let (labels, initial) = uninstall_choices(statuses);
    let all: Vec<_> = (0..statuses.len()).collect();
    let indices = ctx.suspend_progress(|| {
        super::global_mcp_choices::choose_with_initial(
            "Select agents to unregister:",
            "All owned agents",
            &labels,
            &all,
            super::global_mcp_choices::InitialSelection::Items(&initial),
            "select an agent with Space, or pass --agent <name> (or --agent all)",
        )
    })?;
    indices
        .into_iter()
        .map(|index| {
            statuses
                .get(index)
                .map(|status| status.agent)
                .ok_or_else(|| anyhow::anyhow!("agent selection returned an invalid choice"))
        })
        .collect()
}

fn uninstall_choices(
    statuses: &[super::mcp::GlobalAgentRegistration],
) -> (Vec<String>, Vec<usize>) {
    let labels = statuses
        .iter()
        .map(|status| {
            let states: Vec<_> = [
                (status.registered, "installed"),
                (status.missing, "missing"),
                (status.changed, "changed"),
                (status.pending, "pending"),
            ]
            .into_iter()
            .filter_map(|(count, label)| (count > 0).then_some(label))
            .collect();
            format!("{} ({})", status.agent.as_str(), states.join(", "))
        })
        .collect();
    let initial = statuses
        .iter()
        .enumerate()
        .filter_map(|(index, status)| (status.registered > 0).then_some(index))
        .collect();
    (labels, initial)
}

/// Parse an explicit agent subset without reading terminal or machine state.
pub(crate) fn parse_explicit_filter(filter: &str) -> Result<Vec<Agent>> {
    let pieces: Vec<_> = filter.split(',').map(str::trim).collect();
    if pieces.iter().any(|piece| piece.is_empty()) {
        anyhow::bail!("--agent requires nonempty comma-separated agent names");
    }
    if pieces.len() > 1 && pieces.contains(&"all") {
        anyhow::bail!("--agent all must appear alone");
    }
    let mut agents = Vec::new();
    for piece in pieces {
        for agent in Agent::parse_filter(piece)? {
            if !agents.contains(&agent) {
                agents.push(agent);
            }
        }
    }
    Ok(agents)
}

fn choices(agents: &[Agent], defaults: &[Agent]) -> (Vec<String>, Vec<usize>) {
    let labels = agents
        .iter()
        .map(|agent| {
            format!(
                "{}{}",
                agent.as_str(),
                if defaults.contains(agent) {
                    " (found)"
                } else {
                    ""
                }
            )
        })
        .collect();
    let defaults = agents
        .iter()
        .enumerate()
        .filter_map(|(index, agent)| defaults.contains(agent).then_some(index))
        .collect();
    (labels, defaults)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[specmark::verifies(
        "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#REG-GLOBAL-UNINSTALL-DEFAULTS"
    )]
    fn uninstall_phantom_receipt_has_no_initial_selection() {
        let statuses = [super::super::mcp::GlobalAgentRegistration {
            agent: Agent::Cursor,
            registered: 0,
            missing: 2,
            changed: 0,
            pending: 0,
        }];
        let (labels, defaults) = uninstall_choices(&statuses);
        assert_eq!(labels, ["cursor (missing)"]);
        assert!(defaults.is_empty());
    }

    #[test]
    #[specmark::verifies(
        "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#REG-GLOBAL-UNINSTALL-DEFAULTS"
    )]
    fn uninstall_defaults_use_only_verified_native_registrations() {
        use super::super::mcp::GlobalAgentRegistration;
        let statuses = [
            GlobalAgentRegistration {
                agent: Agent::ClaudeCode,
                registered: 1,
                missing: 1,
                changed: 0,
                pending: 0,
            },
            GlobalAgentRegistration {
                agent: Agent::Codex,
                registered: 0,
                missing: 0,
                changed: 1,
                pending: 1,
            },
        ];
        let (labels, defaults) = uninstall_choices(&statuses);
        assert_eq!(defaults, [0]);
        assert_eq!(
            labels,
            ["claude (installed, missing)", "codex (changed, pending)"]
        );
    }

    #[test]
    #[specmark::verifies(
        "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#REG-GLOBAL-AGENT-SELECTION"
    )]
    fn checkbox_agent_defaults_include_only_found_agents() {
        let (labels, defaults) = choices(Agent::ALL, &[Agent::Codex, Agent::ClaudeCode]);
        assert_eq!(defaults, [0, 4]);
        assert_eq!(labels[0], "claude (found)");
        assert_eq!(labels[4], "codex (found)");
        assert_eq!(labels[2], "cursor");
    }

    #[test]
    fn explicit_agent_subset_trims_deduplicates_and_validates_all_pieces() {
        assert_eq!(
            parse_explicit_filter(" codex,claude,codex ").unwrap(),
            [Agent::Codex, Agent::ClaudeCode]
        );
        assert_eq!(parse_explicit_filter(" all ").unwrap(), Agent::ALL);
        for filter in ["", "codex,", ",codex", "codex,unknown", "all,codex"] {
            assert!(parse_explicit_filter(filter).is_err(), "{filter}");
        }
    }

    #[test]
    fn checkbox_agent_empty_detection_retains_manual_choices() {
        let (labels, defaults) = choices(Agent::ALL, &[]);
        assert!(defaults.is_empty());
        assert_eq!(labels.len(), Agent::ALL.len());
        assert!(labels.iter().all(|label| !label.contains("found")));
    }
}
