//! Oracles for the R7.5 A2a one-read witness/observation seams.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-046#laws");

use std::fs;

use tempfile::tempdir;

use crate::scan_tests::{XML_NS, spec_rel, write_spec};
use crate::{
    AuthoredFact, AuthoredSourceObservation, RegistryError, SourceFileWitness, SourceKind,
    observe_authored_source, scan_authored_facts,
};

// --- R7.5 A2a: the one-read witness/observation seams ---

fn available(root: &std::path::Path) -> (Vec<AuthoredFact>, Vec<SourceFileWitness>) {
    match observe_authored_source(root, "org.example/pkg", SourceKind::Host).unwrap() {
        AuthoredSourceObservation::Available { facts, documents } => (facts, documents),
        AuthoredSourceObservation::Invalid { issue, .. } => panic!(
            "expected Available, got Invalid: {}:{}: {}",
            issue.path.display(),
            issue.line,
            issue.message
        ),
    }
}

#[test]
fn a_prose_only_edit_moves_the_witness_but_not_the_facts() {
    let root = tempdir().expect("tempdir");
    let rel = spec_rel("RULE.md");
    write_spec(
        root.path(),
        &rel,
        "# Rules\n\n@fact:STABLE The claim. @status:impl/done\n",
    );
    let (facts_before, witnesses_before) = available(root.path());
    write_spec(
        root.path(),
        &rel,
        "# Rules\n\n@fact:STABLE The claim, now wearing much more prose. @status:impl/done\n",
    );
    let (facts_after, witnesses_after) = available(root.path());

    assert_eq!(facts_before, facts_after, "prose is not a fact axis");
    assert_eq!(witnesses_before.len(), 1);
    assert_eq!(witnesses_after.len(), 1);
    assert_eq!(witnesses_before[0].path, witnesses_after[0].path);
    assert_ne!(
        witnesses_before[0].digest, witnesses_after[0].digest,
        "a raw-byte edit must move the digest"
    );
    assert_ne!(witnesses_before[0].bytes, witnesses_after[0].bytes);
}

#[test]
fn equivalent_md_and_xml_forms_yield_equal_facts_but_distinct_raw_witnesses() {
    let markdown = tempdir().expect("tempdir");
    write_spec(
        markdown.path(),
        &spec_rel("RULE.md"),
        "# Rules\n\n@fact:FIRST First. @status:impl/done\n",
    );
    let xml = tempdir().expect("tempdir");
    write_spec(
        xml.path(),
        &spec_rel("RULE.xml"),
        format!(
            "<spec {XML_NS}><p><fact id=\"FIRST\" status=\"impl/done\">First.</fact></p></spec>"
        )
        .as_str(),
    );

    let (md_facts, md_documents) = available(markdown.path());
    let (xml_facts, xml_documents) = available(xml.path());
    assert_eq!(md_facts, xml_facts);
    assert_eq!(md_documents.len(), 1);
    assert_eq!(xml_documents.len(), 1);
    assert_ne!(
        md_documents[0].digest, xml_documents[0].digest,
        "logically equal, byte-different: the witness binds RAW bytes"
    );
}

#[test]
fn invalid_markdown_xml_and_utf8_return_invalid_with_witnesses_and_no_facts() {
    let root = tempdir().expect("tempdir");
    // A duplicate fact id inside one document is a genuine
    // `from_markdown` failure (markup errors are loud).
    write_spec(
        root.path(),
        &spec_rel("BAD.md"),
        "# Bad\n\n@fact:DUP one\n\n@fact:DUP two\n",
    );
    let observed = observe_authored_source(root.path(), "org.example/pkg", SourceKind::Host);
    let AuthoredSourceObservation::Invalid { documents, issue } = observed.unwrap() else {
        panic!("invalid markdown must be Invalid, not Available");
    };
    assert_eq!(documents.len(), 1, "the read bytes are still witnessed");
    // The pivot's aggregate markup error is positionless at the struct
    // level (`line: 0`) and carries the real 1-based positions in the
    // bounded message — never a 0-based position.
    assert!(
        issue.message.contains("lines 3 and 5"),
        "1-based positions must ride the issue: {issue:?}"
    );
    assert!(issue.path.ends_with("BAD.md"), "{issue:?}");
    let md_wrapper =
        scan_authored_facts(root.path(), "org.example/pkg", SourceKind::Host).unwrap_err();
    assert!(matches!(md_wrapper, RegistryError::SpecParse { .. }));

    let root = tempdir().expect("tempdir");
    write_spec(root.path(), &spec_rel("BAD.xml"), "<spec><p>x</p></spec>");
    let observed = observe_authored_source(root.path(), "org.example/pkg", SourceKind::Host);
    let AuthoredSourceObservation::Invalid { documents, issue } = observed.unwrap() else {
        panic!("a dialect violation must be Invalid");
    };
    assert_eq!(documents.len(), 1);
    assert_eq!(issue.line, 1);
    assert!(issue.message.contains("xmlns"), "{}", issue.message);

    // Invalid UTF-8: present bytes that cannot be trusted as a source.
    let root = tempdir().expect("tempdir");
    let bin = root.path().join(spec_rel("BIN.md"));
    fs::create_dir_all(bin.parent().expect("spec parent")).expect("spec dir");
    std::fs::write(&bin, [0xff, 0xfe].as_slice()).expect("raw non-UTF-8 bytes");
    let observed = observe_authored_source(root.path(), "org.example/pkg", SourceKind::Host);
    let AuthoredSourceObservation::Invalid { documents, issue } = observed.unwrap() else {
        panic!("non-UTF-8 bytes must be Invalid");
    };
    assert_eq!(documents.len(), 1);
    assert_eq!(documents[0].bytes, 2);
    assert_eq!(issue.line, 1);
    assert!(issue.message.contains("UTF-8"), "{}", issue.message);
    let utf8_wrapper =
        scan_authored_facts(root.path(), "org.example/pkg", SourceKind::Host).unwrap_err();
    assert!(matches!(utf8_wrapper, RegistryError::SpecParse { .. }));
}

