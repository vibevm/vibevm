//! The file-destination modal (PROP-037 §10.5 `#copy-dest`): a [`TextField`]
//! path entry plus `Save` / `Cancel` [`Button`]s. `Tab`/arrows cycle focus among
//! the three (path → Save → Cancel), `Enter` on a button acts (Save writes the
//! markdown to the path; Cancel closes back to copy-settings), `Esc` cancels
//! back to the copy-settings modal (the depth-2 cascade: this modal is drawn
//! over copy-settings and captures input first while open).
//!
//! The path field is plain text for now (PROP-037 §10.5 — a later REQ enriches
//! it with validation / picker). The [`TextField`] primitive gives the
//! append-only `type_char`/`backspace` edit + the `█` cursor render.

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-037#copy-dest");

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::{Constraint, Layout, Rect};
use ratatui_core::text::Line;
use specmark::spec;

use super::super::theme::Theme;
use super::super::ui::{Button, TextField, Window};

/// The three focusable controls, in cycle order.
const FOCUS_PATH: usize = 0;
const FOCUS_SAVE: usize = 1;
const FOCUS_CANCEL: usize = 2;

/// The open file-destination modal (PROP-037 §10.5). Owns the path string and
/// which of the three controls (path field / Save / Cancel) holds focus.
#[derive(Debug, Clone)]
pub struct FileDest {
    /// The path the user is typing.
    path: String,
    /// `0` = path field, `1` = Save, `2` = Cancel.
    focus: usize,
}

impl FileDest {
    /// Build the modal with an empty path and the path field focused (the user
    /// opens it to type a path, so the field is the natural default).
    #[must_use]
    pub fn new() -> Self {
        Self {
            path: String::new(),
            focus: FOCUS_PATH,
        }
    }

    /// The current path value.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Append `c` to the path (only when the path field is focused — typing
    /// while a button is focused is a no-op).
    pub fn type_char(&mut self, c: char) {
        if self.focus == FOCUS_PATH {
            self.path.push(c);
        }
    }

    /// Delete the last character of the path (Backspace, path-field-focused).
    pub fn backspace(&mut self) {
        if self.focus == FOCUS_PATH {
            self.path.pop();
        }
    }

    /// Cycle focus forward (Tab): path → Save → Cancel → path.
    pub fn focus_next(&mut self) {
        self.focus = (self.focus + 1) % 3;
    }

    /// Cycle focus backward (Shift+Tab): path → Cancel → Save → path.
    pub fn focus_prev(&mut self) {
        self.focus = (self.focus + 2) % 3;
    }

    /// Whether the path field holds focus.
    #[must_use]
    pub fn is_path_focused(&self) -> bool {
        self.focus == FOCUS_PATH
    }

    /// Whether the Save button holds focus.
    #[must_use]
    pub fn is_save_focused(&self) -> bool {
        self.focus == FOCUS_SAVE
    }

    /// Whether the Cancel button holds focus.
    #[must_use]
    pub fn is_cancel_focused(&self) -> bool {
        self.focus == FOCUS_CANCEL
    }

    /// Advance focus to the Save button (Enter while the path field is focused —
    /// pressing Enter in the path field is the intuitive "I'm done typing").
    pub fn advance_to_save(&mut self) {
        self.focus = FOCUS_SAVE;
    }

