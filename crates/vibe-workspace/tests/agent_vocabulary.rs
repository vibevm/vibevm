use std::collections::BTreeSet;
use vibe_core::agent_ids::AGENTS;
use vibe_specdoc::doc::CondAgent;
use vibe_workspace::system_prompt::HarnessKind;

#[test]
fn harness_kinds_match_agent_vocabulary() {
    let harness_kinds: BTreeSet<&str> = HarnessKind::ALL.iter().map(|kind| kind.as_str()).collect();
    let vocabulary: BTreeSet<&str> = AGENTS.iter().filter_map(|agent| agent.harness).collect();

    assert_eq!(harness_kinds, vocabulary);
}

#[test]
fn specdoc_agents_match_agent_vocabulary() {
    let specdoc_agents: BTreeSet<&str> =
        CondAgent::ALL.iter().map(|agent| agent.as_str()).collect();
    let vocabulary: BTreeSet<&str> = AGENTS.iter().filter_map(|agent| agent.specdoc).collect();

    assert_eq!(specdoc_agents, vocabulary);
}
