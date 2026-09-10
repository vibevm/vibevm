//! bin `rust-ai-native-mcp` — launch the server on stdio for one
//! project root. Agent hosts run this straight from the slot (the
//! [[mcp_server]] registration writes the absolute artifact path);
//! nothing here or below needs vibe (PROP-027 §2.6).

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(
    name = "rust-ai-native-mcp",
    about = "The AI-Native Rust discipline + tcg type oracle over MCP (PROP-027)"
)]
struct Cli {
    /// Project root — where conform.toml / specmap.toml live. Defaults
    /// to the current dir; registrations pass {project_root}.
    #[arg(long, default_value = ".")]
    path: PathBuf,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    // The verbatim-free absolutisation: node-free here, but the same
    // three-times-learned lesson applies to every child this server
    // spawns (rust-analyzer, cargo) — no `\\?\` paths downstream.
    let root = rust_ai_native_tcg_bridge::verbatim_free(
        &cli.path.canonicalize().unwrap_or_else(|_| cli.path.clone()),
    );
    eprintln!(
        "rust-ai-native-mcp: serving `{}` on stdio ({} tools; protocol {})",
        root.display(),
        rust_ai_native_mcp::TOOL_NAMES.len(),
        mcp_core::PROTOCOL_VERSION,
    );
    let tools = rust_ai_native_mcp::tool_set(&root);
    let mut server = mcp_core::Server::new(
        rust_ai_native_mcp::SERVER_NAME,
        env!("CARGO_PKG_VERSION"),
        tools,
    );
    let mut transport = mcp_core::StdioTransport::new();
    server.run(&mut transport)?;
    Ok(())
}
