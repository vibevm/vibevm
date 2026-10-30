//! Authored shell commands under the selected interpreter.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#installer-separation");

use std::path::Path;
use std::process::Command;

use vibe_core::manifest::ShellInterpreter;

use crate::error::InstallerError;

/// The interpreter a command runs under when the option names none.
pub fn host_default() -> ShellInterpreter {
    if cfg!(windows) {
        ShellInterpreter::WindowsCmd
    } else {
        ShellInterpreter::PosixSh
    }
}

pub fn interpreter_name(interpreter: ShellInterpreter) -> &'static str {
    match interpreter {
        ShellInterpreter::PosixSh => "posix_sh",
        ShellInterpreter::WindowsCmd => "windows_cmd",
        ShellInterpreter::WindowsPowershell => "windows_powershell",
    }
}

/// Build the child for one authored command: `/bin/sh -c` on POSIX hosts,
/// `cmd.exe /D /S /C` or `powershell.exe -Command` on Windows.
pub fn command(
    interpreter: Option<ShellInterpreter>,
    script: &str,
    current_dir: &Path,
    app_root: &Path,
) -> Result<Command, InstallerError> {
    let interpreter = interpreter.unwrap_or_else(host_default);
    let mut command = match interpreter {
        ShellInterpreter::PosixSh if cfg!(windows) => {
            let mut command = Command::new("sh");
            command.args(["-c", script]);
            command
        }
        ShellInterpreter::PosixSh => {
            let mut command = Command::new("/bin/sh");
            command.args(["-c", script]);
            command
        }
        ShellInterpreter::WindowsCmd if cfg!(windows) => {
            let mut command = Command::new("cmd.exe");
            command.args(["/D", "/S", "/C", script]);
            command
        }
        ShellInterpreter::WindowsPowershell if cfg!(windows) => {
            let mut command = Command::new("powershell.exe");
            command.args(["-NoProfile", "-NonInteractive", "-Command", script]);
            command
        }
        other => {
            return Err(InstallerError::Interpreter {
                interpreter: interpreter_name(other),
            });
        }
    };
    command
        .current_dir(current_dir)
        .env("VIBE_APP_ROOT", app_root);
    Ok(command)
}

/// Run one authored command to completion with live output.
pub fn run(
    interpreter: Option<ShellInterpreter>,
    script: &str,
    current_dir: &Path,
    app_root: &Path,
    what: &str,
    json: bool,
) -> Result<(), InstallerError> {
    let mut child = command(interpreter, script, current_dir, app_root)?;
    super::run_child(&mut child, what, json)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn posix_commands_see_app_root_and_report_exit_codes() {
        let dir = tempfile::tempdir().unwrap();
        run(
            None,
            "test \"$VIBE_APP_ROOT\" = \"$PWD\" && touch marker",
            dir.path(),
            dir.path(),
            "test command",
            false,
        )
        .unwrap();
        assert!(dir.path().join("marker").is_file());
        let error = run(
            None,
            "exit 7",
            dir.path(),
            dir.path(),
            "failing command",
            true,
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("with code 7"), "{error}");
        assert!(matches!(
            command(
                Some(ShellInterpreter::WindowsCmd),
                "dir",
                dir.path(),
                dir.path()
            ),
            Err(InstallerError::Interpreter { .. })
        ));
    }
}
