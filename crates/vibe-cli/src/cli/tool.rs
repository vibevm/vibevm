//! `vibe tool px …` — vibe's own proxy, lent to another tool
//! (`PROP-068#proxy-tool`).

/// `vibe tool <tool> …`: a facility vibe already runs for itself, made
/// available to a command the user names.
#[derive(Debug, clap::Args)]
pub struct ToolArgs {
    #[command(subcommand)]
    pub command: ToolCommand,
}

#[derive(Debug, clap::Subcommand)]
pub enum ToolCommand {
    /// The vibe-managed px: run another command through it, run it in the
    /// foreground, or report the installed companion.
    Px(ToolPxArgs),
}

#[derive(Debug, clap::Args)]
pub struct ToolPxArgs {
    #[command(subcommand)]
    pub command: ToolPxCommand,
}

#[derive(Debug, clap::Subcommand)]
pub enum ToolPxCommand {
    /// Start px exactly as vibe's own operations do, then run COMMAND with
    /// the proxy environment pointing at it; px is stopped when COMMAND
    /// exits, and its exit code is vibe's.
    Exec(ToolPxExecArgs),
    /// Run px in the foreground until Ctrl-C, printing its address and the
    /// route it resolved — for a tool vibe cannot start itself.
    Serve(ToolPxServeArgs),
    /// The pinned px, its asset for this platform and whether it is
    /// installed: the same report as `vibe self px status`.
    Status,
}

#[derive(Debug, clap::Args)]
pub struct ToolPxExecArgs {
    /// A fixed loopback port for px, instead of `[proxy] px_port` or a
    /// free one per run.
    #[arg(long, value_name = "PORT")]
    pub port: Option<u16>,

    /// Also give the child this variable, `NAME=VALUE`; repeatable. It is
    /// set after the proxy variables, so it can override one.
    #[arg(long = "env", value_name = "NAME=VALUE")]
    pub env: Vec<String>,

    /// Also set this variable to the proxy's URL; repeatable. For a tool
    /// that reads its own name for the proxy (`npm_config_proxy`,
    /// `PIP_PROXY`) and cannot be told the address in advance.
    #[arg(long = "proxy-env", value_name = "NAME")]
    pub proxy_env: Vec<String>,

    /// Run COMMAND with no proxy at all when `[proxy]` configures none,
    /// instead of refusing.
    #[arg(long)]
    pub allow_direct: bool,

    /// The command to run and its arguments, after `--`. Resolved on PATH
    /// (on Windows with `PATHEXT`).
    #[arg(trailing_var_arg = true, required = true, value_name = "COMMAND")]
    pub command: Vec<String>,
}

#[derive(Debug, clap::Args)]
pub struct ToolPxServeArgs {
    /// A fixed loopback port for px, instead of `[proxy] px_port` or a
    /// free one per run.
    #[arg(long, value_name = "PORT")]
    pub port: Option<u16>,

    /// Emit the address and the resolved route as `vibe-px-serve/1`
    /// instead of printing them for a reader.
    #[arg(long)]
    pub json: bool,
}
