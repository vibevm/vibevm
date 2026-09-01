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

    /// One ordered world contains the same package coordinate twice. Keeping
    /// the last row would make order and provider authority depend on an
    /// implementation detail of the index rather than the supplied epoch.
    #[error(
        "package `{package}` occurs more than once in the supplied extension-world epoch \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ORDER-LAW; \
         fix: supply one resolved row per package coordinate)"
    )]
    DuplicatePackage { package: String },

    /// One owner reaches the same effective coordinate more than once in the
    /// supplied graph. Coalescing would erase authored/resolved order and hide
    /// a malformed authority.
    #[error(
        "`{owner}` carries duplicate dependency edge `{requirement}` in the supplied world \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ORDER-LAW; \
         fix: retain one effective edge to that package)"
    )]
    DuplicateEdge { owner: String, requirement: String },

    /// A reachable coordinate is absent from the supplied installed world.
    /// This is malformed, never a tolerable omission: hiding the provider
    /// would silently drop its contributions.
    #[error(
        "`{owner}` reaches `{requirement}`, which is absent from the supplied installed world \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ORDER-LAW; \
         fix: supply a complete resolved world, or run `vibe install` to relock it)"
    )]
    UnlockedRequirement { owner: String, requirement: String },

    /// An installed-world package has no materialised slot.
    #[error(
        "extension-world package `{package}` has no materialised slot `{}` \
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

    /// The lock chooses which physical slot spelling is authoritative. A
    /// manifest cannot redirect that locked row to a different materialisation
    /// genre after the slot has been selected.
    #[error(
        "slot `{}` declares materialization `{declared}` but the root lock requires `{locked}` \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM; \
         fix: rematerialise the locked package with matching copy, hardlink or in-place metadata)",
        .slot.display()
    )]
    SlotMaterializationMismatch {
        slot: PathBuf,
        declared: &'static str,
        locked: &'static str,
    },

    /// A present durable lock is not a regular file.
    #[error(
        "durable lock `{}` is not a regular file \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM; \
         fix: replace it with one regular vibe.lock file, then retry)",
        .path.display()
    )]
    NonRegularLock { path: PathBuf },

    /// A present regular durable lock could not be parsed or validated.
    #[error(
        "durable lock `{}` is invalid: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM; \
         fix: repair or regenerate vibe.lock, then retry)",
        .path.display()
    )]
    InvalidLock { path: PathBuf, reason: String },

    /// A supplied resolved row and its already-parsed package manifest do not
    /// name the same provider.
    #[error(
        "resolved package `{package}` carries manifest identity `{declared}` \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM; \
         fix: repair the resolution/manifest mismatch before boot generation)"
    )]
    ResolutionIdentityMismatch { package: String, declared: String },

    /// Provider identity includes the package content witness. A resolution
    /// without one cannot become an extension-world row by guesswork.
    #[error(
        "resolved package `{package}` carries no content hash for the extension-world epoch \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM; \
         fix: finalize the resolved package content hash before boot generation)"
    )]
    ResolutionWithoutContentHash { package: String },

    /// The canonical semantic manifest frame could not be encoded. Falling
    /// back to a partial identity would allow a stale bound epoch through.
    #[error(
        "resolved package `{package}` cannot enter the ordered-resolution identity: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#BOOTSTRAP-ORDER; \
         fix: repair the typed manifest value before owner-runtime lowering)"
    )]
    ResolutionIdentityEncoding { package: String, reason: String },

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
         fix: ask for a coordinate the supplied installed world contains)"
    )]
    UnknownOwner { owner: String },

    /// The one kernel collector refused this owner-scoped view.
    #[error(transparent)]
    Collection(#[from] CollectionError),
}
