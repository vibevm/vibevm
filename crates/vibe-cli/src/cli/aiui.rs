//! Argument structs for `vibe aiui …` — the agent-facing observation surface
//! (PROP-042). Split from the `cli` hub along command-family lines; the hub
//! re-exports everything, so `crate::cli::X` paths are unchanged.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#command-summary");

use std::path::PathBuf;

use clap::{Args, Subcommand, ValueEnum};

#[derive(Debug, Args)]
pub struct AiuiArgs {
    #[command(subcommand)]
    pub command: AiuiSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum AiuiSubcommand {
    /// Render the `vibe tree` TUI **headlessly** to a symbolic snapshot — no
    /// terminal, deterministic (PROP-042 §1/§4). Optionally drive a key script
    /// with `--send` (e.g. "F2 Down Enter"; `F4`/`F6` are refused), set the grid
    /// with `--size COLSxROWS`, and pick `--format text|cells`. Read-only.
    Render(AiuiRenderArgs),

    /// Launch vibeterm with a control server and wait for it to be ready
    /// (PROP-042 §4). Prints the session id (the vibeterm pid) so later verbs
    /// can target it.
    Open(AiuiOpenArgs),

    /// Send key names and/or literal text to a running vibeterm session.
    Send(AiuiSendArgs),

    /// Read a symbolic text snapshot from a running vibeterm session.
    Snapshot(AiuiSnapshotArgs),

    /// Wait for a running vibeterm session to go idle — deterministic snapshots.
    Wait(AiuiWaitArgs),

    /// Close a running vibeterm session.
    Close(AiuiSessionArgs),

    /// Evaluate a JavaScript expression in the live vibeterm renderer page over
    /// CDP — read its REAL state (the xterm grid's cols/cell metrics, the
    /// scrollbar box) straight from the runtime, with no screenshot. Requires a
    /// `--control` session (PROP-042 §4).
    Inspect(AiuiInspectArgs),

    /// Stop the hosted program (the PTY child) WITHOUT restarting Electron —
    /// frees its binary for a rebuild. The renderer, the CDP endpoint, and the
    /// discovery file all stay live. Pair with `pty-start` for a fast TUI
    /// preview loop (PROP-042 §4).
    PtyStop(AiuiSessionArgs),

    /// (Re)spawn the hosted program at the current grid. Pair with `pty-stop`
    /// around a rebuild for a live TUI preview — the agent sees the change
    /// without reconnecting CDP or relaunching Electron (PROP-042 §4).
    PtyStart(AiuiSessionArgs),

    /// Set the scrollbar policy live: `auto` (hidden for a full-screen TUI,
    /// shown for a shell), `on` (always), `off` (never). The renderer refits the
    /// grid — no Electron restart. Requires a `--control` session (PROP-042 §4).
    Scrollbar(AiuiScrollbarArgs),
}

#[derive(Debug, Args)]
pub struct AiuiOpenArgs {
    /// The command vibeterm runs in its PTY (default: the console `vibe tree`
    /// against the current directory).
    #[arg(long)]
    pub exec: Option<String>,

    /// Terminal grid as `COLSxROWS` (passed to vibeterm).
    #[arg(long)]
    pub size: Option<String>,

    /// Show the OS window (default: headless). A visible control session lets a
    /// human watch and resize it live while the agent drives it.
    #[arg(long)]
    pub visible: bool,

    /// How long to wait (ms) for the control server's discovery file.
    #[arg(long, default_value_t = 8000)]
    pub timeout_ms: u64,
}

#[derive(Debug, Args)]
pub struct AiuiSendArgs {
    /// Key names to send in order, e.g. `F2 Down Enter` (case-insensitive).
    pub keys: Vec<String>,

    /// Literal text to type (sent after the keys).
    #[arg(long)]
    pub text: Option<String>,

    /// The session id (vibeterm pid); defaults to the most recent session.
    #[arg(long)]
    pub session: Option<u32>,
}

#[derive(Debug, Args)]
pub struct AiuiSnapshotArgs {
    /// The session id (vibeterm pid); defaults to the most recent session.
    #[arg(long)]
    pub session: Option<u32>,
}

#[derive(Debug, Args)]
pub struct AiuiWaitArgs {
    /// Consider the terminal idle after this many ms without PTY output.
    #[arg(long, default_value_t = 120)]
    pub idle_ms: u64,

    /// Give up waiting after this many ms.
    #[arg(long, default_value_t = 3000)]
    pub timeout_ms: u64,

    /// The session id (vibeterm pid); defaults to the most recent session.
    #[arg(long)]
    pub session: Option<u32>,
}

#[derive(Debug, Args)]
pub struct AiuiSessionArgs {
    /// The session id (vibeterm pid); defaults to the most recent session.
    #[arg(long)]
    pub session: Option<u32>,
}

#[derive(Debug, Args)]
pub struct AiuiInspectArgs {
    /// A JavaScript expression to evaluate in the live renderer page, e.g.
    /// `JSON.stringify({cols: term.cols, rows: term.rows})`. Its return value is
    /// printed as JSON.
    pub expr: String,

    /// The session id (vibeterm pid); defaults to the most recent session.
    #[arg(long)]
    pub session: Option<u32>,
}

#[derive(Debug, Args)]
pub struct AiuiScrollbarArgs {
    /// The scrollbar policy.
    #[arg(value_enum)]
    pub mode: ScrollbarMode,

    /// The session id (vibeterm pid); defaults to the most recent session.
    #[arg(long)]
    pub session: Option<u32>,
}

/// The `vibe aiui scrollbar` policy (PROP-042 §4).
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ScrollbarMode {
    /// Hidden for a full-screen TUI (alt-screen), shown for a shell.
    Auto,
    /// Always show the scrollbar.
    On,
    /// Never show the scrollbar.
    Off,
}

#[derive(Debug, Args)]
pub struct AiuiRenderArgs {
    /// Project root to analyse — the same resolver `vibe tree` uses.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// Terminal grid as `COLSxROWS`.
    #[arg(long, default_value = "80x24")]
    pub size: String,

    /// A space-separated key script to drive before snapshotting (PROP-042 §3),
    /// e.g. "F2 Down Enter". `F4`/`F6` are refused (side effects).
    #[arg(long, default_value = "")]
    pub send: String,

    /// Snapshot format: `text` (the glyph grid, golden-friendly) or `cells`
    /// (JSON runs with style).
    #[arg(long, value_enum, default_value_t = SnapFormat::Text)]
    pub format: SnapFormat,
}

/// The `vibe aiui render --format` choice (PROP-042 §2).
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SnapFormat {
    /// The glyph grid, one line per row.
    Text,
    /// JSON: run-length-encoded rows carrying fg/bg/modifiers.
    Cells,
}
