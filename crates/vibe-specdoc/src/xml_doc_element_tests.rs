//! The documentation genre's six block elements, one law at a time.
//!
//! `example` and `prompt` are asserted on their verbatim bodies — every
//! byte, the leading newline included — because that is the property
//! their readers exist to keep; `rule` and `derived` on what they point
//! at and deliberately do not store; `note` and `figure` on their prose
//! units and their required attributes.
//!
//! Sibling test cell of [`crate::xml_doc_tests`], which owns the shared
//! fixtures below and the genre's vocabulary gating.

use crate::doc::{Block, DerivedKind, NoteKind, Vocabulary};
use crate::xml_doc_tests::{NS, assert_byte_stable, doc, doc_err, only_block};
use crate::{from_xml_with, to_markdown, to_xml};

/// The full example: every attribute, all three children, byte-stable.
#[test]
fn an_example_carries_its_fixture_lang_exit_and_streams() {
    let body = "  <example id=\"e\" fixture=\"hello-vibe\" lang=\"powershell\" exit=\"1\">\n    \
                <run>vibe nope</run>\n    <expect></expect>\n    \
                <stderr>error: no such command</stderr>\n  </example>\n";
    let d = doc(body);
    match only_block(&d) {
        Block::Example {
            id,
            fixture,
            lang,
            exit,
            run,
            expect,
            stderr,
        } => {
            assert_eq!(id, "e");
            assert_eq!(fixture, "hello-vibe");
            assert_eq!(lang.as_deref(), Some("powershell"));
            assert_eq!(*exit, Some(1));
            assert_eq!(run, "vibe nope");
            assert_eq!(expect, "");
            assert_eq!(stderr.as_deref(), Some("error: no such command"));
        }
        other => panic!("{other:?}"),
    }
    assert_byte_stable(body);
}

/// A golden output is BYTES: its leading newline and its inner blank line
/// are content, not layout, and survive untrimmed (##DOC-VOCAB-VERBATIM-TEXTS).
#[test]
fn a_golden_output_keeps_every_byte_including_its_leading_newline() {
    let body = "  <example id=\"e\" fixture=\"none\">\n    <run>vibe install</run>\n    \
                <expect>\nfirst line\n\nthird line</expect>\n  </example>\n";
    match only_block(&doc(body)) {
        Block::Example { expect, .. } => {
            assert_eq!(expect, "\nfirst line\n\nthird line");
        }
        other => panic!("{other:?}"),
    }
    assert_byte_stable(body);
}

/// An empty `<expect></expect>` is the ASSERTION «this command prints
/// nothing», so the writer must never collapse it to `<expect/>` — that
/// is the difference between an assertion and an absence, and the live
/// corpus spells it the long way twenty times.
#[test]
fn an_empty_expect_keeps_its_pair_form() {
    let body = "  <example id=\"e\" fixture=\"none\">\n    <run>vibe self doctor</run>\n    <expect></expect>\n  </example>\n";
    assert_byte_stable(body);
    let d = doc(body);
    assert!(to_xml(&d).contains("<expect></expect>"), "{}", to_xml(&d));
    assert!(!to_xml(&d).contains("<expect/>"));
}

/// `exit` distinguishes «absent» from «spelled out», like `lang` on a
/// fence: both mean the exit code 0, and both survive the round trip as
/// the author wrote them.
#[test]
fn an_absent_exit_is_not_the_same_bytes_as_a_written_zero() {
    let bare = "  <example id=\"e\" fixture=\"none\">\n    <run>x</run>\n    <expect></expect>\n  </example>\n";
    let spelled = "  <example id=\"e\" fixture=\"none\" exit=\"0\">\n    <run>x</run>\n    <expect></expect>\n  </example>\n";
    assert_byte_stable(bare);
    assert_byte_stable(spelled);
    let (a, b) = (doc(bare), doc(spelled));
    assert_ne!(a, b, "the two spellings are distinguishable in the IR");
    match (only_block(&a), only_block(&b)) {
        (Block::Example { exit: None, .. }, Block::Example { exit: Some(0), .. }) => {}
        other => panic!("{other:?}"),
    }
}

