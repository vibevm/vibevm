//! Pure environment roles. Supplied references describe binding/equality only:
//! constructors do not authenticate content, inspect roots or grant effects.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-ROLE-SEPARATION");

mod compatibility;
mod identity;
mod platform;
mod roles;
pub use compatibility::*;
pub use identity::*;
pub use platform::*;
pub use roles::*;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(
    "environment field {field}: {reason} (governed by spec://org.vibevm.core/vibevm/common/PROP-076#INJECTED-CONSTRUCTION-AUTHORITY)"
)]
pub struct EnvironmentError {
    pub field: &'static str,
    pub reason: &'static str,
}
pub(super) fn invalid(field: &'static str, reason: &'static str) -> EnvironmentError {
    EnvironmentError { field, reason }
}
pub(super) fn unique(
    values: impl IntoIterator<Item = Reference>,
    field: &'static str,
) -> Result<(), EnvironmentError> {
    let mut names = std::collections::BTreeSet::new();
    for value in values {
        if !names.insert(value) {
            return Err(invalid(field, "duplicate named selection"));
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests;
