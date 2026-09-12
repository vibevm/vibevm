//! `derived kind="cli-help"` — a command's own `--help`, taken from the
//! built binary at build time (PROP-045 `##ROW-DOCVOCAB-DERIVED`).
//!
//! The reference is the command line a reader would type: `vibe list
//! --help`. The runner types it, and what comes back is the reference.
//! Nothing of it is stored on the page, because a stored copy of a help
//! screen is a help screen that has already started to drift.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-DERIVED");

use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::error::{DocError, Result};
use crate::examples::command::{Program, parse};

/// What the help generator needs: the binary, and how long it may take.
#[derive(Debug, Clone)]
pub struct HelpEnv {
    pub binary: PathBuf,
    pub timeout_secs: u64,
}

/// Run one documented `--help` line and return what it printed.
///
/// The reference must name `vibe` and must ask for help: a `derived`
/// block that ran an arbitrary command would be a build step hiding in a
/// page, and this generator refuses to be one.
pub fn generate(reference: &str, env: &HelpEnv) -> Result<String> {
    let fail = |message: String| DocError::Derived {
        kind: "cli-help",
        reference: reference.to_owned(),
        message,
    };
    let cmd = parse(reference).map_err(|e| fail(e.to_string()))?;
    if cmd.program != Program::Vibe {
        return Err(fail(
            "a `cli-help` reference names the `vibe` command line a reader would type".into(),
        ));
    }
    if !cmd.args.iter().any(|a| a == "--help" || a == "-h") {
        return Err(fail(
            "a `cli-help` reference asks for help and nothing else — add `--help`".into(),
        ));
    }
    let output = Command::new(&env.binary)
        .args(&cmd.args)
        .stdin(Stdio::null())
        // Help must not vary with the operator's terminal, their colour
        // preference, or a harness that stamps its name on the envelope.
        .env("NO_COLOR", "1")
        .env_remove("VIBE_INVOKED_BY")
        .env_remove("VIBETERM")
        .env_remove("VIBEFRAME")
        .output()
        .map_err(|e| fail(format!("cannot run `{}`: {e}", env.binary.display())))?;
    if !output.status.success() {
        return Err(fail(format!(
            "exited {}: {}",
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(tidy(&String::from_utf8_lossy(&output.stdout)))
}

/// The two differences between a help screen and its published form: the
/// line endings of the platform that produced it, and the name of the
/// executable file, which on Windows is `vibe.exe` and in every command a
/// reader types is `vibe`.
fn tidy(text: &str) -> String {
    let joined = text.replace("\r\n", "\n").replace("vibe.exe", "vibe");
    let mut lines: Vec<&str> = joined.split('\n').map(str::trim_end).collect();
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env() -> HelpEnv {
        HelpEnv {
            binary: PathBuf::from("no-such-binary"),
            timeout_secs: 30,
        }
    }

    #[test]
    fn a_reference_that_is_not_a_help_request_is_refused() {
        let e = generate("vibe install org.acme/x", &env()).expect_err("refused");
        assert!(e.to_string().contains("add `--help`"), "{e}");
        assert!(e.to_string().contains("ROW-DOCVOCAB-DERIVED"), "{e}");
    }

    #[test]
    fn a_reference_to_another_program_is_refused() {
        let e = generate("cargo --help", &env()).expect_err("refused");
        assert!(
            e.to_string().contains("names the `vibe` command line"),
            "{e}"
        );
    }

    #[test]
    fn the_executable_suffix_and_the_platform_line_ending_are_taken_out() {
        assert_eq!(
            tidy("Usage: vibe.exe [OPTIONS]\r\n  -h  help  \r\n\r\n"),
            "Usage: vibe [OPTIONS]\n  -h  help"
        );
    }
}
