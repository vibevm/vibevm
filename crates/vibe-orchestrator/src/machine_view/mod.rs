//! Supplied multi-owner observations and finite in-memory projection pages.
//! No value here authenticates bytes, acquires authority or rereads an owner.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-COMPOSITE-SNAPSHOT");

mod paging;
mod snapshot;

pub use paging::*;
pub use snapshot::*;

#[cfg(test)]
mod tests;
