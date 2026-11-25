//! Terminal-artifact metadata parity and refusal tests. Kept out of
//! `tests.rs` so each Rust file remains below the repository length limit.

use super::*;

const NS: &str = "project";

fn warning_text(warnings: &[Warning]) -> String {
    warnings
        .iter()
        .map(|warning| warning.message.as_str())
        .collect::<Vec<_>>()
        .join("; ")
}

fn one_hash(source: &str) -> String {
    let (units, warnings) = parse_units("spec/DOC.xml", source, NS);
    assert!(warnings.is_empty(), "{}", warning_text(&warnings));
    assert_eq!(units.len(), 1);
    units[0].contentHash.clone()
}

fn rejected(source: &str, needle: &str) {
    let (units, warnings) = parse_units("spec/DOC.xml", source, NS);
    assert!(units.is_empty(), "a dialect error drops the document");
    assert_eq!(warnings.len(), 1, "{}", warning_text(&warnings));
    assert_eq!(warnings[0].code, "xml-dialect");
    assert!(
        warnings[0].message.contains(needle),
        "expected `{needle}` in `{}`",
        warnings[0].message
    );
}

#[test]
fn canonical_projection_matches_markdown_across_every_fact_carrier() {
    let xml = concat!(
        "<spec xmlns=\"https://vibevm.org/spec/1\">\n",
        "  <section id=\"s\" title=\"S\">\n",
        "    <p><fact id=\"P\" status=\"test/done\" requires=\"verification,implementation\">paragraph</fact></p>\n",
        "    <quote><Q fact=\"true\" status=\"spec/done\" requires=\"plan,specification\">quoted</Q></quote>\n",
        "    <list ordered=\"false\"><item><L fact=\"true\" status=\"impl/done\" requires=\"decision,documentation\">listed</L></item></list>\n",
        "    <table><tr><td>H</td></tr><tr><td><fact id=\"T\" status=\"spec/done\" requires=\"disposition,research\">cell</fact></td></tr></table>\n",
        "    <p><RUN fact=\"true\" status=\"test/done\" requires=\"verification\">run this</RUN></p>\n",
        "    <fence lang=\"text\" fact=\"RUN\">proof\n</fence>\n",
        "    <p><EXT fact=\"true\" status=\"spec/done\" requires=\"external\" ref=\"artifact://out\">outside</EXT></p>\n",
        "  </section>\n",
        "</spec>\n",
    );
    let expected_md = concat!(
        "## S {#s}\n\n",
        "@fact:P paragraph @requires:implementation,verification @status:test/done\n\n",
        "> @fact:Q quoted @requires:specification,plan @status:spec/done\n\n",
        "- @fact:L listed @requires:documentation,decision @status:impl/done\n\n",
        "| H |\n",
        "| --- |\n",
        "| @fact:T cell @requires:research,disposition @status:spec/done |\n\n",
        "@fact/code:RUN run this @requires:verification @status:test/done\n\n",
        "```text\n",
        "proof\n\n",
        "```\n\n",
        "@fact:EXT outside @requires:external <status stage=\"spec\" state=\"done\" ref=\"artifact://out\"/>\n\n",
    );

    let doc = super::reader::read_document(xml)
        .unwrap_or_else(|error| panic!("valid XML dialect: {}", error.message));
    let projected = super::mdout::document_lines(&doc).join("\n");
    assert_eq!(format!("{projected}\n"), expected_md);

    let (xml_units, xml_warnings) = parse_units("spec/DOC.xml", xml, NS);
    let (md_units, md_warnings) = crate::mdspec::parse_units("spec/DOC.md", expected_md, NS);
    assert!(xml_warnings.is_empty(), "{}", warning_text(&xml_warnings));
    assert!(md_warnings.is_empty(), "{}", warning_text(&md_warnings));
    assert_eq!(xml_units.len(), md_units.len());
    assert_eq!(
        xml_units
            .iter()
            .map(|unit| unit.anchor.as_str())
            .collect::<Vec<_>>(),
        ["s", "P", "L", "EXT"]
    );
    for (xml_unit, md_unit) in xml_units.iter().zip(&md_units) {
        assert_eq!(xml_unit.uri, md_unit.uri);
        assert_eq!(xml_unit.anchor, md_unit.anchor);
        assert_eq!(xml_unit.heading, md_unit.heading);
        assert_eq!(
            xml_unit.contentHash, md_unit.contentHash,
            "content identity for {}",
            xml_unit.anchor
        );
    }
}

#[test]
fn generic_and_named_facts_lower_to_the_same_canonical_metadata() {
    let generic = concat!(
        "<spec xmlns=\"https://vibevm.org/spec/1\"><p>",
        "<fact id=\"F\" status=\"spec/done\" requires=\"verification,specification\">body</fact>",
        "</p></spec>",
    );
    let named = concat!(
        "<spec xmlns=\"https://vibevm.org/spec/1\"><p>",
        "<F fact=\"true\" status=\"spec/done\" requires=\"verification,specification\">body</F>",
        "</p></spec>",
    );
    let (generic_units, generic_warnings) = parse_units("spec/DOC.xml", generic, NS);
    let (named_units, named_warnings) = parse_units("spec/DOC.xml", named, NS);
    assert!(
        generic_warnings.is_empty(),
        "{}",
        warning_text(&generic_warnings)
    );
    assert!(
        named_warnings.is_empty(),
        "{}",
        warning_text(&named_warnings)
    );
    assert_eq!(generic_units[0].uri, named_units[0].uri);
    assert_eq!(generic_units[0].heading, named_units[0].heading);
    assert_eq!(generic_units[0].contentHash, named_units[0].contentHash);
    assert_eq!(
        generic_units[0].heading,
        "body @requires:specification,verification @status:spec/done"
    );
}

