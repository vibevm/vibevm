//! The client projections' refusals — §6.2's "fails with a capability
//! report", and the projection-shaped section of [`MechanismError`].
//!
//! Split out of the shared enum's own file along the seam the deploy role
//! already established there: these are refusals no OTHER provider can
//! raise, because only an adapter has a *capability matrix* to fall short
//! of. A canonical packaging provider either produces its distributable or
//! refuses its source; an adapter can be handed a perfectly valid source
//! and still have to say "this client cannot represent that".
//!
//! Five variants rather than one string, for the reason the package error
//! cell gives: they are five different REPAIRS. Fix the target's `inputs`;
//! name a canonical plugin instead of something else; rerun the producer
//! whose record is wrong; add the component or stop requesting it; drop the
//! member or choose a client that supports it. A refusal that could not
//! tell them apart would send an operator to the wrong file.
//!
//! [`MechanismError`]: crate::mechanism::MechanismError

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE");

use specmark::spec;
use thiserror::Error;

/// Why a builtin client-projection provider could not project one canonical
/// Agent Plugin for its client.
///
/// ```
/// use vibe_lifecycle::ClientProjectionError;
///
/// let refusal = ClientProjectionError::ComponentMissing {
///     target: "demo-claude".into(),
///     client: "claude",
///     component: "mcp",
///     reason: "the canonical plugin declares no `mcp.json`".into(),
/// };
/// assert!(refusal.to_string().contains("capability"));
/// assert!(refusal.to_string().contains("PROP-054#ONE-MACHINE"));
/// ```
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ClientProjectionError {
    /// §6.3.0.3's "consumes exactly one recorded `agent-plugin` directory
    /// artifact", in the arithmetic direction.
    #[error(
        "[[artifacts.package]] `{target}` declares {found} input(s), but the builtin provider \
         `{provider}` projects exactly one canonical Agent Plugin \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; fix: declare the one \
         `{{ artifact = … }}` input naming the `package:agent-plugin` output this projection \
         adapts — a projection adapts one plugin, and merging two would invent a third)"
    )]
    InputCount {
        target: String,
        provider: &'static str,
        found: usize,
    },

    /// The one declared input is not a RECORDED canonical Agent Plugin.
    ///
    /// The refusal names what the input really is, because that is the
    /// whole information content of the gate: a workspace directory and a
    /// recorded plain `directory` both look exactly like a plugin on disk,
    /// and only the engine's own record can say that one of them is one.
    #[error(
        "[[artifacts.package]] `{target}` projects input `{input}` for `{client}`, which is \
         {found} rather than a recorded `agent-plugin` artifact \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; fix: consume the \
         output of a `package:agent-plugin` target by its artifact id — a client projection adapts \
         a CANONICAL plugin, and no directory becomes one by resembling one)"
    )]
    InputNotAgentPlugin {
        target: String,
        client: &'static str,
        input: String,
        found: String,
    },

    /// The record says `agent-plugin` and the artifact is not a directory.
    ///
    /// Unreachable through this engine's own producers — §6.2's provider
    /// records `shape = "directory"` — and a refusal rather than a
    /// `debug_assert` because a record is a FILE, which a hand or a
    /// half-written run can leave in any state.
    #[error(
        "[[artifacts.package]] `{target}` projects input `{input}`, recorded as an `agent-plugin` \
         whose physical shape is `{shape}` rather than a directory \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY; fix: rerun the \
         `package:agent-plugin` target that produces `{input}` — Agent Plugins 1.0 defines a \
         directory as the package unit, and a record saying otherwise is a broken record)"
    )]
    InputNotDirectory {
        target: String,
        input: String,
        shape: &'static str,
    },

    /// A requested component the canonical plugin does not carry.
    #[error(
        "[[artifacts.package]] `{target}` requests component `{component}` for `{client}`, which \
         this canonical Agent Plugin cannot supply: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; this is the \
         capability report §6.2 requires instead of a silent drop; fix: add the component to the \
         canonical plugin, or stop naming it in `components`)"
    )]
    ComponentMissing {
        target: String,
        client: &'static str,
        component: &'static str,
        reason: String,
    },

    /// A member of the canonical source the selected client cannot express.
    ///
    /// §6.2: "No adapter silently drops an unsupported component: it either
    /// emits an explicit supported subset requested by the manifest or
    /// fails with a capability report." This is that failure, and it is
    /// NEVER a statement that the source is invalid — the member is
    /// perfectly legal portable v1; this client has nowhere to put it.
    #[error(
        "[[artifacts.package]] `{target}` cannot project `{member}` for `{client}`: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; this is the \
         capability report §6.2 requires instead of a silent drop; fix: remove the member from the \
         canonical plugin, or project this plugin for a client whose configuration expresses it — \
         the member itself is valid portable v1)"
    )]
    Unrepresentable {
        target: String,
        client: &'static str,
        member: String,
        reason: String,
    },
}
