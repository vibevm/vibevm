//! Captured variant and partial coverage controls, using the existing tree parser.
#![cfg(test)]
use super::*;

fn emit(coverage: &mut NormalCoverage, address: &str, text: &str) -> String {
    coverage.body(&SpecAddress::parse(address).unwrap(), text)
}

#[cfg(test)]
fn generated(text: &str) -> crate::embed::occurrences::ExpandedBody {
    crate::embed::expand_annotated(
        text,
        "spec://org.test/a/doc",
        &mut |address| {
            Ok(if address.anchor.first().is_some_and(|s| s == "empty") {
                String::new()
            } else {
                "SAME\n".into()
            })
        },
        &mut |_, _, _| {},
    )
    .unwrap()
}

#[cfg(test)]
fn project_generated(
    coverage: &mut NormalCoverage,
    address: &str,
    input: &crate::embed::occurrences::ExpandedBody,
) -> (String, crate::embed::occurrences::EmbedTape) {
    let (body, lines, empty) = coverage.project(
        &SpecAddress::parse(address).unwrap(),
        &input.text,
        Some(&input.tape),
    );
    let tape = input
        .tape
        .retain_lines(&input.text, &body, &lines, &empty)
        .unwrap();
    (body, tape)
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#EMBED-COVERAGE-OWNERSHIP"
)]
fn removed_section_empty_embed_does_not_attach_to_retained_sibling() {
    let mut coverage = NormalCoverage::default();
    let first = generated("# A {#a}\n#embed spec://org.test/e/doc#empty\n");
    let (_, live) = project_generated(&mut coverage, "spec://org.test/a/doc#a", &first);
    assert_eq!(live.boundaries().len(), 2);
    let full = generated("# A {#a}\n#embed spec://org.test/e/doc#empty\n# B {#b}\nKEEP\n");
    let (body, tape) = project_generated(&mut coverage, "spec://org.test/a/doc", &full);
    assert!(!body.contains("# A"));
    assert!(body.contains("KEEP"));
    assert!(tape.boundaries().is_empty());
    assert!(tape.occurrences[0].suppressed);
    assert!(tape.occurrences[0].intrinsic_empty);
}

#[test]
fn empty_preamble_survives_removed_first_heading_and_keeps_its_own_identity() {
    let mut coverage = NormalCoverage::default();
    project_generated(
        &mut coverage,
        "spec://org.test/a/doc#a",
        &generated("# A {#a}\nA\n"),
    );
    let full = generated("#embed spec://org.test/e/doc#empty\n# A {#a}\nA\n# B {#b}\nB\n");
    let (body, tape) = project_generated(&mut coverage, "spec://org.test/a/doc", &full);
    assert!(!body.contains("# A"));
    assert!(body.contains("# B"));
    assert_eq!(tape.boundaries().len(), 2);
    assert!(!tape.occurrences[0].suppressed);
}

#[test]
fn intrinsic_empty_document_has_one_live_tape_then_explicit_suppression() {
    let mut coverage = NormalCoverage::default();
    let source = generated("#embed spec://org.test/e/doc#empty\n");
    let (body, first) = project_generated(&mut coverage, "spec://org.test/a/doc", &source);
    assert!(body.is_empty());
    assert_eq!(first.boundaries().len(), 2);
    let (_, second) = project_generated(&mut coverage, "spec://org.test/a/doc", &source);
    assert!(second.boundaries().is_empty());
    assert!(second.occurrences[0].intrinsic_empty && second.occurrences[0].suppressed);
}

#[test]
fn equal_plain_and_embedded_payloads_have_distinct_generated_meaning() {
    let mut coverage = NormalCoverage::default();
    let plain = generated("# A {#a}\nSAME\n");
    let embedded = generated("# A {#a}\n#embed spec://org.test/e/doc#body\n");
    assert_eq!(plain.text, embedded.text);
    project_generated(&mut coverage, "spec://org.test/a/doc", &plain);
    let (body, tape) = project_generated(&mut coverage, "spec://org.test/a/doc", &embedded);
    assert_eq!(body, embedded.text);
    assert_eq!(tape.boundaries().len(), 2);
}

#[test]
fn canonical_address_keeps_document_and_version_but_not_revision_selector() {
    let mut coverage = NormalCoverage::default();
    let text = "# Root {#qualified-root}\nPAYLOAD\n";
    assert!(emit(&mut coverage, "spec://org.test/a/doc#root~r1", text).contains("PAYLOAD"));
    assert!(emit(&mut coverage, "spec://org.test/a/doc#root~r2", text).is_empty());
    assert!(emit(&mut coverage, "spec://org.test/b/doc#root", text).contains("PAYLOAD"));
    assert!(emit(&mut coverage, "spec://org.test/a@2/doc#root", text).contains("PAYLOAD"));
}

#[test]
fn distinct_captured_variants_are_preserved_without_prose_only_deduplication() {
    let mut coverage = NormalCoverage::default();
    let address = "spec://org.test/a/doc#root";
    let first = "# Root {#qualified-root}\nFIRST\n";
    let second = "# Root {#qualified-root}\nSECOND\n";
    assert!(emit(&mut coverage, address, first).contains("FIRST"));
    assert!(emit(&mut coverage, address, second).contains("SECOND"));
    assert!(emit(&mut coverage, address, first).is_empty());
}

#[test]
fn partial_then_whole_covers_unanchored_remainder_and_leading_fact_once() {
    let mut coverage = NormalCoverage::default();
    let leading = emit(
        &mut coverage,
        "spec://org.test/a/doc#INTRO",
        "##INTRO LEADING_FACT\n",
    );
    let partial = emit(
        &mut coverage,
        "spec://org.test/a/doc#root",
        "# Root {#qualified-root}\nSECTION\n",
    );
    let full =
        "##INTRO LEADING_FACT\n# Root {#qualified-root}\nSECTION\n# Unanchored\nUNANCHORED\n";
    let whole = emit(&mut coverage, "spec://org.test/a/doc", full);
    let repeat = emit(&mut coverage, "spec://org.test/a/doc", full);
    let combined = [leading, partial, whole, repeat].join("\n");
    for marker in ["LEADING_FACT", "SECTION", "UNANCHORED"] {
        assert_eq!(combined.matches(marker).count(), 1, "{combined}");
    }
}
