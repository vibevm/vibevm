//! `vibe aiui` — the agent-facing observation surface (PROP-042). The
//! render-plane verb (`render`) drives the `vibe tree` TUI headlessly and prints
//! a symbolic snapshot; terminal-plane and model-plane verbs land in later
//! campaign phases.

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-042#aiui-cli");

use anyhow::{Result, anyhow};
use specmark::spec;

use crate::cli::{AiuiArgs, AiuiRenderArgs, AiuiSubcommand, SnapFormat};
use crate::output;

/// Run `vibe aiui …`.
pub fn run(_ctx: &output::Context, args: AiuiArgs) -> Result<()> {
    match args.command {
        AiuiSubcommand::Render(a) => render(a),
    }
}

/// `vibe aiui render` — the render plane (PROP-042 §1/§4): build the tree model
/// at `--path`, drive `--send` at `--size`, print the `--format` snapshot.
#[spec(implements = "spec://vibevm/modules/vibe-cli/PROP-042#aiui-cli")]
fn render(a: AiuiRenderArgs) -> Result<()> {
    let (cols, rows) = parse_size(&a.size)?;
    let cells = matches!(a.format, SnapFormat::Cells);
    let out = super::tree::snapshot(&a.path, cols, rows, &a.send, cells)?;
    print!("{out}");
    Ok(())
}

/// Parse a `COLSxROWS` grid spec (case-insensitive `x`).
fn parse_size(s: &str) -> Result<(u16, u16)> {
    let (c, r) = s
        .split_once(['x', 'X'])
        .ok_or_else(|| anyhow!("--size must be COLSxROWS, got `{s}`"))?;
    let cols: u16 = c
        .trim()
        .parse()
        .map_err(|_| anyhow!("bad column count in --size `{s}`"))?;
    let rows: u16 = r
        .trim()
        .parse()
        .map_err(|_| anyhow!("bad row count in --size `{s}`"))?;
    if cols < 20 || rows < 5 {
        return Err(anyhow!("--size too small (min 20x5), got `{s}`"));
    }
    Ok((cols, rows))
}
