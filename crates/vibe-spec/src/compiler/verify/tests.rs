//! Direct verifier invariants for the source, document, gather and emitted
//! carriers: no repair, typed errors, deterministic first failure. The lane
//! level has its own cell, `lane_tests`.

use specmark::verifies;

use super::{IrVerifier, VerificationError};
use crate::compiler::ir::{
    ArtifactContext, DocumentAddress, DocumentIr, Documents, SourceFormatId, SourceIr,
    StaticCompileMode,
};
use crate::compiler::pass::AnyIr;
use crate::compiler::worklist::document_key;
use crate::{DocTree, SpecAddress};

fn spec_address(raw: &str) -> SpecAddress {
    SpecAddress::parse(raw).unwrap()
}

fn spec_source(anchor: &str, text: &str) -> SourceIr {
    SourceIr::new(
        DocumentAddress::Spec(spec_address(&format!(
            "spec://org.demo/pkg/common/{anchor}#{anchor}"
        ))),
        SourceFormatId::new("markdown").unwrap(),
        text,
    )
}

fn static_source(origin: &str, path: &str, text: &str) -> SourceIr {
    SourceIr::new(
        DocumentAddress::StaticEntry {
            origin: origin.to_string(),
            path: path.to_string(),
        },
        SourceFormatId::new("markdown").unwrap(),
        text,
    )
}

fn document(source: SourceIr, text: &str) -> DocumentIr {
    DocumentIr::new(source, DocTree::parse(text))
}

fn verify(ir: &AnyIr) -> Result<(), VerificationError> {
    IrVerifier.verify(ir)
}

