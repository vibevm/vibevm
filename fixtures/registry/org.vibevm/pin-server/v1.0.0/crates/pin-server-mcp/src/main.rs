//! Stub stdio MCP server for the hermetic mcp-kind fixtures. The
//! resolution and registration tests never speak to it; it exists so
//! the `[[binary]]` the `[[mcp_server]]` references is a real,
//! buildable crate (PROP-027 §2.2 over PROP-025).

fn main() {}
