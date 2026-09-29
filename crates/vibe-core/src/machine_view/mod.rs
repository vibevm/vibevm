//! Pure external-view identities and source positions. Owners supply every
//! reference; this module neither mints addresses nor reads resources.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-ENTITY-REFERENCES");

mod identity;
mod location;

pub use identity::*;
pub use location::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineViewError {
    pub field: &'static str,
    pub reason: &'static str,
}

impl std::fmt::Display for MachineViewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "machine view {}: {}", self.field, self.reason)
    }
}

impl std::error::Error for MachineViewError {}

fn invalid(field: &'static str, reason: &'static str) -> MachineViewError {
    MachineViewError { field, reason }
}

#[cfg(test)]
mod tests;
