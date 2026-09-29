//! `mcp-core` — the neutral MCP server transport (MCP-CORE v0.1;
//! MCP-SOVEREIGNTY-PLAN Wave 2). The `mcp`-kind packages (PROP-027)
//! build their servers on this crate: it owns the line-delimited
//! JSON-RPC stdio loop at protocol `2024-11-05` — the exact shape the
//! vibe-mcp product server has spoken in production against every
//! supported agent host — plus the tool registry seam and the stderr
//! capture guard tool adapters wrap their runners in.
//!
//! - [`wire`](crate::JsonRpcMessage) — frame model: requests answered,
//!   notifications absorbed, malformed lines refused in-protocol.
//! - [`Server`] / [`Transport`] — the blocking loop: initialize,
//!   tools/list, tools/call, ping; replayable via
//!   [`testing::Scripted`](crate::testing::Scripted).
//! - [`ToolSet`] / [`Tool`] — name → schema + handler; tool-level
//!   failure is an `isError` RESULT, never a protocol error; tools
//!   never prompt (a server has no interactive channel).
//! - [`capture`] — the process-level stderr guard: a tool run's whole
//!   story, child processes included, captured around the call.
//!
//! Nothing here knows any language, any discipline rule, or vibe: a
//! server built on this crate serves with `vibe` absent from PATH
//! (PROP-027 §2.6).

mod capture;
mod error;
mod server;
mod toolset;
mod wire;

pub use capture::capture;
pub use error::McpCoreError;
pub use server::{Server, StdioTransport, Transport, testing};
pub use toolset::{Tool, ToolDescriptor, ToolOutput, ToolSet};
pub use wire::{
    JsonRpcError, JsonRpcMessage, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse,
    PROTOCOL_VERSION, parse_line,
};
