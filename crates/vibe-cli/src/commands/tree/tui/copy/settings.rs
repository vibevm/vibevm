//! The copy-settings modal (PROP-037 §10.2 `#copy-flow`): two `RadioGroup`s —
//! **format** (Markdown / PNG) and **destination** (clipboard / file) — with
//! `↑`/`↓` moving the selection within the focused group, `Tab`/`Shift+Tab`
//! cycling focus between the two groups, `Enter` confirming, `Esc` cancelling.
//! Confirming with destination = file pushes the [`super::FileDest`] modal over
//! this one (the depth-2 cascade); selecting PNG routes to ComingSoon (§10.4).
//!
//! The modal owns two [`RadioGroup`]s directly — their navigation and render are
//! reused as-is (PROP-037 §2.7); this struct adds only the focus-between-groups
//! policy and the value accessors the confirm logic reads.

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-037#copy-flow");

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::text::Line;
use specmark::spec;

use super::super::theme::Theme;
use super::super::ui::{RadioGroup, Window};

/// The copy format (PROP-037 §10.2). Markdown is real today; PNG is reserved
/// (§10.4 — routes to the ComingSoon placeholder).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyFormat {
    /// Serialize the screen as Markdown (§10.3).
    Markdown,
    /// Rasterize to PNG — reserved; routes to ComingSoon (§10.4).
    Png,
}

/// The copy destination (PROP-037 §10.2/§10.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyDest {
    /// Write to the system clipboard (the default).
    Clipboard,
    /// Write to a file — pushes the file-path modal (§10.5).
    File,
}

/// The open copy-settings modal (PROP-037 §10.2). Owns the two radio groups and
/// which one currently holds the `↑`/`↓` focus.
#[derive(Debug, Clone)]
pub struct CopySettings {
    /// 0 = format group focused, 1 = destination group focused.
    focus: usize,
    format: RadioGroup,
    dest: RadioGroup,
}

impl CopySettings {
    /// Build the modal with both groups at their first option (Markdown /
    /// clipboard — the defaults and the common case).
    #[must_use]
    pub fn new() -> Self {
        Self {
            focus: 0,
            format: RadioGroup::new("Format", vec!["Markdown".into(), "PNG".into()]),
            dest: RadioGroup::new("Destination", vec!["Clipboard".into(), "File".into()]),
        }
    }

    /// The selected format.
    #[must_use]
    pub fn format_value(&self) -> CopyFormat {
        if self.format.selected_index() == 1 {
            CopyFormat::Png
        } else {
            CopyFormat::Markdown
        }
    }

    /// The selected destination.
    #[must_use]
    pub fn dest_value(&self) -> CopyDest {
        if self.dest.selected_index() == 1 {
            CopyDest::File
        } else {
            CopyDest::Clipboard
        }
    }

    /// Move the selection up within the focused group.
    pub fn select_up(&mut self) {
        self.focused_group_mut().select_up();
    }

    /// Move the selection down within the focused group.
    pub fn select_down(&mut self) {
        self.focused_group_mut().select_down();
    }

    /// Cycle focus to the next group (Tab).
    pub fn focus_next(&mut self) {
        self.focus = (self.focus + 1) % 2;
    }

    /// Cycle focus to the previous group (Shift+Tab).
    pub fn focus_prev(&mut self) {
        self.focus = (self.focus + 1) % 2;
    }

    /// The focused group (0 = format, 1 = destination).
    #[allow(dead_code)] // introspection; exercised in tests.
    #[must_use]
    pub fn focused_group(&self) -> usize {
        self.focus
    }

    /// Borrow the focused radio group mutably for navigation.
    fn focused_group_mut(&mut self) -> &mut RadioGroup {
        match self.focus {
            0 => &mut self.format,
            _ => &mut self.dest,
        }
    }

