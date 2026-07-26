//! Markdown-scanner unit tests, out-of-line per the file-length budget
//! (the grammar crate's `src/lib/tests.rs` sets the pattern). Included via
//! `#[cfg(test)] mod tests;`, so `use super::*` is unchanged from the
//! inline form.

use super::*;

const DOC: &str = "spec/test/DOC.md";
const NS: &str = "project";

fn fmt_warnings(w: &[Warning]) -> String {
    w.iter()
        .map(|x| format!("{}:{} [{}] {}", x.file, x.line, x.code, x.message))
        .collect::<Vec<_>>()
        .join("; ")
}

#[test]
fn anchored_heading_becomes_a_unit_with_span_hash() {
    let text = "# Title {#root}\n\nbody one\n\n## Sub {#sub-part}\n\nbody two\n\n## Next {#next-part}\nafter\n";
    let (units, warnings) = parse_units(DOC, text, NS);
    assert!(warnings.is_empty(), "{}", fmt_warnings(&warnings));
    assert_eq!(units.len(), 3);
    assert_eq!(units[0].anchor, "root");
    assert_eq!(units[0].uri, "spec://project/test/DOC#root");
    assert_eq!(units[0].docPath, "test/DOC");
    assert_eq!(units[0].file, DOC);
    assert_eq!(units[0].line, 1);
    // The root unit spans the whole document (no same-or-higher
    // heading follows); the sub unit ends before `## Next`.
    assert_eq!(units[1].anchor, "sub-part");
    assert_eq!(units[2].anchor, "next-part");
    assert_ne!(units[1].contentHash, units[2].contentHash);
}

#[test]
fn unanchored_heading_ends_a_span_but_is_not_a_unit() {
    let text = "## A {#a}\nbody\n## Plain heading\nmore\n## B {#b}\nbody b\n";
    let (units, _) = parse_units(DOC, text, NS);
    assert_eq!(units.len(), 2);
    // A's span must stop at `## Plain heading`.
    let a_hash = units[0].contentHash.clone();
    let (units2, _) = parse_units(DOC, "## A {#a}\nbody\n", NS);
    assert_eq!(a_hash, units2[0].contentHash);
}

#[test]
fn kind_line_parses_kind_revision_status() {
    let text = "### R {#req-x}\n`req r2`\n\nMUST hold.\n\n### P {#req-y}\n`req r1 planned`\n\n### D {#req-z}\n`req r3 disputed(#req-x)` — see the pair.\n";
    let (units, warnings) = parse_units(DOC, text, NS);
    assert!(warnings.is_empty(), "{}", fmt_warnings(&warnings));
    assert!(matches!(units[0].kind.as_deref(), Some(SpecUnitKind::Req)));
    assert_eq!(units[0].revision.as_deref(), Some(&2));
    assert!(units[0].status.is_none());
    assert!(matches!(
        units[1].status.as_deref(),
        Some(SpecUnitStatus::Planned)
    ));
    assert!(matches!(
        units[2].status.as_deref(),
        Some(SpecUnitStatus::Disputed)
    ));
    assert_eq!(units[2].disputes.as_deref(), Some(&"req-x".to_string()));
}

#[test]
fn ordinary_inline_code_is_not_a_kind_line() {
    let text = "### T {#t}\n`vibe install` does things.\n";
    let (units, warnings) = parse_units(DOC, text, NS);
    assert!(warnings.is_empty(), "{}", fmt_warnings(&warnings));
    assert!(units[0].kind.is_none());
}

#[test]
fn malformed_kind_line_warns_but_keeps_the_unit() {
    let text = "### T {#t}\n`req rX`\n";
    let (units, warnings) = parse_units(DOC, text, NS);
    assert_eq!(units.len(), 1);
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].code, "malformed-kind-line");
    let text = "### T {#t}\n`req r1 someday`\n";
    let (_, warnings) = parse_units(DOC, text, NS);
    assert_eq!(warnings[0].code, "malformed-kind-line");
}

#[test]
fn duplicate_anchor_in_one_file_warns_and_keeps_both() {
    let text = "## A {#phases}\none\n## B {#phases}\ntwo\n";
    let (units, warnings) = parse_units(DOC, text, NS);
    assert_eq!(units.len(), 2);
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].code, "duplicate-anchor");
    assert_eq!(warnings[0].line, 3);
}

