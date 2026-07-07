//! The one error enum of the transport layer (Class F): every variant
//! cites the MCP-CORE requirement it guards and names a fix surface, so
//! a failing server run is navigable without this crate's source.

specmark::scope!("spec://discipline-core/mechanisms/MCP-CORE-v0.1#wire");

use thiserror::Error;

/// Transport-layer failures. Protocol-level trouble (unknown method,
/// bad params, a failing tool) never lands here — it travels INSIDE the
/// protocol as a JSON-RPC error response or an `isError` tool result;
/// this enum is for the channel itself dying.
///
/// ```
/// let e = mcp_core::McpCoreError::Io {
///     op: "read".into(),
///     source: std::io::Error::other("gone"),
/// };
/// assert!(e.to_string().contains("MCP-CORE-v0.1#wire"));
/// ```
#[derive(Debug, Error)]
pub enum McpCoreError {
    #[error(
        "stdio transport {op} failed: {source} \
         (violates spec://discipline-core/mechanisms/MCP-CORE-v0.1#wire; \
          fix surface: the host closed the pipe — check the agent host's \
          server log, then the [[mcp_server]] command line it launched)"
    )]
    Io {
        op: String,
        #[source]
        source: std::io::Error,
    },

    #[error(
        "response serialisation failed: {source} \
         (violates spec://discipline-core/mechanisms/MCP-CORE-v0.1#wire; \
          fix surface: a tool returned a value serde_json cannot render — \
          fix that tool's result type)"
    )]
    Serialize {
        #[source]
        source: serde_json::Error,
    },

    #[error(
        "stderr capture {op} failed: {detail} \
         (violates spec://discipline-core/mechanisms/MCP-CORE-v0.1#capture; \
          fix surface: the capture guard could not redirect or restore \
          fd 2 — see mcp_core::capture's platform notes)"
    )]
    Capture { op: String, detail: String },
}