/// The children are ordered and unrepeatable: the writer emits one order,
/// so accepting another would break byte idempotence the day such a page
/// existed.
#[test]
fn example_children_are_ordered_and_unrepeatable() {
    let out_of_order =
        doc_err("  <example id=\"e\" fixture=\"none\"><expect>x</expect><run>y</run></example>\n");
    assert!(
        out_of_order.contains("out of order or repeated"),
        "{out_of_order}"
    );
    let repeated = doc_err(
        "  <example id=\"e\" fixture=\"none\"><run>a</run><expect>x</expect><expect>y</expect></example>\n",
    );
    assert!(repeated.contains("out of order or repeated"), "{repeated}");
    let foreign = doc_err(
        "  <example id=\"e\" fixture=\"none\"><run>a</run><expect>x</expect><p>no</p></example>\n",
    );
    assert!(
        foreign.contains("no <p> element (inside <example>)"),
        "{foreign}"
    );
}

/// An example without its expected output cannot be checked, and an
/// example nobody can address cannot be cited by a translation.
#[test]
fn an_example_needs_an_id_a_fixture_a_run_and_an_expect() {
    assert!(
        doc_err("  <example fixture=\"none\"><run>a</run><expect></expect></example>\n")
            .contains("needs an `id`")
    );
    assert!(
        doc_err("  <example id=\"e\"><run>a</run><expect></expect></example>\n")
            .contains("needs a `fixture`")
    );
    assert!(
        doc_err("  <example id=\"e\" fixture=\"none\"><run>a</run></example>\n")
            .contains("needs an <expect>")
    );
    assert!(
        doc_err("  <example id=\"e\" fixture=\"none\"><expect></expect></example>\n")
            .contains("out of order")
    );
    assert!(
        doc_err("  <example id=\"e\" fixture=\"none\"><run>  </run><expect></expect></example>\n")
            .contains("<run> command is empty")
    );
}

/// The translation form: a reference, with no body and no example of its
/// own (##ROW-DOCVOCAB-EXAMPLE-REF).
#[test]
fn an_example_ref_carries_no_example_of_its_own() {
    let body = "  <example ref=\"version\"/>\n";
    assert_byte_stable(body);
    match only_block(&doc(body)) {
        Block::ExampleRef { id } => assert_eq!(id, "version"),
        other => panic!("{other:?}"),
    }
    for attr in ["id=\"e\"", "fixture=\"none\"", "lang=\"sh\"", "exit=\"0\""] {
        let err = doc_err(&format!("  <example ref=\"v\" {attr}/>\n"));
        assert!(err.contains("carries no"), "{attr}: {err}");
    }
    let with_body = doc_err("  <example ref=\"v\"><run>x</run></example>\n");
    assert!(with_body.contains("is empty — found"), "{with_body}");
}

// --- rule (##ROW-DOCVOCAB-RULE) ----------------------------------------

/// The address's form is checked; the anchor's existence is the pipeline's
/// job, and a revision pin is nobody's (##DOC-VOCAB-RULE-ADDRESS): it is
/// recorded so the author's bytes survive, and dropped from the citation.
#[test]
fn a_rule_records_a_revision_pin_and_never_cites_with_it() {
    let body = "  <rule ref=\"spec://org.vibevm.core/vibevm/common/PROP-019#ROOT-DEFAULT~r3\"/>\n";
    assert_byte_stable(body);
    match only_block(&doc(body)) {
        Block::Rule { uri, rev } => {
            assert_eq!(
                uri,
                "spec://org.vibevm.core/vibevm/common/PROP-019#ROOT-DEFAULT"
            );
            assert_eq!(*rev, Some(3));
        }
        other => panic!("{other:?}"),
    }
    // The citation the projection shows is the unpinned one.
    let md = to_markdown(&doc(body));
    assert!(md.contains("#ROOT-DEFAULT>"), "{md}");
    assert!(!md.contains("~r3"), "a citation is live, not pinned: {md}");
}

#[test]
fn a_rule_address_must_be_a_spec_address_with_an_anchor() {
    assert!(doc_err("  <rule ref=\"PROP-019#ROOT\"/>\n").contains("cites a spec address"));
    assert!(doc_err("  <rule ref=\"spec://g/n/p\"/>\n").contains("names no anchor"));
    assert!(doc_err("  <rule ref=\"spec://g/n/p#\"/>\n").contains("names no anchor"));
    assert!(doc_err("  <rule/>\n").contains("needs a `ref`"));
    assert!(doc_err("  <rule ref=\"spec://g/n/p#A~r0\"/>\n").contains("invalid revision pin"));
    // A body in an element the dialect spells empty is loud, not ignored.
    assert!(doc_err("  <rule ref=\"spec://g/n/p#A\">text</rule>\n").contains("is empty — found"));
}

