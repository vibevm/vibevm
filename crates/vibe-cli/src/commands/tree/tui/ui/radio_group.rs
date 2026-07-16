//! [`RadioGroup`] — a group of mutually-exclusive options, exactly one selected
//! (PROP-037 §2.7 `#radio-group`).
//!
//! ## Strategy — why invent, not wrap (PROP-037 §2.1)
//!
//! `rat-widget`'s selection widgets (`rat_widget::list::List` +
//! `ListState`/`RowSelection`, the `edit`/`tag` widgets) are stateful in the
//! rat-salsa/rat-focus sense: rendering threads a `&mut …State` carrying a
//! `FocusFlag`, and selection flows through `HandleEvent`/`HasFocus`. This TUI's
//! controller routes raw `ct_event!` macros to direct `App` mutators and threads
//! no `FocusFlag`/`FocusBuilder` anywhere (see [`super::button`] for the full
//! reasoning). Wrapping a stateful widget now would drag the focus subsystem in
//! before any single-choice dialog is ready to own it — the same trap `Button`
//! avoids by inventing.
//!
//! Phase 7 therefore takes §2.1 option 3 (invent on `ratatui_core`) for the
//! render surface, styled exclusively through [`theme`] — including the
//! selected/unselected marks, which come from [`theme::flag_on_glyph`] (●) and
//! [`theme::flag_off_glyph`] (○), never a hard-coded literal. `↑`/`↓`/`Enter`
//! are wired at the controller layer when a single-choice dialog owns a
//! `RadioGroup`; the primitive is render + navigate for now.
//!
//! [`super::button`]: super::button

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-037#radio-group");

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use specmark::spec;

use super::super::theme;

/// A group of mutually-exclusive options — exactly one is selected (PROP-037
/// §2.7). Backs single-choice settings (the copy format/destination, §10.2; a
/// later copy-settings modal composes this primitive).
///
/// The selected option is marked with [`theme::flag_on_glyph`] (●), the rest
/// with [`theme::flag_off_glyph`] (○) — the theme vocabulary, so the marks
/// degrade through every rendering tier (PROP-037 §2.2.2/§2.2.3) and a restyle
/// never touches this struct.
// Phase-7 component foundation; lights up when a single-choice dialog (the
// copy-settings §10.2 modal) composes it. Matches the `theme` module's allow.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct RadioGroup {
    label: String,
    options: Vec<String>,
    selected: usize,
}

#[allow(dead_code)]
impl RadioGroup {
    /// Build a radio group with `label` (rendered as the title) and `options`.
    /// Starts with the first option selected (a radio group always has exactly
    /// one); call [`RadioGroup::selected`] to move the selection.
    #[must_use]
    pub fn new(label: impl Into<String>, options: Vec<String>) -> Self {
        Self {
            label: label.into(),
            options,
            selected: 0,
        }
    }

    /// Set the selected index (builder), clamped to the valid range. A radio
    /// group with no options leaves the selection at 0.
    #[must_use]
    pub fn selected(mut self, selected: usize) -> Self {
        if !self.options.is_empty() {
            self.selected = selected.min(self.options.len() - 1);
        }
        self
    }

    /// The group's label (its title).
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// The option labels.
    #[must_use]
    pub fn options(&self) -> &[String] {
        &self.options
    }

    /// The index of the selected option.
    #[must_use]
    pub fn selected_index(&self) -> usize {
        self.selected
    }

    /// Move the selection down, wrapping.
    pub fn select_down(&mut self) {
        if !self.options.is_empty() {
            self.selected = (self.selected + 1) % self.options.len();
        }
    }

    /// Move the selection up, wrapping.
    pub fn select_up(&mut self) {
        if !self.options.is_empty() {
            let n = self.options.len();
            self.selected = (self.selected + n - 1) % n;
        }
    }

    /// Render the group into `area`: the label as a title row, then one row per
    /// option — the selected marked [`theme::flag_on_glyph`] (●), the rest
    /// [`theme::flag_off_glyph`] (○) (PROP-037 §2.7). No-op if `area` cannot
    /// hold the label row.
    #[spec(implements = "spec://vibevm/modules/vibe-cli/PROP-037#radio-group")]
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.height < 2 {
            return;
        }
        // The label row, styled as a title.
        buf.set_stringn(
            area.x,
            area.y,
            &self.label,
            area.width as usize,
            theme::title(),
        );
        // One row per option under the label.
        for (i, option) in self.options.iter().enumerate() {
            let y = area.y + 1 + i as u16;
            if y >= area.y + area.height {
                break;
            }
            let is_selected = i == self.selected;
            let glyph = if is_selected {
                theme::flag_on_glyph()
            } else {
                theme::flag_off_glyph()
            };
            let glyph_style = if is_selected {
                theme::accent()
            } else {
                theme::dim()
            };
            buf.set_stringn(area.x, y, glyph, area.width as usize, glyph_style);
            buf.set_stringn(
                area.x + 2,
                y,
                option,
                area.width.saturating_sub(2) as usize,
                theme::text(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui_core::layout::Position;

    /// The selected option carries the on-glyph; the others the off-glyph.
    #[test]
    fn render_marks_the_selected_option() {
        let g = RadioGroup::new("Format", vec!["Markdown".into(), "PNG".into()]).selected(1);
        let area = Rect::new(0, 0, 20, 4);
        let mut buf = Buffer::empty(area);
        g.render(area, &mut buf);

        // Row 1 (Markdown) is the off-mark; row 2 (PNG) is the on-mark.
        let on = theme::flag_on_glyph();
        let off = theme::flag_off_glyph();
        assert_eq!(buf[(Position::new(0, 1))].symbol(), off, "Markdown is off");
        assert_eq!(buf[(Position::new(0, 2))].symbol(), on, "PNG is on");
        // The label is the title row.
        assert_eq!(buf[(Position::new(0, 0))].symbol(), "F", "label row");
    }

    /// `select_up`/`select_down` wrap around the option list.
    #[test]
    fn navigation_wraps() {
        let mut g = RadioGroup::new("x", vec!["a".into(), "b".into(), "c".into()]);
        assert_eq!(g.selected_index(), 0);
        g.select_up(); // wraps 0 -> 2
        assert_eq!(g.selected_index(), 2);
        g.select_down(); // 2 -> 0
        assert_eq!(g.selected_index(), 0);
        g.select_down(); // 0 -> 1
        assert_eq!(g.selected_index(), 1);
    }

    /// `selected()` builder clamps out-of-range input; accessors round-trip.
    #[test]
    fn selected_clamps_and_accessors_round_trip() {
        let g = RadioGroup::new("Format", vec!["a".into(), "b".into()]).selected(99);
        assert_eq!(g.selected_index(), 1, "clamped to the last option");
        assert_eq!(g.label(), "Format");
        assert_eq!(g.options(), &["a", "b"]);
    }

    /// A too-short area (no room for label + an option) is a no-op.
    #[test]
    fn render_is_a_noop_when_too_short() {
        let g = RadioGroup::new("x", vec!["a".into()]);
        let area = Rect::new(0, 0, 10, 1);
        let mut buf = Buffer::empty(area);
        g.render(area, &mut buf);
        for x in 0..area.width {
            assert_eq!(buf[(Position::new(x, 0))].symbol(), " ");
        }
    }
}
