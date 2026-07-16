//! Argument struct for `vibe term` — launch the vibeterm terminal (PROP-042 §5
//! `#vibe-term`). Split from the `cli` hub; the hub re-exports it.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#command-summary");

use clap::Args;

#[derive(Debug, Args)]
pub struct TermArgs {
    /// The command to run in the terminal. Defaults to the detected interactive
    /// shell (Windows: PowerShell 7 `pwsh` if present, else Windows PowerShell
    /// 5.1; other platforms: `$SHELL`, else `/bin/sh`).
    #[arg(long)]
    pub exec: Option<String>,

    /// Terminal columns (passed through to vibeterm).
    #[arg(long)]
    pub cols: Option<u16>,

    /// Terminal rows (passed through to vibeterm).
    #[arg(long)]
    pub rows: Option<u16>,
}
