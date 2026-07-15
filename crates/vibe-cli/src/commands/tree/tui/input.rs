//! Key handling (PROP-036 §2.11). Navigation, folding, the detail modal, and
//! quit. The modal is captive: when open, only `Enter`/`Esc` act and everything
//! else is swallowed.

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-036#tui");

use anyhow::Result;
use rat_salsa::Control;
use rat_widget::event::ct_event;
use ratatui_crossterm::crossterm::event::Event;

use super::AppEvent;
use super::state::{App, RowNode};

/// Handle one terminal event, returning the rat-salsa control-flow verdict.
pub fn handle(event: &Event, app: &mut App) -> Result<Control<AppEvent>> {
    // The modal captures input while open (§2.11).
    if app.modal_open {
        return Ok(handle_modal(event, app));
    }

    let control = match event {
        ct_event!(key press 'q') => Control::Quit,
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
        ct_event!(key press 'F') | ct_event!(key press SHIFT - 'F') => {
            app.toggle_fold_all();
            Control::Changed
        }
        ct_event!(keycode press Enter) => {
            open_modal(app);
            Control::Changed
        }
        // Ordering (`n`) and display mode (`x`) — PROP-036 §2.11.
        ct_event!(key press 'n') => {
            app.cycle_ordering();
            Control::Changed
        }
        ct_event!(key press 'x') => {
            app.cycle_display_mode();
            Control::Changed
        }
        // Static/dynamic priority swap (`t`) — SubTables section order + Tabs order.
        ct_event!(key press 't') => {
            app.swap_priority();
            Control::Changed
        }
        // Tab switching (Tabs mode only; inert otherwise).
        ct_event!(keycode press Tab) | ct_event!(key press ']') => {
            app.next_tab();
            Control::Changed
        }
        ct_event!(key press '[') => {
            app.prev_tab();
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
