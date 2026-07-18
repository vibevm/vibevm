//! VibeTree — a GUI launcher that opens the project tree in the vibeterm desktop
//! terminal (`vibe tree -t`). A GUI-subsystem binary so a double-click never
//! flashes a console (PROP-043 #spawn); all the logic is in `vibe-launcher`.
#![cfg_attr(windows, windows_subsystem = "windows")]

specmark::scope!("spec://vibevm/modules/vibe-launcher/PROP-043#registry");

fn main() -> std::process::ExitCode {
    vibe_launcher::run(&["tree", "-t"])
}
