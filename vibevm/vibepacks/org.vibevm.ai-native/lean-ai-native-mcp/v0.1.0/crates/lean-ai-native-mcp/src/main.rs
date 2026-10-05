//! Start one Lean checking server with a fixed consumer root.

use std::path::PathBuf;

use anyhow::{Context as _, Result, ensure};
use clap::Parser;

#[derive(Parser)]
#[command(
    name = "lean-ai-native-mcp",
    version,
    about = "AI-Native Lean checking over MCP"
)]
struct Cli {
    /// Consumer project root. MCP registration supplies {project_root}.
    #[arg(long, default_value = ".")]
    path: PathBuf,
    /// Trusted Lean distribution directory. Tool calls cannot override it.
    #[arg(long)]
    toolchain_dir: Option<PathBuf>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let project_root = cli
        .path
        .canonicalize()
        .context("opening the configured project root")?;
    ensure!(
        project_root.is_dir(),
        "the configured project root is not a directory"
    );
    let startup_dir = std::env::current_dir().context("reading the server startup directory")?;
    let context = lean_ai_native_check::Context {
        project_root,
        toolchain_dir: cli.toolchain_dir.map(|path| startup_dir.join(path)),
    };
    let mut server = mcp_core::Server::new(
        lean_ai_native_mcp::SERVER_NAME,
        env!("CARGO_PKG_VERSION"),
        lean_ai_native_mcp::tool_set(&context),
    );
    server.run(&mut mcp_core::StdioTransport::new())?;
    Ok(())
}
