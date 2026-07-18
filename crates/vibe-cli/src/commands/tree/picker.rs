//! The VibeTree "no project here" folder picker (VIBE-LAUNCHERS).
//!
//! Shown only when `vibe tree -t` (the VibeTree GUI launcher) is started outside
//! any project *and* nothing was opened before — so a first-ever double-click
//! from `~/opt/bin` has a way to choose what to show, instead of an error dialog.
//! A native OS folder chooser (`rfd`); returns the chosen directory, or `None`
//! when the user cancels (a clean no-op, never an error).

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-036#command");

use std::path::PathBuf;

/// Open the native folder chooser and return the picked directory, or `None` on
/// cancel. Blocking — the caller is a one-shot launch, not an event loop.
pub(super) fn pick_project_folder() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_title("VibeTree — choose a vibe project folder")
        .pick_folder()
}
