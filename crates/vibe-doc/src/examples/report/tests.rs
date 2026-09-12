//! What a report says, and what it refuses to say.

use std::path::PathBuf;

use super::{JsonVerdict, Outcome, Report, Verdict, diff};

fn outcome(id: &str, verdict: Verdict) -> Outcome {
    Outcome {
        page: "start/first-project.xml".into(),
        id: id.into(),
        fixture: "empty".into(),
        run: "vibe init hello".into(),
        verdict,
        json: Vec::new(),
    }
}

fn report(outcomes: Vec<Outcome>) -> Report {
    Report {
        outcomes,
        unreadable: Vec::new(),
        sandbox_root: PathBuf::from("."),
    }
}

#[test]
fn a_divergence_and_a_failure_are_red_and_a_skip_is_not() {
    assert!(
        !report(vec![outcome(
            "a",
            Verdict::Differ {
                diff: String::new()
            }
        )])
        .ok()
    );
    assert!(
        !report(vec![outcome(
            "a",
            Verdict::Failed {
                message: String::new()
            }
        )])
        .ok()
    );
    assert!(
        report(vec![outcome(
            "a",
            Verdict::Skipped {
                reason: "waits on the release".into()
            }
        )])
        .ok()
    );
    assert!(report(vec![outcome("a", Verdict::Match)]).ok());
}

#[test]
fn an_unreadable_page_is_red_even_when_every_example_matched() {
    let mut r = report(vec![outcome("a", Verdict::Match)]);
    r.unreadable.push(crate::pages::UnreadablePage {
        rel: "agent/how-agents-read-this-manual.xml".into(),
        path: PathBuf::from("x"),
        message: "unknown audience value `agent`".into(),
    });
    assert!(!r.ok());
    assert!(r.render().contains("does not parse"));
}

#[test]
fn a_document_without_a_schema_reads_as_unchecked_never_as_passed() {
    let mut o = outcome("a", Verdict::Match);
    o.json.push(JsonVerdict {
        command: "install:closure-diff".into(),
        schema: None,
        violations: Vec::new(),
    });
    let text = report(vec![o]).render();
    assert!(text.contains("UNCHECKED, not passed"), "{text}");
    assert!(!text.contains("valid against"));
}

#[test]
fn the_skip_line_carries_its_reason_so_a_green_panel_still_explains_itself() {
    let text = report(vec![outcome(
        "a",
        Verdict::Skipped {
            reason: "not captured yet (release): the installer runs from a distribution".into(),
        },
    )])
    .render();
    assert!(text.contains("not captured yet (release)"), "{text}");
}

#[test]
fn the_counts_separate_a_first_capture_from_a_re_blessing() {
    let c = report(vec![
        outcome("a", Verdict::Accepted { filled: true }),
        outcome("b", Verdict::Accepted { filled: false }),
    ])
    .counts();
    assert_eq!(c.captured, 1);
    assert_eq!(c.reblessed, 1);
}

#[test]
fn the_diff_points_at_the_first_line_that_parted() {
    let text = diff("stdout", "a\nb\nc", "a\nB\nc");
    assert!(text.contains("first difference at line 2"), "{text}");
    assert!(text.contains("  - b\n"), "{text}");
    assert!(text.contains("  + B\n"), "{text}");
    assert!(
        !text.contains("  - a\n"),
        "the common head is trimmed: {text}"
    );
}

#[test]
fn a_wholly_changed_stream_is_bounded_rather_than_printed_forever() {
    let want: String = (0..40).map(|n| format!("old {n}\n")).collect();
    let got: String = (0..40).map(|n| format!("new {n}\n")).collect();
    let text = diff("stdout", &want, &got);
    assert!(text.contains("more golden line(s)"), "{text}");
    assert!(text.contains("more captured line(s)"), "{text}");
}
