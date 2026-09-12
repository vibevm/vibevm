//! The documentation genre's own laws (PROP-045 §7), element by element.
//!
//! The live corpus (`tests/docs_corpus.rs`) covers `example` with `run`
//! and `expect`, `rule`, `derived`, `prompt` and a guarded SECTION. What
//! it has no page for yet — `note`, `figure`, `example ref`, `stderr`, the
//! `lang` and `exit` attributes, a guarded BLOCK, `assert="none"`, CDATA
//! and a revision pin — is covered here, so the vocabulary is verified
//! wider than its current use.

use crate::doc::{Block, Cond, CondAgent, CondOs, DerivedKind, NoteKind, SpecDoc, Vocabulary};
use crate::{from_markdown, from_xml, from_xml_with, to_markdown, to_xml};

const NS: &str = "xmlns=\"https://vibevm.org/spec/1\"";

/// One documentation page from its body, read with the genre open.
fn doc(body: &str) -> SpecDoc {
    let xml = format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<spec {NS}>\n{body}</spec>\n");
    from_xml_with(&xml, Vocabulary::Doc).unwrap_or_else(|e| panic!("{e}\n{xml}"))
}

/// The refusal a body earns with the genre open.
fn doc_err(body: &str) -> String {
    let xml = format!("<spec {NS}>\n{body}</spec>\n");
    from_xml_with(&xml, Vocabulary::Doc)
        .map(|_| ())
        .expect_err("must be refused")
        .message
}

/// The body's only block, unwrapped from its slot.
fn only_block(d: &SpecDoc) -> &Block {
    assert_eq!(d.preamble.len(), 1, "{:?}", d.preamble);
    &d.preamble[0].block
}

/// XML → IR → XML for a genre body, byte for byte.
fn assert_byte_stable(body: &str) {
    let xml = format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<spec {NS}>\n{body}</spec>\n");
    let ir = from_xml_with(&xml, Vocabulary::Doc).unwrap_or_else(|e| panic!("{e}\n{xml}"));
    assert_eq!(to_xml(&ir), xml, "not byte-stable");
}

// --- the gate (##DOC-VOCAB-BY-KIND, ##DOC-VOCAB-LOUD-IN-SPEC) -----------

/// Every one of the six element names is refused by the spec reader, and
/// the refusal names the genre and cites the rule — never a silent skip,
/// and never the older «the dialect has no <X> element», which would send
/// an author looking for a typo.
#[test]
fn the_genre_is_closed_under_the_spec_reader() {
    for body in [
        "  <example id=\"e\" fixture=\"none\"><run>x</run><expect></expect></example>\n",
        "  <rule ref=\"spec://g/n/p#A\"/>\n",
        "  <derived kind=\"cli-help\" ref=\"vibe --help\"/>\n",
        "  <note kind=\"tip\">t</note>\n",
        "  <figure src=\"a.png\" alt=\"a\"><caption>c</caption></figure>\n",
        "  <prompt id=\"p\">do it<assert>true</assert></prompt>\n",
    ] {
        let err = from_xml(&format!("<spec {NS}>\n{body}</spec>\n"))
            .map(|_| ())
            .expect_err("the spec vocabulary must refuse it");
        assert!(
            err.message.contains("documentation vocabulary"),
            "{body}: {err}"
        );
        assert!(
            err.message
                .contains("spec://org.vibevm.core/vibevm/common/PROP-045#DOC-VOCAB-BY-KIND"),
            "{body}: the refusal must cite the rule: {err}"
        );
    }
}

/// …and each reads into its own variant with the genre open.
#[test]
fn the_genre_reads_under_the_doc_reader() {
    assert!(matches!(
        only_block(&doc(
            "  <example id=\"e\" fixture=\"none\">\n    <run>x</run>\n    <expect></expect>\n  </example>\n"
        )),
        Block::Example { .. }
    ));
    assert!(matches!(
        only_block(&doc("  <example ref=\"e\"/>\n")),
        Block::ExampleRef { .. }
    ));
    assert!(matches!(
        only_block(&doc("  <rule ref=\"spec://g/n/p#A\"/>\n")),
        Block::Rule { .. }
    ));
    assert!(matches!(
        only_block(&doc("  <derived kind=\"cli-help\" ref=\"vibe --help\"/>\n")),
        Block::Derived { .. }
    ));
    assert!(matches!(
        only_block(&doc("  <note kind=\"warning\">Careful.</note>\n")),
        Block::Note { .. }
    ));
    assert!(matches!(
        only_block(&doc(
            "  <figure src=\"media/a.png\" alt=\"A tree\">\n    <caption>The tree.</caption>\n  </figure>\n"
        )),
        Block::Figure { .. }
    ));
    assert!(matches!(
        only_block(&doc(
            "  <prompt id=\"p\">Do the thing.\n    <assert>true</assert>\n  </prompt>\n"
        )),
        Block::Prompt { .. }
    ));
}

