//! The T7 document-subject carrier on the wire: it round-trips byte-exact, it
//! is REQUIRED rather than defaulted, it is carried rather than re-derived
//! from the address, every provider arm survives the typed conversion, the two
//! absences stay apart, and a `declared_path` obeys the `paths` contract.

use std::path::PathBuf;

use specmark::verifies;

use super::super::super::ir::{
    DocumentAddress, DocumentProvider, DocumentSubject, SourceFormatId, SourceIr,
};
use super::super::super::pass::AnyIr;
use super::super::{IrWireError, decode, encode_compact};
use crate::SpecAddress;

fn corpus() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("formats/corpora/compiler_ir/e1")
}

/// Every valid corpus document that carries a `source_doc` at all — the five
/// carriers a missing subject could hide in.
const SUBJECT_BEARING: [&str; 5] = [
    "source_document.json",
    "source_document_reached.json",
    "document_document.json",
    "documents_artifact.json",
    "closure_artifact_compat.json",
];

fn valid(name: &str) -> Vec<u8> {
    std::fs::read(corpus().join("valid").join(name)).unwrap()
}

/// Strip the `subject` member from every `source_doc` in one carrier.
fn drop_subjects(value: &mut serde_json::Value) -> usize {
    match value {
        serde_json::Value::Object(map) => {
            let is_source_doc = map.contains_key("format") && map.contains_key("text");
            let mut removed = usize::from(is_source_doc && map.remove("subject").is_some());
            for child in map.values_mut() {
                removed += drop_subjects(child);
            }
            removed
        }
        serde_json::Value::Array(items) => items.iter_mut().map(drop_subjects).sum(),
        _ => 0,
    }
}

/// The corpus carries a subject whose declared path the decoder could NOT have
/// re-derived: `boot/10-guide.md` is not the address' `manual/guide.md`. The
/// whole document then re-encodes to the authored bytes.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#WHOLE-IR-WIRE")]
fn a_carried_subject_survives_the_strict_round_trip_exactly() {
    let bytes = valid("source_document.json");
    let ir = decode(&bytes).unwrap();
    let AnyIr::Source(source) = &ir else {
        panic!("source_document.json is the source carrier")
    };
    let DocumentAddress::Spec(address) = source.address() else {
        panic!("the corpus source is spec-addressed")
    };
    assert_eq!(address.doc_path, "manual/guide.md");
    assert_eq!(source.subject().declared_path(), "boot/10-guide.md");
    assert_ne!(source.subject().declared_path(), address.doc_path);
    assert_eq!(source.subject().provider(), &DocumentProvider::Undetermined);

    let round: serde_json::Value = serde_json::from_slice(&encode_compact(&ir).unwrap()).unwrap();
    let authored: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(round, authored);
}

/// The two absences are two carriers, not one: the DECLARED corpus document
/// says `undetermined` (an owner exists and was not resolved) and the REACHED
/// one says `unclaimed` (no row declared it, and none ever will). Collapsing
/// the arms makes this red.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#WHOLE-IR-WIRE")]
fn the_two_absences_are_distinguishable_across_the_wire() {
    let declared = decode(&valid("source_document.json")).unwrap();
    let reached = decode(&valid("source_document_reached.json")).unwrap();
    let (AnyIr::Source(declared), AnyIr::Source(reached)) = (&declared, &reached) else {
        panic!("both corpus documents are source carriers")
    };
    assert_eq!(
        declared.subject().provider(),
        &DocumentProvider::Undetermined
    );
    assert_eq!(reached.subject().provider(), &DocumentProvider::Unclaimed);
    assert_ne!(declared.subject().provider(), reached.subject().provider());

    // And the reached document is a reached document: its declared path IS its
    // own `doc_path`, because no row named a different one.
    let DocumentAddress::Spec(address) = reached.address() else {
        panic!("the reached corpus source is spec-addressed")
    };
    assert_eq!(reached.subject().declared_path(), address.doc_path);
}

