//! The credential-free selected-world prompt resolver.
//!
//! This is the shared half of `AgentBackend::resolve_prompt`, moved out of
//! the CLI so every surface composes the same value: the CLI pairs it with
//! the paid `vibe-llm` completion half, and a hosted surface pairs it with
//! no completion capability at all. The resolver owns **no** configuration,
//! credential, endpoint, model, token path, transport or provider response
//! body — nothing here reads user config or reaches a network, which is what
//! lets it run before the freshness decision, on fresh rows too.
//!
//! Resolution is pinned to the **executing provider instance**. The request
//! carries that provider's own root — the selected dependency slot or the
//! selected workspace node — and the resolver treats it as the self root for
//! exactly that coordinate. Nothing here searches `vibedeps` for a coordinate,
//! so the "freshest installed version" rule can never serve an instance the
//! lock did not select, and a workspace member's prompt can never fall through
//! to the workspace root's colliding document. Cross-package `#embed` targets
//! inside the closure still resolve through the ordinary selected world.

use std::path::PathBuf;

use specmark::spec;
use vibe_spec::{
    DirectiveKind, Directives, FileResolver, FsSectionSource, SectionSource, SelfCoordinate,
    SpecAddress,
};

use super::{PromptRequest, ResolvedPrompt};

/// The closed value that turns one provider-scoped [`PromptRequest`] into
/// [`ResolvedPrompt`] bytes — credential-free, scan-free, selected-world-only.
///
/// The name is the contract: every coordinate this resolver reaches — the
/// prompt's own provider through `provider_root`, and every cross-package
/// `#embed` target through `selected_world` — is an exact, already-selected
/// instance. It is constructed from the canonical workspace root the caller
/// already carries, and that root is used only as the selected world's base,
/// never re-derived or discovered.
///
/// Fails with plain `String` reasons: the lifecycle wraps them in
/// `AgentError::PromptUnresolved`, and the upper surface owns the remediation
/// prose, so this cell carries no error vocabulary of its own.
///
/// ```
/// use std::collections::BTreeMap;
/// use std::fs;
/// use vibe_lifecycle::agent::{PromptRequest, SelectedWorldPromptResolver};
/// use vibe_spec::SelectedPackage;
///
/// let ws = tempfile::tempdir().unwrap();
/// let provider = ws.path().join("vibedeps/org.demo.tools/1.0.0");
/// let doc = provider.join("spec/common/PROMPT-001.md");
/// fs::create_dir_all(doc.parent().unwrap()).unwrap();
/// fs::write(&doc, "# Prompt {#root}\n\nWrite the guide.\n").unwrap();
///
/// let mut selected = BTreeMap::new();
/// selected.insert(
///     ("org.demo".to_string(), "tools".to_string()),
///     SelectedPackage::new("1.0.0", &provider),
/// );
/// let resolver = SelectedWorldPromptResolver::new(ws.path());
/// let request = PromptRequest {
///     address: "spec://org.demo/tools/common/PROMPT-001#root".into(),
///     provider_root: provider,
///     provider_group: "org.demo".into(),
///     provider_name: "tools".into(),
///     selected_world: selected,
/// };
/// let resolved = resolver.resolve(&request).unwrap();
/// assert!(resolved.text.contains("Write the guide."));
/// assert!(resolved.unsupported.is_empty());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#H-AGENT")]
pub struct SelectedWorldPromptResolver {
    /// The canonical workspace root the caller already holds. The base of the
    /// selected world — never located here, and never used to answer a
    /// coordinate the selected map does not name.
    workspace_root: PathBuf,
}

impl SelectedWorldPromptResolver {
    /// Build the resolver from the canonical workspace root the caller
    /// already carries. Carried, never discovered: a resolver built from a
    /// root its caller does not hold could resolve a prompt against a
    /// different snapshot of the tree than the run it serves.
    #[must_use]
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
        }
    }

    /// Resolve one already provider-scoped address inside the exact provider
    /// instance the request names: the addressed section, its recursive
    /// `#embed` closure through the request's selected world, and the
    /// composition directives this handler does not perform. Credential-free.
    ///
    /// The expanded bytes are the value the caller binds into the freshness
    /// fingerprint, so editing an embedded document reruns the execution
    /// exactly as editing the prompt itself does.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#H-AGENT")]
    pub fn resolve(&self, request: &PromptRequest) -> Result<ResolvedPrompt, String> {
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