#[test]
fn invalid_anchor_warns_and_skips() {
    // A digit head — `{#Bad_Anchor}` used to serve here and is a legal id
    // since DRIFT-034, so the fixture moved to the one shape still refused.
    let text = "## A {#9lives}\nbody\n";
    let (units, warnings) = parse_units(DOC, text, NS);
    assert!(units.is_empty());
    assert_eq!(warnings[0].code, "invalid-anchor");
}

#[test]
fn a_heading_anchor_may_be_written_the_way_a_fact_id_may() {
    // The widening, at the grain a document is actually written in: an
    // underscore and the upper register both mint units where the kebab law
    // would have warned and skipped them.
    let text = "# D {#root}\n\n## A {#Some_Anchor}\none\n\n## B {#TWO-TREES}\ntwo\n";
    let (units, warnings) = parse_units(DOC, text, NS);
    assert!(warnings.is_empty(), "{}", fmt_warnings(&warnings));
    let anchors: Vec<&str> = units.iter().map(|u| u.anchor.as_str()).collect();
    assert_eq!(anchors, ["root", "Some_Anchor", "TWO-TREES"]);
}

#[test]
fn a_heading_anchor_and_a_fact_id_differing_only_in_case_are_two_names() {
    // The house convention: a section heading `{#two-trees}` with that
    // section's lead normative fact `##TWO-TREES` under it. The two differ
    // byte for byte, so they are two units and no duplicate — detection is
    // byte-exact and nothing folds case. Widening the anchor grammar did not
    // change that; it is why no fold was needed (DRIFT-034 §2).
    let text = "# D {#root}\n\n## Two trees {#two-trees}\n\n##TWO-TREES The lead fact.\n";
    let (units, warnings) = parse_units(DOC, text, NS);
    assert!(warnings.is_empty(), "{}", fmt_warnings(&warnings));
    assert_eq!(unit(&units, "two-trees").line, 3);
    assert_eq!(unit(&units, "TWO-TREES").line, 5);
    // …and the byte-exact duplicate the widening newly *permits* writing is
    // still caught, unchanged.
    let dup = "# D {#root}\n\n## Two trees {#TWO-TREES}\n\n##TWO-TREES The lead fact.\n";
    let (_, warnings) = parse_units(DOC, dup, NS);
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].code, "duplicate-anchor");
}

#[test]
fn root_spec_docs_are_scanned_and_other_root_md_is_not() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("spec")).unwrap();
    std::fs::write(
        dir.path().join("spec").join("X.md"),
        "## In tree {#in-tree}\nbody\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("ROOT-SPEC.md"),
        "# demo {#root}\n\n## Section 5. The task graph {#task-graph}\nbody\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("README.md"), "# Readme {#root}\n").unwrap();
    let cfg = Config {
        root_spec_docs: vec!["ROOT-SPEC.md".into()],
        ..Config::default()
    };
    let (units, warnings) = scan_spec_tree(dir.path(), &cfg);
    assert!(warnings.is_empty(), "{}", fmt_warnings(&warnings));
    let uris: Vec<&str> = units.iter().map(|u| u.uri.as_str()).collect();
    assert!(uris.contains(&"spec://project/X#in-tree"));
    assert!(uris.contains(&"spec://project/ROOT-SPEC#root"));
    assert!(uris.contains(&"spec://project/ROOT-SPEC#task-graph"));
    // README-class root markdown stays out of the inventory.
    assert_eq!(units.len(), 3);
}

#[test]
fn external_specs_resolve_under_their_own_namespace_and_are_skipped_when_absent() {
    let dir = tempfile::tempdir().unwrap();
    let ext = dir.path().join("vibedeps/some-flow/0.3.0/spec");
    std::fs::create_dir_all(ext.join("mechanisms")).unwrap();
    std::fs::write(
        ext.join("mechanisms/ENGINE-X-v0.1.md"),
        "## Rules {#rules}\n`req r1`\n\nbody\n",
    )
    .unwrap();
    let cfg = Config {
        external_specs: vec![
            crate::config::ExternalSpec {
                namespace: "some-flow".into(),
                root: "vibedeps/some-flow/0.3.0/spec".into(),
            },
            // A not-yet-installed package: skipped, never fatal.
            crate::config::ExternalSpec {
                namespace: "ghost".into(),
                root: "vibedeps/ghost/1.0.0/spec".into(),
            },
        ],
        ..Config::default()
    };
    let units = scan_external_units(dir.path(), &cfg);
    assert_eq!(units.len(), 1);
    assert_eq!(
        units[0].uri,
        "spec://some-flow/mechanisms/ENGINE-X-v0.1#rules"
    );
    assert_eq!(units[0].revision.as_deref(), Some(&1));
}

