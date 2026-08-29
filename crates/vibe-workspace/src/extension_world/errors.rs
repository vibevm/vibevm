//! The durable world adapter's failure surface, out-of-line so the adapter
//! cell keeps its own file-length budget — the same split
//! `compile_trace/errors.rs` makes for the same reason.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM");

use std::path::PathBuf;

use specmark::spec;
use thiserror::Error;
use vibe_extension_registry::CollectionError;

/// The durable world adapter's failure surface. Every message names the
/// offending coordinate and a fix surface (Class-F grammar), because each of
/// these is a repairable state of the installed tree, never a program bug.
#[derive(Debug, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
pub enum ExtensionWorldError {
    /// A manifest component the model still carries as a bare string does not
    /// spell a package name. Refused typed and by component: the alternative
    /// — a panic, or a silent fallback identity — would hand the kernel an
    /// identity no grammar admits.
    #[error(
        "manifest component `{component}` value `{spelling}` is not a package name: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM; \
         fix: correct `{component}` to the kebab-case package-name grammar)"
    )]
    UntypedComponent {
        component: &'static str,
        spelling: String,
        reason: String,
    },

    /// A requirement or locked dependency edge carries no group, so it names
    /// no coordinate the lock can be indexed by.
    #[error(
        "`{owner}` reaches `{edge}`, which carries no group \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM; \
         fix: qualify the reference with its reverse-FQDN group, then run `vibe install`)"
    )]
    UngroupedEdge { owner: String, edge: String },

    /// A reachable coordinate is absent from the root lock. After the install
    /// barrier this is a malformed durable world, never a tolerable omission:
    /// hiding the provider would silently drop its contributions.
    #[error(
        "`{owner}` reaches `{requirement}`, which is absent from the root lock \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ORDER-LAW; \
         fix: run `vibe install` to relock the world)"
    )]
    UnlockedRequirement { owner: String, requirement: String },

    /// A locked package has no materialised slot.
    #[error(
        "locked package `{package}` has no materialised slot `{}` \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM; \
         fix: run `vibe install` to materialise the slot)",
        .slot.display()
    )]
    MissingSlot { package: String, slot: PathBuf },

    /// A slot manifest could not be read or validated. The inner error is
    /// boxed — `vibe_core::Error` is large, and an unboxed copy would bloat
    /// every `Result` in this cell (`clippy::result_large_err`).
    #[error(
        "slot manifest `{}` could not be read \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM; \
         fix: repair or remove the slot, then run `vibe install` — the underlying error names \
         the defect)",
        .manifest.display()
    )]
    UnreadableSlot {
        manifest: PathBuf,
        #[source]
        source: Box<vibe_core::Error>,
    },

    /// A materialised slot carries no `[package]` identity, so it can supply
    /// no provider the lock row names.
    #[error(
        "slot `{}` carries no `[package]` identity \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM; \
         fix: remove or repair the slot, then run `vibe install`)",
        .slot.display()
    )]
    SlotWithoutPackage { slot: PathBuf },

    /// A materialised slot declares an identity the lock does not. Its
    /// declarations would otherwise enter the world under the lock's key.
    #[error(
        "slot `{}` declares `{declared}` but the root lock requires `{locked}` \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM; \
         fix: remove or repair the slot, then run `vibe install`)",
        .slot.display()
    )]
    SlotIdentityMismatch {
        slot: PathBuf,
        declared: String,
        locked: String,
    },

    /// `[active].stack` names no installed stack in this owner's closure.
    #[error(
        "`{owner}` declares `[active].stack = \"{stack}\"`, which names no installed stack in its \
         dependency closure \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ORDER-LAW; \
         fix: require that stack package, or correct the short name)"
    )]
    UnresolvedActiveStack { owner: String, stack: String },

    /// `[active].stack` matches more than one installed stack.
    #[error(
        "`{owner}` declares `[active].stack = \"{stack}\"`, which is ambiguous across {candidates} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ORDER-LAW; \
         fix: use a unique stack short name)"
    )]
    AmbiguousActiveStack {
        owner: String,
        stack: String,
        candidates: String,
    },

    /// A lane owner was requested that this world does not install.
    #[error(
        "`{owner}` is not an installed package of this world, so it owns no unit lane in it \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#COMPILE-ACTIVATION; \
         fix: ask for a coordinate the root lock installs)"
    )]
    UnknownOwner { owner: String },

    /// The one kernel collector refused this owner-scoped view.
    #[error(transparent)]
    Collection(#[from] CollectionError),
}
