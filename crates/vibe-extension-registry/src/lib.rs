//! `vibe-extension-registry` — the one pure extension registry kernel.
//!
//! This is the *extension* registry: the shared declaration, activation,
//! ordering, selector and view collector for every `[[extension]]`
//! contribution (PROP-054 §3) — not the package registry (`vibe-registry`,
//! the git/index package source). Lifecycle contributions and compiler
//! transforms share this one machine; adapters may turn a lock or
//! materialised world into the owned [`ExtensionWorld`] input and later
//! crates may execute plans over its rows, but no second collector exists.
//!
//! Collection is pure. The kernel performs no filesystem, environment,
//! resolver, workspace or process access: provider roots arrive as
//! already-resolved path data it never reads, installed rows arrive in the
//! caller-supplied lock order, and host controls arrive already parsed. Its
//! runtime dependencies are exactly `vibe-core` (manifest grammar and typed
//! coordinates), `glob` (selector patterns), `specmark` and `thiserror` —
//! fenced by test, not by convention.
//!
//! Extracted unchanged from `vibe-lifecycle::registry` (R4.0).
//! `vibe-lifecycle` re-exports every public item of this crate at the same
//! paths for one transition, so existing callers keep type identity with no
//! flag day; execution, dispatch, state fingerprints and the executable plan
//! stay above the kernel. The `vibe-ext` name stays reserved for the future
//! R5 native plugin SDK and is deliberately not this crate.

#![forbid(unsafe_code)]

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ORDER-LAW");

mod collect;
mod model;
mod selector;
mod view;

pub use collect::{
    CollectionError, CollectionNotice, collect_extensions, collect_extensions_with_presets,
};
pub use model::{
    ContributionTier, DependencyExtensionSource, DependencyProvider, DependencyProviderId,
    ExtensionProvider, ExtensionRegistry, ExtensionRegistryRow, ExtensionWorld,
    HostExtensionSource, HostIdentity, HostProvider, RegistryView, SyntheticPresetSource,
    lane_owner_host,
};
pub use selector::SelectorSubject;
pub use view::RegistryState;

#[cfg(test)]
mod fence_tests;
#[cfg(test)]
mod tests;
