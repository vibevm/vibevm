//! Pure collection, control application, selection, and ordering.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ORDER-LAW");

mod collect;
mod model;
mod selector;
mod view;

pub use collect::{CollectionError, CollectionNotice, collect_extensions};
pub use model::{
    ContributionTier, DependencyExtensionSource, DependencyProvider, DependencyProviderId,
    EffectiveManifestKind, ExecutableContribution, ExecutablePlan, ExtensionProvider,
    ExtensionRegistry, ExtensionRegistryRow, ExtensionWorld, HostExtensionSource, HostIdentity,
    HostProvider, RegistryView,
};
pub use selector::SelectorSubject;
pub use view::RegistryState;

#[cfg(test)]
mod tests;
