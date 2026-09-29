//! Supplied multi-owner observations and finite in-memory projection pages.
//! No value here authenticates bytes, acquires authority or rereads an owner.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-COMPOSITE-SNAPSHOT");

mod page_wire;
mod paging;
mod snapshot;
mod snapshot_wire;

pub use page_wire::*;
pub use paging::*;
pub use snapshot::*;
pub use snapshot_wire::*;

#[cfg(test)]
mod adapter_tests;
#[cfg(test)]
mod tests;
