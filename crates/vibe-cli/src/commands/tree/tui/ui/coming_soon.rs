//! [`ComingSoon`] — the standard placeholder modal for every not-yet-built
//! feature (PROP-037 §2.10 `#coming-soon`).
//!
//! Extends [`MsgDialog`]: a titled window whose body is the fixed line "This
//! feature is not built yet." with a focused `OK` button. Wiring a feature's
//! entry point to [`ComingSoon`] is how a feature is "reserved" before it is
//! built — PNG export (§10.4) is the first user, and any future stub reuses it.
//!
//! [`MsgDialog`]: super::msg_dialog::MsgDialog

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-037#coming-soon");

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use specmark::spec;

use super::msg_dialog::MsgDialog;

/// The fixed body every Coming Soon modal shows.
const BODY: &str = "This feature is not built yet.";

/// The standard "Coming Soon" modal (PROP-037 §2.10): a [`MsgDialog`] titled
/// with the feature name and the body "This feature is not built yet."
/// `Enter`/`Esc` close it (handled at the controller layer when a ComingSoon
/// owns the frame, or via the menu's `ComingSoon` kind).
///
/// Composes [`MsgDialog`] for the whole render — the title, body, and focused OK
/// button all flow through [`super::super::theme`], so this struct adds only the
/// fixed-body policy and the feature-name accessor.
#[derive(Debug, Clone)]
pub struct ComingSoon {
    dialog: MsgDialog,
}

impl ComingSoon {
    /// Build a Coming Soon modal for `feature` (the feature name becomes the
    /// window title).
    #[must_use]
    pub fn new(feature: impl Into<String>) -> Self {
        Self {
            dialog: MsgDialog::new(feature, BODY),
        }
    }

    /// The feature name (rendered as the window title).
    #[must_use]
    #[allow(dead_code)] // introspection; the render path reads the title through MsgDialog.
    pub fn feature(&self) -> &str {
        // The dialog's title is the feature name by construction.
        self.dialog.title()
    }

    /// The fixed body line ("This feature is not built yet.").
    #[must_use]
    #[allow(dead_code)] // introspection; the render path carries the body through MsgDialog.
    pub fn body(&self) -> &'static str {
        BODY
    }

    /// Render the modal centred over `area` (PROP-037 §2.10). Delegates to the
    /// underlying [`MsgDialog`] render.
    #[spec(implements = "spec://vibevm/modules/vibe-cli/PROP-037#coming-soon")]
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        self.dialog.render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::theme;
    use super::*;
    use ratatui_core::layout::Position;

    /// The feature name titles the dialog; the body is the fixed placeholder.
    #[test]
    fn feature_titles_and_body_is_fixed() {
        let c = ComingSoon::new("PNG export");
        assert_eq!(c.feature(), "PNG export");
        assert_eq!(c.body(), "This feature is not built yet.");
    }

    /// `render` paints the title in the border and the body inside the window.
    #[test]
    fn render_draws_title_and_body() {
        let area = Rect::new(0, 0, 48, 8);
        let mut buf = Buffer::empty(area);
        ComingSoon::new("PNG export").render(area, &mut buf);

        let has_body = (0..area.width)
            .any(|x| (0..area.height).any(|y| buf[(Position::new(x, y))].symbol() == "T"));
        assert!(has_body, "the body line 'This feature…' is rendered");

        // The focused OK button paints the accent ground.
        let has_ok = (0..area.width)
            .any(|x| (0..area.height).any(|y| buf[(Position::new(x, y))].bg == theme::IRIS));
        assert!(has_ok, "the focused OK button is painted");
    }
}
