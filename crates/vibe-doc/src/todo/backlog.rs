//! Documentation debt — the `docs:` lines of the host's `BACKLOG.md`
//! (PROP-057 `##OBS-MAINTENANCE-TOOLS`, `MAINTENANCE.md` §2.1).
//!
//! When a change to the product cannot carry its documentation in the
//! same commit, the commit carries a line of debt instead. The queue
//! counts those lines and sorts them by severity; it does not judge them,
//! because a debt line is already a decision somebody took with a reason.
//!
//! The shape is one line: `docs:` at the start of what a person reads,
//! after whatever list marker or table pipe the file's own layout puts in
//! front of it, and a severity somewhere in it. Reading a line rather
//! than a structured record is the point — debt is written by whoever is
//! in a hurry, and a form that needs care would not be filled in.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#OBS-MAINTENANCE-TOOLS");

use std::path::Path;

use crate::error::{DocError, Result};

/// The prefix a debt line opens with.
pub const PREFIX: &str = "docs:";

/// The severities the project uses, in the order they are read
/// (`BACKLOG.md` §severity). One vocabulary in the project, not two.
pub const SEVERITIES: [&str; 3] = ["P1", "P2", "P3"];

/// One line of documentation debt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Debt {
    /// 1-based line of the file.
    pub line: usize,
    /// `P1`, `P2`, `P3`, or nothing when the line names none.
    pub severity: Option<String>,
    /// What the line says, from the prefix on.
    pub text: String,
}

/// Read the debt lines of a file.
///
/// ```
/// let tmp = tempfile::tempdir().unwrap();
/// let path = tmp.path().join("BACKLOG.md");
/// std::fs::write(
///     &path,
///     "# Backlog\n\n- docs: P2 the deploy page does not mention --dry-run\n",
/// )
/// .unwrap();
/// let debt = vibe_doc::todo::backlog::read(&path).unwrap();
/// assert_eq!(debt.len(), 1);
/// assert_eq!(debt[0].severity.as_deref(), Some("P2"));
/// assert_eq!(debt[0].line, 3);
/// ```
pub fn read(path: &Path) -> Result<Vec<Debt>> {
    let text = std::fs::read_to_string(path).map_err(|e| DocError::io("reading", path, e))?;
    let mut out = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let Some(rest) = opens_a_debt_line(line) else {
            continue;
        };
        out.push(Debt {
            line: index + 1,
            severity: severity(rest),
            text: format!("{PREFIX}{rest}").trim().to_owned(),
        });
    }
    Ok(out)
}

/// What follows `docs:` when the line is one, and nothing when it is not.
///
/// The markers stripped are the ones a Markdown file puts in front of a
/// line a person wrote: a list bullet, a table pipe, a bold marker, and
/// the fact anchor this project's own corpus carries. A mention of
/// `docs:` in the middle of a sentence is not a debt line, which is why
/// only the head of the line is looked at.
fn opens_a_debt_line(line: &str) -> Option<&str> {
    let mut head = line.trim();
    loop {
        let trimmed = head
            .trim_start_matches(['-', '*', '|', '>', '#', ' ', '\t'])
            .trim_start();
        let trimmed = match trimmed.split_once(' ') {
            Some((first, rest)) if first.starts_with("@fact:") => rest.trim_start(),
            _ => trimmed,
        };
        let trimmed = trimmed.trim_start_matches("**").trim_start();
        if trimmed == head {
            break;
        }
        head = trimmed;
    }
    head.strip_prefix(PREFIX)
}

/// The severity named in a line, if one is.
fn severity(text: &str) -> Option<String> {
    for name in SEVERITIES {
        if text
            .split(|c: char| !c.is_ascii_alphanumeric())
            .any(|word| word == name)
        {
            return Some(name.to_owned());
        }
    }
    None
}

#[cfg(test)]
mod tests;
