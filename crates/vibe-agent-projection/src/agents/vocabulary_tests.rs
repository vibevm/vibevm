use super::Agent;
use std::collections::BTreeSet;
use vibe_core::agent_ids::AGENTS;

#[test]
fn agent_keys_come_from_vocabulary() {
    for agent in Agent::ALL {
        assert_eq!(Some(agent.as_str()), agent.id().row().projection);
        assert_eq!(Agent::from_id(agent.id()), Some(*agent));
        assert_eq!(Agent::parse_filter(agent.as_str()).unwrap(), vec![*agent]);
    }

    let projection_ids: BTreeSet<_> = Agent::ALL.iter().map(|agent| agent.id()).collect();
    let vocabulary_ids: BTreeSet<_> = AGENTS
        .iter()
        .filter(|row| row.projection.is_some())
        .map(|row| row.id)
        .collect();
    assert_eq!(projection_ids, vocabulary_ids);

    for (key, agent) in [
        ("qwen", Agent::QwenCode),
        ("qwen-code", Agent::QwenCode),
        ("claude-code", Agent::ClaudeCode),
        ("claude-code-desktop", Agent::ClaudeCodeDesktop),
        ("hermes", Agent::Hermes),
    ] {
        assert_eq!(Agent::parse_filter(key).unwrap(), vec![agent]);
    }

    assert_eq!(Agent::parse_filter("all").unwrap(), Agent::ALL.to_vec());
    assert!(Agent::parse_filter("gemini").is_err());
}
