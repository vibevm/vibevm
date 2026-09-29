//! Separate on-demand byte-framed machine-session values.
//! The legacy MCP JSON-RPC line transport remains independent.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-LOCAL-FRAME");

pub mod framing;
pub mod state;