// --- Source ------------------------------------------------------------

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn a_source_carrier_with_identity_and_arbitrary_text_passes() {
    let source = spec_source("root", "");
    verify(&AnyIr::Source(source)).unwrap();
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn a_blank_static_entry_identity_is_a_typed_error() {
    let blank = SourceIr::new(
        DocumentAddress::StaticEntry {
            origin: "  ".to_string(),
            path: "boot/entry.md".to_string(),
        },
        SourceFormatId::new("markdown").unwrap(),
        "text",
    );
    let error = verify(&AnyIr::Source(blank)).unwrap_err();
    assert!(
        matches!(
            error,
            VerificationError::BlankSourceIdentity {
                field: "static origin"
            }
        ),
        "{error:?}"
    );
}

// --- Document ----------------------------------------------------------

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn a_parsed_document_passes_the_tree_and_anchor_gates() {
    let document = document(
        spec_source("root", "# Doc {#root}\nbody\n"),
        "# Doc {#root}\nbody\n",
    );
    verify(&AnyIr::Document(document)).unwrap();
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn a_document_tree_with_a_repeated_fact_is_rejected_with_its_address() {
    let source = spec_source("root", "");
    let tree = DocTree::parse("# A {#a}\n##shared one\n## B {#b}\n##shared two\n");
    let document = DocumentIr::new(source, tree);
    let error = verify(&AnyIr::Document(document)).unwrap_err();
    match error {
        VerificationError::DuplicateId { address, duplicate } => {
            assert_eq!(duplicate.id, "shared");
            assert!(address.contains("common/root"), "{address}");
        }
        other => panic!("expected the DuplicateId gate, got {other:?}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn a_document_with_a_corrupt_arena_is_a_typed_error_never_a_panic() {
    let source = spec_source("root", "");
    let tree = DocTree::corrupt_for_test(Vec::new(), 0);
    let document = DocumentIr::new(source, tree);
    assert!(matches!(
        verify(&AnyIr::Document(document)),
        Err(VerificationError::DocTree { .. })
    ));
}

// --- Documents gather --------------------------------------------------

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn an_empty_gather_batch_is_valid() {
    verify(&AnyIr::Documents(Documents::new(Vec::new()))).unwrap();
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn a_repeated_pinless_spec_key_is_refused_with_both_positions() {
    let first = document(spec_source("root", "# One {#root}\n"), "# One {#root}\n");
    let second = document(spec_source("root", "# Two {#root}\n"), "# Two {#root}\n");
    let error = verify(&AnyIr::Documents(Documents::new(vec![first, second]))).unwrap_err();
    match error {
        VerificationError::DuplicateDocument { first, second, key } => {
            assert_eq!((first, second), (0, 1));
            assert!(key.contains("common/root"), "{key}");
        }
        other => panic!("expected a gather duplicate, got {other:?}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn revision_pins_collide_but_anchors_stay_distinct() {
    let pinned = SourceIr::new(
        DocumentAddress::Spec(spec_address("spec://org.demo/pkg/common/doc#root~r1")),
        SourceFormatId::new("markdown").unwrap(),
        "# Root {#root}\n",
    );
    let repinned = SourceIr::new(
        DocumentAddress::Spec(spec_address("spec://org.demo/pkg/common/doc#root~r2")),
        SourceFormatId::new("markdown").unwrap(),
        "# Root {#root}\n",
    );
    let collided = verify(&AnyIr::Documents(Documents::new(vec![
        document(pinned, "# Root {#root}\n"),
        document(repinned, "# Root {#root}\n"),
    ])));
    assert!(
        matches!(
            collided,
            Err(VerificationError::DuplicateDocument {
                first: 0,
                second: 1,
                ..
            })
        ),
        "{collided:?}"
    );

    let other_anchor = document(
        spec_source("other", "# Other {#other}\n"),
        "# Other {#other}\n",
    );
    verify(&AnyIr::Documents(Documents::new(vec![
        document(spec_source("root", "# Root {#root}\n"), "# Root {#root}\n"),
        other_anchor,
    ])))
    .unwrap();
}

/// The gather guard keys on the very `document_key` the maps that can overwrite
/// a document use. The joined spelling `static:{origin}\0{path}` these two pairs
/// once shared is exactly what the typed key removes, so the pair must (a) stay
/// distinct under the production key and (b) survive the gather guard together.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn static_entry_keys_are_typed_so_a_joined_spelling_collision_stays_distinct() {
    let first = DocumentAddress::StaticEntry {
        origin: "boot".to_string(),
        path: "a\0b/entry.md".to_string(),
    };
    let second = DocumentAddress::StaticEntry {
        origin: "boot\0a".to_string(),
        path: "b/entry.md".to_string(),
    };
    let joined = |address: &DocumentAddress| match address {
        DocumentAddress::StaticEntry { origin, path } => format!("static:{origin}\0{path}"),
        DocumentAddress::Spec(_) => unreachable!("the fixture is a static pair"),
    };
    assert_eq!(
        joined(&first),
        joined(&second),
        "the fixture must collide under the joined spelling, or it proves nothing"
    );
    assert_ne!(
        document_key(&first),
        document_key(&second),
        "the typed production key separates them"
    );

    let batch = Documents::new(vec![
        document(
            SourceIr::new(
                first,
                SourceFormatId::new("markdown").unwrap(),
                "# A {#a}\n",
            ),
            "# A {#a}\n",
        ),
        document(
            SourceIr::new(
                second,
                SourceFormatId::new("markdown").unwrap(),
                "# B {#b}\n",
            ),
            "# B {#b}\n",
        ),
    ]);
    verify(&AnyIr::Documents(batch)).unwrap();
}

/// The other direction: a genuinely repeated typed key is still refused, so the
/// finer key did not simply disable the guard.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn a_repeated_static_entry_key_is_still_refused() {
    let batch = Documents::new(vec![
        document(
            static_source("boot", "entry.md", "# A {#a}\n"),
            "# A {#a}\n",
        ),
        document(
            static_source("boot", "entry.md", "# B {#b}\n"),
            "# B {#b}\n",
        ),
    ]);
    assert!(matches!(
        verify(&AnyIr::Documents(batch)),
        Err(VerificationError::DuplicateDocument {
            first: 0,
            second: 1,
            ..
        })
    ));
}

// --- Emitted -----------------------------------------------------------

fn emitted(bytes: Vec<u8>) -> AnyIr {
    let mut artifact = crate::compiler::ir::EmittedIr::testing(
        ArtifactContext::compatibility(StaticCompileMode::Plain),
        bytes.clone(),
    );
    artifact.provenance.bytes_digest = crate::compiler::emit::emitted_bytes_digest(&bytes);
    AnyIr::Emitted(artifact)
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn non_utf8_emitted_bytes_pass_generic_verification() {
    // A backend may emit arbitrary bytes: no UTF-8 or marker parse belongs at
    // the generic emitted level. A blanket text assumption turns this red.
    verify(&emitted(vec![0xEF, 0xBB, 0xBF, 0x00, 0xFF, 0xFE])).unwrap();
}

/// A real post-stamp mutation: the artifact is built with its honest digest and
/// verifies, and only then is one byte changed. Both halves are asserted, so
/// deleting the mutation turns this red instead of leaving it trivially green.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn emitted_bytes_mutated_after_their_provenance_was_stamped_are_refused() {
    let AnyIr::Emitted(honest) = emitted(b"original bytes".to_vec()) else {
        unreachable!("the helper builds an emitted carrier")
    };
    verify(&AnyIr::Emitted(honest.clone())).expect("the stamped artifact authenticates");

    let mut mutated = honest;
    mutated.bytes[0] = b'0';
    assert!(matches!(
        verify(&AnyIr::Emitted(mutated)),
        Err(VerificationError::EmittedBytesDigest)
    ));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn verification_never_repairs_the_carrier_it_rejects() {
    // Borrow the invalid carrier, assert the error, then prove the value is
    // byte-identical: the `&AnyIr` signature is the compile-time no-repair seam.
    let carrier = AnyIr::Document(DocumentIr::new(
        spec_source("root", ""),
        DocTree::parse("# A {#a}\n##shared one\n## B {#b}\n##shared two\n"),
    ));
    let before = format!("{carrier:?}");
    let error = verify(&carrier);
    assert!(error.is_err());
    assert_eq!(before, format!("{carrier:?}"));
}