/// A carrier that omits the subject is refused at the strict generated reader,
/// where a defaulted subject would otherwise silently decide which transforms
/// the document is in scope for.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#WHOLE-IR-WIRE")]
fn a_missing_subject_is_refused_rather_than_defaulted() {
    for name in SUBJECT_BEARING {
        let mut document: serde_json::Value = serde_json::from_slice(&valid(name)).unwrap();
        assert!(
            drop_subjects(&mut document) > 0,
            "{name} must really carry a subject to drop"
        );
        let error = decode(&serde_json::to_vec(&document).unwrap()).unwrap_err();
        let IrWireError::Reader { detail } = &error else {
            panic!("{name}: the refusal is the strict reader's, got {error:?}")
        };
        assert!(detail.contains("missing field"), "{name}: {detail}");
        assert!(detail.contains("subject"), "{name}: {detail}");
    }
}

/// The corpus itself exercises every provider arm, so no arm survives only in
/// a hand-built test value.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#WHOLE-IR-WIRE")]
fn the_corpus_exercises_every_provider_arm() {
    let mut kinds: Vec<String> = Vec::new();
    for name in SUBJECT_BEARING {
        let document: serde_json::Value = serde_json::from_slice(&valid(name)).unwrap();
        collect_kinds(&document, &mut kinds);
    }
    kinds.sort();
    kinds.dedup();
    assert_eq!(
        kinds,
        [
            "dependency",
            "host-coordinate",
            "host-ungrouped",
            "host-virtual-workspace",
            "unclaimed",
            "undetermined",
        ],
    );
}

fn collect_kinds(value: &serde_json::Value, out: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(map) => {
            if let Some(provider) = map.get("provider")
                && map.contains_key("declared_path")
                && let Some(kind) = provider.get("kind").and_then(serde_json::Value::as_str)
            {
                out.push(kind.to_string());
            }
            for child in map.values() {
                collect_kinds(child, out);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                collect_kinds(item, out);
            }
        }
        _ => {}
    }
}

/// Every provider arm survives domain → wire → domain as the same typed value.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#WHOLE-IR-WIRE")]
fn every_provider_arm_round_trips_domain_wire_domain() {
    let group = vibe_core::Group::parse("org.demo").unwrap();
    let name = vibe_core::PackageName::parse("lib").unwrap();
    for provider in [
        DocumentProvider::Unclaimed,
        DocumentProvider::Undetermined,
        DocumentProvider::Dependency {
            group: group.clone(),
            name: name.clone(),
        },
        DocumentProvider::HostUngrouped {
            name: "demo".to_string(),
        },
        DocumentProvider::HostCoordinate {
            group: group.clone(),
            name: name.clone(),
        },
        DocumentProvider::HostVirtualWorkspace,
    ] {
        let source = SourceIr::new(
            DocumentAddress::Spec(
                SpecAddress::parse("spec://org.demo/lib/manual/guide.md#root").unwrap(),
            ),
            SourceFormatId::canonical_markdown(),
            DocumentSubject::declared(provider.clone(), "boot/10-guide.md"),
            "# Guide {#root}\n",
        );
        let carrier = AnyIr::Source(source.clone());
        let back = decode(&encode_compact(&carrier).unwrap()).unwrap();
        let AnyIr::Source(actual) = back else {
            panic!("the source carrier stays a source carrier")
        };
        assert_eq!(actual, source, "{provider:?} did not survive the wire");
    }
}

