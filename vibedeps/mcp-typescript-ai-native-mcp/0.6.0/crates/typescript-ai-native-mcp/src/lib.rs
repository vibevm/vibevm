//! `typescript-ai-native-mcp` — the AI-Native TypeScript discipline
//! served over MCP (PROP-027; the package brief carries the parity
//! map). Seventeen tools on the neutral mcp-core transport: the twelve
//! discipline commands (thin adapters over the SAME lib fns the
//! `typescript-ai-native` / `typescript-ai-native-conform` /
//! `typescript-ai-native-specmap` CLIs call, each wrapped in the stderr-capture
//! guard) and the five tcg ops over one persistent LanguageService
//! session (lazy-spawned, respawn-once). Serving needs no vibe
//! anywhere on the machine — the PROP-027 §2.6 acceptance.

specmark::scope!("spec://typescript-ai-native-mcp/tools/discipline-mcp-typescript#root");

use std::path::Path;

use mcp_core::ToolSet;

pub mod tools_discipline;
pub mod tools_tcg;

/// The agent-visible server name ([[mcp_server]] `name`).
pub const SERVER_NAME: &str = "typescript-ai-native";

/// Assemble the full tool set for a project root: 12 discipline + 5
/// tcg = 17 tools, the whole `typescript-ai-native` + `typescript-ai-native-tcg`
/// command surface (the TS umbrella has no ledger).
///
/// ```
/// let set = typescript_ai_native_mcp::tool_set(std::path::Path::new("."));
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

    /// P2's enumeration half: tools/list is exactly the declared
    /// inventory, in stable order.
    #[test]
    fn the_tool_set_is_exactly_the_declared_inventory() {
        let set = tool_set(Path::new("."));
        let names: Vec<String> = set.descriptors().into_iter().map(|d| d.name).collect();
        assert_eq!(names, TOOL_NAMES);
    }
}