#[test]
fn absence_presence_and_membership_have_distinct_content_identity() {
    let source = |requires: &str| {
        format!(
            "<spec xmlns=\"https://vibevm.org/spec/1\"><p><fact id=\"F\" status=\"spec/done\"{requires}>body</fact></p></spec>"
        )
    };
    let absent = one_hash(&source(""));
    let specification = one_hash(&source(" requires=\"specification\""));
    let plan = one_hash(&source(" requires=\"plan\""));
    assert_ne!(absent, specification);
    assert_ne!(specification, plan);

    let (xml_units, warnings) = parse_units("spec/DOC.xml", &source(""), NS);
    let (md_units, md_warnings) =
        crate::mdspec::parse_units("spec/DOC.md", "@fact:F body @status:spec/done\n", NS);
    assert!(warnings.is_empty(), "{}", warning_text(&warnings));
    assert!(md_warnings.is_empty(), "{}", warning_text(&md_warnings));
    assert_eq!(xml_units[0].contentHash, md_units[0].contentHash);
}

#[test]
fn invalid_requirement_sets_are_loud_dialect_errors() {
    for (requires, needle) in [
        ("", "non-empty"),
        (",plan", "empty member"),
        ("plan,", "empty member"),
        ("plan,,research", "empty member"),
        ("plan,other", "unknown"),
        ("plan,plan", "duplicate"),
        ("plan, research", "unknown"),
    ] {
        let xml = format!(
            "<spec xmlns=\"https://vibevm.org/spec/1\"><p><fact id=\"F\" status=\"spec/done\" requires=\"{requires}\">x</fact></p></spec>"
        );
        rejected(&xml, needle);
    }
}

#[test]
fn requirements_need_an_address_status_and_non_void_claim() {
    rejected(
        "<spec xmlns=\"https://vibevm.org/spec/1\"><p><F fact=\"true\" requires=\"plan\">x</F></p></spec>",
        "must carry a final `status`",
    );
    rejected(
        "<spec xmlns=\"https://vibevm.org/spec/1\"><p><fact id=\"F\" requires=\"plan\">x</fact></p></spec>",
        "must carry a final `status`",
    );
    rejected(
        "<spec xmlns=\"https://vibevm.org/spec/1\"><p><F fact=\"true\" status=\"spec/void\" requires=\"plan\">x</F></p></spec>",
        "state=void",
    );
    rejected(
        "<spec xmlns=\"https://vibevm.org/spec/1\"><table><tr><td><fact status=\"spec/done\" requires=\"plan\">x</fact></td></tr></table></spec>",
        "must have an address",
    );
    rejected(
        "<spec xmlns=\"https://vibevm.org/spec/1\"><p><fact id=\"9bad\" status=\"spec/done\" requires=\"plan\">x</fact></p></spec>",
        "valid fact address",
    );
}

#[test]
fn external_requires_exactly_one_nonempty_fact_reference() {
    for xml in [
        "<spec xmlns=\"https://vibevm.org/spec/1\"><p><F fact=\"true\" status=\"spec/done\" requires=\"external\">x</F></p></spec>",
        "<spec xmlns=\"https://vibevm.org/spec/1\"><p><F fact=\"true\" status=\"spec/done\" requires=\"external\" ref=\"\">x</F></p></spec>",
        "<spec xmlns=\"https://vibevm.org/spec/1\"><p><F fact=\"true\" status=\"spec/done\" requires=\"external\" ref=\"   \">x</F></p></spec>",
    ] {
        rejected(xml, "exactly one non-empty fact `ref`");
    }
    let duplicate = "<spec xmlns=\"https://vibevm.org/spec/1\"><p><F fact=\"true\" status=\"spec/done\" requires=\"external\" ref=\"one\" ref=\"two\">x</F></p></spec>";
    rejected(duplicate, "ill-formed attribute");

    let valid = "<spec xmlns=\"https://vibevm.org/spec/1\"><p><F fact=\"true\" status=\"spec/done\" requires=\"external\" ref=\"artifact://one\">x</F></p></spec>";
    let (units, warnings) = parse_units("spec/DOC.xml", valid, NS);
    assert!(warnings.is_empty(), "{}", warning_text(&warnings));
    assert_eq!(units.len(), 1);
    assert!(units[0].heading.contains("@requires:external <status"));
    assert!(units[0].heading.contains("ref=\"artifact://one\""));
}

#[test]
fn requires_remains_fact_owned_in_the_closed_xml_dialect() {
    for source in [
        "<spec xmlns=\"https://vibevm.org/spec/1\" requires=\"plan\"/>",
        "<spec xmlns=\"https://vibevm.org/spec/1\"><section id=\"s\" title=\"S\" requires=\"plan\"><p>x</p></section></spec>",
        "<spec xmlns=\"https://vibevm.org/spec/1\"><p requires=\"plan\">x</p></spec>",
        "<spec xmlns=\"https://vibevm.org/spec/1\"><status stage=\"spec\" state=\"done\" requires=\"plan\"/></spec>",
        "<spec xmlns=\"https://vibevm.org/spec/1\"><fence requires=\"plan\">x</fence></spec>",
    ] {
        rejected(source, "no `requires` attribute");
    }
}
