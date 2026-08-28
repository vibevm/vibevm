//! The CLI's agent backend: `vibe-llm` behind the lifecycle's agent seam.
//!
//! This cell owns exactly one thing the lifecycle deliberately does not: how
//! a validated prompt becomes one provider call. Everything about what may be
//! *produced* — the output contract, the strict result parse, the contained
//! atomic write — lives in `vibe_lifecycle::agent`, and so does everything
//! about how a `spec://` prompt address becomes text:
//! [`SelectedWorldPromptResolver`](vibe_lifecycle::agent::SelectedWorldPromptResolver),
//! the credential-free shared resolver both surfaces compose. The CLI backend
//! owns that value plus only its `[llm]` sidecar, so `resolve_prompt` is a
//! thin delegation and a later MCP surface reuses the identical behaviour
//! instead of copying it.
//!
//! Resolution runs before the freshness decision, on fresh rows too, because
//! the shared resolver is credential-free by contract.
//! [`CliAgentBackend::complete`] is the first line that reads user config,
//! resolves a credential or builds a transport.

use std::path::PathBuf;
use std::sync::Arc;

use specmark::spec;
use vibe_core::manifest::LlmSection;
use vibe_core::user_config::UserConfig;
use vibe_lifecycle::agent::{
    AgentBackend, AgentCompletion, AgentRequest, AgentUsage, PromptRequest, ResolvedPrompt,
    SelectedWorldPromptResolver,
};
use vibe_llm::{
    ChatInput, ChatMessage, ChatRole, LLMProvider, ReqwestChatTransport, SystemCredentialReader,
    resolve_effective_config,
};

/// The remediation PROP-054 `##AGENT-CLI` requires: a selected agent
/// contribution never degrades to a silent skip.
const NO_PROVIDER: &str = "no LLM provider is configured for this project; \
     configure user `[llm]` (provider = \"openai-compatible\", model, endpoint) in the vibe \
     settings `config.toml`, or run this lifecycle under an agent host";

#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI")]
pub(crate) struct CliAgentBackend {
    resolver: SelectedWorldPromptResolver,
    project_llm: Option<LlmSection>,
}

impl CliAgentBackend {
    pub(crate) fn new(workspace_root: PathBuf, project_llm: Option<LlmSection>) -> Self {
        Self {
            resolver: SelectedWorldPromptResolver::new(workspace_root),
            project_llm,
        }
    }

    /// Read the layered configuration and build the provider. Called once per
    /// non-fresh agent execution, never at plan time and never at preparation.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI")]
    fn provider(&self) -> Result<Box<dyn LLMProvider>, String> {
        let user = UserConfig::load().map_err(|error| format!("{NO_PROVIDER} ({error})"))?;
        let selected =
            vibe_core::settings::user_config_path().unwrap_or_else(|| PathBuf::from("config.toml"));
        let effective = resolve_effective_config(
            user.llm.as_ref(),
            &selected,
            self.project_llm.as_ref(),
            &SystemCredentialReader,
        )
        .map_err(|error| format!("{NO_PROVIDER}; the configuration present is unusable: {error}"))?
        .ok_or_else(|| NO_PROVIDER.to_string())?;
        let transport = Arc::new(ReqwestChatTransport::new().map_err(|error| error.to_string())?);
        let provider = effective
            .into_provider(transport)
            .map_err(|error| error.to_string())?;
        Ok(Box::new(provider))
    }
}

impl AgentBackend for CliAgentBackend {
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#H-AGENT")]
    fn resolve_prompt(&self, request: &PromptRequest) -> Result<ResolvedPrompt, String> {
        self.resolver.resolve(request)
    }

    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI")]
    fn complete(&self, request: &AgentRequest) -> Result<AgentCompletion, String> {
        let provider = self.provider()?;
        let input = ChatInput::new(vec![
            ChatMessage::new(ChatRole::System, request.system.clone()),
            ChatMessage::new(ChatRole::User, request.user.clone()),
        ])
        .map_err(|error| error.to_string())?;
        // `ProviderError` is already body-free and key-free by construction;
        // rendering it here adds no provider bytes to the lifecycle failure.
        let output = provider.chat(&input).map_err(|error| error.to_string())?;
        Ok(AgentCompletion {
            text: output.content().to_string(),
            usage: output.usage().map(|usage| AgentUsage {
                prompt_tokens: usage.prompt_tokens(),
                completion_tokens: usage.completion_tokens(),
                total_tokens: usage.total_tokens(),
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    /// The agent backend is built from a CARRIED root — never located.
    ///
    /// `install_agent_backend_from` used to call `lease_root(project_root)`,
    /// which ran a whole extra `Workspace::discover` and, when that discovery
    /// failed, swallowed the error and quietly fell back to the selected root.
    /// So a command that had already leased a workspace root could hand its
    /// agent a DIFFERENT root — and prompt resolution is pinned to that root, so
    /// a workspace member's `spec://` address could fall through to the wrong
    /// document with nothing saying so.
    ///
    /// The signature is half the proof: it takes `&Path` and an
    /// `Option<&Manifest>`, so there is no path for it to locate. This scan is
    /// the other half — it reads the production sources and refuses a locator
    /// call anywhere in the construction path.
    ///
    /// The mutation this kills is reintroducing `lease_root(...)` (or a bare
    /// `Workspace::discover`) at any of the four call sites.
    #[test]
    fn no_agent_backend_construction_locates_a_root() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/commands");
        let mut offenders: Vec<String> = Vec::new();
        let mut stack = vec![src];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
                    continue;
                }
                let body = std::fs::read_to_string(&path)
                    .unwrap()
                    .replace(char::from(13), "");
                for (index, line) in body.lines().enumerate() {
                    let trimmed = line.trim_start();
                    // Prose about the defect is exactly what this file carries,
                    // so comments are not the subject.
                    if trimmed.starts_with("//") {
                        continue;
                    }
                    if !line.contains("install_agent_backend") {
                        continue;
                    }
                    // The construction line itself, plus the two that follow it,
                    // are where a locator would be spelled.
                    let window: String = body
                        .lines()
                        .skip(index)
                        .take(4)
                        .collect::<Vec<_>>()
                        .join(" ");
                    for locator in ["lease_root(", "Workspace::discover("] {
                        if window.contains(locator) {
                            offenders.push(format!(
                                "{}:{} builds the agent backend through `{locator}`",
                                path.display(),
                                index + 1,
                            ));
                        }
                    }
                }
            }
        }
        assert!(
            offenders.is_empty(),
            "the agent backend takes the root its caller already holds: {offenders:#?}",
        );
    }
}