/// The discriminator (##DOC-VOCAB-DISCRIMINATOR): the four names that
/// already live in the corpus as named sections keep that meaning under
/// BOTH vocabularies, because a section carries `title=` and a block never
/// does. Without this rule, opening the genre would silently redefine
/// eight canonical files.
#[test]
fn a_title_makes_a_named_section_under_either_vocabulary() {
    for name in ["example", "rule", "derived", "note", "figure", "prompt"] {
        let body = format!("  <{name} title=\"A heading\">\n    <p>body</p>\n  </{name}>\n");
        for vocab in [Vocabulary::Spec, Vocabulary::Doc] {
            let d = from_xml_with(&format!("<spec {NS}>\n{body}</spec>\n"), vocab)
                .unwrap_or_else(|e| panic!("{name} under {vocab:?}: {e}"));
            assert_eq!(d.sections.len(), 1, "{name} under {vocab:?}");
            assert_eq!(d.sections[0].id.as_deref(), Some(name));
            assert_eq!(d.sections[0].title, "A heading");
            assert!(
                d.preamble.is_empty(),
                "{name}: it is a section, not a block"
            );
        }
    }
}

/// …and the writer agrees: the elementability blacklist does NOT grow for
/// the genre, so one IR serialises to the same bytes under either
/// vocabulary (##DOC-VOCAB-DISCRIMINATOR).
#[test]
fn the_writer_is_vocabulary_blind() {
    let md = "# T {#t}\n\n## A heading {#example}\n\nbody\n";
    let ir = from_markdown(md).expect("parses");
    let xml = to_xml(&ir);
    assert!(xml.contains("<example title=\"A heading\">"), "{xml}");
    // Both readers give the same IR back, so the bytes mean one thing.
    assert_eq!(from_xml_with(&xml, Vocabulary::Spec).unwrap(), ir);
    assert_eq!(from_xml_with(&xml, Vocabulary::Doc).unwrap(), ir);
}

// --- example (##ROW-DOCVOCAB-EXAMPLE) ----------------------------------

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

// --- when (##ROW-DOCVOCAB-WHEN, ##DOC-VOCAB-WHEN-SLOT) -----------------

/// `when` is a property of the SLOT, so it works on every block kind and
/// on sections — a paragraph, a table and an example alike. That is the
/// whole reason it is not a field on three of the variants.
#[test]
fn when_guards_any_slot_and_survives_the_round_trip() {
    let body = "  <p when=\"os:windows\">Windows only.</p>\n  \
                <fence lang=\"text\" when=\"os:linux\">linux</fence>\n  \
                <list ordered=\"false\" when=\"agent:codex\">\n    <item>one</item>\n  </list>\n  \
                <table when=\"os:macos\">\n    <tr>\n      <td>a</td>\n    </tr>\n  </table>\n  \
                <quote when=\"agent:claude\">quoted</quote>\n  \
                <note kind=\"tip\" when=\"os:windows\">tip</note>\n";
    assert_byte_stable(body);
    let d = doc(body);
    let conditions: Vec<Option<Cond>> = d.preamble.iter().map(|n| n.when).collect();
    assert_eq!(
        conditions,
        vec![
            Some(Cond::Os(CondOs::Windows)),
            Some(Cond::Os(CondOs::Linux)),
            Some(Cond::Agent(CondAgent::Codex)),
            Some(Cond::Os(CondOs::Macos)),
            Some(Cond::Agent(CondAgent::Claude)),
            Some(Cond::Os(CondOs::Windows)),
        ]
    );
}

/// A guarded section, in both the named and the generic form.
#[test]
fn when_guards_a_section_in_both_of_its_forms() {
    let named =
        "  <windows title=\"On Windows\" when=\"os:windows\">\n    <p>x</p>\n  </windows>\n";
    assert_byte_stable(named);
    assert_eq!(doc(named).sections[0].when, Some(Cond::Os(CondOs::Windows)));
    let generic = "  <section id=\"2-fast\" title=\"On macOS\" when=\"os:macos\">\n    <p>x</p>\n  </section>\n";
    assert_byte_stable(generic);
    assert_eq!(doc(generic).sections[0].when, Some(Cond::Os(CondOs::Macos)));
}

