//! `vibe mcp serve` — start the MCP server over stdio.
//!
//! Spec: PROP-004 §5.1 + ROADMAP §M1.7. The CLI delegates to
//! [`vibe_mcp::Server::stdio`]; everything past the dispatch is the
//! library crate's job.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use vibe_core::manifest::ProjectManifest;
use vibe_mcp::{Server, ServerContext};

use crate::cli::{McpArgs, McpServeArgs, McpSubcommand};
use crate::output;

pub fn run(_ctx: &output::Context, args: McpArgs) -> Result<()> {
    match args.command {
        McpSubcommand::Serve(sub) => run_serve(sub),
    }
}

fn run_serve(args: McpServeArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let server_ctx = ServerContext::new(project_root);
    let mut server = Server::stdio(server_ctx);
    server.run().context("MCP server I/O error")?;
    Ok(())
}

fn resolve_project_root(path: &Path) -> Result<PathBuf> {
    let canonical = path
        .canonicalize()
        .with_context(|| format!("canonicalizing `{}`", path.display()))?;
    let stripped = super::init::strip_unc_public(canonical);
    if !stripped.join(ProjectManifest::FILENAME).exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first or pass `--path <dir>`",
            stripped.display()
        );
    }
    Ok(stripped)
}
