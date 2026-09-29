//! XML-scanner unit tests, out-of-line per the file-length budget (the
//! markdown scanner's `mdspec/tests.rs` sets the pattern). Included via
//! `#[cfg(test)] mod tests;`, so `use super::*` reaches the whole module.

use super::*;
use crate::config::{Config, ExternalSpec, SectionGrain};
use crate::generated::specmap::{SpecUnitKind, SpecUnitStatus};

const NS: &str = "project";

fn fmt_warnings(w: &[Warning]) -> String {
    w.iter()
        .map(|x| format!("{}:{} [{}] {}", x.file, x.line, x.code, x.message))
        .collect::<Vec<_>>()
        .join("; ")
}

/// One document, two serialisations: the XML fixture and its canonical
/// Markdown twin (byte-exact the projection emits — the trailing blank line
/// included, which is why the twin ends in two newlines).
const XML_FORM: &str = concat!(
    "<spec xmlns=\"https://vibevm.org/spec/1\">\n",
    "  <title id=\"root\">Demo document</title>\n",
    "  <status stage=\"spec\" state=\"work\"/>\n",
    "  <p><fact id=\"LEAD\" status=\"spec/done\">The lead fact.</fact></p>\n",
    "  <section id=\"laws\" title=\"The laws\">\n",
    "    <p>`req r2`</p>\n",
    "    <p>Body prose.</p>\n",
    "    <list ordered=\"false\">\n",
    "      <item><fact id=\"ITEM-FACT\" status=\"impl/done\">an item fact</fact></item>\n",
    "      <item>plain item</item>\n",
    "    </list>\n",
    "    <fence lang=\"text\">one\n",
    "two</fence>\n",
    "    <section id=\"nested\" title=\"Nested\">\n",
    "      <p><fact id=\"NESTED\" status=\"impl/work\">nested fact body</fact></p>\n",
    "    </section>\n",
    "  </section>\n",
    "</spec>\n",
);

/// The same document with section and fact identity carried by element names.
const NAMED_XML_FORM: &str = concat!(
    "<spec xmlns=\"https://vibevm.org/spec/1\">\n",
    "  <title id=\"root\">Demo document</title>\n",
    "  <status stage=\"spec\" state=\"work\"/>\n",
    "  <p><LEAD fact=\"true\" status=\"spec/done\">The lead fact.</LEAD></p>\n",
    "  <laws title=\"The laws\">\n",
    "    <p>`req r2`</p>\n",
    "    <p>Body prose.</p>\n",
    "    <list ordered=\"false\">\n",
    "      <item><ITEM-FACT fact=\"true\" status=\"impl/done\">an item fact</ITEM-FACT></item>\n",
    "      <item>plain item</item>\n",
    "    </list>\n",
    "    <fence lang=\"text\">one\n",
    "two</fence>\n",
    "    <nested title=\"Nested\">\n",
    "      <p><NESTED fact=\"true\" status=\"impl/work\">nested fact body</NESTED></p>\n",
    "    </nested>\n",
    "  </laws>\n",
    "</spec>\n",
);

const MD_TWIN: &str = concat!(
    "# Demo document {#root}\n\n",
    "<status stage=\"spec\" state=\"work\"/>\n\n",
    "@fact:LEAD The lead fact. @status:spec/done\n\n",
    "## The laws {#laws}\n\n",
    "`req r2`\n\n",
    "Body prose.\n\n",
    "- @fact:ITEM-FACT an item fact @status:impl/done\n",
    "- plain item\n\n",
    "```text\none\ntwo\n```\n\n",
    "### Nested {#nested}\n\n",
    "@fact:NESTED nested fact body @status:impl/work\n\n",
);

/// The comparable identity of a unit — everything EXCEPT `file` (the two
/// forms are two files) and `line` (native positions are the point). The
/// generated wire types carry no `Debug`, so the enums spell themselves.
fn kind_str(u: &SpecUnit) -> &'static str {
    match u.kind.as_deref() {
        Some(SpecUnitKind::Prop) => "prop",
        Some(SpecUnitKind::Req) => "req",
        Some(SpecUnitKind::Design) => "design",
        Some(SpecUnitKind::Guide) => "guide",
        None => "-",
    }
}

