//! An `<assert>`, run (PROP-057 `##STYLE-PROMPT-FIRST`).
//!
//! The norm calls an assert «a shell command that must exit zero after
//! the agent's work», and the corpus writes exactly three programs:
//! `vibe`, `test` and `grep`. They are dispatched here rather than handed
//! to a shell, for the reason the example runner already gives and one
//! more.
//!
//! The example runner's reason: what a page shows must be what a reader
//! can type, and a check that goes through whatever shell the machine has
//! is a check that passes for reasons the page does not state.
//!
//! The extra reason belongs to this check alone: it runs on Windows,
//! where neither `test` nor `grep` exists. A `test -f` that fails because
//! the machine has no `test` reports the agent's work as broken, which is
//! the worst kind of red.
//!
//! This is a set of its OWN, not a widening of the example runner's. An
//! example shows a command to a reader and may only show `vibe`; an
//! assert is the page's own check on a result and may ask whether a file
//! exists. Widening the example set to fit the asserts would let a page
//! document `grep`.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#STYLE-PROMPT-FIRST");

use std::path::Path;
use std::process::{Command, Stdio};

use crate::error::{DocError, Result};
use crate::examples::command::split;
use crate::examples::fixture::HOME_DIR;
use crate::examples::normalize::display_path;
use crate::examples::sandbox::{CLEARED, Capture};

/// What an assert's command line names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Program {
    /// The built `vibe` binary — the same one the examples run.
    Vibe,
    /// `test -f <path>`, `test -e <path>`, `test -d <path>`, each with an
    /// optional leading `!`.
    Test,
    /// `grep -q <pattern> <path>`.
    Grep,
}

/// One parsed assert.
#[derive(Debug, Clone)]
pub struct Line {
    pub program: Program,
    pub args: Vec<String>,
    /// The line as the page wrote it, for the report.
    pub source: String,
}

/// Recognise the program an assert names.
///
/// ```
/// use vibe_doc::prompts::assert::{parse, Program};
///
/// assert_eq!(parse("vibe check --quiet").unwrap().program, Program::Vibe);
/// assert_eq!(parse("test ! -e vibe.lock").unwrap().program, Program::Test);
/// assert!(parse("curl https://example.com").is_err());
/// ```
pub fn parse(line: &str) -> Result<Line> {
    if line.contains('|') || line.contains('>') || line.contains("&&") {
        return Err(DocError::Command {
            command: line.to_owned(),
            message: "a pipeline, a redirect or a chain needs a shell, and this runner is \
                      not one — write one assert per check"
                .into(),
        });
    }
    let mut words = split(line)?.into_iter();
    let Some(head) = words.next() else {
        return Err(DocError::Command {
            command: line.to_owned(),
            message: "the assert is empty".into(),
        });
    };
    let program = match head.as_str() {
        "vibe" | "vibe.exe" => Program::Vibe,
        "test" | "[" => Program::Test,
        "grep" => Program::Grep,
        other => {
            return Err(DocError::Command {
                command: line.to_owned(),
                message: format!(
                    "`{other}` is not one of the programs an assert may run (vibe, test, \
                     grep); an assert is a check on the result, not a script"
                ),
            });
        }
    };
    Ok(Line {
        program,
        args: words.collect(),
        source: line.to_owned(),
    })
}

/// Run one assert in the sandbox and return what it left behind.
pub fn run(line: &Line, binary: &Path, cwd: &Path, sandbox: &Path, timeout_secs: u64) -> Capture {
    match line.program {
        Program::Vibe => vibe(line, binary, cwd, sandbox, timeout_secs),
        Program::Test => test(&line.args, cwd),
        Program::Grep => grep(&line.args, cwd),
    }
}

/// `vibe …`, with the same isolation the examples get: the sandbox's own
/// settings home, no colour, and every behavioural variable removed, so
/// the assert answers about the agent's work and not about the operator's
/// environment.
fn vibe(line: &Line, binary: &Path, cwd: &Path, sandbox: &Path, timeout_secs: u64) -> Capture {
    let mut command = Command::new(binary);
    command
        .args(&line.args)
        .current_dir(cwd)
        .stdin(Stdio::null());
    for key in CLEARED {
        command.env_remove(key);
    }
    command
        .env("VIBE_SETTINGS", display_path(&sandbox.join(HOME_DIR)))
        .env("NO_COLOR", "1");
    super::spawn(command, None, timeout_secs)
        .unwrap_or_else(|e| failed(format!("`{}`: {e}", line.source)))
}

/// `test [!] -f|-e|-d <path>` — the file questions the corpus asks.
fn test(args: &[String], cwd: &Path) -> Capture {
    let (negated, rest) = match args.split_first() {
        Some((first, rest)) if first == "!" => (true, rest),
        _ => (false, args),
    };
    let [flag, path] = rest else {
        return failed("`test` takes an operator and one path".to_owned());
    };
    let full = cwd.join(path);
    let held = match flag.as_str() {
        "-f" => full.is_file(),
        "-d" => full.is_dir(),
        "-e" => full.exists(),
        "-s" => full.metadata().map(|m| m.len() > 0).unwrap_or(false),
        other => {
            return failed(format!(
                "`test {other}` is not one of the operators an assert may use (-f, -d, -e, -s)"
            ));
        }
    };
    Capture {
        code: i32::from(held == negated),
        stdout: String::new(),
        stderr: String::new(),
    }
}

/// `grep -q <pattern> <path>` — the one form the corpus writes.
///
/// The pattern is a regular expression, as `grep`'s is. A pattern that
/// will not compile is a failed assert with the reason, never a silent
/// mismatch.
fn grep(args: &[String], cwd: &Path) -> Capture {
    let rest: Vec<&String> = args.iter().filter(|a| a.as_str() != "-q").collect();
    let [pattern, path] = rest[..] else {
        return failed("`grep` takes a pattern and one path".to_owned());
    };
    let full = cwd.join(path);
    let Ok(text) = std::fs::read_to_string(&full) else {
        return Capture {
            code: 2,
            stdout: String::new(),
            stderr: format!("grep: {}: no such file\n", path.as_str()),
        };
    };
    let found = match regex::Regex::new(pattern) {
        Ok(re) => re.is_match(&text),
        Err(e) => return failed(format!("`grep` pattern `{pattern}`: {e}")),
    };
    Capture {
        code: i32::from(!found),
        stdout: String::new(),
        stderr: String::new(),
    }
}

fn failed(message: String) -> Capture {
    Capture {
        code: 2,
        stdout: String::new(),
        stderr: format!("{message}\n"),
    }
}

#[cfg(test)]
mod tests;