/// A carrier claiming a provider `kind` the schema does not name is refused by
/// the strict generated reader: the discriminator is a closed vocabulary, so an
/// unknown arm can never be read as one of the six.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#WHOLE-IR-WIRE")]
fn an_unknown_provider_kind_is_refused() {
    for kind in ["absent", "host", "Dependency", ""] {
        let mut document: serde_json::Value =
            serde_json::from_slice(&valid("source_document.json")).unwrap();
        document["doc"]["subject"]["provider"] = serde_json::json!({"kind": kind});
        let error = decode(&serde_json::to_vec(&document).unwrap()).unwrap_err();
        let IrWireError::Reader { detail } = &error else {
            panic!("`{kind}`: the closed vocabulary is the reader's, got {error:?}")
        };
        assert!(
            detail.contains("unknown variant") || detail.contains("kind"),
            "`{kind}`: {detail}"
        );
    }
}

/// A carrier claiming an ill-formed coordinate is refused by the very
/// constructor the coordinate type already owns — the provider is typed
/// identity, never a stored display string.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#WHOLE-IR-WIRE")]
fn an_ill_formed_provider_coordinate_is_refused_by_the_domain_constructor() {
    for (member, spelling) in [("group", "Not-A-Group"), ("name", "Not-Kebab")] {
        let mut document: serde_json::Value =
            serde_json::from_slice(&valid("source_document.json")).unwrap();
        document["doc"]["subject"]["provider"] =
            serde_json::json!({"kind": "dependency", "group": "org.demo", "name": "lib"});
        document["doc"]["subject"]["provider"][member] = serde_json::json!(spelling);
        let error = decode(&serde_json::to_vec(&document).unwrap()).unwrap_err();
        let IrWireError::Construction(detail) = &error else {
            panic!("{member}: the domain law owns the grammar, got {error:?}")
        };
        assert!(
            detail.contains(&format!("subject provider {member}")),
            "{detail}"
        );
    }
}

/// The `paths` contract at the wire boundary: a backslashed `declared_path` is
/// refused by the scalar-identity gate rather than carried into a selector
/// dimension that would silently match nothing with it.
///
/// The forward-slashed control passes through the same call, so the red is the
/// separator and not the surrounding edit.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#WHOLE-IR-WIRE")]
fn a_backslashed_declared_path_is_refused_at_the_wire() {
    for spelling in ["boot\\10-guide.md", "boot\\", "a\\b/c.md"] {
        let mut document: serde_json::Value =
            serde_json::from_slice(&valid("source_document.json")).unwrap();
        document["doc"]["subject"]["declared_path"] = serde_json::json!(spelling);
        let error = decode(&serde_json::to_vec(&document).unwrap()).unwrap_err();
        let IrWireError::Gate { gate, detail } = &error else {
            panic!("{spelling:?}: the spelling phase owns it, got {error:?}")
        };
        assert_eq!(*gate, "scalar-ids");
        assert!(detail.contains("subject declared path"), "{detail}");
        assert!(detail.contains("forward-slashed"), "{detail}");
    }

    let mut control: serde_json::Value =
        serde_json::from_slice(&valid("source_document.json")).unwrap();
    control["doc"]["subject"]["declared_path"] = serde_json::json!("boot/other/10-guide.md");
    decode(&serde_json::to_vec(&control).unwrap()).unwrap();
}

/// The contract is the SEPARATOR and nothing more. A `./` prefix and an
/// absolute-looking path both still cross: it is unverified whether such
/// values legitimately arrive today, and refusing one that already flows would
/// be a regression wearing a fix's clothes.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#WHOLE-IR-WIRE")]
fn only_the_separator_is_enforced_on_a_declared_path() {
    for spelling in ["./boot/10-guide.md", "/boot/10-guide.md", "../10-guide.md"] {
        let mut document: serde_json::Value =
            serde_json::from_slice(&valid("source_document.json")).unwrap();
        document["doc"]["subject"]["declared_path"] = serde_json::json!(spelling);
        let ir = decode(&serde_json::to_vec(&document).unwrap())
            .unwrap_or_else(|error| panic!("{spelling:?} must still cross: {error}"));
        let AnyIr::Source(source) = &ir else {
            panic!("the source carrier stays a source carrier")
        };
        assert_eq!(source.subject().declared_path(), spelling);
    }
}
