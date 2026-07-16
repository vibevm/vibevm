//! The interactive `vibe prefs` settings TUI (PROP-041) — a **surface** over
//! PROP-040's data layer. It renders [`ResolvedPrefs`] and captures edits
//! through PROP-040's `inspect`/`get`/`list`/`set` (§1 `#surface-not-engine`);
//! it owns no preference logic, schema, or merge — those are PROP-040's. Built
//! from the PROP-037 component library (`ui::` facade) + the visual-language
//! [`Theme`](crate::commands::tree::tui::theme::Theme) (§1
//! `#built-on-tree-tui`): it composes the same components + glyph vocabulary as
//! the `vibe tree` TUI without re-inventing widgets.
//!
//! ## S1 — the foundation
//!
//! Phase S1 ships the rat-salsa wiring ([`run`]), the page registry
//! ([`registry`]), the built-in `vibe.tree.*` page declarations ([`settings`]),
//! the settings-tree widget ([`page_tree`]), the [`state::PrefsApp`] model, and
//! the draw + key pass. The right pane renders a **placeholder** panel for the
//! open page; the per-type edit form (§4 `#form-per-type`) is S2. The launch
//! entry lives in [`super`](crate::commands::prefs) (`vibe prefs ui`).
//!
//! ## rat-salsa 4.x
//!
//! [`run`] takes four `fn` pointers — `init` / `render` / `event` / `error` —
//! plus the global facilities and [`state::PrefsApp`], mirroring the tree TUI's
//! structure. The state owns the resolved prefs + schema + context, the page
//! registry, the fold set + selection, and the open page.

specmark::scope!("spec://vibevm/modules/vibe-settings/PROP-041#overview");

pub mod catalogue;
pub mod dispatch;
pub mod form;
pub mod input;
pub mod lint;
pub mod page_tree;
pub mod registry;
pub mod render;
pub mod search;
pub mod settings;
pub mod state;

use anyhow::Result;
use rat_salsa::poll::PollCrossterm;
use rat_salsa::{Control, RunConfig, SalsaAppContext, run_tui};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_crossterm::crossterm::event::Event;
use vibe_settings::resolver::ResolvedPrefs;
use vibe_settings::schema::Schema;

use state::{PrefsApp, PrefsCtx};

/// The rat-salsa application event. `PollCrossterm` turns every terminal event
/// into one of these via the `From` impl below; the base settings TUI carries
/// no application-specific events yet (S2 will add form-action events, §8).
pub enum AppEvent {
    /// A raw crossterm terminal event.
    Event(Event),
}

impl From<Event> for AppEvent {
    fn from(value: Event) -> Self {
        AppEvent::Event(value)
    }
}

/// rat-salsa's global facilities. The bare [`SalsaAppContext`] is all the
/// settings TUI needs (same as the tree TUI).
type Global = SalsaAppContext<AppEvent, anyhow::Error>;

/// Launch the settings TUI over a resolved snapshot + schema + session context
/// (PROP-041 §1). `run_tui` owns terminal setup/teardown (raw mode, alt-screen,
/// panic restore), so this never touches the terminal directly. A fatal error
/// captured during the loop is re-raised after the terminal is restored.
pub fn run(prefs: ResolvedPrefs, schema: Schema, ctx: PrefsCtx) -> Result<()> {
    let mut app = PrefsApp::new(prefs, schema, ctx);
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
    if let Some(err) = app.fatal.take() {
        return Err(err);
    }
    Ok(())
}

/// Select the first row so navigation and the highlight have an anchor.
fn init(app: &mut PrefsApp, _ctx: &mut Global) -> Result<()> {
    app.select_first();
    Ok(())
}

/// Draw one frame into the full-screen buffer.
fn render_frame(area: Rect, buf: &mut Buffer, app: &mut PrefsApp, _ctx: &mut Global) -> Result<()> {
    render::draw(area, buf, app);
    Ok(())
}

/// Route a terminal event to the key handler.
fn handle_event(
    event: &AppEvent,
    app: &mut PrefsApp,
    _ctx: &mut Global,
) -> Result<Control<AppEvent>> {
    match event {
        AppEvent::Event(inner) => input::handle(inner, app),
    }
}

/// Capture a fatal error and quit cleanly; [`run`] re-raises it afterward.
fn handle_error(
    err: anyhow::Error,
    app: &mut PrefsApp,
    _ctx: &mut Global,
) -> Result<Control<AppEvent>> {
    app.fatal = Some(err);
    Ok(Control::Quit)
}
