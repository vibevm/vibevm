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

mod flatten;
mod input;
mod menu;
mod modal;
mod modes;
mod render;
mod search;
mod state;
mod theme;

use anyhow::Result;
use rat_salsa::poll::PollCrossterm;
use rat_salsa::{Control, RunConfig, SalsaAppContext, run_tui};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_crossterm::crossterm::event::Event;

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
/// restore), so this never touches the terminal directly.
pub fn run(tree: PackageTree) -> Result<()> {
    let mut app = App::new(tree);
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
