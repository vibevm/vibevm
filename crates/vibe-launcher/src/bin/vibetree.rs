//! VibeTree — a launcher that opens the project tree in the vibeterm desktop
//! terminal, OR — when run from a terminal — upgrades that terminal in place
//! (`vibe tree`), no second window (PROP-043, PROP-042 §5.1). All logic is in
//! `vibe-launcher`.
//!
//! Deliberately **console-subsystem** (no `windows_subsystem = "windows"`): the
//! in-terminal case needs the shell to wait for us, which it does not do for a
//! GUI-subsystem process. `run_terminal_aware` hides the console on the
//! double-click path so that entry stays windowless.

specmark::scope!("spec://vibevm/modules/vibe-launcher/PROP-043#registry");

fn main() -> std::process::ExitCode {
    vibe_launcher::run_terminal_aware(&["tree", "-t"])
}
