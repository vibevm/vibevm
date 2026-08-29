//! Lossless read views over the retained registry.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-REGISTRY");

use super::{ExtensionRegistry, RegistryView, SelectorSubject};

/// Effective state of one retained declaration for an explicit selector
/// subject. Precedence is part of the query contract.
///
/// ```
/// use vibe_extension_registry::RegistryState;
///
/// assert_ne!(RegistryState::Disabled, RegistryState::Effective);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryState {
    Disabled,
    Inactive,
    SelectorMismatch,
    Effective,
}

impl RegistryView<'_> {
    /// Resolve state without dropping the declaration that produced it.
    #[must_use]
    pub const fn state(&self) -> RegistryState {
        if self.row.is_disabled() {
            RegistryState::Disabled
        } else if !self.row.active_by_default() && !self.row.is_activated() {
            RegistryState::Inactive
        } else if !self.selector_matches {
            RegistryState::SelectorMismatch
        } else {
            RegistryState::Effective
        }
    }
}

impl ExtensionRegistry {
    /// Every retained row exactly once in the same effective tier/order view
    /// execution planning uses. State is evaluated, never filtered.
    #[must_use]
    pub fn exhaustive(&self, subject: SelectorSubject<'_>) -> Vec<RegistryView<'_>> {
        self.effective_order
            .iter()
            .map(|index| &self.rows[*index])
            .map(|row| RegistryView {
                row,
                selector_matches: row.selector.matches(subject),
            })
            .collect()
    }
}