fn status_str(u: &SpecUnit) -> &'static str {
    match u.status.as_deref() {
        Some(SpecUnitStatus::Planned) => "planned",
        Some(SpecUnitStatus::Disputed) => "disputed",
        None => "-",
    }
}

fn identity(u: &SpecUnit) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}|{}|{}",
        u.uri,
        u.anchor,
        u.heading,
        u.contentHash,
        kind_str(u),
        u.revision
            .as_deref()
            .map(|r| r.to_string())
            .unwrap_or("-".into()),
        status_str(u),
        u.disputes
            .as_deref()
            .map(|d| d.as_str().to_string())
            .unwrap_or("-".into()),
    )
}

#[test]
fn one_document_both_forms_gives_one_unit_set() {
    // THE parity law: the same document in XML and in its canonical
    // Markdown spelling yields the SAME units — anchors, uris, doc-paths,
    // headings, kinds, revisions AND content hashes. Only `file` (two
    // files) and `line` (native positions, asserted below) differ.
    let (xu, xw) = parse_units("spec/test/DOC.xml", XML_FORM, NS);
    let (mu, mw) = crate::mdspec::parse_units("spec/test/DOC.md", MD_TWIN, NS);
    assert!(xw.is_empty(), "xml: {}", fmt_warnings(&xw));
    assert!(mw.is_empty(), "md: {}", fmt_warnings(&mw));
    assert_eq!(xu.len(), mu.len(), "same unit count");
    for (x, m) in xu.iter().zip(mu.iter()) {
        assert_eq!(
            identity(x),
            identity(m),
            "unit `{}` vs `{}`",
            identity(x),
            identity(m)
        );
        assert_eq!(x.docPath, m.docPath);
        assert_eq!(x.file, "spec/test/DOC.xml");
        assert_eq!(m.file, "spec/test/DOC.md");
    }
    let anchors: Vec<&str> = xu.iter().map(|u| u.anchor.as_str()).collect();
    assert_eq!(
        anchors,
        ["root", "LEAD", "laws", "ITEM-FACT", "nested", "NESTED"],
        "document order"
    );
    // kind/revision ride from the leading `<p>` exactly as from the kind
    // line: `laws` is `req r2`; the title (a status line first) and
    // `nested` (a fact paragraph first) carry none.
    assert!(matches!(xu[2].kind.as_deref(), Some(SpecUnitKind::Req)));
    assert_eq!(xu[2].revision.as_deref(), Some(&2));
    assert!(xu[0].kind.is_none());
    assert!(xu[4].kind.is_none());
}

#[test]
fn generic_and_fully_named_xml_forms_give_one_unit_set() {
    let (generic, generic_warnings) = parse_units("spec/test/DOC.xml", XML_FORM, NS);
    let (named, named_warnings) = parse_units("spec/test/DOC.xml", NAMED_XML_FORM, NS);
    assert!(
        generic_warnings.is_empty(),
        "generic: {}",
        fmt_warnings(&generic_warnings)
    );
    assert!(
        named_warnings.is_empty(),
        "named: {}",
        fmt_warnings(&named_warnings)
    );
    assert_eq!(generic.len(), named.len());
    for (generic, named) in generic.iter().zip(named.iter()) {
        assert_eq!(identity(generic), identity(named));
        assert_eq!(generic.docPath, named.docPath);
    }
}

#[test]
fn named_section_positions_are_native_source_lines() {
    let (units, warnings) = parse_units("spec/test/DOC.xml", NAMED_XML_FORM, NS);
    assert!(warnings.is_empty(), "{}", fmt_warnings(&warnings));
    let line_of = |anchor: &str| units.iter().find(|u| u.anchor == anchor).map(|u| u.line);
    assert_eq!(line_of("laws"), Some(5), "the <laws> line");
    assert_eq!(line_of("nested"), Some(14), "the <nested> line");
    assert_eq!(line_of("NESTED"), Some(15), "the nested fact line");
}

