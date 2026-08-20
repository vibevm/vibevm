//! The tool registry — what an installed project lets an agent invoke.
//!
//! The boot lane already tells an agent which language disciplines a project
//! follows: each language package contributes a snippet, so "which guides do
//! I hold?" is answered before it is asked. What it does not answer is what
//! those packages brought that can be *run* — the binaries and the MCP
//! servers. That data exists in the lockfile slots and reaches no agent.
//! This is the answer to that question.
//!
//! It is a **library** first, deliberately. `flow:omnichannel` asks that a
//! capability live in a library with thin surfaces over it, and this is that
//! flow's first consumer: `vibe tools` renders this list, an MCP tool returns
//! the same list as JSON, and neither computes anything the other does not.
//! Delete either surface on paper and nothing but presentation is lost.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-025#manifest");

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::bins::{BinsError, collect_binaries, collect_mcp_servers};

/// How a tool is invoked — the two channels a package can declare today.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolChannel {
    /// A PATH-facing executable (`[[binary]]`), run by a human or a script.
    Binary,
    /// An MCP server (`[[mcp_server]]`), spoken to by an agent over stdio.
    Mcp,
}

impl ToolChannel {
    pub const fn as_str(self) -> &'static str {
        match self {
            ToolChannel::Binary => "binary",
            ToolChannel::Mcp => "mcp",
        }
    }
}

/// One invocable thing an installed package brought.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tool {
    /// The name the caller uses — the binary name, or the agent-visible
    /// server name.
    pub name: String,
    pub channel: ToolChannel,
    /// `<group>/<name>` of the declaring package.
    pub package: String,
    /// The declaring package's version, when the channel records one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// The declaration's own description, verbatim. Absent means the
    /// package did not write one — never a summary invented here, because a
    /// description an agent reads must be the author's.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Every invocable tool the project's lockfile slots declare, binaries and
/// MCP servers together, sorted by channel then name.
///
/// A missing lockfile is an empty registry rather than an error: a project
/// with nothing installed has nothing to invoke, which is an answer.
///
/// The two channels are *not* merged into one entry when an MCP server is
/// served by a binary that is itself declared. Both are true and an agent
/// needs both: the binary is what a human runs, the server is what an agent
/// speaks to, and collapsing them would hide one of the two.
pub fn collect_tools(project_root: &Path) -> Result<Vec<Tool>, BinsError> {
    let mut out: Vec<Tool> = Vec::new();

    for b in collect_binaries(project_root)? {
        out.push(Tool {
            name: b.decl.name.clone(),
            channel: ToolChannel::Binary,
            package: b.package.clone(),
            version: None,
            description: b.decl.description.clone(),
        });
    }

    for s in collect_mcp_servers(project_root)? {
        out.push(Tool {
            name: s.decl.name.clone(),
            channel: ToolChannel::Mcp,
            package: s.binary.package.clone(),
            version: Some(s.version.clone()),
            description: s.decl.description.clone(),
        });
    }

    out.sort_by(|a, b| {
        a.channel
            .as_str()
            .cmp(b.channel.as_str())
            .then_with(|| a.name.cmp(&b.name))
    });
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_project_with_no_lockfile_has_an_empty_registry() {
        // Not an error: "nothing is installed" is a legitimate answer, and a
        // surface that errored here would make `vibe tools` unusable in a
        // fresh tree.
        let tmp = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            tmp.path().join("vibe.toml"),
            "[project]\nname = \"x\"\nversion = \"0.0.1\"\n",
        )
        .expect("manifest");
        let tools = collect_tools(tmp.path()).expect("collects");
        assert!(tools.is_empty());
    }

    #[test]
    fn the_channel_names_are_the_wire_form() {
        // These strings reach JSON output and an agent's eyes; pin them so a
        // rename cannot happen silently.
        assert_eq!(ToolChannel::Binary.as_str(), "binary");
        assert_eq!(ToolChannel::Mcp.as_str(), "mcp");
    }
}
