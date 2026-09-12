//! The journal's open half — entries that still owe a decision
//! (PROP-057 `##OBS-MAINTENANCE-TOOLS`, `MAINTENANCE.md` §4).
//!
//! The journal is one table that is only ever appended to, and its last
//! column is «→ regulation»: what rule this entry confirms, changes or
//! creates, or «observation, no action, because…». The law is that the
//! column does not stay empty longer than a monthly loop — so the one
//! number worth counting is how many entries are still empty in it.
//!
//! The metric table counts entries and decisions separately; one number
//! says the same thing, because the target is that every entry has one
//! and zero is where it should be.
//!
//! The column is found by its heading rather than by its position, and in
//! either language the journal may be written in: a journal that grew a
//! column would otherwise be counted by the wrong one, silently.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#OBS-MAINTENANCE-TOOLS");

use std::path::Path;

use crate::error::{DocError, Result};

/// What the decision column is called, in the two languages this project
/// writes its journals in.
const DECISION_HEADINGS: [&str; 2] = ["регламент", "regulation"];

/// How many entries still carry an empty decision.
///
/// ```
/// let tmp = tempfile::tempdir().unwrap();
/// let path = tmp.path().join("JOURNAL.md");
/// std::fs::write(
///     &path,
///     "| Id | What happened | → regulation |\n|---|---|---|\n\
///      | J-001 | the runner tripped | the loop starts with `git status` |\n\
///      | J-002 | a linter false positive |  |\n",
/// )
/// .unwrap();
/// assert_eq!(vibe_doc::todo::journal::undecided(&path).unwrap(), 1);
/// ```
pub fn undecided(path: &Path) -> Result<usize> {
    let text = std::fs::read_to_string(path).map_err(|e| DocError::io("reading", path, e))?;
    let mut column: Option<usize> = None;
    let mut open = 0usize;
    for line in text.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            // A table ends where the pipes do, and a journal may carry
            // prose above and below it.
            column = None;
            continue;
        }
        let cells = cells(trimmed);
        if let Some(at) = decision_column(&cells) {
            column = Some(at);
            continue;
        }
        let Some(at) = column else {
            continue;
        };
        // The `|---|---|` rule under a heading is layout, not an entry.
        if cells
            .iter()
            .all(|cell| !cell.is_empty() && cell.chars().all(|c| c == '-' || c == ':'))
        {
            continue;
        }
        if cells.get(at).is_none_or(|cell| cell.is_empty()) {
            open += 1;
        }
    }
    Ok(open)
}

/// The cells of one table row, trimmed.
fn cells(line: &str) -> Vec<&str> {
    line.trim_matches('|')
        .split('|')
        .map(str::trim)
        .collect::<Vec<_>>()
}

/// Which cell of a heading row is the decision, when this row is one.
fn decision_column(cells: &[&str]) -> Option<usize> {
    cells.iter().position(|cell| {
        let lowered = cell.to_lowercase();
        DECISION_HEADINGS
            .iter()
            .any(|heading| lowered.contains(heading))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn journal(text: &str) -> usize {
        let tmp = tempfile::tempdir().expect("a temporary directory");
        let path = tmp.path().join("JOURNAL.md");
        std::fs::write(&path, text).expect("the journal");
        undecided(&path).expect("a reading")
    }

    #[test]
    fn the_heading_rule_under_a_table_is_not_an_entry() {
        assert_eq!(
            journal("| Id | → регламент |\n|---|---|\n| J-001 | done |\n"),
            0
        );
    }

    #[test]
    fn an_entry_with_an_empty_decision_is_counted() {
        assert_eq!(
            journal("| Id | → регламент |\n|---|---|\n| J-001 |  |\n| J-002 | yes |\n"),
            1
        );
    }

    #[test]
    fn the_column_is_found_by_its_heading_and_not_by_its_place() {
        assert_eq!(
            journal("| Id | → регламент | Evidence |\n|---|---|---|\n| J-001 |  | a command |\n"),
            1
        );
    }

    #[test]
    fn a_file_with_no_journal_table_owes_nothing() {
        assert_eq!(journal("# A journal\n\nNothing yet.\n"), 0);
    }

    #[test]
    fn prose_between_two_tables_closes_the_first() {
        assert_eq!(
            journal(
                "| Id | → регламент |\n|---|---|\n| J-001 |  |\n\nSome prose.\n\n\
                 | A | B |\n|---|---|\n| x |  |\n"
            ),
            1,
            "a second table with no decision column carries no entries to count"
        );
    }
}
