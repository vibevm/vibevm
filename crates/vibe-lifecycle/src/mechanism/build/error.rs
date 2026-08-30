//! The build-phase engine's one error enum — the layer above the
//! provider's.
//!
//! The split is by *who is wrong*. A [`MechanismError`] says the target,
//! its config or the toolchain is wrong; a [`BuildError`] says the graph,
//! the routing or the engine's own record keeping is. Folding them into
//! one enum would put "your `select` matched nothing" beside "this world
//! installs no provider for `build:cargo`", and a reader repairing one
//! would have to guess which surface the other belongs to.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE");

use specmark::spec;
use thiserror::Error;
use vibe_extension_registry::MechanismResolutionError;

use crate::mechanism::MechanismError;

/// Why the build phase could not execute one declared target.
///
/// ```
/// use vibe_lifecycle::BuildError;
///
/// let refusal = BuildError::TransportNotLanded {
///     key: "build:cargo".into(),
///     pin: "org.example/build-tools#cargo-v2".into(),
///     kind: "native".into(),
/// };
/// assert!(refusal.to_string().contains("not yet landed"));
/// assert!(refusal.to_string().contains("org.example/build-tools#cargo-v2"));
/// ```
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
#[derive(Debug, Error)]
pub enum BuildError {
    /// §3.1's four-step law refused to select a provider.
    #[error(transparent)]
    Resolution(#[from] MechanismResolutionError),

    /// The selected provider ran and refused.
    #[error(transparent)]
    Provider(#[from] MechanismError),

    /// Selection landed on a provider whose handler needs the
    /// out-of-process transport, which is a later atom. §5.0.1: such a
    /// target "refuses typed, naming the transport as not-yet-landed
    /// rather than pretending".
    ///
    /// This is what proves routing is real: the builtin was NOT run.
    #[error(
        "`{key}` selected provider `{pin}`, whose handler kind `{kind}` needs the out-of-process \
         mechanism transport, which is not yet landed; the target was NOT built by the builtin \
         instead \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; fix: remove the \
         `[mechanisms]` route or the target's `provider` \
         pin to fall back to the shipped builtin, or wait for the provider-transport atom)"
    )]
    TransportNotLanded {
        key: String,
        pin: String,
        kind: String,
    },

    /// Selection landed on an engine-owned row this build phase does not
    /// implement. Reachable only if the builtin table grows a build-role
    /// row before its adapter exists.
    #[error(
        "`{key}` selected builtin provider `{pin}` (`{name}`), which this build phase does not \
         implement \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; fix: route `{key}` \
         to a provider that implements it, or land the \
         builtin's adapter)"
    )]
    UnknownBuiltinProvider {
        key: String,
        pin: String,
        name: String,
    },

    /// A build target consumes an artifact no build target in this set
    /// produces.
    #[error(
        "[[artifacts.build]] `{target}` consumes artifact `{input}`, which no build target in this \
         execution produces \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY; fix: execute \
         the complete build graph — build inputs \
         resolve to build outputs under the phase-forward law)"
    )]
    UnknownInput { target: String, input: String },

    /// The build targets do not form a DAG. A validated manifest cannot
    /// reach this; a programmatically built target set can.
    #[error(
        "[[artifacts.build]] targets form a cycle: {cycle} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY; fix: break the \
         cycle — a build graph is executed in \
         dependency order, which a cycle has none of)"
    )]
    Cycle { cycle: String },

    /// The injected clock value is not an RFC 3339 timestamp.
    #[error(
        "artifact `{output}` cannot be stamped: `{value}` is not an RFC 3339 timestamp ({reason}) \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY; fix: \
         pass the run's own RFC 3339 clock value)"
    )]
    RecordClock {
        output: String,
        value: String,
        reason: String,
    },

    /// The engine built a record its own A2 cell refuses. Always a bug in
    /// this engine, and it stops here rather than reaching a reader.
    #[error(
        "the artifact record for `{output}` does not satisfy the record laws: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY; fix: this is a \
         defect in the producing engine — a record \
         that does not validate is never written)"
    )]
    RecordInvalid { output: String, reason: String },

    /// The validated record could not be serialised.
    #[error(
        "the artifact record for `{output}` could not be encoded: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY; fix: \
         this is a defect in the producing engine)"
    )]
    RecordEncode { output: String, reason: String },

    /// The record could not be published to the engine-owned state home.
    #[error(
        "the artifact record for `{output}` could not be written to `{path}`: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY; fix: make the \
         selected project's `.vibe/` writable, then \
         rerun the build)"
    )]
    RecordWrite {
        output: String,
        path: String,
        reason: String,
    },
}
