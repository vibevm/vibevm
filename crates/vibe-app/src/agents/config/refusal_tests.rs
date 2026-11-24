//! In-memory registry refusals; no settings or account state is touched.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-064#selection");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-064#presets");

use super::*;

#[test]
fn agent_registry_refusals_explain_empty_and_near_miss() {
    let mut config = AgentsConfig::default();
    let error = config.remove("myag").unwrap_err();
    assert_eq!(error.code(), "agent_unknown");
    assert_eq!(
        error.to_string(),
        "agent myag is not defined in agents.toml \
         (violates spec://org.vibevm.core/vibevm/common/PROP-064#selection; \
         fix: `vibe agent list` or `vibe agent presets`): no agent is defined there"
    );
    assert!(config.agent.is_empty());

    for (spelling, expected) in [
        (
            "claude",
            "preset claude is not built in · did you mean claude-code? \
             (violates spec://org.vibevm.core/vibevm/common/PROP-064#agent-definition; \
             fix: name preset `claude-code`)",
        ),
        (
            "qwen",
            "preset qwen is not built in · did you mean qwen-code? \
             (violates spec://org.vibevm.core/vibevm/common/PROP-064#agent-definition; \
             fix: name preset `qwen-code`)",
        ),
    ] {
        let error = config
            .add(
                "myag",
                AgentDefinition {
                    preset: Some(spelling.into()),
                    ..AgentDefinition::default()
                },
            )
            .unwrap_err();
        assert_eq!(error.code(), "agent_definition_invalid");
        assert_eq!(error.to_string(), expected);
        assert!(
            config.agent.is_empty(),
            "a refused {spelling} add persists nothing"
        );
    }

    let known = "claude-code, codex, opencode, qwen-code, gemini, cursor, hermes, pi";
    let error = config.use_agent("nosuch").unwrap_err();
    assert_eq!(error.code(), "agent_unknown");
    assert_eq!(
        error.to_string(),
        format!(
            "agent `nosuch` is neither defined in agents.toml nor a built-in preset; known: {known} \
             (violates spec://org.vibevm.core/vibevm/common/PROP-064#selection; fix: name a listed agent, \
             see `vibe agent list` and `vibe agent presets`)"
        )
    );
    assert!(config.agent.is_empty());
    assert_eq!(config.default, None);

    for spelling in ["unrelated", "claude-extra", "Qwen", "qwen "] {
        let error = config
            .add(
                "myag",
                AgentDefinition {
                    preset: Some(spelling.into()),
                    ..AgentDefinition::default()
                },
            )
            .unwrap_err();
        assert_eq!(error.code(), "agent_definition_invalid");
        assert_eq!(
            error.to_string(),
            format!(
                "agent `myag` cannot be used: preset `{spelling}` is not built in; presets: {known} \
                 (violates spec://org.vibevm.core/vibevm/common/PROP-064#agent-definition; \
                 fix: give the definition a command and both argument templates, or name a preset)"
            )
        );
        assert!(config.agent.is_empty());
    }

    // Similar prose is not a typed hint: only the two exact reasons qualify.
    let reason = "prefix preset claude is not built in · did you mean claude-code?";
    let error = AgentError::Definition {
        name: "myag".into(),
        reason: reason.into(),
    };
    assert_eq!(
        error.to_string(),
        format!(
            "agent `myag` cannot be used: {reason} \
             (violates spec://org.vibevm.core/vibevm/common/PROP-064#agent-definition; \
             fix: give the definition a command and both argument templates, or name a preset)"
        )
    );
}