// --- derived (##ROW-DOCVOCAB-DERIVED) ----------------------------------

/// The generated text is NOT stored — storing it is how documentation
/// starts lying — and the kind is a closed list with a hint.
#[test]
fn derived_stores_the_generator_not_its_output() {
    for (spelling, kind) in [
        ("cli-help", DerivedKind::CliHelp),
        ("jtd-schema", DerivedKind::JtdSchema),
        ("manifest-field", DerivedKind::ManifestField),
    ] {
        let body = format!("  <derived kind=\"{spelling}\" ref=\"vibe --help\"/>\n");
        assert_byte_stable(&body);
        match only_block(&doc(&body)) {
            Block::Derived { kind: k, reference } => {
                assert_eq!(*k, kind);
                assert_eq!(reference, "vibe --help");
            }
            other => panic!("{other:?}"),
        }
    }
    let hint = doc_err("  <derived kind=\"cli-halp\" ref=\"x\"/>\n");
    assert!(hint.contains("did you mean `cli-help`?"), "{hint}");
    assert!(doc_err("  <derived ref=\"x\"/>\n").contains("needs a `kind`"));
    assert!(doc_err("  <derived kind=\"cli-help\"/>\n").contains("needs a `ref`"));
}

// --- note (##ROW-DOCVOCAB-NOTE) ----------------------------------------

/// A call-out's body is a UNIT, so it takes a fact anchor and a status
/// like any other unit — which is what makes a note citable.
#[test]
fn a_note_body_is_an_addressable_unit() {
    for (spelling, kind) in [
        ("note", NoteKind::Note),
        ("tip", NoteKind::Tip),
        ("warning", NoteKind::Warning),
    ] {
        let body = format!("  <note kind=\"{spelling}\">Mind the gap.</note>\n");
        assert_byte_stable(&body);
        match only_block(&doc(&body)) {
            Block::Note { kind: k, body: u } => {
                assert_eq!(*k, kind);
                assert_eq!(u.text, "Mind the gap.");
                assert!(u.fact.is_none());
            }
            other => panic!("{other:?}"),
        }
    }
    let anchored = "  <note kind=\"warning\"><THE-RISK fact=\"true\" status=\"doc/work\">Mind the gap.</THE-RISK></note>\n";
    assert_byte_stable(anchored);
    match only_block(&doc(anchored)) {
        Block::Note { body, .. } => {
            assert_eq!(body.fact.as_ref().unwrap().id.as_deref(), Some("THE-RISK"));
            assert!(body.fact.as_ref().unwrap().status.is_some());
        }
        other => panic!("{other:?}"),
    }
    let hint = doc_err("  <note kind=\"warn\">x</note>\n");
    assert!(hint.contains("did you mean `warning`?"), "{hint}");
    assert!(doc_err("  <note>x</note>\n").contains("needs a `kind`"));
}

// --- figure (##ROW-DOCVOCAB-FIGURE) ------------------------------------

/// An image nobody can read is not documentation: `alt` is required, and
/// so is the caption.
#[test]
fn a_figure_needs_a_source_an_alt_and_one_caption() {
    let body = "  <figure src=\"media/tree.png\" alt=\"The dependency tree\">\n    \
                <caption>Two roots, one shared package.</caption>\n  </figure>\n";
    assert_byte_stable(body);
    match only_block(&doc(body)) {
        Block::Figure { src, alt, caption } => {
            assert_eq!(src, "media/tree.png");
            assert_eq!(alt, "The dependency tree");
            assert_eq!(caption.text, "Two roots, one shared package.");
        }
        other => panic!("{other:?}"),
    }
    assert!(
        doc_err("  <figure alt=\"a\"><caption>c</caption></figure>\n").contains("needs a `src`")
    );
    assert!(
        doc_err("  <figure src=\"a.png\"><caption>c</caption></figure>\n")
            .contains("needs an `alt`")
    );
    assert!(doc_err("  <figure src=\"a.png\" alt=\"a\"/>\n").contains("needs a <caption>"));
    let twice = doc_err(
        "  <figure src=\"a.png\" alt=\"a\"><caption>one</caption><caption>two</caption></figure>\n",
    );
    assert!(twice.contains("holds one <caption>"), "{twice}");
    let foreign = doc_err("  <figure src=\"a.png\" alt=\"a\"><p>c</p></figure>\n");
    assert!(foreign.contains("only <caption>"), "{foreign}");
}