/// The condition vocabulary is closed, with a nearest-legal hint — and
/// `installed:` is deliberately outside it: a page varies by the reader's
/// platform and agent, never by what some project has installed.
#[test]
fn when_values_are_a_closed_list_with_a_hint() {
    let hint = doc_err("  <p when=\"os:widnows\">x</p>\n");
    assert!(hint.contains("did you mean `os:windows`?"), "{hint}");
    let unknown = doc_err("  <p when=\"tuesday\">x</p>\n");
    assert!(unknown.contains("unknown `when` condition"), "{unknown}");
    let installed = doc_err("  <p when=\"installed:org.vibevm.world/wal\">x</p>\n");
    assert!(
        installed.contains("unknown `when` condition"),
        "{installed}"
    );
    assert_eq!(Cond::all().len(), 8, "three systems and five agents");
}

/// Under the spec vocabulary `when` stays what it always was: a foreign
/// attribute, refused loudly. The dialect gains no conditional vocabulary.
#[test]
fn when_is_a_foreign_attribute_under_the_spec_reader() {
    for body in [
        "  <p when=\"os:windows\">x</p>\n",
        "  <table when=\"os:windows\">\n    <tr>\n      <td>a</td>\n    </tr>\n  </table>\n",
        "  <section id=\"s\" title=\"S\" when=\"os:windows\"/>\n",
    ] {
        let err = from_xml(&format!("<spec {NS}>\n{body}</spec>\n"))
            .map(|_| ())
            .expect_err("no conditional vocabulary in the spec dialect");
        assert!(err.message.contains("`when` attribute"), "{body}: {err}");
        assert!(
            err.message.contains("vocabulary is closed"),
            "{body}: {err}"
        );
    }
}

/// `when` is never accepted on a CHILD of the genre's elements: the
/// condition guards a block, not half of one.
#[test]
fn when_is_refused_on_the_genre_s_children() {
    let err = doc_err(
        "  <example id=\"e\" fixture=\"none\"><run when=\"os:windows\">x</run><expect></expect></example>\n",
    );
    assert!(
        err.contains("the <run> element has no `when` attribute"),
        "{err}"
    );
}

// --- verbatim texts and CDATA (##DOC-VOCAB-VERBATIM-TEXTS) -------------

/// CDATA is admitted in exactly the genre's verbatim elements, by an
/// explicit list rather than by dropping the check — and the refusal
/// elsewhere names that list.
#[test]
fn cdata_is_legal_in_the_verbatim_elements_and_named_elsewhere() {
    let body = "  <example id=\"e\" fixture=\"none\">\n    \
                <run>vibe query '$.a &lt; $.b'</run>\n    \
                <expect>a &lt; b &amp;&amp; c &gt; d</expect>\n  </example>\n";
    assert_byte_stable(body);
    match only_block(&doc(body)) {
        Block::Example { run, expect, .. } => {
            assert_eq!(run, "vibe query '$.a < $.b'");
            assert_eq!(expect, "a < b && c > d");
        }
        other => panic!("{other:?}"),
    }
    // The same text authored as CDATA reads identically…
    let cdata = format!(
        "<spec {NS}>\n  <example id=\"e\" fixture=\"none\">\n    <run><![CDATA[vibe query '$.a < $.b']]></run>\n    <expect><![CDATA[a < b && c > d]]></expect>\n  </example>\n</spec>\n"
    );
    let from_cdata = from_xml_with(&cdata, Vocabulary::Doc).expect("CDATA is legal here");
    match only_block(&from_cdata) {
        Block::Example { run, expect, .. } => {
            assert_eq!(run, "vibe query '$.a < $.b'");
            assert_eq!(expect, "a < b && c > d");
        }
        other => panic!("{other:?}"),
    }
    // …and the writer re-spells it as escaped text, one canonical form.
    assert!(
        to_xml(&from_cdata).contains("&lt;"),
        "{}",
        to_xml(&from_cdata)
    );
    // Outside that list the refusal names where CDATA may live.
    let err = doc_err("  <p><![CDATA[x]]></p>\n");
    assert!(err.contains("<run>, <expect>, <stderr>, <assert>"), "{err}");
}