#[test]
fn positions_are_native_source_lines() {
    // The engine side lives without the projection caveat (PROP-045 §4):
    // each unit's line is the element's line in the XML source, not the
    // projection's.
    let (units, warnings) = parse_units("spec/test/DOC.xml", XML_FORM, NS);
    assert!(warnings.is_empty(), "{}", fmt_warnings(&warnings));
    let line_of = |anchor: &str| units.iter().find(|u| u.anchor == anchor).map(|u| u.line);
    assert_eq!(line_of("root"), Some(2), "the <title> line");
    assert_eq!(line_of("LEAD"), Some(4), "the <fact> line");
    assert_eq!(line_of("laws"), Some(5), "the <section> line");
    assert_eq!(line_of("ITEM-FACT"), Some(9), "the item's <fact> line");
    assert_eq!(line_of("nested"), Some(14));
    assert_eq!(line_of("NESTED"), Some(15));
}

#[test]
fn a_foreign_element_is_a_loud_error() {
    let xml = "<spec xmlns=\"https://vibevm.org/spec/1\">\n  <bogus/>\n</spec>\n";
    let (units, warnings) = parse_units("spec/test/DOC.xml", xml, NS);
    assert!(units.is_empty(), "the document is dropped whole");
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].code, "xml-dialect");
    assert_eq!(warnings[0].line, 2, "the native line of the construct");
    assert!(
        warnings[0].message.contains("no <bogus>"),
        "{}",
        warnings[0].message
    );
    assert!(
        warnings[0].message.contains("vocabulary is closed"),
        "{}",
        warnings[0].message
    );
}

#[test]
fn named_fact_discriminator_accepts_only_true() {
    for value in ["false", "yes"] {
        let xml = format!(
            "<spec xmlns=\"https://vibevm.org/spec/1\"><p><CLAIM fact=\"{value}\">x</CLAIM></p></spec>"
        );
        let (units, warnings) = parse_units("spec/test/DOC.xml", &xml, NS);
        assert!(units.is_empty());
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].code, "xml-dialect");
        assert!(
            warnings[0].message.contains("must be \"true\""),
            "{}",
            warnings[0].message
        );
    }
}

#[test]
fn unknown_unit_element_without_fact_discriminator_stays_unknown() {
    let xml = concat!(
        "<spec xmlns=\"https://vibevm.org/spec/1\">",
        "<p><CLAIM>x</CLAIM></p></spec>"
    );
    let (units, warnings) = parse_units("spec/test/DOC.xml", xml, NS);
    assert!(units.is_empty());
    assert_eq!(warnings[0].code, "xml-dialect");
    assert!(warnings[0].message.contains("no <CLAIM>"));
}

#[test]
fn named_fact_name_obeys_elementability_and_fact_id_grammar() {
    for (element, needle) in [("bad.id", "fact-id grammar"), ("table", "not elementable")] {
        let xml = format!(
            "<spec xmlns=\"https://vibevm.org/spec/1\"><p><{element} fact=\"true\">x</{element}></p></spec>"
        );
        let (units, warnings) = parse_units("spec/test/DOC.xml", &xml, NS);
        assert!(units.is_empty());
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].code, "xml-dialect");
        assert!(
            warnings[0].message.contains(needle),
            "{}",
            warnings[0].message
        );
    }
}

#[test]
fn a_foreign_attribute_is_a_loud_error() {
    let xml = concat!(
        "<spec xmlns=\"https://vibevm.org/spec/1\">\n",
        "  <section id=\"x\" title=\"T\" bogus=\"1\"><p>t</p></section>\n",
        "</spec>\n"
    );
    let (units, warnings) = parse_units("spec/test/DOC.xml", xml, NS);
    assert!(units.is_empty());
    assert_eq!(warnings[0].code, "xml-dialect");
    assert!(
        warnings[0].message.contains("no `bogus` attribute"),
        "{}",
        warnings[0].message
    );
    assert_eq!(warnings[0].line, 2);
}

#[test]
fn dtd_processing_instruction_and_entity_are_refused() {
    for (xml, needle) in [
        (
            "<!DOCTYPE spec>\n<spec xmlns=\"https://vibevm.org/spec/1\"/>\n",
            "DTD",
        ),
        (
            "<?pi?>\n<spec xmlns=\"https://vibevm.org/spec/1\"/>\n",
            "processing instruction",
        ),
        (
            "<spec xmlns=\"https://vibevm.org/spec/1\">\n  <p>&custom;</p>\n</spec>\n",
            "entities",
        ),
    ] {
        let (_, warnings) = parse_units("spec/test/DOC.xml", xml, NS);
        assert_eq!(warnings[0].code, "xml-dialect", "{xml}");
        assert!(
            warnings[0].message.contains(needle),
            "{}",
            warnings[0].message
        );
    }
}

