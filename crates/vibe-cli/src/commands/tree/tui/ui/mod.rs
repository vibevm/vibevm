//! The reusable view-component library (PROP-037 §2 "The component library").
//!
//! This is the abstraction that stops "a million implementations of the same
//! control" from accumulating: every modal, menu, and dialog in the TUI composes
//! these components, and a restyle touches only [`super::theme`]. The component
//! strategy (PROP-037 §2.1 — wrap, extend, invent) is applied per component:
//!
//! - [`Window`] — the bordered, titled, centered panel (§2.3). Extracted from
//!   the three call sites (`modal`, `menu`, `search`) that each inlined the same
//!   centered-popup pattern. Composes `theme::panel()`/`border()`/`title()`.
//! - [`Button`] — the labelled, focusable control (§2.5). Invented on
//!   `ratatui_core` for now; see `button`'s module doc for the wrap-vs-invent
//!   reasoning.
//! - [`MsgDialog`] — a `Window` + body + `OK` button (§2.10). The shared base
//!   for the `ComingSoon` placeholder modal and the quit-confirm dialog.
//!
//! Call sites never reach past this module to `rat_widget::`/`ratatui_widgets::`
//! for the window pattern (PROP-037 §2.1) — they talk to [`Window`] instead.

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-037#components");

pub mod button;
pub mod msg_dialog;
pub mod window;

// `Button` and `MsgDialog` are the Phase-3 component foundation; they light up
// when P6 (quit-confirm) / P7 (ComingSoon) compose them. `Window` is live today
// (the three popup call sites). Matches the `theme` module's Phase-3 `#[allow]`.
#[allow(unused_imports)]
pub use button::Button;
#[allow(unused_imports)]
pub use msg_dialog::MsgDialog;
pub use window::Window;
