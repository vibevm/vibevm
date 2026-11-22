//! The one owner of agent identity and of the key each surface stores.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#AGENT-VOCABULARY");

/// An agent's identity, independent of the keys stored by each surface.
/// ```
/// use vibe_core::agent_ids::AgentId;
/// assert_eq!(AgentId::from_harness("qwen"), Some(AgentId::Qwen));
/// assert_eq!(AgentId::Qwen.row().projection, Some("qwen-code"));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AgentId {
    /// Claude Code.
    Claude,
    /// Claude Code Desktop.
    ClaudeDesktop,
    /// Cursor.
    Cursor,
    /// OpenCode.
    OpenCode,
    /// Codex.
    Codex,
    /// Qwen Code.
    Qwen,
    /// Gemini.
    Gemini,
    /// Hermes.
    Hermes,
    /// Pi.
    Pi,
}

/// The keys each surface stores for one agent.
/// ```
/// use vibe_core::agent_ids::{AgentId, AGENTS};
/// assert_eq!(AGENTS[AgentId::Pi as usize].id, AgentId::Pi);
/// assert!(AgentId::Pi.row().reference_binding);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentRow {
    /// The agent's canonical identity.
    pub id: AgentId,
    /// The launch and AI-reference key.
    pub harness: Option<&'static str>,
    /// The MCP and skill key used by `vibe mcp install --agent`.
    pub projection: Option<&'static str>,
    /// The agents.toml preset name.
    pub preset: Option<&'static str>,
    /// The specdoc `when` agent key.
    pub specdoc: Option<&'static str>,
    /// Whether the agent may keep an AI reference binding.
    pub reference_binding: bool,
}

/// Per-agent surface keys in `AgentId` order.
pub const AGENTS: [AgentRow; 9] = [
    AgentRow {
        id: AgentId::Claude,
        harness: Some("claude"),
        projection: Some("claude"),
        preset: Some("claude-code"),
        specdoc: Some("claude"),
        reference_binding: false,
    },
    AgentRow {
        id: AgentId::ClaudeDesktop,
        harness: None,
        projection: Some("claude-desktop"),
        preset: None,
        specdoc: Some("claude-desktop"),
        reference_binding: false,
    },
    AgentRow {
        id: AgentId::Cursor,
        harness: None,
        projection: Some("cursor"),
        preset: Some("cursor"),
        specdoc: Some("cursor"),
        reference_binding: false,
    },
    AgentRow {
        id: AgentId::OpenCode,
        harness: Some("opencode"),
        projection: Some("opencode"),
        preset: Some("opencode"),
        specdoc: Some("opencode"),
        reference_binding: false,
    },
    AgentRow {
        id: AgentId::Codex,
        harness: Some("codex"),
        projection: Some("codex"),
        preset: Some("codex"),
        specdoc: Some("codex"),
        reference_binding: false,
    },
    AgentRow {
        id: AgentId::Qwen,
        harness: Some("qwen"),
        projection: Some("qwen-code"),
        preset: Some("qwen-code"),
        specdoc: Some("qwen-code"),
        reference_binding: true,
    },
    AgentRow {
        id: AgentId::Gemini,
        harness: None,
        projection: None,
        preset: Some("gemini"),
        specdoc: None,
        reference_binding: false,
    },
    AgentRow {
        id: AgentId::Hermes,
        harness: Some("hermes"),
        projection: Some("hermes"),
        preset: Some("hermes"),
        specdoc: None,
        reference_binding: false,
    },
    AgentRow {
        id: AgentId::Pi,
        harness: Some("pi"),
        projection: Some("pi"),
        preset: Some("pi"),
        specdoc: None,
        reference_binding: true,
    },
];

impl AgentId {
    /// Return this agent's surface-key row.
    pub const fn row(self) -> &'static AgentRow {
        &AGENTS[self as usize]
    }

    /// Find the agent with the given launch and AI-reference key.
    pub fn from_harness(key: &str) -> Option<AgentId> {
        AGENTS
            .iter()
            .find(|row| row.harness == Some(key))
            .map(|row| row.id)
    }

    /// Find the agent with the given MCP and skill projection key.
    pub fn from_projection(key: &str) -> Option<AgentId> {
        AGENTS
            .iter()
            .find(|row| row.projection == Some(key))
            .map(|row| row.id)
    }

    /// Find the agent with the given agents.toml preset name.
    pub fn from_preset(name: &str) -> Option<AgentId> {
        AGENTS
            .iter()
            .find(|row| row.preset == Some(name))
            .map(|row| row.id)
    }
}

#[cfg(test)]
mod tests {
    use super::{AGENTS, AgentId};
    use std::collections::HashSet;

    #[test]
    fn every_surface_key_is_unique_and_round_trips() {
        let mut harnesses = HashSet::new();
        let mut projections = HashSet::new();
        let mut presets = HashSet::new();
        let mut specdocs = HashSet::new();

        for (i, row) in AGENTS.iter().enumerate() {
            assert_eq!(row.id as usize, i);
            assert_eq!(row.id.row(), row);
            if let Some(key) = row.harness {
                assert!(harnesses.insert(key), "duplicate harness key: {key}");
                assert_eq!(AgentId::from_harness(key), Some(row.id));
            }
            if let Some(key) = row.projection {
                assert!(projections.insert(key), "duplicate projection key: {key}");
                assert_eq!(AgentId::from_projection(key), Some(row.id));
            }
            if let Some(key) = row.preset {
                assert!(presets.insert(key), "duplicate preset key: {key}");
                assert_eq!(AgentId::from_preset(key), Some(row.id));
            }
            if let Some(key) = row.specdoc {
                assert!(specdocs.insert(key), "duplicate specdoc key: {key}");
            }
        }

        assert_eq!(AgentId::from_harness("qwen"), Some(AgentId::Qwen));
        assert_eq!(AgentId::from_projection("qwen-code"), Some(AgentId::Qwen));
        assert_eq!(AgentId::from_preset("claude-code"), Some(AgentId::Claude));
        assert_eq!(AgentId::from_harness("gemini"), None);
        assert_eq!(AgentId::from_harness("qwen-code"), None);
        let reference_bindings: HashSet<_> = AGENTS
            .iter()
            .filter(|row| row.reference_binding)
            .map(|row| row.id)
            .collect();
        assert_eq!(
            reference_bindings,
            HashSet::from([AgentId::Pi, AgentId::Qwen])
        );
    }
}
