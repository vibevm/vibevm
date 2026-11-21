//! Unit tests for script interpreter selection. [`InterpreterProbe`] is
//! faked so selection is asserted without depending on the machine's `PATH`.
//!
//! Out-of-line per `#[cfg(test)] #[path] mod tests;`. Non-`#[test]` helpers
//! carry `#[cfg(test)]` so the file-grain conform frontend scopes their
//! `unwrap`s as test code (matching `install/tests.rs`).

use super::*;

use std::fs;

use specmark::verifies;
use tempfile::tempdir;

#[test]
fn generic_interpreter_probe_uses_the_version_flag() {
    assert_eq!(interpreter_probe_args("bash"), ["--version"]);
}

#[cfg(windows)]
#[test]
fn windows_powershell_probe_uses_its_native_command_grammar() {
    assert_eq!(
        interpreter_probe_args("powershell"),
        [
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "$null = $PSVersionTable.PSVersion"
        ]
    );
}

/// Probe that reports a fixed set of interpreters as present.
#[cfg(test)]
struct FakeProbe {
    present: Vec<String>,
}

#[cfg(test)]
impl FakeProbe {
    fn with(progs: &[&str]) -> Self {
        FakeProbe {
            present: progs.iter().map(|s| s.to_string()).collect(),
        }
    }
}

#[cfg(test)]
impl InterpreterProbe for FakeProbe {
    fn has(&self, program: &str) -> bool {
        self.present.iter().any(|p| p == program)
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#H-SCRIPT")]
fn unix_selects_sh_via_bash() {
    let slot = tempdir().unwrap();
    fs::create_dir_all(slot.path().join("hooks")).unwrap();
    fs::write(slot.path().join("hooks/prepare.sh"), "#!/bin/sh\n").unwrap();
    let base = Path::new("hooks/prepare");

    let with_bash = FakeProbe::with(&["bash"]);
    let inv = select_invocation(slot.path(), base, Platform::Unix, &with_bash).unwrap();
    assert_eq!(inv.interpreter, "bash");
    assert!(inv.script.ends_with("prepare.sh"));

    // No bash on PATH → nothing usable.
    let no_bash = FakeProbe::with(&[]);
    assert!(select_invocation(slot.path(), base, Platform::Unix, &no_bash).is_none());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#H-SCRIPT")]
fn windows_prefers_sh_then_falls_back_to_ps1() {
    let slot = tempdir().unwrap();
    fs::create_dir_all(slot.path().join("hooks")).unwrap();
    fs::write(slot.path().join("hooks/prepare.sh"), "#!/bin/sh\n").unwrap();
    fs::write(slot.path().join("hooks/prepare.ps1"), "Write-Host hi\n").unwrap();
    let base = Path::new("hooks/prepare");

    // Both interpreters present → Git Bash wins.
    let both = FakeProbe::with(&["bash", "powershell"]);
    let inv = select_invocation(slot.path(), base, Platform::Windows, &both).unwrap();
    assert_eq!(inv.interpreter, "bash");

    // Only PowerShell present → fall back to the .ps1.
    let ps_only = FakeProbe::with(&["powershell"]);
    let inv2 = select_invocation(slot.path(), base, Platform::Windows, &ps_only).unwrap();
    assert_eq!(inv2.interpreter, "powershell");
    assert!(inv2.script.ends_with("prepare.ps1"));

    // Neither → None.
    let none = FakeProbe::with(&[]);
    assert!(select_invocation(slot.path(), base, Platform::Windows, &none).is_none());
}