// --- the Markdown projection (##DOC-VOCAB-MD-ONE-WAY) ------------------

/// The projection of every element, pinned. It is a READING surface: the
/// host scanners see the documentation only through it, so what it says is
/// what `facts check` and `progress` count.
#[test]
fn the_markdown_projection_of_each_element_is_pinned() {
    let d = doc(
        "  <example id=\"e\" fixture=\"none\">\n    <run>vibe --version</run>\n    \
         <expect>vibe 1.0.0</expect>\n    <stderr>warning</stderr>\n  </example>\n  \
         <example ref=\"e\"/>\n  \
         <rule ref=\"spec://g/n/p#ANCHOR\"/>\n  \
         <derived kind=\"cli-help\" ref=\"vibe --help\"/>\n  \
         <note kind=\"warning\">Mind the gap.</note>\n  \
         <figure src=\"media/a.png\" alt=\"A tree\">\n    <caption>The tree.</caption>\n  </figure>\n  \
         <prompt id=\"p\">Install vibe.\n    <needs>network</needs>\n    \
         <outcome>a version prints</outcome>\n    <assert>vibe --version</assert>\n  </prompt>\n",
    );
    let md = to_markdown(&d);
    assert!(md.contains("```sh\nvibe --version\n```"), "{md}");
    assert!(md.contains("```output\nvibe 1.0.0\n```"), "{md}");
    assert!(md.contains("```stderr\nwarning\n```"), "{md}");
    assert!(
        md.contains("Example `e` is copied from the source page"),
        "{md}"
    );
    assert!(md.contains("> <spec://g/n/p#ANCHOR>"), "{md}");
    assert!(
        md.contains("```text\ngenerated from cli-help: vibe --help\n```"),
        "{md}"
    );
    assert!(md.contains("> **Warning**\n> Mind the gap."), "{md}");
    assert!(md.contains("![A tree](media/a.png)\n\nThe tree."), "{md}");
    assert!(md.contains("```prompt\nInstall vibe.\n```"), "{md}");
    assert!(md.contains("- needs: network"), "{md}");
    assert!(md.contains("outcome: a version prints"), "{md}");
    assert!(md.contains("- assert: `vibe --version`"), "{md}");
    // The projection must PARSE, because the scanners read it.
    from_markdown(&md).unwrap_or_else(|e| panic!("{e}\n{md}"));
}

/// A guarded slot projects as a sub-heading naming the condition, one
/// level under its section (##ROW-DOCVOCAB-WHEN-MD); a guarded SECTION's
/// own heading names it.
#[test]
fn a_guarded_slot_projects_as_a_sub_heading() {
    let d = doc(
        "  <install title=\"Install\">\n    <p when=\"os:windows\">Unpack the archive.</p>\n  </install>\n  \
         <mac title=\"On macOS\" when=\"os:macos\">\n    <p>Use the script.</p>\n  </mac>\n",
    );
    let md = to_markdown(&d);
    assert!(md.contains("### os:windows\n\nUnpack the archive."), "{md}");
    assert!(md.contains("## On macOS (os:macos) {#mac}"), "{md}");
    from_markdown(&md).unwrap_or_else(|e| panic!("{e}\n{md}"));
}

/// `assert="none"` survives into the projection, so the style linter reads
/// the same fact from the Markdown as from the XML.
#[test]
fn an_illustrative_prompt_projects_its_missing_assert() {
    let md = to_markdown(&doc(
        "  <prompt id=\"p\" assert=\"none\">Ask for a tour.</prompt>\n",
    ));
    assert!(md.contains("- assert: none"), "{md}");
}

/// The recorded degradation, on one page rather than the whole corpus:
/// the genre's projection re-parses to a DIFFERENT IR. It is the law, and
/// it is why authoring documentation is XML only.
#[test]
fn doc_genre_is_not_round_trippable_through_markdown() {
    let d = doc(
        "  <example id=\"e\" fixture=\"none\">\n    <run>vibe --version</run>\n    \
         <expect>vibe 1.0.0</expect>\n  </example>\n",
    );
    let md = to_markdown(&d);
    let back = from_markdown(&md).expect("the projection parses");
    assert_ne!(back, d, "an example cannot come back from Markdown");
    // What comes back is the honest thing: two fences.
    assert_eq!(back.preamble.len(), 2);
    assert!(matches!(back.preamble[0].block, Block::Fence { .. }));
    assert!(matches!(back.preamble[1].block, Block::Fence { .. }));
}
