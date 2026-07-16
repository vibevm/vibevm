//! Key handling (PROP-036 §2.11, PROP-037 §5/§7/§13). Navigation, folding, the
//! detail modal, the F-key overlays, and quit. The modal and every overlay are
//! captive: when open, only their own keys act and everything else is swallowed.
//!
//! ## Base-mode routing (PROP-037 §5.1 `#keymap-registry`, §13.3 `#as-keymap`)
//!
//! At the base screen (no overlay open) the flow is:
//! 1. **F-key overlay commands** (F1 search, F2 sort, F3 mode, F6 copy) — these
//!    open overlays or run a surface-level copy that is not in the action
//!    catalogue, so they stay direct.
//! 2. **Keymap resolution** — convert the event to a `vibe_actions::Key` and
//!    `resolve` against the `vibe.tree` keymap built from [`search::catalogue`].
//!    On `Found(addr)` the shared [`dispatch::dispatch_by_addr`] applies the
//!    effect (the same function the Search Everywhere ACTIONS provider uses).
//! 3. **Direct tree-nav** — arrows pan, `Space` folds, `Enter` opens the card,
//!    `Shift+Left`/`Shift+Right` switch tabs, `Esc` opens the quit-confirm.
//!    These are navigation keys (exempt from the F-key scheme, PROP-037 §5.3);
//!    they may also be in the keymap, but the direct handling stays so
//!    navigation is instant and unaffected by the resolver's enablement gate.

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-036#tui");

use anyhow::Result;
use rat_salsa::Control;
use rat_widget::event::ct_event;
use ratatui_crossterm::crossterm::event::{Event, KeyCode, KeyEventKind};
use vibe_actions::Match;

use super::AppEvent;
use super::copy;
use super::dispatch;
use super::keymap_bridge;
use super::menu::{self, MenuState};
use super::search::{self, SearchState};
use super::state::{App, RowNode};

/// Handle one terminal event, returning the rat-salsa control-flow verdict.
pub fn handle(event: &Event, app: &mut App) -> Result<Control<AppEvent>> {
    // Any input clears a pending footer flash (PROP-037 §10).
    app.flash = None;

    // A terminal resize must repaint the whole surface. rat-salsa's event loop
    // renders only when a handler returns `Control::Changed`; it never
    // auto-repaints on resize (every rat-salsa example handles it explicitly),
    // so the old `_ => Control::Continue` default silently dropped
    // `Event::Resize` and left the display garbled until the next keypress. The
    // same drop also swallowed the resize crossterm emits at startup (on
    // entering the alternate screen), which is why the status line was missing
    // from the first frame. Handle it before the modal gate so a resize
    // repaints even while the detail popup is open (PROP-036 §2.11); ratatui
    // re-sizes its back-buffer on the next `draw`, so one `Changed` suffices.
    if let Event::Resize(..) = event {
        return Ok(Control::Changed);
    }

    // The Search Everywhere window captures input while open (PROP-037 §7.3).
    if app.search.is_some() {
        return Ok(handle_search(event, app));
    }

    // An F-key menu captures input while open (PROP-037 §7.1/§7.2).
    if app.menu.is_some() {
        return Ok(handle_menu(event, app));
    }

    // The modal captures input while open (§2.11).
    if app.modal_open {
        return Ok(handle_modal(event, app));
    }

    // The quit-confirm dialog captures input while open (PROP-037 §7.4).
    if app.confirm_quit {
        return Ok(handle_confirm_quit(event, app));
    }

    // F1 opens Search Everywhere; F2/F3 open the sort / mode menus (PROP-037 §7).
    if is_press_fkey(event, 1) {
        let state = SearchState::open(app);
        app.search = Some(state);
        return Ok(Control::Changed);
    }
    if is_press_fkey(event, 2) {
        let sort = MenuState::sort(app);
        app.menu = Some(sort);
        return Ok(Control::Changed);
    }
    if is_press_fkey(event, 3) {
        let mode = MenuState::mode(app);
        app.menu = Some(mode);
        return Ok(Control::Changed);
    }
    // F6 copies the current view to the clipboard as Markdown (PROP-037 §10).
    if is_press_fkey(event, 6) {
        let msg = copy::copy(app);
        app.flash = Some(msg);
        return Ok(Control::Changed);
    }

    // Keymap resolution (PROP-037 §13.3): convert the event to a Key and ask
    // the vibe.tree keymap which action it means. On a Found, dispatch by
    // address through the same path the Search Everywhere ACTIONS provider
    // uses; on NoMatch / NeedMoreChords, fall through to the direct tree-nav
    // keys below.
    if let Some(key) = keymap_bridge::event_to_key(event) {
        let km = search::catalogue::build_keymap();
        match km.resolve(std::slice::from_ref(&key), |addr| {
            dispatch::enabled_in_base(app, addr)
        }) {
            Match::Found(addr, _) => return Ok(dispatch::dispatch_by_addr(app, &addr)),
            Match::NoMatch | Match::NeedMoreChords => {}
        }
    }

    // Direct tree-nav keys — always handled here so navigation is instant and
    // unaffected by the resolver's enablement gate (PROP-037 §5.3). `Space`,
    // `Enter`, and `q` are also in the keymap (and handled there first when
    // their action is enabled); the direct handling below is the fallback for
    // when the resolver gates them off (e.g. Space outside its enablement).
    let control = match event {
        ct_event!(keycode press Up) => {
            move_up(app);
            Control::Changed
        }
        ct_event!(keycode press Down) => {
            move_down(app);
            Control::Changed
        }
        ct_event!(keycode press Left) => {
            app.h_offset = app.h_offset.saturating_sub(2);
            Control::Changed
        }
        ct_event!(keycode press Right) => {
            let max = app.max_name_width.saturating_sub(1);
            app.h_offset = app.h_offset.saturating_add(2).min(max);
            Control::Changed
        }
        ct_event!(key press ' ') => {
            app.toggle_fold_selected();
            Control::Changed
        }
        ct_event!(keycode press Enter) => {
            open_modal(app);
            Control::Changed
        }
        // Tab switching — Shift+Left / Shift+Right (Tabs mode only; inert
        // otherwise — `step_tab` guards on DisplayMode::Tabs). PROP-037 §5.3:
        // plain Left/Right stay tree horizontal-pan, which is why tab-switch
        // takes Shift. Also bound in the keymap as Tab/'[', but the Shift-arrow
        // path stays direct so it works even when the keymap gates tab.next off.
        ct_event!(keycode press SHIFT-Left) => {
            app.prev_tab();
            Control::Changed
        }
        ct_event!(keycode press SHIFT-Right) => {
            app.next_tab();
            Control::Changed
        }
        // Esc at the base screen opens the quit-confirm (PROP-037 §7.4).
        ct_event!(keycode press Esc) => {
            app.confirm_quit = true;
            Control::Changed
        }
        _ => Control::Continue,
    };
    Ok(control)
}