#[test]
fn fenced_sample_headings_are_not_units_and_do_not_cut_spans() {
    let text = "## Real {#real-unit}\nbody\n```markdown\n## Sample {#req-sample}\n`req r2`\n```\ntail\n## Next {#next-unit}\n";
    let (units, warnings) = parse_units(DOC, text, NS);
    assert!(warnings.is_empty(), "{}", fmt_warnings(&warnings));
    assert_eq!(units.len(), 2);
    assert_eq!(units[0].anchor, "real-unit");
    assert_eq!(units[1].anchor, "next-unit");
    // The fenced sample stays inside real-unit's span (the hash
    // covers it), it just isn't a unit of its own.
    let (units2, _) = parse_units(
        DOC,
        "## Real {#real-unit}\nbody\ntail\n## Next {#next-unit}\n",
        NS,
    );
    assert_ne!(units[0].contentHash, units2[0].contentHash);
}

#[test]
fn hash_is_line_ending_invariant() {
    let lf = "## A {#a}\nbody\n";
    let crlf = "## A {#a}\r\nbody\r\n";
    let (u1, _) = parse_units(DOC, lf, NS);
    let (u2, _) = parse_units(DOC, crlf, NS);
    assert_eq!(u1[0].contentHash, u2[0].contentHash);
}

// ----- `##<ID>` fact anchors (PROP-014 §2.1 fact amendment) -----

/// Find the unique fact/heading unit carrying `anchor`.
fn unit<'a>(units: &'a [SpecUnit], anchor: &str) -> &'a SpecUnit {
    units
        .iter()
        .find(|u| u.anchor == anchor)
        .unwrap_or_else(|| panic!("no unit `#{anchor}`"))
}

#[test]
fn paragraph_fact_anchor_becomes_an_untyped_unit() {
    // `##<ID>` as the first token of a paragraph mints a fact unit; the
    // kebab register names a service fact.
    let text = "# Doc {#root}\n\n##my-fact The service rule holds.\n";
    let (units, warnings) = parse_units(DOC, text, NS);
    assert!(warnings.is_empty(), "{}", fmt_warnings(&warnings));
    assert_eq!(units.len(), 2);
    let f = unit(&units, "my-fact");
    assert_eq!(f.uri, "spec://project/test/DOC#my-fact");
    assert_eq!(f.docPath, "test/DOC");
    assert_eq!(f.line, 3);
    assert_eq!(f.heading, "The service rule holds.");
    // Untyped: a fact carries no kind / revision / status line.
    assert!(f.kind.is_none());
    assert!(f.revision.is_none());
    assert!(f.status.is_none());
}

#[test]
fn list_item_fact_anchor_upper_is_addressable() {
    // An UPPER-SLUG id (a normative fact) after a list marker.
    let text = "# Doc {#root}\n\n- ##FACT-A The normative contract.\n";
    let (units, warnings) = parse_units(DOC, text, NS);
    assert!(warnings.is_empty(), "{}", fmt_warnings(&warnings));
    let f = unit(&units, "FACT-A");
    assert_eq!(f.uri, "spec://project/test/DOC#FACT-A");
    assert_eq!(f.line, 3);
    assert_eq!(f.heading, "The normative contract.");
}

#[test]
fn every_list_marker_flavour_and_indent_is_recognised() {
    // `-`, `*`, `+`, `N.`, `N)` at any indent all carry a fact anchor.
    let text = "# D {#root}\n\n- ##dash x\n* ##star x\n+ ##plus x\n1. ##dot x\n  2) ##paren x\n";
    let (units, warnings) = parse_units(DOC, text, NS);
    assert!(warnings.is_empty(), "{}", fmt_warnings(&warnings));
    for a in ["dash", "star", "plus", "dot", "paren"] {
        assert_eq!(unit(&units, a).heading, "x");
    }
}

#[test]
fn nested_list_item_fact_anchor_is_its_own_unit() {
    let text = "# Doc {#root}\n\n- ##OUTER outer.\n  - ##INNER inner.\n";
    let (units, warnings) = parse_units(DOC, text, NS);
    assert!(warnings.is_empty(), "{}", fmt_warnings(&warnings));
    // The nested item is its own unit — not folded into the outer one.
    assert_eq!(unit(&units, "OUTER").line, 3);
    let inner = unit(&units, "INNER");
    assert_eq!(inner.line, 4);
    assert_eq!(inner.heading, "inner.");
}

