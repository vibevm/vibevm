//! What a documented command line means, and what the runner will run
//! (PROP-057 `##PIPE-EXAMPLE-RUNNER`).
//!
//! The runner is NOT a shell. It splits the line the page shows, looks at
//! the program, and dispatches a closed set: the built `vibe` binary, the
//! `cargo` a fixture recipe needs to make a real build tree, and `cat`,
//! which a page uses to show a file it just told the reader to write.
//! Anything else is refused by name.
//!
//! The reason is the reader. A page that needs a pipeline, a redirect or a
//! shell builtin is showing something the reader cannot copy into a fresh
//! terminal on another platform and get the same result; refusing it keeps
//! the manual's commands honest instead of quietly running them through
//! whatever shell the machine happens to have.
//!
//! `cat` is implemented here rather than shelled out for the same reason:
//! Windows has no `cat`, and a page that says `cat vibe.toml` is showing
//! the file's content, not a program's output.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-EXAMPLE-RUNNER");

use crate::error::{DocError, Result};

/// The program a command line names, once the runner has recognised it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Program {
    /// The built `vibe` binary under test.
    Vibe,
    /// `cargo`, for the fixture recipes that build a real crate in place.
    Cargo,
    /// `cat <path>` — print a file's bytes. The runner's own, because the
    /// page is showing a file and not a program.
    Cat,
}

/// One parsed command line.
#[derive(Debug, Clone)]
pub struct CommandLine {
    pub program: Program,
    pub args: Vec<String>,
    /// The line as written, for the report.
    pub source: String,
}

/// Split a command line the way a reader's shell would for the forms a
/// documented command uses: whitespace-separated words, with double and
/// single quotes grouping and a backslash escaping inside double quotes.
///
/// ```
/// let parts = vibe_doc::examples::command::split("vibe explain \"spec://a/b#C\"").unwrap();
/// assert_eq!(parts, ["vibe", "explain", "spec://a/b#C"]);
/// ```
pub fn split(line: &str) -> Result<Vec<String>> {
    let mut words: Vec<String> = Vec::new();
    let mut word = String::new();
    let mut started = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            c if c.is_whitespace() => {
                if started {
                    words.push(std::mem::take(&mut word));
                    started = false;
                }
            }
            '"' => {
                started = true;
                for inner in chars.by_ref() {
                    if inner == '"' {
                        break;
                    }
                    word.push(inner);
                }
            }
            '\'' => {
                started = true;
                for inner in chars.by_ref() {
                    if inner == '\'' {
                        break;
                    }
                    word.push(inner);
                }
            }
            _ => {
                started = true;
                word.push(c);
            }
        }
    }
    if started {
        words.push(word);
    }
    if words.is_empty() {
        return Err(DocError::Command {
            command: line.to_owned(),
            message: "the command line is empty".into(),
        });
    }
    Ok(words)
}

/// Recognise the program a command line names.
///
/// ```
/// use vibe_doc::examples::command::{parse, Program};
/// let cmd = parse("vibe list --path hello-vibe").unwrap();
/// assert_eq!(cmd.program, Program::Vibe);
/// assert_eq!(cmd.args, ["list", "--path", "hello-vibe"]);
/// ```
pub fn parse(line: &str) -> Result<CommandLine> {
    if line.contains('|') || line.contains('>') || line.contains('<') {
        return Err(DocError::Command {
            command: line.to_owned(),
            message: "a pipeline or a redirect needs a shell, and the runner is not one".into(),
        });
    }
    let mut words = split(line)?.into_iter();
    let Some(head) = words.next() else {
        return Err(DocError::Command {
            command: line.to_owned(),
            message: "the command line is empty".into(),
        });
    };
    let rest: Vec<String> = words.collect();
    let program = match head.as_str() {
        "vibe" | "vibe.exe" => Program::Vibe,
        "cargo" => Program::Cargo,
        "cat" => Program::Cat,
        other => {
            return Err(DocError::Command {
                command: line.to_owned(),
                message: format!(
                    "`{other}` is not one of the programs this runner executes \
                     (vibe, cargo, cat); a page that needs a shell is showing a \
                     command a reader cannot copy verbatim"
                ),
            });
        }
    };
    Ok(CommandLine {
        program,
        args: rest,
        source: line.to_owned(),
    })
}

#[cfg(test)]
mod tests;
