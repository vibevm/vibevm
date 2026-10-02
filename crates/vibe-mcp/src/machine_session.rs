//! Separate on-demand byte-framed machine-session values.
//! The legacy MCP JSON-RPC line transport remains independent.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-LOCAL-FRAME");

pub mod fixed;
pub mod framing;
pub mod incremental;
pub mod package;
pub mod responsive;
pub mod runner;
pub mod state;