    /// Render the modal centred over `area` (PROP-037 §10.2): a titled
    /// [`Window`] with the two radio groups stacked, the focused group marked
    /// with the theme `▸` focus arrow, and a key hint on the last row.
    #[spec(implements = "spec://vibevm/modules/vibe-cli/PROP-037#copy-flow")]
    pub fn render(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        if area.width < 24 || area.height < 11 {
            return;
        }
        // Each group renders label(1) + 2 options = 3 rows; plus a one-row gap
        // between them and the hint row. Window adds its own two border rows.
        let width = 32u16.min(area.width.saturating_sub(2));
        let height = 11u16.min(area.height);
        let inner = Window::centered(
            area,
            buf,
            Line::styled(" Copy ", theme.title()),
            width,
            height,
            theme,
        );

        // Format group at the top of the inner rect.
        let fmt_area = Rect::new(inner.x, inner.y, inner.width, 3);
        render_section(&self.format, fmt_area, buf, theme, self.focus == 0);

        // Destination group one row below the format group's last option.
        let dest_y = inner.y + 4;
        let dest_area = Rect::new(inner.x, dest_y, inner.width, 3);
        render_section(&self.dest, dest_area, buf, theme, self.focus == 1);

        // Key hint on the last inner row.
        let hint_y = inner.y + inner.height.saturating_sub(1);
        buf.set_stringn(
            inner.x,
            hint_y,
            " \u{2191}/\u{2193}  \u{2022}  Tab  \u{2022}  Enter  \u{2022}  Esc",
            inner.width as usize,
            theme.dim(),
        );
    }
}

impl Default for CopySettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Render one radio group with a focus marker: the theme `▸` glyph
/// (`fold_collapsed`) in accent marks the focused group, a space the unfocused
/// one. The [`RadioGroup`] renders into a rect inset by two cells so the marker
/// sits clear of its label.
fn render_section(group: &RadioGroup, area: Rect, buf: &mut Buffer, theme: &Theme, focused: bool) {
    let marker = if focused {
        theme.glyphs().fold_collapsed
    } else {
        " "
    };
    let marker_style = if focused { theme.accent() } else { theme.dim() };
    buf.set_stringn(area.x, area.y, marker, area.width as usize, marker_style);
    let inset = Rect {
        x: area.x + 2,
        width: area.width.saturating_sub(2),
        ..area
    };
    group.render(inset, buf, theme);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui_core::layout::Position;

    #[test]
    fn defaults_are_markdown_and_clipboard_with_format_focused() {
        let cs = CopySettings::new();
        assert_eq!(cs.format_value(), CopyFormat::Markdown);
        assert_eq!(cs.dest_value(), CopyDest::Clipboard);
        assert_eq!(cs.focused_group(), 0, "format group focused by default");
    }

    #[test]
    fn navigation_moves_the_selection_within_the_focused_group() {
        let mut cs = CopySettings::new();
        // Format group focused: select_down → PNG.
        cs.select_down();
        assert_eq!(cs.format_value(), CopyFormat::Png);
        assert_eq!(cs.dest_value(), CopyDest::Clipboard, "dest untouched");
        cs.select_up();
        assert_eq!(cs.format_value(), CopyFormat::Markdown);
    }

    #[test]
    fn tab_cycles_focus_between_the_two_groups() {
        let mut cs = CopySettings::new();
        assert_eq!(cs.focused_group(), 0);
        cs.focus_next();
        assert_eq!(cs.focused_group(), 1, "Tab → destination group");
        // Now ↑/↓ moves the destination selection.
        cs.select_down();
        assert_eq!(cs.dest_value(), CopyDest::File);
        assert_eq!(cs.format_value(), CopyFormat::Markdown, "format untouched");
        cs.focus_next();
        assert_eq!(cs.focused_group(), 0, "Tab wraps back to format");
    }

    #[test]
    fn render_paints_both_radio_group_labels() {
        let theme = Theme::default();
        let cs = CopySettings::new();
        let area = Rect::new(0, 0, 40, 14);
        let mut buf = Buffer::empty(area);
        cs.render(area, &mut buf, &theme);
        // Both labels appear somewhere in the rendered buffer.
        let has_format = (0..area.width)
            .any(|x| (0..area.height).any(|y| buf[Position::new(x, y)].symbol() == "F"));
        let has_dest = (0..area.width)
            .any(|x| (0..area.height).any(|y| buf[Position::new(x, y)].symbol() == "D"));
        assert!(has_format, "the Format group label is rendered");
        assert!(has_dest, "the Destination group label is rendered");
    }
}
