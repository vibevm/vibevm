//! Pure collection, control application, selection, and ordering — the
//! compatibility re-export of the extracted registry kernel.
//!
//! The collector itself moved unchanged into the lower
//! `vibe-extension-registry` crate (R4.0) so workspace, spec and every
//! surface can share the one machine without depending on this crate. Every
//! moved public item keeps its `vibe_lifecycle::` path through this module
//! for one transition — no flag day, no copied types. The lifecycle-owned
//! values that sit above the kernel stay here: [`EffectiveManifestKind`]
//! (report/workspace metadata collection never reads) and
//! [`ExecutableContribution`]/[`ExecutablePlan`] (the execution-shaped plan).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ORDER-LAW");

pub use vibe_extension_registry::*;

mod manifest_kind;
mod plan;

pub use manifest_kind::EffectiveManifestKind;
pub use plan::{ExecutableContribution, ExecutablePlan};

#[cfg(test)]
mod compat_tests {
    /// Compiles only when both arguments name the same type — a copied
    /// wrapper or a re-declared struct would not unify.
    fn same_type<T>(_: Option<T>, _: Option<T>) {}

    /// Compiles only when both arguments are the same function *item* —
    /// a forwarding wrapper has its own item type and would not unify.
    fn same_item<T>(_: T, _: T) {}

    /// R4.0: the compatibility surface must be the moved kernel items, not
    /// copies. Every public item the glob re-export carries is pinned
    /// through both paths — the crate-root public list (`vibe_lifecycle::X`,
    /// so dropping or wrapping an item there fails to compile) and the
    /// kernel's own `vibe_extension_registry::X`.
    #[test]
    fn reexports_are_the_moved_items_not_copies() {
        same_type(
            None::<crate::CollectionError>,
            None::<vibe_extension_registry::CollectionError>,
        );
        same_type(
            None::<crate::CollectionNotice>,
            None::<vibe_extension_registry::CollectionNotice>,
        );
        same_type(
            None::<crate::ContributionTier>,
            None::<vibe_extension_registry::ContributionTier>,
        );
        same_type(
            None::<crate::DependencyExtensionSource>,
            None::<vibe_extension_registry::DependencyExtensionSource>,
        );
        same_type(
            None::<crate::DependencyProvider>,
            None::<vibe_extension_registry::DependencyProvider>,
        );
        same_type(
            None::<crate::DependencyProviderId>,
            None::<vibe_extension_registry::DependencyProviderId>,
        );
        same_type(
            None::<crate::ExtensionProvider>,
            None::<vibe_extension_registry::ExtensionProvider>,
        );
        same_type(
            None::<crate::ExtensionRegistry>,
            None::<vibe_extension_registry::ExtensionRegistry>,
        );
        same_type(
            None::<crate::ExtensionRegistryRow>,
            None::<vibe_extension_registry::ExtensionRegistryRow>,
        );
        same_type(
            None::<crate::ExtensionWorld>,
            None::<vibe_extension_registry::ExtensionWorld>,
        );
        same_type(
            None::<crate::HostExtensionSource>,
            None::<vibe_extension_registry::HostExtensionSource>,
        );
        same_type(
            None::<crate::HostIdentity>,
            None::<vibe_extension_registry::HostIdentity>,
        );
        same_type(
            None::<crate::HostProvider>,
            None::<vibe_extension_registry::HostProvider>,
        );
        same_type(
            None::<crate::RegistryState>,
            None::<vibe_extension_registry::RegistryState>,
        );
        same_type(
            None::<crate::RegistryView>,
            None::<vibe_extension_registry::RegistryView>,
        );
        same_type(
            None::<crate::SelectorSubject>,
            None::<vibe_extension_registry::SelectorSubject>,
        );
        same_type(
            None::<crate::SyntheticPresetSource>,
            None::<vibe_extension_registry::SyntheticPresetSource>,
        );
        same_item(
            crate::collect_extensions,
            vibe_extension_registry::collect_extensions,
        );
        same_item(
            crate::collect_extensions_with_presets,
            vibe_extension_registry::collect_extensions_with_presets,
        );
    }
}
