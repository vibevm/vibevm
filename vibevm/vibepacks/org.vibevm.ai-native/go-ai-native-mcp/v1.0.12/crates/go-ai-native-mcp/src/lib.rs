//! `go-ai-native-mcp` — the AI-Native Go discipline served over MCP
//! (PROP-027; the package brief carries the parity map). Seventeen
//! tools on the neutral mcp-core transport: the twelve discipline
//! commands (thin adapters over the SAME lib fns the `go-ai-native` /
//! `go-ai-native-conform` / `go-ai-native-specmap` CLIs call, each
//! wrapped in the stderr-capture guard so the report is the run's
//! whole story) and the five tcg ops over one persistent gopls session
//! (lazy-spawned, respawn-once). Serving needs no vibe anywhere on the
//! machine — the PROP-027 §2.6 acceptance.

specmark::scope!("spec://go-ai-native-mcp/tools/discipline-mcp-go#root");

use std::path::Path;

use mcp_core::ToolSet;

pub mod tools_discipline;
pub mod tools_tcg;

/// The agent-visible server name ([[mcp_server]] `name`).
pub const SERVER_NAME: &str = "go-ai-native";

/// Assemble the full tool set for a project root: 12 discipline + 5
/// tcg = 17 tools, the whole `go-ai-native` + `go-ai-native-tcg`
/// command surface (the Go umbrella has no ledger verb — seventeen,
/// the TS count).
///
/// ```
/// let set = go_ai_native_mcp::tool_set(std::path::Path::new("."));
/// let names: Vec<String> = set.descriptors().into_iter().map(|d| d.name).collect();
/// assert_eq!(names.len(), 17);
/// assert!(names.contains(&"floor".to_string()));
/// assert!(names.contains(&"tcg_validate".to_string()));
/// ```
pub fn tool_set(root: &Path) -> ToolSet {
    let mut set = ToolSet::new();
    for tool in tools_discipline::discipline_tools(root) {
        set.register(tool);
    }
    for tool in tools_tcg::tcg_tools(root) {
        set.register(tool);
    }
    set
}

/// The full declared inventory, in the ToolSet's stable order — the
/// parity-enumeration test and the brief's map both pin against this
/// one list.
pub const TOOL_NAMES: [&str; 17] = [
    "codemod_add_cell",
    "conform_check",
    "conform_freeze",
    "fast_loop",
    "floor",
    "health",
    "init",
    "specmap_check",
    "specmap_write",
    "tcg_bench",
    "tcg_complete",
    "tcg_scope",
    "tcg_type",
    "tcg_validate",
    "test_gate",
    "trace_explain",
    "tripwire",
];

#[cfg(test)]
mod tests {
    use super::*;

    /// The enumeration half of the parity map: tools/list is exactly
    /// the declared inventory, in stable order.
    #[test]
    fn the_tool_set_is_exactly_the_declared_inventory() {
        let set = tool_set(Path::new("."));
        let names: Vec<String> = set.descriptors().into_iter().map(|d| d.name).collect();
        assert_eq!(names, TOOL_NAMES);
    }
}
