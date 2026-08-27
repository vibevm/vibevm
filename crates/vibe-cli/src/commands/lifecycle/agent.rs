//! The CLI's agent backend: `vibe-llm` behind the lifecycle's agent seam.
//!
//! This cell owns exactly two things the lifecycle deliberately does not: how
//! a `spec://` prompt address becomes text, and how a validated prompt becomes
//! one provider call. Every rule about what may be produced — the output
//! contract, the strict result parse, the contained atomic write — lives in
//! `vibe_lifecycle::agent`, so a later MCP surface reuses that behaviour
//! instead of copying it.
//!
//! Resolution is pinned to the **executing provider instance**. The request
//! carries that provider's own root — the selected dependency slot or the
//! selected workspace node — and the resolver treats it as the self root for
//! exactly that coordinate. Nothing here searches `vibedeps` for a coordinate,
//! so the "freshest installed version" rule can never serve an instance the
//! lock did not select, and a workspace member's prompt can never fall through
//! to the workspace root's colliding document. Cross-package `#embed` targets
//! inside the closure still resolve through the ordinary selected world.
//!
//! `resolve_prompt` is credential-free by contract: it runs before the
//! freshness decision, on fresh rows too. [`CliAgentBackend::complete`] is the
//! first line that reads user config, resolves a credential or builds a
//! transport.

use std::path::PathBuf;
use std::sync::Arc;

use specmark::spec;
use vibe_core::manifest::LlmSection;
use vibe_core::user_config::UserConfig;
use vibe_lifecycle::agent::{
    AgentBackend, AgentCompletion, AgentRequest, AgentUsage, PromptRequest, ResolvedPrompt,
};
use vibe_llm::{
    ChatInput, ChatMessage, ChatRole, LLMProvider, ReqwestChatTransport, SystemCredentialReader,
    resolve_effective_config,
};
use vibe_spec::{
    DirectiveKind, Directives, FileResolver, FsSectionSource, SectionSource, SelfCoordinate,
    SpecAddress,
};

/// The remediation PROP-054 `##AGENT-CLI` requires: a selected agent
/// contribution never degrades to a silent skip.
const NO_PROVIDER: &str = "no LLM provider is configured for this project; \
     configure user `[llm]` (provider = \"openai-compatible\", model, endpoint) in the vibe \
     settings `config.toml`, or run this lifecycle under an agent host";

#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI")]
pub(crate) struct CliAgentBackend {
    workspace_root: PathBuf,
    project_llm: Option<LlmSection>,
}

impl CliAgentBackend {
    pub(crate) fn new(workspace_root: PathBuf, project_llm: Option<LlmSection>) -> Self {
        Self {
            workspace_root,
            project_llm,
        }
    }

    pub(crate) fn for_plan(plan: &super::world::RitualPlan) -> Self {
        Self::new(plan.workspace_root.clone(), plan.llm.clone())
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
        let address = SpecAddress::parse(&request.address).map_err(|error| error.to_string())?;
        // Self root = the executing provider's own directory; every OTHER
        // coordinate resolves through the lock-selected world and nothing else.
        // The lifecycle has already proven the address names this provider, so
        // the self arm is the one that fires for the prompt itself; the map is
        // what a cross-package `#embed` inside the closure must go through.
        let source = FsSectionSource::new(FileResolver::with_selected_world(
            &request.provider_root,
            &self.workspace_root,
            request.selected_world.clone(),
            SelfCoordinate::new(
                Some(request.provider_group.clone()),
                request.provider_name.clone(),
            ),
        ));
        let text = source.section_text(&address)?;
        // `#embed` is the whole of this handler's composition, and it is
        // recursive: the expanded bytes are what the fingerprint binds, so
        // editing an embedded document reruns the execution exactly as editing
        // the prompt itself does.
        let expanded =
            vibe_spec::expand_embeds(&text, &source).map_err(|error| error.to_string())?;
        if expanded.trim().is_empty() {
            return Err("the addressed prompt document/anchor is empty".into());
        }
        Ok(ResolvedPrompt {
            unsupported: unsupported_composition(&expanded),
            text: expanded,
        })
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

/// Composition directives that survive `#embed` expansion because this handler
/// does not perform them. Scanning the *expanded* text is what makes the scan
/// complete: an embedded document's own `#use` lands in these bytes, and a
/// directive inside a fence or a comment is masked by the same parser the
/// compiler uses, so a prompt that merely *documents* the syntax is not
/// refused for it.
fn unsupported_composition(expanded: &str) -> Vec<String> {
    Directives::parse(expanded)
        .directives
        .iter()
        .filter(|directive| matches!(directive.kind, DirectiveKind::Use | DirectiveKind::Source))
        .map(|directive| format!("{} {}", directive.kind.keyword(), directive.address.raw))
        .collect()
}
