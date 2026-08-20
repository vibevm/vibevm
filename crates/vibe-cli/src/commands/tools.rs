//! `vibe tools` — the registry of what an installed project can invoke.
//!
//! A **surface**, in the sense `flow:omnichannel` gives the word: it parses
//! flags, calls `vibe_workspace::tools::collect_tools`, and renders. It
//! decides nothing. The MCP tool over the same capability calls the same
//! function and renders JSON; delete either and only presentation is lost.
//!
//! Why the capability exists: the boot lane already tells an agent which
//! language disciplines a project follows, so "which guides do I hold?" is
//! answered before it is asked. What binaries and servers those packages
//! brought is a different question, and until now it reached no agent.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-025#manifest");

use std::path::Path;

use anyhow::Result;
use vibe_workspace::tools::{ToolChannel, collect_tools};

/// Render the registry as a table, or as JSON under `--json`.
pub fn run(project_root: &Path, json: bool) -> Result<()> {
    let tools = collect_tools(project_root)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&tools)?);
        return Ok(());
    }

    if tools.is_empty() {
        println!("vibe tools: nothing installed declares a binary or an MCP server");
        return Ok(());
    }

    let name_w = tools.iter().map(|t| t.name.len()).max().unwrap_or(4).max(4);
    let pkg_w = tools
        .iter()
        .map(|t| t.package.len())
        .max()
        .unwrap_or(7)
        .max(7);

    println!(
        "{:<name_w$}  {:<6}  {:<pkg_w$}  description",
        "name", "how", "package"
    );
    for t in &tools {
        println!(
            "{:<name_w$}  {:<6}  {:<pkg_w$}  {}",
            t.name,
            t.channel.as_str(),
            t.package,
            t.description.as_deref().unwrap_or("—"),
        );
    }

    let bins = tools
        .iter()
        .filter(|t| t.channel == ToolChannel::Binary)
        .count();
    let mcps = tools.len() - bins;
    println!("\n{} tool(s): {bins} binary, {mcps} mcp", tools.len());
    Ok(())
}