    /// Render the modal centred over `area` (PROP-037 §10.5): a titled
    /// [`Window`], a "Path:" label + the [`TextField`], a one-row gap, the
    /// `Save` / `Cancel` [`Button`]s side by side, and a key hint on the last
    /// row. The focused control carries its theme highlight (the `█` cursor for
    /// the field, the accent bar for a button).
    #[spec(implements = "spec://vibevm/modules/vibe-cli/PROP-037#copy-dest")]
    pub fn render(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        if area.width < 28 || area.height < 9 {
            return;
        }
        let width = 48u16.min(area.width.saturating_sub(2));
        let height = 9u16.min(area.height);
        let inner = Window::centered(
            area,
            buf,
            Line::styled(" Copy to file ", theme.title()),
            width,
            height,
            theme,
        );

        // label row, field row, gap, button row, hint row.
        let [label_row, field_row, _gap, btn_row, hint_row] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .areas(inner);

        buf.set_stringn(
            label_row.x,
            label_row.y,
            "Path:",
            label_row.width as usize,
            theme.dim(),
        );

        let field = TextField::new()
            .focused(self.is_path_focused())
            .with_value(self.path.clone());
        field.render(field_row, buf, theme);

        // Save / Cancel side by side, centred as a pair.
        let save = Button::new("Save").focused(self.is_save_focused());
        let cancel = Button::new("Cancel").focused(self.is_cancel_focused());
        let pair_w = save
            .width()
            .saturating_add(cancel.width())
            .saturating_add(2);
        let start_x = btn_row.x + btn_row.width.saturating_sub(pair_w) / 2;
        save.render(Rect::new(start_x, btn_row.y, save.width(), 1), buf, theme);
        cancel.render(
            Rect::new(start_x + save.width() + 2, btn_row.y, cancel.width(), 1),
            buf,
            theme,
        );

        buf.set_stringn(
            hint_row.x,
            hint_row.y,
            " type path  \u{2022}  Tab  \u{2022}  Enter  \u{2022}  Esc",
            hint_row.width as usize,
            theme.dim(),
        );
    }
}

impl Default for FileDest {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui_core::layout::Position;

    #[test]
    fn new_starts_empty_with_the_path_field_focused() {
        let fd = FileDest::new();
        assert_eq!(fd.path(), "");
        assert!(fd.is_path_focused());
        assert!(!fd.is_save_focused());
        assert!(!fd.is_cancel_focused());
    }

    #[test]
    fn typing_and_backspace_edit_the_path_only_when_focused() {
        let mut fd = FileDest::new();
        for c in "/tmp/x.md".chars() {
            fd.type_char(c);
        }
        assert_eq!(fd.path(), "/tmp/x.md");
        fd.backspace();
        assert_eq!(fd.path(), "/tmp/x.m");
        // Move focus to Save; typing is now a no-op.
        fd.advance_to_save();
        assert!(fd.is_save_focused());
        fd.type_char('!');
        assert_eq!(
            fd.path(),
            "/tmp/x.m",
            "typing ignored while Save is focused"
        );
    }

    #[test]
    fn tab_cycles_path_save_cancel() {
        let mut fd = FileDest::new();
        assert!(fd.is_path_focused());
        fd.focus_next();
        assert!(fd.is_save_focused());
        fd.focus_next();
        assert!(fd.is_cancel_focused());
        fd.focus_next();
        assert!(fd.is_path_focused(), "wraps back to path");
    }

    #[test]
    fn shift_tab_cycles_backward() {
        let mut fd = FileDest::new();
        fd.focus_prev();
        assert!(fd.is_cancel_focused(), "path → Cancel going backward");
        fd.focus_prev();
        assert!(fd.is_save_focused());
    }

    #[test]
    fn render_paints_the_text_field_and_both_buttons() {
        let theme = Theme::default();
        let mut fd = FileDest::new();
        for c in "out.md".chars() {
            fd.type_char(c);
        }
        let area = Rect::new(0, 0, 52, 11);
        let mut buf = Buffer::empty(area);
        fd.render(area, &mut buf, &theme);
        // The path value is rendered.
        let has_path = (0..area.width)
            .any(|x| (0..area.height).any(|y| buf[Position::new(x, y)].symbol() == "o"));
        assert!(has_path, "the path field value is rendered");
        // The Save and Cancel labels appear.
        let has_save = (0..area.width)
            .any(|x| (0..area.height).any(|y| buf[Position::new(x, y)].symbol() == "S"));
        let has_cancel = (0..area.width)
            .any(|x| (0..area.height).any(|y| buf[Position::new(x, y)].symbol() == "C"));
        assert!(has_save, "the Save button is rendered");
        assert!(has_cancel, "the Cancel button is rendered");
    }
}