#[test]
fn a_disjoint_anchor_split_brain_is_invalid_with_both_witnesses() {
    let root = tempdir().expect("tempdir");
    write_spec(
        root.path(),
        &spec_rel("RULE.md"),
        "# Rules\n\n@fact:ONLY_MD The markdown anchor. @status:impl/done\n",
    );
    write_spec(
        root.path(),
        &spec_rel("RULE.xml"),
        format!(
            "<spec {XML_NS}><p><fact id=\"ONLY_XML\" status=\"impl/done\">the xml anchor</fact></p></spec>"
        )
        .as_str(),
    );

    let observed = observe_authored_source(root.path(), "org.example/pkg", SourceKind::Host);
    let AuthoredSourceObservation::Invalid { documents, issue } = observed.unwrap() else {
        panic!("a split brain must be Invalid");
    };
    // The read pass completed before the collision refusal: BOTH raw
    // documents of the pair are witnessed — no fake empty-set digest.
    assert_eq!(documents.len(), 2);
    assert_eq!(
        documents
            .iter()
            .map(|w| w.path.as_str())
            .collect::<Vec<_>>(),
        [spec_rel("RULE.md"), spec_rel("RULE.xml")]
    );
    assert_ne!(documents[0].digest, documents[1].digest);
    assert!(issue.message.contains("one document, one form"));
    assert_eq!(issue.line, 1);
}

#[test]
fn a_duplicate_full_address_is_invalid_with_all_witnesses() {
    let root = tempdir().expect("tempdir");
    write_spec(
        root.path(),
        &spec_rel("PROP-009-first.md"),
        "# First\n\n@fact:LAW The law. @status:impl/done\n",
    );
    write_spec(
        root.path(),
        &spec_rel("PROP-009-second.md"),
        "# Second\n\n@fact:LAW The law again. @status:impl/done\n",
    );

    let observed = observe_authored_source(root.path(), "org.example/pkg", SourceKind::Host);
    let AuthoredSourceObservation::Invalid { documents, issue } = observed.unwrap() else {
        panic!("a duplicate address must be Invalid");
    };
    assert_eq!(documents.len(), 2);
    assert!(
        issue
            .message
            .contains("duplicate full fact address `spec://org.example/pkg/PROP-009#LAW`"),
        "{}",
        issue.message
    );
}

#[test]
fn witness_order_is_deterministic_and_digest_shape_is_exact() {
    let root = tempdir().expect("tempdir");
    write_spec(root.path(), &spec_rel("B.md"), "# B\n\n@fact:B b.\n");
    write_spec(root.path(), &spec_rel("A.md"), "# A\n\n@fact:A a.\n");

    let (.., documents) = available(root.path());
    let paths: Vec<&str> = documents.iter().map(|w| w.path.as_str()).collect();
    let mut sorted = paths.clone();
    sorted.sort();
    assert_eq!(paths, sorted, "witnesses come back in sorted order");
    for witness in &documents {
        assert!(witness.digest.starts_with("sha256:"));
        assert_eq!(witness.digest.len(), "sha256:".len() + 64);
        assert!(
            witness
                .digest
                .bytes()
                .skip("sha256:".len())
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        );
        let on_disk = std::fs::read(root.path().join(&witness.path)).unwrap();
        assert_eq!(witness.bytes, on_disk.len() as u64);
    }
}

#[test]
fn no_observation_value_carries_the_body_canary() {
    let canary = "CANARY-PROSE-7f3a9c";
    let root = tempdir().expect("tempdir");
    write_spec(
        root.path(),
        &spec_rel("RULE.md"),
        format!("# Rules\n\n@fact:KEPT Claim with {canary} inside. @status:impl/done\n").as_str(),
    );
    let observed = observe_authored_source(root.path(), "org.example/pkg", SourceKind::Host);
    assert!(!format!("{:?}", observed.unwrap()).contains(canary));

    let bad = tempdir().expect("tempdir");
    write_spec(
        bad.path(),
        &spec_rel("BAD.md"),
        format!("# Bad\n\n@fact:BROKEN Broken {canary}. @status:not-a-stage/done\n").as_str(),
    );
    let observed = observe_authored_source(bad.path(), "org.example/pkg", SourceKind::Host);
    assert!(!format!("{:?}", observed.unwrap()).contains(canary));
}

#[test]
fn the_extension_dispatch_lives_only_in_the_pivot() {
    // RED 1's fence: the md/xml projection decision may not be
    // re-decided beside the pivot. The scanner expresses form-blindness
    // by containing no extension comparison of its own.
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/scan.rs"),
    )
    .unwrap();
    assert!(
        !source.contains("== Some(\"xml\")") && !source.contains("== Some(\"md\")"),
        "a second extension decision appeared in the scanner"
    );
}