#[test]
fn kind_comes_from_the_first_paragraph_only() {
    let head = "<spec xmlns=\"https://vibevm.org/spec/1\">\n";
    let tail = "</spec>\n";
    // A kind line in a LATER paragraph is prose, not a kind.
    let later = format!(
        "{head}<section id=\"s\" title=\"S\">\n  <p>intro</p>\n  <p>`req r1`</p>\n</section>{tail}"
    );
    let (units, warnings) = parse_units("spec/test/DOC.xml", &later, NS);
    assert!(warnings.is_empty(), "{}", fmt_warnings(&warnings));
    assert!(units[0].kind.is_none());
    // A list first, then the kind paragraph: the projection's first body
    // line is the item marker — no kind.
    let list_first = format!(
        "{head}<section id=\"s\" title=\"S\">\n  <list ordered=\"false\"><item>i</item></list>\n  <p>`req r1`</p>\n</section>{tail}"
    );
    let (units, warnings) = parse_units("spec/test/DOC.xml", &list_first, NS);
    assert!(warnings.is_empty(), "{}", fmt_warnings(&warnings));
    assert!(units[0].kind.is_none());
    // A section <status> first: the status element line is what the Markdown
    // form puts on the first body line — no kind.
    let status_first = format!(
        "{head}<section id=\"s\" title=\"S\">\n  <status stage=\"spec\" state=\"done\"/>\n  <p>`req r1`</p>\n</section>{tail}"
    );
    let (units, _) = parse_units("spec/test/DOC.xml", &status_first, NS);
    assert!(units[0].kind.is_none());
    // A malformed kind line warns and keeps the unit, as in markdown.
    let malformed =
        format!("{head}<section id=\"s\" title=\"S\">\n  <p>`req rX`</p>\n</section>{tail}");
    let (units, warnings) = parse_units("spec/test/DOC.xml", &malformed, NS);
    assert_eq!(units.len(), 1);
    assert_eq!(warnings[0].code, "malformed-kind-line");
}

#[test]
fn section_ids_and_fact_ids_share_one_namespace() {
    let xml = concat!(
        "<spec xmlns=\"https://vibevm.org/spec/1\">\n",
        "  <section id=\"dup\" title=\"T\">\n",
        "    <p><fact id=\"dup\">twice</fact></p>\n",
        "  </section>\n",
        "</spec>\n"
    );
    let (units, warnings) = parse_units("spec/test/DOC.xml", xml, NS);
    assert_eq!(units.len(), 2, "both units kept, as in markdown");
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].code, "duplicate-anchor");
    assert_eq!(warnings[0].line, 3, "the fact's native line");
}

#[test]
fn an_invalid_section_id_warns_and_skips_but_content_still_scans() {
    let xml = concat!(
        "<spec xmlns=\"https://vibevm.org/spec/1\">\n",
        "  <section id=\"9lives\" title=\"T\">\n",
        "    <p><fact id=\"OK\">body</fact></p>\n",
        "  </section>\n",
        "</spec>\n"
    );
    let (units, warnings) = parse_units("spec/test/DOC.xml", xml, NS);
    assert_eq!(warnings[0].code, "invalid-anchor");
    let anchors: Vec<&str> = units.iter().map(|u| u.anchor.as_str()).collect();
    assert_eq!(anchors, ["OK"], "the fact under the skipped heading mints");
}

#[test]
fn table_and_quote_facts_are_below_the_unit_grain() {
    // A <td>/<quote> fact is a progress fact, not a specmap unit: the
    // markdown scanner reads units only from a line's or item's FIRST
    // token, and the projection's table/quote lines open with `|` / `> `.
    // The section unit's hash still covers them (they are in its span).
    let xml = concat!(
        "<spec xmlns=\"https://vibevm.org/spec/1\">\n",
        "  <section id=\"s\" title=\"S\">\n",
        "    <table><tr><td>H</td></tr><tr><td><fact status=\"impl/done\">cell</fact></td></tr></table>\n",
        "    <quote><fact id=\"Q\">quoted</fact></quote>\n",
        "  </section>\n",
        "</spec>\n"
    );
    let (units, warnings) = parse_units("spec/test/DOC.xml", xml, NS);
    assert!(warnings.is_empty(), "{}", fmt_warnings(&warnings));
    let anchors: Vec<&str> = units.iter().map(|u| u.anchor.as_str()).collect();
    assert_eq!(anchors, ["s"]);
}

