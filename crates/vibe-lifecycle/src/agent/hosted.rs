//! The credential-free hosted backend — the named no-spend
//! [`AgentBackend`] an agent-hosted lifecycle surface injects (R7.4 A15c1).
//!
//! A hosted surface resolves prompts exactly like the CLI does — through
//! A14's [`SelectedWorldPromptResolver`] — but pays for nothing: its agent
//! rows run under `RunAgentMode::Agent`, and the engine parks every such
//! row at the hosted handoff *before* paid dispatch. This backend is that
//! posture as a value: one field (the shared resolver), no
//! `[llm]` table, no user configuration, no model, no endpoint, no token
//! path and no transport — nothing a surface could accidentally pay with.
//!
//! `complete` is a named internal canary, not a remediation: reaching it
//! means the engine dispatched a paid call under a backend that has no paid
//! half, which is an invariant break to report, never a configuration
//! problem to fix. That is why the refusal deliberately offers no
//! provider/API configuration advice — a hosted surface cannot pay, and a
//! remediation implying it could would be false.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-HANDSHAKE");

use std::path::PathBuf;

use specmark::spec;

use super::resolver::SelectedWorldPromptResolver;
use super::{AgentBackend, AgentCompletion, AgentRequest, PromptRequest, ResolvedPrompt};

/// The hosted no-spend backend: A14's exact
/// [`SelectedWorldPromptResolver`] for the credential-free half, and a
/// named internal canary for the paid half.
///
/// Stored state is exactly one field — the resolver. No manifest, `[llm]`
/// table, user configuration, provider, model, endpoint, token path or
/// transport rides, so an agent-hosted surface cannot smuggle a paying
/// capability in by injecting this backend.
///
/// ```
/// use std::collections::BTreeMap;
/// use std::fs;
/// use vibe_lifecycle::agent::{HostedAgentBackend, PromptRequest};
/// use vibe_spec::SelectedPackage;
///
/// let ws = tempfile::tempdir().unwrap();
/// let provider = ws.path().join("vibedeps/org.demo.tools/1.0.0");
/// let doc = provider.join("spec/common/PROMPT-001.md");
/// fs::create_dir_all(doc.parent().unwrap()).unwrap();
/// fs::write(&doc, "# Prompt {#root}\n\nWrite the guide.\n").unwrap();
///
/// let backend = HostedAgentBackend::new(ws.path());
/// let request = PromptRequest {
///     address: "spec://org.demo/tools/common/PROMPT-001#root".into(),
///     provider_root: provider,
///     provider_group: "org.demo".into(),
///     provider_name: "tools".into(),
///     selected_world: BTreeMap::new(),
/// };
/// // The credential-free half resolves exactly like the CLI's.
/// assert!(vibe_lifecycle::agent::AgentBackend::resolve_prompt(&backend, &request)
///     .unwrap()
///     .text
///     .contains("Write the guide."));
/// // The paid half refuses, named and typed — it has none.
/// let refusal = vibe_lifecycle::agent::AgentBackend::complete(
///     &backend,
///     &vibe_lifecycle::agent::AgentRequest {
///         key: "org.demo/tools#produce".into(),
///         phase: "create".into(),
///         system: String::new(),
///         user: String::new(),
///     },
/// )
/// .unwrap_err();
/// assert!(refusal.contains("internal"));
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-HANDSHAKE")]
pub struct HostedAgentBackend {
    resolver: SelectedWorldPromptResolver,
}

impl HostedAgentBackend {
    /// Build the hosted backend from the canonical workspace root the
    /// surface already holds — the same carried, never-discovered root the
    /// shared resolver is built from.
    #[must_use]
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            resolver: SelectedWorldPromptResolver::new(workspace_root),
        }
    }
}

/// The one refusal the paid half can ever give. A hosted agent row parks
/// before paid dispatch, so reaching this means an engine invariant broke —
/// the message names that and nothing else (no configuration remediation:
/// a hosted surface has no paying half to configure).
const COMPLETION_CANARY: &str = "internal invariant break: a hosted agent row parks before \
     paid dispatch, so completion must be unreachable — this backend has no paid half; \
     report the invariant break rather than retrying it";

impl AgentBackend for HostedAgentBackend {
    /// The credential-free half: EXACT delegation to the shared
    /// [`SelectedWorldPromptResolver`], byte-for-byte — a hosted prompt and
    /// a CLI prompt resolve identically, so the bytes the freshness
    /// fingerprint binds cannot differ by surface.
    fn resolve_prompt(&self, request: &PromptRequest) -> Result<ResolvedPrompt, String> {
        self.resolver.resolve(request)
    }

    /// The paid half: never performed. Reaching this method is an internal
    /// invariant break (the engine parked the row before dispatch), so the
    /// refusal is a named canary, not a remediation.
    fn complete(&self, _request: &AgentRequest) -> Result<AgentCompletion, String> {
        Err(COMPLETION_CANARY.into())
    }
}

#[cfg(test)]
mod storage_tests {
    use super::HostedAgentBackend;
    use crate::agent::SelectedWorldPromptResolver;

    /// Stored state is EXACTLY the shared resolver — no manifest, `[llm]`,
    /// user-config, provider, model, endpoint, token-path or transport
    /// carrier can grow without this going red. In-crate by necessity: the
    /// exact-field destructure needs the private field.
    #[test]
    fn the_backend_stores_exactly_the_shared_resolver() {
        fn destructure(backend: HostedAgentBackend) -> SelectedWorldPromptResolver {
            let HostedAgentBackend { resolver } = backend;
            resolver
        }
        let _ = destructure;
    }
}
