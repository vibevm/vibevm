//! Windows native installers: an EXE runs directly, an MSI through
//! `msiexec /i`, both with structured arguments and never a shell.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#installer-separation");

use std::path::Path;
use std::process::Command;

use specmark::spec;

use crate::error::InstallerError;

/// Which native installer format a verified file is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#installer-separation")]
pub enum NativeKind {
    Exe,
    Msi,
}

impl NativeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Exe => "exe",
            Self::Msi => "msi",
        }
    }
}

/// Run a verified native installer file with its authored arguments.
pub fn run(
    kind: NativeKind,
    file: &Path,
    arguments: &[String],
    current_dir: &Path,
    json: bool,
) -> Result<(), InstallerError> {
    if !cfg!(windows) {
        return Err(InstallerError::WindowsOnly {
            method: kind.as_str(),
        });
    }
    let mut command = match kind {
        NativeKind::Exe => Command::new(file),
        NativeKind::Msi => {
            let mut command = Command::new("msiexec.exe");
            command.arg("/i").arg(file);
            command
        }
    };
    command.args(arguments).current_dir(current_dir);
    super::run_child(&mut command, &format!("{} installer", kind.as_str()), json)
}
