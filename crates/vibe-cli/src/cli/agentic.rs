//! Argument structs for `vibe agentic …` and the top-level `vibe command`
//! (PROP-018 §2.7, §2.10) — the agentic relay surface.
//!
//! `vibe agentic <op>` are the relay *producers*: an op that needs
//! reasoning parks an instruction in `.vibe/agentic/command.md` instead of
//! acting. `vibe command` is the single *consumer*: it drains the pending
//! instruction so the calling agent can carry it out on its own LLM.

specmark::scope!("spec://vibevm/common/PROP-018#relay");

use std::path::PathBuf;

use clap::Subcommand;

#[derive(Debug, clap::Args)]
pub struct AgenticArgs {
    #[command(subcommand)]
    pub command: AgenticSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum AgenticSubcommand {
    /// Compose an "explain this project" instruction and park it for the
    /// calling agent to execute (PROP-018 §2.10). Does no LLM work itself
    /// and writes nothing but the relay mailbox — fetch the instruction
    /// with `vibe command`, then carry it out.
    Explain(AgenticExplainArgs),
}

#[derive(Debug, clap::Args)]
pub struct AgenticExplainArgs {
    /// Project root with `vibe.toml`. Defaults to current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}

/// `vibe command` — drain the agentic relay.
#[derive(Debug, clap::Args)]
pub struct CommandArgs {
    /// Project root with `vibe.toml`. Defaults to current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}