/// The captive modal handler: `Enter`/`Esc` close; everything else is swallowed.
fn handle_modal(event: &Event, app: &mut App) -> Control<AppEvent> {
    match event {
        ct_event!(keycode press Esc) | ct_event!(keycode press Enter) => {
            app.modal_open = false;
            Control::Changed
        }
        _ => Control::Unchanged,
    }
}

/// The captive quit-confirm handler (PROP-037 §7.4): `Enter` quits, `Esc`
/// cancels; everything else is swallowed.
fn handle_confirm_quit(event: &Event, app: &mut App) -> Control<AppEvent> {
    match event {
        ct_event!(keycode press Enter) => Control::Quit,
        ct_event!(keycode press Esc) => {
            app.confirm_quit = false;
            Control::Changed
        }
        _ => Control::Unchanged,
    }
}

/// True for an `F<n>` key-press event.
fn is_press_fkey(event: &Event, n: u8) -> bool {
    matches!(event, Event::Key(k) if k.code == KeyCode::F(n) && k.kind == KeyEventKind::Press)
}

/// The captive Search Everywhere handler (PROP-037 §7.3): typing filters, Up/Down
/// move the selection, `Tab`/`Shift+Tab` cycle the category tabs, `Enter` runs the
/// selection, `Esc` closes.
fn handle_search(event: &Event, app: &mut App) -> Control<AppEvent> {
    let Event::Key(k) = event else {
        return Control::Unchanged;
    };
    if k.kind != KeyEventKind::Press {
        return Control::Unchanged;
    }
    match k.code {
        KeyCode::Esc => {
            app.search = None;
            Control::Changed
        }
        KeyCode::Enter => search::confirm(app),
        KeyCode::Up => with_search(app, |s| s.select_up()),
        KeyCode::Down => with_search(app, |s| s.select_down()),
        KeyCode::Tab => with_search(app, |s| s.next_tab()),
        KeyCode::BackTab => with_search(app, |s| s.prev_tab()),
        KeyCode::Backspace => with_search(app, |s| s.backspace()),
        KeyCode::Char(c) => with_search(app, move |s| s.type_char(c)),
        _ => Control::Unchanged,
    }
}

/// Run a mutation on the open search window and request a repaint.
fn with_search(app: &mut App, f: impl FnOnce(&mut SearchState)) -> Control<AppEvent> {
    if let Some(state) = app.search.as_mut() {
        f(state);
    }
    Control::Changed
}

/// The captive F-key menu handler (PROP-037 §7.1/§7.2): Up/Down move, `Enter`
/// applies the highlighted option, `Esc` closes.
fn handle_menu(event: &Event, app: &mut App) -> Control<AppEvent> {
    let Event::Key(k) = event else {
        return Control::Unchanged;
    };
    if k.kind != KeyEventKind::Press {
        return Control::Unchanged;
    }
    match k.code {
        KeyCode::Esc => {
            app.menu = None;
            Control::Changed
        }
        KeyCode::Enter => {
            menu::confirm(app);
            Control::Changed
        }
        KeyCode::Up => {
            if let Some(m) = app.menu.as_mut() {
                m.select_up();
            }
            Control::Changed
        }
        KeyCode::Down => {
            if let Some(m) = app.menu.as_mut() {
                m.select_down();
            }
            Control::Changed
        }
        _ => Control::Unchanged,
    }
}

/// Move the selection up one row, keeping it visible.
fn move_up(app: &mut App) {
    if app.rows.is_empty() {
        return;
    }
    if app.table.selected().is_none() {
        app.table.select(Some(0));
        return;
    }
    app.table.move_up(1);
    app.table.scroll_to_selected();
}

/// Move the selection down one row, keeping it visible.
fn move_down(app: &mut App) {
    if app.rows.is_empty() {
        return;
    }
    if app.table.selected().is_none() {
        app.table.select(Some(0));
        return;
    }
    app.table.move_down(1);
    app.table.scroll_to_selected();
}

/// Open the detail modal, unless the selected row is a bare separator.
fn open_modal(app: &mut App) {
    let has_detail = matches!(
        app.selected_row().map(|r| r.node),
        Some(RowNode::Package(_)) | Some(RowNode::Missing)
    );
    if has_detail {
        app.modal_open = true;
    }
}
