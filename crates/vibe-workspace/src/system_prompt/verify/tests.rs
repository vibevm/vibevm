//! What composition refuses, and what it only reports.
//!
//! The split is the point: `#SYS-NO-RECURSION` refuses what the composer
//! wrote itself, while `#SYS-IMPORT-INERTNESS` counts what the composer
//! carries verbatim and leaves the decision to `vibe prompt check`.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-IMPORT-INERTNESS",
    r = 1
);

use super::*;
use crate::system_prompt::{MarkerKind, marker_line};

#[test]
fn a_surviving_managed_block_marker_refuses_in_either_artifact() {
    for marker in [BLOCK_OPEN, BLOCK_CLOSE] {
        let text = format!("a\n{marker}\nb\n");
        let error = refuse_managed_block_markers(&text).expect_err("a marker refuses");
        let message = error.to_string();
        assert!(message.contains("line 2"), "{message}");
        assert!(message.contains("no router can enter it"), "{message}");
        assert!(
            verify_no_recursion(&text, &[]).is_err(),
            "the prefix check includes it"
        );
    }
}

/// An inline mention is ordinary prose. `locate_block` matches exactly a
/// standalone marker line, and so does this.
#[test]
fn an_inline_mention_of_the_marker_is_ordinary_text() {
    let text = "the `<vibevm>` markers are vibe's own\n";
    refuse_managed_block_markers(text).expect("prose is prose");
    verify_no_recursion(text, &[]).expect("prose is prose");
}

#[test]
fn a_lane_marker_line_may_not_reach_the_prefix() {
    let line = marker_line(MarkerKind::Begin, "g/n", "a.xml", "0123456789ab").unwrap();
    let error = verify_no_recursion(&format!("x\n{line}\n"), &[])
        .expect_err("a marker in the prefix refuses");
    assert!(
        error
            .to_string()
            .contains("lane segment marker line reached the prefix"),
        "{error}"
    );
}

/// The `@` refusal is scoped to the composer's own generated text. The lane
/// of this very repository carries 52 line-initial `@spec://` tokens that
/// `#SYS-LANE-VERBATIM` forbids rewriting, so a whole-bytes refusal would
/// make the feature unavailable to the project that specified it.
#[test]
fn an_import_line_refuses_inside_generated_text_and_passes_outside_it() {
    let text = "@spec://carried/verbatim#root\n@generated/line\n";
    verify_no_recursion(text, &[])
        .expect("a carried @ token in an authored body is reported, never refused");
    let generated = text.find("@generated").expect("present");
    let generated = [generated..text.len(), text.len()..text.len()];
    let error =
        verify_no_recursion(text, &generated).expect_err("an @ line the composer wrote refuses");
    assert!(
        error
            .to_string()
            .contains("own generated text carries an `@` import line"),
        "{error}"
    );
    // A generated range that covers no `@`-initial line leaves the carried
    // token alone — the scope is the line, not the artifact.
    let tail = ["@carried\n".len()..19, 19..19];
    verify_no_recursion("@carried\ngenerated\n", &tail)
        .expect("only a covered `@` line is refused");
}

#[test]
fn a_marker_shaped_line_in_a_segment_body_refuses_and_names_the_segment() {
    let line = marker_line(MarkerKind::End, "g/n", "a.xml", "0123456789ab").unwrap();
    let error = refuse_marker_shaped_lines("project.template", &format!("one\ntwo\n{line}\n"))
        .expect_err("a marker-shaped line refuses");
    let message = error.to_string();
    assert!(message.contains("project.template"), "{message}");
    assert!(message.contains("its line 3"), "{message}");
}

#[test]
fn the_census_counts_every_reference_token_and_classifies_each_by_its_fence() {
    let body = "\
intro\n\
@spec://one#a\n\
```\n\
@spec://fenced#b\n\
```\n\
@spec://two#c tail\n\
see @spec://three#d mid-line\n\
mail\u{0040}example.com and a bare @ are neither\n";
    let census = ImportInertness::of("project.lane", body);
    assert_eq!(
        census.len(),
        4,
        "four reference tokens; an e-mail-shaped word and a bare `@` are not"
    );
    assert_eq!(census.unfenced(), 3);
    assert_eq!(census.line_initial(), 3);
    assert!(!census.is_inert());
    assert_eq!(census.sites[0].line, 2);
    assert_eq!(census.sites[0].column, 0);
    assert!(!census.sites[0].fenced);
    assert!(census.sites[1].fenced, "inside the fence");
    assert_eq!(census.sites[2].token, "@spec://two#c");
    assert_eq!(census.sites[3].token, "@spec://three#d");
    assert!(!census.sites[3].line_initial);
    assert_eq!(census.first(2).len(), 2);
}

#[test]
fn a_body_with_no_at_token_is_inert() {
    let census = ImportInertness::of("project.template", "# My frame\n\nordinary text\n");
    assert!(census.is_empty());
    assert!(census.is_inert());
}

/// Several bodies accumulate into one census, each site naming its own
/// segment — which is how a report says *where* the tokens are.
#[test]
fn the_census_accumulates_across_segments() {
    let mut census = ImportInertness::default();
    census.add("project.template", "@a/one\n");
    census.add("project.lane", "@b/two\n");
    assert_eq!(census.len(), 2);
    assert_eq!(census.sites[0].segment, "project.template");
    assert_eq!(census.sites[1].segment, "project.lane");
}
