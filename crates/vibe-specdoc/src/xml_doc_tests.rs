//! The documentation genre's own laws (PROP-045 §7), element by element.
//!
//! The live corpus (`tests/docs_corpus.rs`) covers `example` with `run`
//! and `expect`, `rule`, `derived`, `prompt` and a guarded SECTION. What
//! it has no page for yet — `note`, `figure`, `example ref`, `stderr`, the
//! `lang` and `exit` attributes, a guarded BLOCK, `assert="none"`, CDATA
//! and a revision pin — is covered here, so the vocabulary is verified
//! wider than its current use.
//!
//! This cell holds the shared fixtures, the vocabulary gating, the `when`
//! guard and CDATA. The six elements have their own laws asserted in
//! [`crate::xml_doc_element_tests`], and the one-way Markdown projection
//! in [`crate::xml_doc_markdown_tests`]; both read the fixtures below, so
//! every cell scans the same documents.

use crate::doc::{Block, Cond, CondAgent, CondOs, SpecDoc, Vocabulary};
use crate::{from_markdown, from_xml, from_xml_with, to_xml};

pub(crate) const NS: &str = "xmlns=\"https://vibevm.org/spec/1\"";

/// One documentation page from its body, read with the genre open.
pub(crate) fn doc(body: &str) -> SpecDoc {
    let xml = format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<spec {NS}>\n{body}</spec>\n");
    from_xml_with(&xml, Vocabulary::Doc).unwrap_or_else(|e| panic!("{e}\n{xml}"))
}

/// The refusal a body earns with the genre open.
pub(crate) fn doc_err(body: &str) -> String {
    let xml = format!("<spec {NS}>\n{body}</spec>\n");
    from_xml_with(&xml, Vocabulary::Doc)
        .map(|_| ())
        .expect_err("must be refused")
        .message
}

/// The body's only block, unwrapped from its slot.
pub(crate) fn only_block(d: &SpecDoc) -> &Block {
    assert_eq!(d.preamble.len(), 1, "{:?}", d.preamble);
    &d.preamble[0].block
}

/// XML → IR → XML for a genre body, byte for byte.
pub(crate) fn assert_byte_stable(body: &str) {
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
