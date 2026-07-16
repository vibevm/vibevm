//! The interactive `vibe tree` TUI (PROP-036 §2.11), on the rat-salsa 4.x
//! event-loop + rat-widget table stack.
//!
//! rat-salsa 4.x is not trait-based: [`run_tui`] takes four `fn` pointers —
//! `init` / `render` / `event` / `error` — plus a global facilities struct and
//! the application state. Here the state is [`App`] (the model, the derived
//! visible rows, the fold set, the selection, the pan, and the modal flag) and
//! the global is the bare [`SalsaAppContext`] — no extra facilities are needed.
//!
//! Submodules keep every file under the discipline's 600-line budget:
//! [`state`] (state + flatten), [`render`] (draw), [`input`] (keys), and
//! [`modal`] (the detail popup).

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-036#tui");

mod copy;
mod dispatch;
mod flatten;
mod input;
/// The crossterm → `vibe_actions::Key` bridge is shared with the `vibe prefs`
/// TUI (PROP-041 §8 `#commands-are-actions` reuses the same keymap resolver),
/// so it is crate-visible.
pub(crate) mod keymap_bridge;
mod keyscript;
mod menu;
mod modal;
/// `pub(crate)` — the model-plane projection is read by the `vibe aiui state`
/// handler (PROP-039 §11.2/§11.3, prototyped on the TUI).
pub(crate) mod model_view;
mod modes;
mod render;
mod row;
mod search;
mod snapshot;
// `pub(crate)` — the `vibe.tree.*` schema + palette/tier mapping is read by the
// `vibe prefs` settings TUI (PROP-041) so the two surfaces share one theme.
pub(crate) mod settings;
mod shape;
mod state;
// `pub(crate)` — shared with the `vibe prefs` settings TUI (PROP-041 §1
// #built-on-tree-tui): it composes the same `Theme` + glyph vocabulary + `ui::`
// component library instead of re-inventing them.
pub(crate) mod theme;
pub(crate) mod ui;

use anyhow::Result;
use rat_salsa::poll::PollCrossterm;
use rat_salsa::{Control, RunConfig, SalsaAppContext, run_tui};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_crossterm::crossterm::event::Event;
use specmark::spec;

use super::model::PackageTree;
use state::App;

/// The rat-salsa application event. `PollCrossterm` turns every terminal event
/// into one of these via the `From` impl below; the base TUI carries no
/// application-specific events yet.
pub enum AppEvent {
    /// A raw crossterm terminal event.
    Event(Event),
}

impl From<Event> for AppEvent {
    fn from(value: Event) -> Self {
        AppEvent::Event(value)
    }
}

/// rat-salsa's global facilities. The bare [`SalsaAppContext`] is all the base
/// TUI needs — it is both the concrete global type and its own `SalsaContext`
/// implementation.
type Global = SalsaAppContext<AppEvent, anyhow::Error>;

/// Launch the interactive TUI over an already-built model (PROP-036 §2.11).
///
/// `run_tui` owns terminal setup and teardown (raw mode, alt-screen, and panic
/// restore), so this never touches the terminal directly. The active theme
/// (palette + tier) and the persisted UI state (mode / sort / shape /
/// static-first) are loaded from the `vibe.tree.*` settings before the loop
/// starts (PROP-037 §9); a missing or corrupt settings file falls back to the
/// built-in defaults and is swallowed + warned, never a hard error.
pub fn run(tree: PackageTree) -> Result<()> {
    let mut app = App::new(tree);
    app.apply_prefs(settings::TreeSettings::new());
    let mut global = Global::default();
    run_tui(
        init,
        render_frame,
        handle_event,
        handle_error,
        &mut global,
        &mut app,
        RunConfig::default()?.poll(PollCrossterm),
    )?;
    // A failure captured during the loop is re-raised now that the terminal is
    // restored (returning `Err` from the error handler would re-enter it).
    if let Some(err) = app.fatal.take() {
        return Err(err);
    }
    Ok(())
}

/// Render the `vibe tree` TUI headlessly to a snapshot string — the AIUI render
/// plane (PROP-042 §1 `#render-plane`). Builds a fresh [`App`] over `tree` (theme
/// defaults, no settings load → deterministic), drives the `send` key script
/// through the real [`input::handle`], paints one frame into an off-screen
/// [`Buffer`] of `cols×rows`, and projects it to the `text` (or `cells`)
/// snapshot (§2). No terminal, no alt-screen, no rat-salsa loop.
#[spec(implements = "spec://vibevm/modules/vibe-cli/PROP-042#render-plane")]
pub(crate) fn snapshot_headless(
    tree: PackageTree,
    cols: u16,
    rows: u16,
    send: &str,
    cells: bool,
) -> Result<String> {
    let script = keyscript::parse(send)?;
    let mut app = App::new(tree);
    if !app.rows.is_empty() {
        app.table.select(Some(0));
    }
    for ev in &script {
        // Drive the model; the `Control` verdict is irrelevant headlessly.
        let _ = input::handle(ev, &mut app)?;
    }
    let area = Rect::new(0, 0, cols, rows);
    let mut buf = Buffer::empty(area);
    render::draw(area, &mut buf, &mut app);
    Ok(if cells {
        serde_json::to_string_pretty(&snapshot::to_cells(&buf))?
    } else {
        snapshot::to_text(&buf)
    })
}

/// Project the `vibe tree` TUI state headlessly to a serialisable
/// [`model_view::TreeModelView`] — the AIUI model plane (PROP-039 §11.2/§11.3,
/// PROP-042 §4 `state`). Builds a fresh [`App`] over `tree` (theme defaults, no
/// settings load → deterministic), drives the `send` key script through the real
/// [`input::handle`], and projects the resulting state. The semantic sibling of
/// [`snapshot_headless`]: same `(tree, script)`, but the model instead of the
/// glyph grid — for flow/state assertions with no rendering at all.
#[spec(implements = "spec://vibevm/modules/vibe-cli/PROP-042#aiui-cli")]
pub(crate) fn state_headless(tree: PackageTree, send: &str) -> Result<model_view::TreeModelView> {
    let script = keyscript::parse(send)?;
    let mut app = App::new(tree);
    if !app.rows.is_empty() {
        app.table.select(Some(0));
    }
    for ev in &script {
        // Drive the model; the `Control` verdict is irrelevant headlessly.
        let _ = input::handle(ev, &mut app)?;
    }
    Ok(app.model_view())
}

/// Select the first row so navigation and the highlight have an anchor.
fn init(app: &mut App, _ctx: &mut Global) -> Result<()> {
    if !app.rows.is_empty() {
        app.table.select(Some(0));
    }
    Ok(())
}

/// Draw one frame into the full-screen buffer.
fn render_frame(area: Rect, buf: &mut Buffer, app: &mut App, _ctx: &mut Global) -> Result<()> {
    render::draw(area, buf, app);
    Ok(())
}

/// Route a terminal event to the key handler.
fn handle_event(event: &AppEvent, app: &mut App, _ctx: &mut Global) -> Result<Control<AppEvent>> {
    match event {
        AppEvent::Event(inner) => input::handle(inner, app),
    }
}

/// Capture a fatal error and quit cleanly; [`run`] re-raises it afterward.
fn handle_error(err: anyhow::Error, app: &mut App, _ctx: &mut Global) -> Result<Control<AppEvent>> {
    app.fatal = Some(err);
    Ok(Control::Quit)
}