#[test]
fn fact_span_covers_indented_continuations() {
    // A list item's fact spans its continuation lines, so editing a
    // continuation changes the unit's content hash.
    let with = "# D {#root}\n\n- ##item lead.\n  a continuation line.\n";
    let edited = "# D {#root}\n\n- ##item lead.\n  a DIFFERENT continuation.\n";
    let (u1, _) = parse_units(DOC, with, NS);
    let (u2, _) = parse_units(DOC, edited, NS);
    assert_ne!(unit(&u1, "item").contentHash, unit(&u2, "item").contentHash);
}

#[test]
fn fact_anchor_inside_a_fence_is_not_a_unit() {
    let text = "# D {#root}\n\n```md\n##inside-fence fenced.\n```\ntail\n";
    let (units, warnings) = parse_units(DOC, text, NS);
    assert!(warnings.is_empty(), "{}", fmt_warnings(&warnings));
    assert!(units.iter().all(|u| u.anchor != "inside-fence"));
    assert_eq!(units.len(), 1);
}

#[test]
fn heading_line_is_never_a_fact_anchor() {
    // A `## Heading {#anchor}` line is a heading unit, and an
    // unanchored `## Plain` heading mints nothing — neither is a fact.
    let text = "# D {#root}\n\n## Sub {#sub}\nbody\n\n## Plain heading\nmore\n";
    let (units, warnings) = parse_units(DOC, text, NS);
    assert!(warnings.is_empty(), "{}", fmt_warnings(&warnings));
    let anchors: Vec<&str> = units.iter().map(|u| u.anchor.as_str()).collect();
    assert_eq!(anchors, ["root", "sub"]);
}

#[test]
fn hashhash_with_no_space_is_a_fact_not_a_heading() {
    // `## Heading` (space) is a heading; `##Heading` (no space) is a fact
    // whose id happens to be `Heading` — the space is what makes a heading.
    let text = "# D {#root}\n\n##Heading is a fact id here.\n";
    let (units, warnings) = parse_units(DOC, text, NS);
    assert!(warnings.is_empty(), "{}", fmt_warnings(&warnings));
    assert_eq!(unit(&units, "Heading").heading, "is a fact id here.");
}

#[test]
fn invalid_fact_id_after_hashes_is_silently_prose() {
    // A non-letter head (`##9bad`), a glyph glued to the id (`##bad!`), a
    // bare `##!`, and a `###` run are all prose: no unit and — unlike a
    // malformed heading anchor — no warning.
    let text =
        "# D {#root}\n\n##9bad not an id.\n\n##bad! also not.\n\n##! nor this.\n\n### not a fact\n";
    let (units, warnings) = parse_units(DOC, text, NS);
    assert!(warnings.is_empty(), "{}", fmt_warnings(&warnings));
    assert_eq!(units.len(), 1);
    assert_eq!(units[0].anchor, "root");
}

#[test]
fn fact_id_colliding_with_a_heading_anchor_warns() {
    // One address space per document: a fact id equal to an earlier
    // heading anchor is a duplicate-anchor warning, both units kept.
    let text = "# D {#root}\n\n## Sub {#dup}\nbody\n\n##dup A fact reusing the anchor.\n";
    let (units, warnings) = parse_units(DOC, text, NS);
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].code, "duplicate-anchor");
    // The fact is the second occurrence — the warning lands on its line.
    assert_eq!(warnings[0].line, 6);
    assert_eq!(units.iter().filter(|u| u.anchor == "dup").count(), 2);
}

#[test]
fn duplicate_fact_ids_warn_like_heading_anchors() {
    let text = "# D {#root}\n\n##dup one.\n\n##dup two.\n";
    let (units, warnings) = parse_units(DOC, text, NS);
    assert_eq!(units.iter().filter(|u| u.anchor == "dup").count(), 2);
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].code, "duplicate-anchor");
    assert_eq!(warnings[0].line, 5);
}

#[test]
fn a_lead_paragraph_fact_and_its_list_item_facts_coexist() {
    // A block with a lead paragraph anchor followed by list items: the
    // lead and each anchored item are their own units; an unmarked item
    // mints nothing.
    let text = "# D {#root}\n\n##lead The lead line.\n- ##a first item\n- second, unmarked\n- ##c third item\n";
    let (units, warnings) = parse_units(DOC, text, NS);
    assert!(warnings.is_empty(), "{}", fmt_warnings(&warnings));
    let anchors: Vec<&str> = units.iter().map(|u| u.anchor.as_str()).collect();
    assert_eq!(anchors, ["root", "lead", "a", "c"]);
}
