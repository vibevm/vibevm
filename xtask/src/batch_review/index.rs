//! C11 — the task index must agree with the tasks that exist.
//!
//! The failure this exists for happened twice in one campaign and was found
//! only by accident: `MARKUP-B2` and `MARKUP-B5` were written, dispatched,
//! reviewed and landed, and neither ever got a row in `tasks/INDEX.md`. A cold
//! reader — the resume path this whole discipline is built around — would have
//! seen `MARKUP-B1` at the top of the table and concluded nothing else had run.
//!
//! **An index that stops recording one genre of task while still recording
//! another is worse than one that records nothing**, because the rows that are
//! there read as evidence that the file is being kept.
//!
//! Both directions are checked and neither is inferred:
//!
//! - a task file with no row — the failure that happened;
//! - a row naming a task that does not exist — the same defect mirrored, which
//!   a rename or a deletion produces.
//!
//! The denominator is a directory listing on one side and the table's own id
//! column on the other. Both are exact, so this check has no judgement in it
//! and no approximation to declare.

use std::collections::BTreeSet;
use std::path::Path;

use super::report::Report;

/// The ids named by the index table's first column.
///
/// A row is any line starting with `|`. The header (`| id | …`) and the
/// delimiter (`|---|---|`) are structure, not rows — the same distinction
/// `##COUNTABLE-UNITS` draws for table bodies, and it is drawn here for the
/// same reason: counting them would put two ids in the set that name nothing.
pub(super) fn index_ids(markdown: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for line in markdown.lines() {
        let line = line.trim();
        if !line.starts_with('|') {
            continue;
        }
        let Some(first) = line.split('|').nth(1) else {
            continue;
        };
        let id = first
            .trim()
            .trim_matches(|c| c == '`' || c == '*' || c == ' ')
            .to_string();
        if id.is_empty() || id.eq_ignore_ascii_case("id") {
            continue;
        }
        // The delimiter row: `---`, `:--`, `--:` and friends.
        if id.chars().all(|c| c == '-' || c == ':') {
            continue;
        }
        out.insert(id);
    }
    out
}

/// The task ids on disk: every `tasks/*.md` except the index itself.
pub(super) fn task_ids(tasks_dir: &Path) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let Ok(entries) = std::fs::read_dir(tasks_dir) else {
        return out;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        let Some(stem) = p.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        if stem == "INDEX" {
            continue;
        }
        out.insert(stem.to_string());
    }
    out
}

/// C11 — every task has a row, and every row has a task.
pub(super) fn c11_task_index(campaign: &Path, r: &mut Report) {
    let tasks_dir = campaign.join("tasks");
    let index_path = tasks_dir.join("INDEX.md");
    let Ok(markdown) = std::fs::read_to_string(&index_path) else {
        r.fail(
            "C11 task index",
            format!("no index at {}", index_path.display()),
        );
        return;
    };
    let on_disk = task_ids(&tasks_dir);
    if on_disk.is_empty() {
        // A zero denominator must never read as clean -- the campaign's single
        // most repeated defect, and the reason C1 refuses an empty scope too.
        r.fail(
            "C11 task index",
            format!(
                "no task files under {} -- refusing a zero denominator",
                tasks_dir.display()
            ),
        );
        return;
    }
    let indexed = index_ids(&markdown);

    let unindexed: Vec<&str> = on_disk.difference(&indexed).map(String::as_str).collect();
    let orphan_rows: Vec<&str> = indexed.difference(&on_disk).map(String::as_str).collect();

    if unindexed.is_empty() && orphan_rows.is_empty() {
        r.ok(
            "C11 task index",
            format!(
                "{} task(s), every one with a row and no row without one",
                on_disk.len()
            ),
        );
        return;
    }
    if !unindexed.is_empty() {
        r.fail(
            "C11 task index",
            format!(
                "{} task file(s) with NO row in INDEX.md: {}",
                unindexed.len(),
                unindexed.join(", ")
            ),
        );
    }
    if !orphan_rows.is_empty() {
        r.fail(
            "C11b index rows",
            format!(
                "{} row(s) naming a task that does not exist: {}",
                orphan_rows.len(),
                orphan_rows.join(", ")
            ),
        );
    }
}

// ------------------------------------------------------------- controls
#[cfg(test)]
mod tests {
    use super::*;

    const INDEX: &str = "\
# Task index

| id | title | executor | status |
|---|---|---|---|
| MARKUP-B6 | `typescript-ai-native-lang` | opus | **done** |
| DRIFT-037 | a skill's frontmatter | opus | queued |

Prose after the table, with a | pipe | in it that is not a row.
";

    fn scratch(files: &[&str], index: &str) -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        let tasks = d.path().join("tasks");
        std::fs::create_dir_all(&tasks).unwrap();
        std::fs::write(tasks.join("INDEX.md"), index).unwrap();
        for f in files {
            std::fs::write(tasks.join(format!("{f}.md")), "# task\n").unwrap();
        }
        d
    }

    #[test]
    fn the_header_and_delimiter_rows_are_not_ids() {
        let ids = index_ids(INDEX);
        assert!(!ids.contains("id"), "header row read as a task: {ids:?}");
        assert!(
            !ids.iter().any(|i| i.chars().all(|c| c == '-')),
            "delimiter row read as a task: {ids:?}"
        );
        assert_eq!(ids.len(), 2, "expected exactly two ids, got {ids:?}");
    }

    #[test]
    fn an_id_wrapped_in_markup_still_matches() {
        let ids = index_ids("| `MARKUP-B6` | x | y | z |\n| **DRIFT-037** | x | y | z |\n");
        assert!(
            ids.contains("MARKUP-B6") && ids.contains("DRIFT-037"),
            "{ids:?}"
        );
    }

    /// The failure that happened: a task ran, landed, and was never indexed.
    #[test]
    fn a_task_with_no_row_is_caught() {
        let d = scratch(&["MARKUP-B6", "DRIFT-037", "MARKUP-B5"], INDEX);
        let mut r = Report::default();
        c11_task_index(d.path(), &mut r);
        assert!(
            r.caught().iter().any(|c| c.starts_with("C11 ")),
            "an unindexed task must fail: {:?}",
            r.caught()
        );
    }

    #[test]
    fn a_row_with_no_task_is_caught() {
        let d = scratch(&["MARKUP-B6"], INDEX);
        let mut r = Report::default();
        c11_task_index(d.path(), &mut r);
        assert!(
            r.caught().iter().any(|c| c.starts_with("C11b")),
            "an orphan row must fail: {:?}",
            r.caught()
        );
    }

    #[test]
    fn an_agreeing_pair_stays_green() {
        let d = scratch(&["MARKUP-B6", "DRIFT-037"], INDEX);
        let mut r = Report::default();
        c11_task_index(d.path(), &mut r);
        assert!(!r.failed(), "clean pair went red: {:?}", r.caught());
    }

    #[test]
    fn an_empty_tasks_dir_refuses_rather_than_passing() {
        let d = scratch(&[], INDEX);
        let mut r = Report::default();
        c11_task_index(d.path(), &mut r);
        assert!(r.failed(), "a zero denominator must never read as clean");
    }

    #[test]
    fn a_missing_index_is_a_failure_not_a_skip() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("tasks")).unwrap();
        let mut r = Report::default();
        c11_task_index(d.path(), &mut r);
        assert!(r.failed());
    }
}
