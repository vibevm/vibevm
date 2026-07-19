//! VibeFrame — a GUI launcher that opens the vibeframe desktop terminal on a
//! detected shell (`vibe frame`). vibeframe is the simple terminal frame VibeTree
//! runs in (a copy of vibeterm's minimal single-window terminal). A GUI-subsystem
//! binary so a double-click never flashes a console (PROP-043 #spawn); it carries
//! VibeTerm.exe's icon (owner directive). All logic is in `vibe-launcher`.
#![cfg_attr(windows, windows_subsystem = "windows")]

specmark::scope!("spec://vibevm/modules/vibe-launcher/PROP-043#registry");

fn main() -> std::process::ExitCode {
    vibe_launcher::run(&["frame"])
}
