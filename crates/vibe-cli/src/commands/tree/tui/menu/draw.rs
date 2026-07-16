//! The F-key menu rendering (PROP-037 §5.4/§7.1/§7.2). Split out of [`super`]
//! along the model-vs-view seam so the model + navigation + `confirm` policy
//! stays under the 600-line budget. [`draw`] is the single entry `render.rs`
//! calls; [`draw_groups`] lays out the focus groups (the active one accent-
//! framed), [`draw_option`] stamps one option row through the theme vocabulary.

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-037#f2-sort-menu");

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::text::Line;

use super::super::state::App;
use super::super::theme::Theme;
use super::super::ui::{ComingSoon, Group, Window};
use super::{MenuGroup, MenuKind, MenuOption};

/// Draw the menu centered over `area` (drawn after the base, before nothing —
/// the card / search windows are separate captive modes, never open together).
///
/// `pub` so [`super`] can re-export it as `menu::draw` for `render.rs`; the
/// `draw` module itself is private, so this fn is only reachable through that
/// re-export.
pub fn draw(area: Rect, buf: &mut Buffer, app: &App) {
    let Some(menu) = app.menu.as_ref() else {
        return;
    };
    if area.width < 20 || area.height < 5 {
        return;
    }
    let theme = &app.theme;
    match &menu.kind {
        MenuKind::ComingSoon => {
            ComingSoon::new(&menu.title).render(area, buf, theme);
        }
        MenuKind::Groups {
            groups,
            active_group,
            ..
        } => {
            draw_groups(area, buf, &menu.title, groups, *active_group, theme);
        }
    }
}

/// Lay the groups out inside a centered [`Window`]. A single group (F3) renders
/// as a flat list with no group chrome (§7.2 "no group chrome needed"); two or
/// more (F2) frame each group with a [`Group`] whose name sits top-right and
/// whose focused/unfocused state marks the active focus-group (PROP-037 §5.4).
fn draw_groups(
    area: Rect,
    buf: &mut Buffer,
    title: &str,
    groups: &[MenuGroup],
    active_group: usize,
    theme: &Theme,
) {
    let multi = groups.len() > 1;
    let label_w = groups
        .iter()
        .flat_map(|g| g.options.iter().map(|o| o.label.chars().count()))
        .chain(groups.iter().map(|g| g.name.chars().count()))
        .chain(std::iter::once(title.chars().count()))
        .max()
        .unwrap_or(10);
    let w = (label_w as u16 + 8).clamp(24, area.width.saturating_sub(4));

    // Inner content height: the hint row + the options + group framing.
    let total_opts: usize = groups.iter().map(|g| g.options.len()).sum();
    let body_h = if multi {
        // each group = 2 border + options; one-row gaps between groups; + hint row
        groups
            .iter()
            .map(|g| g.options.len() + 2)
            .sum::<usize>()
            .saturating_add(groups.len().saturating_sub(1))
            + 1
    } else {
        total_opts + 3 // a blank row + options + hint row
    };
    let h = (body_h as u16 + 2) // + the window's own two border rows
        .clamp(5, area.height.saturating_sub(2));

    let inner = Window::centered(
        area,
        buf,
        Line::styled(format!(" {title} "), theme.title()),
        w,
        h,
        theme,
    );
    let hint_row = inner.y + inner.height.saturating_sub(1);

    if multi {
        let mut y = inner.y;
        for (gi, group) in groups.iter().enumerate() {
            let gh = group.options.len() as u16 + 2;
            if y + gh > hint_row {
                break;
            }
            let garea = Rect::new(inner.x, y, inner.width, gh);
            // The active focus-group is accent-framed; the rest render dim
            // (PROP-037 §5.4 — the user sees where `Tab` has landed).
            let ginner = Group::named(&group.name)
                .focused(gi == active_group)
                .render(garea, buf, theme);
            for (oi, option) in group.options.iter().enumerate() {
                let oy = ginner.y + oi as u16;
                if oy >= hint_row {
                    break;
                }
                let rect = Rect::new(ginner.x, oy, ginner.width, 1);
                // Only the active group shows a highlight bar (`Enter`'s target);
                // inactive groups show their `●`/`○` value marks only.
                let is_cursor = gi == active_group && oi == group.cursor;
                draw_option(rect, buf, option, is_cursor, theme);
            }
            y += gh + 1; // a one-row gap between framed groups
        }
    } else {
        // Single group: flat list, no group chrome (preserves the F3 look).
        let group = &groups[0];
        let list_top = inner.y + 1; // a blank row under the title
        for (i, option) in group.options.iter().enumerate() {
            let y = list_top + i as u16;
            if y >= hint_row {
                break;
            }
            let rect = Rect::new(inner.x + 1, y, inner.width.saturating_sub(2), 1);
            draw_option(rect, buf, option, i == group.cursor, theme);
        }
    }

    // The key hint on the last inner row. Multi-group advertises `Tab` for
    // focus-group cycling (PROP-037 §5.4); single-group omits it.
    let hint = if multi {
        " \u{2191}/\u{2193}  \u{2022}  Tab  \u{2022}  Enter  \u{2022}  Esc"
    } else {
        " \u{2191}/\u{2193}  \u{2022}  Enter  \u{2022}  Esc"
    };
    buf.set_stringn(
        inner.x + 1,
        hint_row,
        hint,
        inner.width.saturating_sub(2) as usize,
        theme.dim(),
    );
}

/// Draw one option row: the theme on/off mark plus the label, on the selection
/// bar when this is the cursor row. Marks come from the theme vocabulary
/// (`flag_on`/`flag_off` glyphs) — never a literal.
fn draw_option(rect: Rect, buf: &mut Buffer, option: &MenuOption, is_cursor: bool, theme: &Theme) {
    let mark = if option.checked {
        theme.glyphs().flag_on
    } else {
        theme.glyphs().flag_off
    };
    if is_cursor {
        buf.set_style(rect, theme.selection());
        buf.set_stringn(
            rect.x,
            rect.y,
            format!("{mark} {}", option.label),
            rect.width as usize,
            theme.selection(),
        );
    } else {
        let mark_style = if option.checked {
            theme.accent()
        } else {
            theme.dim()
        };
        buf.set_stringn(rect.x, rect.y, mark, rect.width as usize, mark_style);
        buf.set_stringn(
            rect.x + 2,
            rect.y,
            &option.label,
            rect.width.saturating_sub(2) as usize,
            theme.text(),
        );
    }
}
