//! vibeterm host integration for `vibe tree` (VIBE-LAUNCHERS / PROP-042).
//!
//! When `vibe tree` is launched **inside** vibeterm (the desktop terminal sets
//! `VIBETERM=1` in its PTY), it should not open a *second* window — it upgrades
//! the current terminal in place: the console TUI runs right here, and vibeterm's
//! window/taskbar icon is swapped to `vibetree` for the duration (reverting to
//! its launch icon on exit). The swap is an in-band OSC the vibeterm renderer
//! intercepts (`OSC 7773 ; <icon-name> ST`); in any other terminal it is a
//! harmless no-op (unknown OSC sequences are discarded).

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-042#in-place-upgrade");

use std::io::Write;

use anyhow::Result;

/// The env vars a vibe desktop terminal sets in its PTY so a nested `vibe tree`
/// knows it already runs inside one (and need not spawn another window):
/// `VIBETERM` (the complex terminal) or `VIBEFRAME` (the simple frame).
const TERMINAL_ENVS: [&str; 2] = ["VIBETERM", "VIBEFRAME"];

/// The custom OSC code the vibeterm renderer listens on to swap its icon.
const OSC_SET_ICON: &str = "7773";

/// Whether this process runs inside a vibe desktop terminal PTY (vibeterm or
/// vibeframe).
pub(super) fn in_vibeterm() -> bool {
    TERMINAL_ENVS
        .iter()
        .any(|k| std::env::var_os(k).is_some_and(|v| !v.is_empty()))
}

/// Ask vibeterm to swap its window/taskbar icon to the named app-family icon
/// (`vibetree`), or — with an empty `name` — revert to its launch icon. An
/// in-band `OSC 7773 ; <name> BEL` the vibeterm renderer intercepts; a no-op in
/// any other terminal. Best-effort: a write failure (closed pipe) is ignored.
fn set_vibeterm_icon(name: &str) {
    let mut out = std::io::stdout().lock();
    let _ = write!(out, "\x1b]{OSC_SET_ICON};{name}\x07");
    let _ = out.flush();
}

/// Run the console tree TUI as a temporary vibeterm "upgrade": swap the icon to
/// `vibetree` for the session and revert to the launch icon after, so the window
/// reads as a VibeTree terminal while the tree is open. Outside vibeterm this is
/// a transparent wrapper that just runs `f`.
pub(super) fn run_upgraded<F: FnOnce() -> Result<()>>(f: F) -> Result<()> {
    if !in_vibeterm() {
        return f();
    }
    set_vibeterm_icon("vibetree");
    let out = f();
    set_vibeterm_icon(""); // revert to vibeterm's launch icon
    out
}
