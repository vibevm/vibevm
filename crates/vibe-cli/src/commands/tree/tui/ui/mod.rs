//! The reusable view-component library (PROP-037 §2 "The component library").
//!
//! This is the abstraction that stops "a million implementations of the same
//! control" from accumulating: every modal, menu, and dialog in the TUI composes
//! these components, and a restyle touches only [`super::theme`]. The component
//! strategy (PROP-037 §2.1 — wrap, extend, invent) is applied per component:
//!
//! - [`Window`] — the bordered, titled, centered panel (§2.3). Extracted from
//!   the three call sites (`modal`, `menu`, `search`) that each inlined the same
//!   centered-popup pattern. Composes `theme.panel()`/`border()`/`title()`.
//! - [`Card`] — a `Window` laid out as a labelled vertical form (§2.9 `#card`,
//!   §8 `#detail-card`): bold field headers, wrapped values, blank-line
//!   spacing, and the theme `✕` close glyph. The detail modal renders through
//!   it instead of a glued text blob.
//! - [`Group`] — a bordered frame clustering children, with an optional name at
//!   the frame's top-right (§2.6). Wraps `Block` (stateless frame, no focus
//!   graph). The F2 sort/shape menu frames its groups with this.
//! - [`Button`] — the labelled, focusable control (§2.5). Invented on
//!   `ratatui_core`; see `button`'s module doc for the wrap-vs-invent reasoning.
//! - [`RadioGroup`] — mutually-exclusive options, exactly one selected (§2.7).
//!   Invented on `ratatui_core` (rat-widget's selection widgets are stateful);
//!   the selected option carries the theme on-glyph, the rest the off-glyph.
//! - [`TextField`] — a single-line editable text input (§2.8). Invented minimal
//!   on `ratatui_core` (rat-widget's edit widget is stateful); `type_char`/
//!   `backspace` + a `█` cursor when focused.
//! - [`MsgDialog`] — a `Window` + body + `OK` button (§2.10). The shared base
//!   for the [`ComingSoon`] placeholder modal and the quit-confirm dialog.
//! - [`ComingSoon`] — the standard placeholder for every not-yet-built feature
//!   (§2.10): a `MsgDialog` titled with the feature name and the fixed body
//!   "This feature is not built yet." PNG export (§10.4) is the first user.
//!
//! Call sites never reach past this module to `rat_widget::`/`ratatui_widgets::`
//! for the window/group pattern (PROP-037 §2.1) — they talk to [`Window`] /
//! [`Group`] instead.

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-037#components");

use ratatui_core::layout::Rect;

/// Interior horizontal padding inside a window frame, in cells (PROP-037
/// §2.2.5 `#spacing`). Content never touches the border — a clear left/right
/// margin between the frame and what it holds.
pub const PAD_X: u16 = 2;
/// Interior vertical padding inside a window frame, in rows (PROP-037 §2.2.5).
/// A blank row under the title and above the base, so content breathes.
pub const PAD_Y: u16 = 1;
/// The inset of a control within its own [`Group`] frame, in cells (PROP-037
/// §2.2.5). Lighter than [`PAD_X`] — the group frame already contains the
/// control; the gutter just keeps its glyphs off the stroke.
pub const GUTTER: u16 = 1;

/// Inset `rect` by the standard interior padding ([`PAD_X`] / [`PAD_Y`]),
/// returning the content rect (PROP-037 §2.2.5 `#spacing`). Saturating, so a
/// rect too small to hold the padding collapses to a zero-size inner instead of
/// underflowing.
#[must_use]
pub fn inner_pad(rect: Rect) -> Rect {
    Rect {
        x: rect.x.saturating_add(PAD_X),
        y: rect.y.saturating_add(PAD_Y),
        width: rect.width.saturating_sub(2 * PAD_X),
        height: rect.height.saturating_sub(2 * PAD_Y),
    }
}

pub mod button;
pub mod card;
pub mod coming_soon;
pub mod group;
pub mod msg_dialog;
pub mod radio_group;
pub mod text_field;
pub mod window;

// `Button`, `RadioGroup`, `TextField`, and `MsgDialog`/`ComingSoon` are the
// component foundation; several light up as their owning dialogs land (P6
// quit-confirm, P7 ComingSoon/PNG, and the later copy-settings §10.2 / file-path
// §10.5 modals). `Window`, `Group`, and `Card` are live today. Matches the
// `theme` module's Phase-3 `#[allow]`.
#[allow(unused_imports)]
pub use button::Button;
pub use card::Card;
#[allow(unused_imports)]
pub use coming_soon::ComingSoon;
#[allow(unused_imports)]
pub use group::Group;
#[allow(unused_imports)]
pub use msg_dialog::MsgDialog;
#[allow(unused_imports)]
pub use radio_group::RadioGroup;
#[allow(unused_imports)]
pub use text_field::TextField;
pub use window::Window;
