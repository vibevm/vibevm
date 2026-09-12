//! What counts as a line of documentation debt, and what does not.

use super::*;

fn debt(text: &str) -> Vec<Debt> {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    let path = tmp.path().join("BACKLOG.md");
    std::fs::write(&path, text).expect("the file");
    read(&path).expect("a reading")
}

#[test]
fn the_markers_a_markdown_file_puts_in_front_are_stripped() {
    let lines = debt(
        "- docs: P2 one\n\
         * docs: P3 two\n\
         | docs: P1 three |\n\
         > docs: four\n\
         docs: five\n",
    );
    assert_eq!(lines.len(), 5);
    assert_eq!(
        lines.iter().map(|d| d.severity.clone()).collect::<Vec<_>>(),
        vec![
            Some("P2".to_string()),
            Some("P3".to_string()),
            Some("P1".to_string()),
            None,
            None,
        ]
    );
}

#[test]
fn this_projects_own_fact_anchor_is_a_marker_too() {
    let lines = debt("- @fact:DEBT-DEPLOY **docs:** P1 the deploy page is silent\n");
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].severity.as_deref(), Some("P1"));
}

#[test]
fn a_mention_in_the_middle_of_a_sentence_is_not_a_debt_line() {
    assert!(
        debt("The convention is to write docs: followed by a severity.\n").is_empty(),
        "only the head of a line opens a debt row — otherwise the file's own prose \
         about the convention becomes debt"
    );
}

#[test]
fn a_line_carries_its_number_so_the_queue_can_point_at_it() {
    let lines = debt("# Backlog\n\n## P2\n\n- docs: P2 the deploy page\n");
    assert_eq!(lines[0].line, 5);
    assert!(lines[0].text.starts_with("docs:"));
}

#[test]
fn a_severity_is_a_word_and_not_a_substring() {
    let lines = debt("- docs: the P1000 sensor page is missing\n");
    assert_eq!(
        lines[0].severity, None,
        "`P1000` is not `P1`, and a queue that read it as one would report debt \
         nobody filed"
    );
}

#[test]
fn a_file_with_no_debt_reads_as_no_debt() {
    assert!(debt("# Backlog\n\nNothing to say.\n").is_empty());
}
