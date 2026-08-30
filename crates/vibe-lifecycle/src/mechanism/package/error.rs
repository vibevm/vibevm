//! The package-phase engine's one error enum — the layer above the
//! provider's, and the exact sibling of [`BuildError`].
//!
//! The split is the same one R8-CARGO drew, by *who is wrong*. A
//! [`MechanismError`] says the target, its config or a declared resource
//! is wrong; a [`PackageError`] says the graph, the routing, the consumed
//! artifact's record or the engine's own output root is. The input family
//! is the part with no build twin, and it is deliberately five variants
//! rather than one: "nothing produced that artifact", "the record is
//! unreadable", "the recorded file is gone" and "the recorded digest no
//! longer matches" are four different repairs, and §6.0.2 requires each to
//! refuse BY NAME.
//!
//! [`BuildError`]: crate::BuildError

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE");

use specmark::spec;
use thiserror::Error;
use vibe_extension_registry::MechanismResolutionError;

use crate::mechanism::MechanismError;
use crate::mechanism::record::RecordError;

/// Why the package phase could not execute one declared target.
///
/// ```
/// use vibe_lifecycle::PackageError;
///
/// let refusal = PackageError::InputNotRecorded {
///     target: "demo-skill".into(),
///     input: "vibe-helper.exe".into(),
/// };
/// assert!(refusal.to_string().contains("never guessed"));
/// assert!(refusal.to_string().contains("vibe-helper.exe"));
/// ```
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
#[derive(Debug, Error)]
pub enum PackageError {
    /// §3.1's four-step law refused to select a provider.
    #[error(transparent)]
    Resolution(#[from] MechanismResolutionError),

    /// The selected provider ran and refused.
    #[error(transparent)]
    Provider(#[from] MechanismError),

    /// The engine's own record keeping refused — the SHARED cell's error.
    #[error(transparent)]
    Record(#[from] RecordError),

    /// Selection landed on a provider whose handler needs the
    /// out-of-process transport, which is a later atom. §6.0.1: "a
    /// non-builtin selection refuses by its name."
    ///
    /// This is what proves routing is real: the builtin was NOT run.
    #[error(
        "`{key}` selected provider `{pin}`, whose handler kind `{kind}` needs the out-of-process \
         mechanism transport, which is not yet landed; the target was NOT packaged by the builtin \
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

    /// Selection landed on an engine-owned row this package phase does not
    /// implement. Reachable only if the builtin table grows a package-role
    /// row before its adapter exists.
    #[error(
        "`{key}` selected builtin provider `{pin}` (`{name}`), which this package phase does not \
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

    /// The package targets do not form a DAG. A validated manifest cannot
    /// reach this; a programmatically built target set can.
    #[error(
        "[[artifacts.package]] targets form a cycle: {cycle} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY; fix: break the \
         cycle — a package graph is executed in \
         dependency order, which a cycle has none of)"
    )]
    Cycle { cycle: String },

    /// A consumed artifact has no record in the engine's own state, so
    /// there is no proven path to read it from.
    #[error(
        "[[artifacts.package]] `{target}` consumes artifact `{input}`, which this project has no \
         artifact record for \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY; fix: run the \
         phase that produces `{input}` first — a consumed \
         artifact is read from its record under `.vibe/state/artifacts/`, and its path is never \
         guessed)"
    )]
    InputNotRecorded { target: String, input: String },

    /// The record exists and cannot be read or does not satisfy the A2
    /// laws — a corrupt record is refused, never partially believed.
    #[error(
        "[[artifacts.package]] `{target}` consumes artifact `{input}`, whose artifact record is \
         unusable: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY; fix: rerun the \
         producing phase so the record is rewritten)"
    )]
    InputRecordUnusable {
        target: String,
        input: String,
        reason: String,
    },

    /// The record names a file that is not there — or is a link, or is
    /// not a regular file.
    #[error(
        "[[artifacts.package]] `{target}` consumes artifact `{input}`, recorded at `{path}`, where \
         no readable regular file is: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY; fix: rerun the \
         producing phase; a recorded artifact that \
         vanished is never packaged from a stale record)"
    )]
    InputArtifactMissing {
        target: String,
        input: String,
        path: String,
        reason: String,
    },

    /// The file is there and is not the file the record describes.
    #[error(
        "[[artifacts.package]] `{target}` consumes artifact `{input}` at `{path}`, whose bytes \
         digest to `{found}` but whose record says `{recorded}` \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY; fix: rerun the \
         producing phase; a package is never built from \
         an artifact that changed behind its own record)"
    )]
    InputStale {
        target: String,
        input: String,
        path: String,
        recorded: String,
        found: String,
    },

    /// A `{ path }` input's spelling may not be joined to the project
    /// root at all.
    #[error(
        "[[artifacts.package]] `{target}` declares input path `{input}`, which is not a usable \
         project-relative path: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY; fix: name a \
         relative path inside the project — a packaging \
         input never escapes the workspace)"
    )]
    InputPathUnsafe {
        target: String,
        input: String,
        reason: String,
    },

    /// A `{ path }` input names nothing readable in the workspace.
    #[error(
        "[[artifacts.package]] `{target}` declares input path `{input}`, where no readable regular \
         file is: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY; fix: add the \
         file, or drop the declaration — a declared \
         resource is never silently skipped)"
    )]
    InputSourceMissing {
        target: String,
        input: String,
        reason: String,
    },

    /// The engine could not prepare its own output directory for the
    /// target. Preparing it is the engine's job precisely so a stale
    /// distributable from a previous run cannot end up inside a fresh
    /// directory digest.
    #[error(
        "the engine-owned package output directory `{path}` for [[artifacts.package]] `{target}` \
         could not be prepared: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; fix: make the \
         project's `target/` writable and remove whatever \
         occupies that path, then rerun)"
    )]
    OutputRoot {
        target: String,
        path: String,
        reason: String,
    },
}
