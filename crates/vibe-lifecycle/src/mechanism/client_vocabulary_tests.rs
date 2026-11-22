use super::client_projection::client::ProjectionClient;
use super::deploy::plugin::PluginClient;
use super::deploy::skill::client::SkillClient;
use vibe_core::agent_ids::AgentId;

#[test]
fn deploy_clients_match_agent_vocabulary() {
    let words = SkillClient::ALL
        .into_iter()
        .map(SkillClient::as_str)
        .chain(PluginClient::ALL.into_iter().map(PluginClient::as_str))
        .chain(
            [
                ProjectionClient::Claude,
                ProjectionClient::Codex,
                ProjectionClient::OpenCode,
            ]
            .into_iter()
            .map(ProjectionClient::as_str),
        );

    for word in words {
        let id = AgentId::from_harness(word)
            .unwrap_or_else(|| panic!("client word {word:?} has no canonical harness identity"));
        assert_eq!(
            id.row().projection,
            Some(word),
            "client word {word:?} differs from its canonical projection key"
        );
    }
}
