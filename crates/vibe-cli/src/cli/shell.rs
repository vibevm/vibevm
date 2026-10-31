//! Grammar for `vibe shell`, the interactive shell with installed
//! applications on PATH.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#shell-command");

pub(super) const LONG_ABOUT: &str = "Start an interactive shell with installed applications on PATH.\n\
    \n\
    PATH begins with the opt/bin directory of the project home vibevm/vibeopt\n\
    (when the working directory is a project whose home exists), then that of\n\
    the user settings root; -g selects the user settings root alone. Every\n\
    reserved application command has its proxy there. Each --use SLOT=KEY selects a\n\
    provider for this shell only, through VIBE_APP_USE; an inherited VIBE_APP_USE\n\
    is kept and a later choice wins per slot. The global default is untouched.\n\
    Each --with PKG[@REQ] puts an installed (typically hidden) application's\n\
    commands first on this shell's PATH through the same VIBE_APP_USE choice.\n\
    The environment entries installed applications recorded are applied at once.\n\
    \n\
    The shell is the parent bash or zsh (Linux/macOS) or cmd, Windows PowerShell\n\
    or PowerShell (Windows); otherwise /bin/sh on Linux, /bin/zsh on macOS and\n\
    the system cmd.exe on Windows. Requires an interactive terminal; --json and\n\
    --unattended are refused. Returns the shell's exit status.";

/// Arguments for `vibe shell`.
#[derive(Debug, Default, clap::Args)]
pub struct ShellArgs {
    /// Select the user settings root alone, even inside a project.
    #[arg(short = 'g', long)]
    pub global: bool,

    /// Choose the provider of one slot for this shell; may repeat.
    #[arg(long = "use", value_name = "SLOT=KEY")]
    pub uses: Vec<String>,

    /// Put an installed application, hidden ones included, on this shell's
    /// PATH only (PROP-059 `#hidden-installations`); may repeat.
    #[arg(long = "with", value_name = "PKG[@REQ]")]
    pub with: Vec<String>,
}
