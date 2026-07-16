//! `vibe aiui` — the agent-facing observation surface (PROP-042). The **render
//! plane** (`render`) drives the `vibe tree` TUI headlessly to a symbolic
//! snapshot (no terminal). The **control plane** (`open`/`send`/`snapshot`/
//! `wait`/`close`) drives and observes a *running* vibeterm over its loopback
//! control server (§4). The **model plane** (`state`) projects the TUI state to a
//! serialisable `ModelView` (PROP-039 §11.2/§11.3).

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-042#aiui-cli");

mod cdp;
mod control;

use anyhow::{Result, anyhow};
use specmark::spec;

use crate::cli::{AiuiArgs, AiuiRenderArgs, AiuiStateArgs, AiuiSubcommand, SnapFormat};
use crate::output;

/// Run `vibe aiui …`.
pub fn run(_ctx: &output::Context, args: AiuiArgs) -> Result<()> {
    match args.command {
        AiuiSubcommand::Render(a) => render(a),
        AiuiSubcommand::State(a) => state(a),
        AiuiSubcommand::Open(a) => control::open(a),
        AiuiSubcommand::Send(a) => control::send(a),
        AiuiSubcommand::Snapshot(a) => control::snapshot(a),
        AiuiSubcommand::Wait(a) => control::wait(a),
        AiuiSubcommand::Close(a) => control::close(a),
        AiuiSubcommand::Inspect(a) => cdp::inspect(a),
        AiuiSubcommand::PtyStart(a) => control::pty_start(a),
        AiuiSubcommand::PtyStop(a) => control::pty_stop(a),
        AiuiSubcommand::Scrollbar(a) => control::scrollbar(a),
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

/// `vibe aiui state` — the model plane (PROP-039 §11.2/§11.3, PROP-042 §4):
/// build the tree model at `--path`, drive `--send`, print the serialised
/// `ModelView` — display mode, ordering, the active tab, the selection, the
/// visible rows, which modals are open. Structured state, never pixels; for
/// flow/state assertions with no rendering at all.
#[spec(implements = "spec://vibevm/modules/vibe-cli/PROP-042#aiui-cli")]
fn state(a: AiuiStateArgs) -> Result<()> {
    let view = super::tree::state(&a.path, &a.send)?;
    let json = serde_json::to_string_pretty(&view)?;
    println!("{json}");
    Ok(())
}

/// Parse a `COLSxROWS` grid spec (case-insensitive `x`), enforcing a floor.
pub(super) fn parse_size(s: &str) -> Result<(u16, u16)> {
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