// --- prompt (##ROW-DOCVOCAB-PROMPT) ------------------------------------

/// The whole prompt: body first, then needs, outcome and the asserts, in
/// that order, repeatable only in the asserts.
#[test]
fn a_prompt_holds_its_body_then_needs_outcome_and_asserts() {
    let body = "  <prompt id=\"install\">Install vibe on this machine.\n    \
                <needs>network access to github.com</needs>\n    \
                <outcome>`vibe --version` prints a version</outcome>\n    \
                <assert>vibe --version</assert>\n    \
                <assert>vibe self doctor</assert>\n  </prompt>\n";
    assert_byte_stable(body);
    match only_block(&doc(body)) {
        Block::Prompt {
            id,
            text,
            needs,
            outcome,
            asserts,
        } => {
            assert_eq!(id, "install");
            assert_eq!(text, "Install vibe on this machine.");
            assert_eq!(needs.as_deref(), Some("network access to github.com"));
            assert_eq!(
                outcome.as_deref(),
                Some("`vibe --version` prints a version")
            );
            assert_eq!(asserts, &["vibe --version", "vibe self doctor"]);
        }
        other => panic!("{other:?}"),
    }
    let out_of_order = doc_err(
        "  <prompt id=\"p\">do it<outcome>o</outcome><needs>n</needs><assert>true</assert></prompt>\n",
    );
    assert!(
        out_of_order.contains("out of order or repeated"),
        "{out_of_order}"
    );
    let body_after = doc_err("  <prompt id=\"p\">do it<assert>true</assert>tail</prompt>\n");
    assert!(
        body_after.contains("comes before its <needs>"),
        "{body_after}"
    );
}

/// A prompt cannot be checked by its output the way an example can, so it
/// needs an assert — and the ONE way to have none is to say so, which is
/// what the style linter reads (PROP-057 ##STYLE-PROMPT-FIRST).
#[test]
fn a_prompt_without_an_assert_must_declare_assert_none() {
    let silent = doc_err("  <prompt id=\"p\">Look at this.</prompt>\n");
    assert!(silent.contains("needs at least one <assert>"), "{silent}");
    let body = "  <prompt id=\"p\" assert=\"none\">Ask your agent to explain the tree.</prompt>\n";
    assert_byte_stable(body);
    match only_block(&doc(body)) {
        Block::Prompt { asserts, .. } => assert!(asserts.is_empty()),
        other => panic!("{other:?}"),
    }
    let both = doc_err("  <prompt id=\"p\" assert=\"none\">x<assert>true</assert></prompt>\n");
    assert!(both.contains("declares that it has no check"), "{both}");
    let bogus = doc_err("  <prompt id=\"p\" assert=\"vibe --version\">x</prompt>\n");
    assert!(bogus.contains("only ever \"none\""), "{bogus}");
    assert!(doc_err("  <prompt>x<assert>true</assert></prompt>\n").contains("needs an `id`"));
    assert!(
        doc_err("  <prompt id=\"p\"><assert>true</assert></prompt>\n")
            .contains("asks the agent nothing")
    );
}

/// An example id and a prompt id mint into the SHARED id namespace, so
/// either can be cited as `spec://…#id` and neither can collide with a
/// section anchor or a fact.
#[test]
fn example_and_prompt_ids_share_the_one_id_namespace() {
    let collision = doc_err(
        "  <prompt id=\"same\">x<assert>true</assert></prompt>\n  \
         <example id=\"same\" fixture=\"none\"><run>y</run><expect></expect></example>\n",
    );
    assert!(
        collision.contains("defined twice in this file"),
        "{collision}"
    );
    let with_section = from_xml_with(
        &format!(
            "<spec {NS}>\n  <example id=\"dup\" fixture=\"none\"><run>y</run><expect></expect></example>\n  <dup title=\"A section\"/>\n</spec>\n"
        ),
        Vocabulary::Doc,
    )
    .map(|_| ())
    .expect_err("an id is an id");
    assert!(
        with_section.message.contains("defined twice"),
        "{with_section}"
    );
}
