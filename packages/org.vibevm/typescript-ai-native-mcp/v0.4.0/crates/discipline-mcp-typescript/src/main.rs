//! bin `discipline-mcp-typescript` — launch the server on stdio for
//! one project root. Agent hosts run this straight from the slot (the
//! [[mcp_server]] registration writes the absolute artifact path);
//! nothing here or below needs vibe (PROP-027 §2.6).

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(
    name = "discipline-mcp-typescript",
    about = "The AI-Native TypeScript discipline + tcg type oracle over MCP (PROP-027)"
)]
struct Cli {
    /// Project root — where conform.toml / specmap.toml live. Defaults
    /// to the current dir; registrations pass {project_root}.
    #[arg(long, default_value = ".")]
    path: PathBuf,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    // node refuses `\\?\`-verbatim paths — the thrice-learned lesson;
    // every child this server spawns goes through the oracle bridge's
    // own verbatim_free, and the root starts clean here.
    let root = tcg_oracle_bridge::verbatim_free(
        &cli.path.canonicalize().unwrap_or_else(|_| cli.path.clone()),
    );
    eprintln!(
        "discipline-mcp-typescript: serving `{}` on stdio ({} tools; protocol {})",
        root.display(),
        discipline_mcp_typescript::TOOL_NAMES.len(),
        mcp_core::PROTOCOL_VERSION,
    );
    let tools = discipline_mcp_typescript::tool_set(&root);
    let mut server = mcp_core::Server::new(
        discipline_mcp_typescript::SERVER_NAME,
        env!("CARGO_PKG_VERSION"),
        tools,
    );
    let mut transport = mcp_core::StdioTransport::new();
    server.run(&mut transport)?;
    Ok(())
}