#[test]
fn a_typed_fact_is_below_the_unit_grain_on_both_sides() {
    // The `@fact/code:` binding is PROGRESS grain, not specmap grain: the
    // markdown scanner does not read that spelling as an anchor, so the
    // XML side mints no unit for a fact the next block's fence binds. The
    // typed line still rides INSIDE the section's span (the hashes agree).
    let xml = concat!(
        "<spec xmlns=\"https://vibevm.org/spec/1\">\n",
        "  <section id=\"s\" title=\"S\">\n",
        "    <p><fact id=\"RUN\" status=\"impl/done\">run this</fact></p>\n",
        "    <fence lang=\"bash\" fact=\"RUN\">cargo test\n</fence>\n",
        "  </section>\n",
        "</spec>\n"
    );
    let md = concat!(
        "## S {#s}\n\n",
        "@fact/code:RUN run this @status:impl/done\n\n",
        "```bash\ncargo test\n\n```\n\n"
    );
    let (xu, xw) = parse_units("spec/test/DOC.xml", xml, NS);
    let (mu, mw) = crate::mdspec::parse_units("spec/test/DOC.md", md, NS);
    assert!(xw.is_empty(), "{}", fmt_warnings(&xw));
    assert!(mw.is_empty(), "{}", fmt_warnings(&mw));
    assert!(
        xu.iter().all(|u| u.anchor != "RUN"),
        "the typed fact mints no XML unit"
    );
    assert!(
        mu.iter().all(|u| u.anchor != "RUN"),
        "the typed fact mints no MD unit either"
    );
    // The section units agree — hash included: the typed line and the
    // bound fence are inside the span on both sides.
    assert_eq!(xu.len(), 1);
    assert_eq!(mu.len(), 1);
    assert_eq!(identity(&xu[0]), identity(&mu[0]));
}

#[test]
fn long_section_measures_native_lines_at_leaf_grain() {
    // A leaf section spanning 4 native lines (`<section>` line 2 through
    // `</section>` line 5, inclusive) fires at threshold 4; at 5 it does
    // not. A container is skipped at leaf grain, measured at all.
    let leaf = concat!(
        "<spec xmlns=\"https://vibevm.org/spec/1\">\n", // 1
        "  <section id=\"s\" title=\"S\">\n",           // 2
        "    <p>one</p>\n",                             // 3
        "    <p>two</p>\n",                             // 4
        "  </section>\n",                               // 5
        "</spec>\n"                                     // 6
    );
    let (_, w) = parse_units_with("spec/test/DOC.xml", leaf, NS, 4, SectionGrain::Leaf);
    assert_eq!(
        w.iter().filter(|x| x.code == "long-section").count(),
        1,
        "{}",
        fmt_warnings(&w)
    );
    assert_eq!(w[0].line, 2);
    let (_, w) = parse_units_with("spec/test/DOC.xml", leaf, NS, 5, SectionGrain::Leaf);
    assert!(w.iter().all(|x| x.code != "long-section"));
    // The same leaf measured at `all` grain behaves identically; a nested
    // container fires only there.
    let container = concat!(
        "<spec xmlns=\"https://vibevm.org/spec/1\">\n",
        "  <section id=\"p\" title=\"P\">\n",
        "    <section id=\"c\" title=\"C\">\n",
        "      <p>x</p>\n",
        "    </section>\n",
        "  </section>\n",
        "</spec>\n"
    );
    let (_, w) = parse_units_with("spec/test/DOC.xml", container, NS, 5, SectionGrain::Leaf);
    assert!(
        w.iter().all(|x| x.code != "long-section"),
        "{}",
        fmt_warnings(&w)
    );
    let (_, w) = parse_units_with("spec/test/DOC.xml", container, NS, 5, SectionGrain::All);
    assert_eq!(w.iter().filter(|x| x.code == "long-section").count(), 1);
}

#[test]
fn the_walk_reads_xml_under_the_same_address() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("vibevm/vibespecs/test")).unwrap();
    std::fs::write(dir.path().join("vibevm/vibespecs/test/DOC.xml"), XML_FORM).unwrap();
    let (units, warnings) = crate::mdspec::scan_spec_tree(dir.path(), &Config::default());
    assert!(warnings.is_empty(), "{}", fmt_warnings(&warnings));
    assert_eq!(units.len(), 6);
    assert_eq!(units[0].uri, "spec://project/test/DOC#root");
    assert_eq!(units[0].docPath, "test/DOC");
    assert_eq!(units[0].file, "vibevm/vibespecs/test/DOC.xml");
}

#[test]
fn a_pair_in_one_directory_is_loud_and_both_halves_skipped() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("vibevm/vibespecs")).unwrap();
    std::fs::write(
        dir.path().join("vibevm/vibespecs/A.md"),
        "## A {#a}\nmd body\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("vibevm/vibespecs/A.xml"),
        "<spec xmlns=\"https://vibevm.org/spec/1\"><section id=\"a\" title=\"A\"><p>xml body</p></section></spec>",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("vibevm/vibespecs/B.md"),
        "## B {#b}\nbody\n",
    )
    .unwrap();
    let (units, warnings) = crate::mdspec::scan_spec_tree(dir.path(), &Config::default());
    assert_eq!(warnings.len(), 1, "{}", fmt_warnings(&warnings));
    assert_eq!(warnings[0].code, "pair-collision");
    assert_eq!(warnings[0].line, 0);
    assert!(
        warnings[0].message.contains("one document, one form"),
        "{}",
        warnings[0].message
    );
    assert!(
        warnings[0].message.contains("vibevm/vibespecs/A.md"),
        "{}",
        warnings[0].message
    );
    assert!(
        warnings[0].message.contains("vibevm/vibespecs/A.xml"),
        "{}",
        warnings[0].message
    );
    // Both halves skipped — no units from A in either form, B unaffected.
    let anchors: Vec<&str> = units.iter().map(|u| u.anchor.as_str()).collect();
    assert_eq!(anchors, ["b"]);
    // The same stem in another directory is two documents, not a pair.
    std::fs::create_dir_all(dir.path().join("vibevm/vibespecs/deep")).unwrap();
    std::fs::write(
        dir.path().join("vibevm/vibespecs/deep/A.xml"),
        "<spec xmlns=\"https://vibevm.org/spec/1\"/>",
    )
    .unwrap();
    let (units, warnings) = crate::mdspec::scan_spec_tree(dir.path(), &Config::default());
    assert_eq!(
        warnings.len(),
        1,
        "the removed pair still collides, the deep same-stem does not: {}",
        fmt_warnings(&warnings)
    );
    assert!(units.iter().all(|u| u.anchor != "a"));
}

#[test]
fn external_xml_units_mint_under_their_namespace() {
    let dir = tempfile::tempdir().unwrap();
    let ext = dir.path().join("vibedeps/some-flow/0.3.0/spec/mechanisms");
    std::fs::create_dir_all(&ext).unwrap();
    std::fs::write(
        ext.join("ENGINE-X-v0.1.xml"),
        concat!(
            "<spec xmlns=\"https://vibevm.org/spec/1\">\n",
            "  <section id=\"rules\" title=\"Rules\">\n",
            "    <p>`req r1`</p>\n",
            "  </section>\n",
            "</spec>\n"
        ),
    )
    .unwrap();
    let cfg = Config {
        external_specs: vec![ExternalSpec {
            namespace: "some-flow".into(),
            root: "vibedeps/some-flow/0.3.0/spec".into(),
        }],
        ..Config::default()
    };
    let units = crate::mdspec::scan_external_units(dir.path(), &cfg);
    assert_eq!(units.len(), 1);
    assert_eq!(
        units[0].uri,
        "spec://some-flow/mechanisms/ENGINE-X-v0.1#rules"
    );
    assert_eq!(units[0].revision.as_deref(), Some(&1));
}
