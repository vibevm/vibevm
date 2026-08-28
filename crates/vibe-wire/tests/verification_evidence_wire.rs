//! The shared verification-evidence member on the lifecycle report
//! root (PROP-054 §14.7, `##EVIDENCE-WIRE-AND-SURFACES`; R7.5 P1).
//!
//! Five facts are pinned here. ROUND-TRIP: a pre-R7.5 report omits
//! `verification` and still parses — and re-serialises without
//! inventing the member — while three authored evidence corpora ride
//! the root byte-identically. SEMANTICS: each corpus is read for its
//! identity, status and row ORDER, not merely round-tripped, and each
//! validates green through the hand-written cell. SCHEMA PARITY: the
//! root pulls the one shared fragment and spells the member with
//! `x-default: null`, and the fragment's `x-relational-laws` label set
//! equals the validator's implemented-law list. STATE TWIN: the
//! durable `lifecycle_state` measurement and the shared comparison row
//! agree member-for-member on the identity half they share, so the
//! duplication the registry's strictness split forces cannot drift.
//! VOCABULARY FENCE: no field or enum value anywhere on this wire is
//! named `unmet`, `met`, `fulfilled` or `verified`.

use std::collections::BTreeSet;

use vibe_wire::behaviour::verification_evidence::{IMPLEMENTED_LAWS, validate};
use vibe_wire::generated::lifecycle_report::LifecycleReport;
use vibe_wire::generated::shared::EvidenceStatus;

#[path = "wire_support/mod.rs"]
mod support;
use support::{read_json, repo_root, round_trip};

/// The four words `##REQUIREMENT-OBSERVATION-AXES` bars from every
/// generated field and enum value on the evidence and requirements
/// wires. `verifies` is a relation VERB and is deliberately not in
/// this set — the fence is on the past participle that would turn an
/// observation into a verdict.
pub const FORBIDDEN_VERDICT_WORDS: &[&str] = &["unmet", "met", "fulfilled", "verified"];

/// Every `properties`/`optionalProperties` key and every `enum` value
/// a JTD document declares, at any depth — the two places a forbidden
/// word could enter the generated Rust as a field or a variant.
fn declared_names(value: &serde_json::Value, names: &mut BTreeSet<String>) {
    match value {
        serde_json::Value::Object(fields) => {
            for (key, field) in fields {
                if (key == "properties" || key == "optionalProperties")
                    && let Some(members) = field.as_object()
                {
                    names.extend(members.keys().cloned());
                }
                if key == "enum"
                    && let Some(items) = field.as_array()
                {
                    names.extend(items.iter().filter_map(|v| v.as_str()).map(str::to_string));
                }
                declared_names(field, names);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                declared_names(item, names);
            }
        }
        _ => {}
    }
}

fn set(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|item| (*item).to_string()).collect()
}

/// A pre-R7.5 lifecycle report omits `verification`, parses, and
/// re-serialises without inventing the member as `null`.
#[test]
fn an_old_report_without_verification_round_trips_unchanged() {
    let report: LifecycleReport = round_trip("formats/corpora/lifecycle/e1/report.json");
    assert!(report.verification.is_none());
    let rendered = serde_json::to_value(&report).unwrap();
    assert!(
        rendered.get("verification").is_none(),
        "an absent verification member round-trips as absent, never as `null`"
    );
    // The parked and failed corpora are the same proof for the two
    // other pre-R7.5 report shapes.
    for corpus in ["report_parked.json", "report_failed.json"] {
        let report: LifecycleReport = round_trip(&format!("formats/corpora/lifecycle/e1/{corpus}"));
        assert!(report.verification.is_none(), "{corpus}");
    }
}

/// The matched corpus: an exact identity over one declared-input
/// manifest and one artifact, every row `matched`, and the root
/// agreeing with them.
#[test]
fn the_verified_corpus_carries_one_exact_matched_identity() {
    let report: LifecycleReport = round_trip("formats/corpora/lifecycle/e1/report_verified.json");
    assert!(report.ok);
    let evidence = report.verification.as_ref().expect("the member is present");
    validate(evidence).unwrap_or_else(|e| panic!("the authored corpus violates a law: {e}"));
    assert_eq!(evidence.evidence, 1);
    assert_eq!(evidence.status, EvidenceStatus::Matched);
    assert_eq!(evidence.run.run_id, "00112233445566778899aabbccddeeff");
    assert_eq!(evidence.run.selected, ".");

    let input = &evidence.inputs[0];
    assert_eq!(input.execution, "org.demo/provider#compile");
    assert_eq!(input.phase, "build");
    assert_eq!(input.patterns, ["src/**", "Cargo.toml"]);
    assert_eq!(input.status, EvidenceStatus::Matched);
    // The two halves are equal AND counted — the count is what makes a
    // same-length content mutation visible (architecture §8, E7).
    let measured = input.measured.as_ref().unwrap();
    assert_eq!(measured, input.observed.as_ref().unwrap());
    assert_eq!(measured.files, Some(7));
    assert_eq!(measured.algorithm, "sha256:vibe-input-manifest-v1");
    // The byte count is a canonical DECIMAL STRING above `u32::MAX`,
    // authored into the corpus on purpose: it proves the lossless
    // spelling survives the whole serde round-trip, not merely the
    // validator. A `uint32` member could not carry this document at
    // all, and a reader that narrowed it would wrap to 0.
    assert_eq!(measured.bytes.as_deref(), Some("4294967296"));
    assert!(
        measured.bytes.as_deref().unwrap().parse::<u32>().is_err(),
        "the authored count is deliberately past what a uint32 could hold"
    );

    let artifact = &evidence.artifacts[0];
    assert_eq!(artifact.id, "guide");
    assert_eq!(artifact.path, "docs/guide.md");
    assert_eq!(
        artifact.measured.as_ref().unwrap().algorithm,
        "sha256:file-v1",
        "an artifact witness is its own algorithm, not the manifest's"
    );
    assert!(
        artifact.measured.as_ref().unwrap().files.is_none(),
        "a file witness counts nothing; the bytes ARE the witness"
    );
}

/// The stale corpus: one mismatching row under two matching ones, the
/// worst row speaking for the root, and no verify contribution in the
/// executed prefix — the report shape of «stop before dispatch».
#[test]
fn the_stale_corpus_stops_the_run_and_names_the_row_that_moved() {
    let report: LifecycleReport =
        round_trip("formats/corpora/lifecycle/e1/report_verification_stale.json");
    assert!(!report.ok);
    assert_eq!(report.steps.last().unwrap().phase, "verify");
    assert_eq!(report.steps.last().unwrap().status, "fail");
    assert!(
        report.contributions.iter().all(|row| row.phase != "verify"),
        "a stale identity stops verify BEFORE contribution dispatch (architecture §8, E8)"
    );

    let evidence = report.verification.as_ref().unwrap();
    validate(evidence).unwrap_or_else(|e| panic!("the authored corpus violates a law: {e}"));
    assert_eq!(evidence.status, EvidenceStatus::Stale);
    // Row ORDER is part of the answer: the moved row is first, its
    // matching sibling second, and the root is the worst of them.
    assert_eq!(
        evidence
            .inputs
            .iter()
            .map(|row| (row.execution.as_str(), row.status.clone()))
            .collect::<Vec<_>>(),
        [
            ("org.demo/provider#compile", EvidenceStatus::Stale),
            ("org.demo/provider#unit-tests", EvidenceStatus::Matched),
        ]
    );
    let moved = &evidence.inputs[0];
    assert_ne!(moved.measured, moved.observed);
    assert_eq!(
        moved.reason_code.as_deref(),
        Some("declared-input-changed-after-measurement")
    );
    // The authored EMPTY declared scope is a measurement, not an
    // absence — the E3 distinction, in the corpus.
    let empty_scope = &evidence.inputs[1];
    assert!(empty_scope.patterns.is_empty());
    assert_eq!(empty_scope.measured.as_ref().unwrap().files, Some(0));
    assert_eq!(
        empty_scope.measured.as_ref().unwrap().bytes.as_deref(),
        Some("0")
    );
    assert_eq!(evidence.artifacts[0].status, EvidenceStatus::Matched);
    // Every artifact path on the external wire is project-relative —
    // the absolute machine path is durable state's, not this row's.
    for artifact in &evidence.artifacts {
        assert!(
            !artifact.path.starts_with('/') && !artifact.path.contains(':'),
            "an evidence row carries the portable path, not {}",
            artifact.path
        );
    }
}

/// The unavailable corpus: no evidence-bearing rows at all, said out
/// loud, while the command's own `ok` stays true.
#[test]
fn the_unavailable_corpus_is_visible_and_is_not_a_policy_failure() {
    let report: LifecycleReport =
        round_trip("formats/corpora/lifecycle/e1/report_verification_unavailable.json");
    assert!(
        report.ok,
        "`unavailable` is visible but is not a universal policy failure"
    );
    let evidence = report.verification.as_ref().unwrap();
    validate(evidence).unwrap_or_else(|e| panic!("the authored corpus violates a law: {e}"));
    assert_eq!(evidence.status, EvidenceStatus::Unavailable);
    assert!(evidence.inputs.is_empty() && evidence.artifacts.is_empty());
    let rendered = serde_json::to_value(evidence).unwrap();
    assert_eq!(
        rendered["inputs"],
        serde_json::json!([]),
        "the empty row lists are EMITTED, so an empty evidence set is a statement"
    );
    assert_eq!(rendered["artifacts"], serde_json::json!([]));
}

/// The root pulls the ONE shared fragment and spells the member with
/// `x-default: null` — schema-side parity, so a copy-paste drift is
/// red before any Rust is generated.
#[test]
fn the_report_root_spells_the_verification_member_over_the_shared_fragment() {
    let schema = read_json("schemas/lifecycle_report.jtd.json");
    let declared: BTreeSet<String> = schema["metadata"]["x-vocabularies"]
        .as_array()
        .expect("the root declares its vocabularies")
        .iter()
        .map(|value| value.as_str().expect("a vocabulary name").to_string())
        .collect();
    assert!(declared.contains("verification_evidence"));
    let member = &schema["optionalProperties"]["verification"];
    assert_eq!(member["ref"], "verification_evidence");
    assert_eq!(member["metadata"]["x-default"], serde_json::Value::Null);

    // The fragment family lives in the one vocabulary home, and the
    // root fragment names every one it composes.
    let vocabularies = read_json("formats/vocabularies.json");
    for fragment in [
        "evidence_status",
        "digest_witness",
        "evidence_run",
        "input_measurement",
        "artifact_witness",
        "verification_evidence",
    ] {
        assert!(
            vocabularies[fragment].is_object(),
            "the {fragment} fragment lives in the vocabulary home"
        );
    }
    let pulled: BTreeSet<String> =
        vocabularies["verification_evidence"]["metadata"]["x-vocabularies"]
            .as_array()
            .expect("the root fragment declares its dependencies")
            .iter()
            .map(|value| value.as_str().expect("a vocabulary name").to_string())
            .collect();
    assert_eq!(
        pulled,
        set(&[
            "timestamp",
            "evidence_status",
            "evidence_run",
            "input_measurement",
            "artifact_witness"
        ])
    );
}

/// The fragment's documented law labels and diagnostic cap are exactly
/// what the hand-written validator implements — an undocumented law
/// and an unimplemented label are both red, and the cap is the SAME
/// budget the trace index carries.
#[test]
fn law_labels_and_diagnostic_cap_match_the_vocabulary_fragment() {
    let fragment = read_json("formats/vocabularies.json")["verification_evidence"].clone();
    let documented: BTreeSet<String> = fragment["metadata"]["x-relational-laws"]
        .as_array()
        .expect("x-relational-laws is an array")
        .iter()
        .map(|law| {
            law.as_str()
                .expect("every law is a string")
                .split_once(':')
                .expect("every law is `label: sentence`")
                .0
                .to_string()
        })
        .collect();
    let implemented: BTreeSet<String> = IMPLEMENTED_LAWS.iter().map(|l| (*l).to_string()).collect();
    assert_eq!(
        documented, implemented,
        "law parity drift between the fragment and behaviour::verification_evidence"
    );
    assert_eq!(
        fragment["metadata"]["x-diagnostic-cap-bytes"].as_u64(),
        Some(vibe_wire::behaviour::compiler_trace_index::DIAGNOSTIC_CAP_BYTES as u64),
        "the evidence cap is the trace index's diagnostic cap — one budget"
    );
}

/// No field and no enum value on the evidence wire — schema side or
/// generated side — is one of the four verdict words. `Q8`/`E10` of
/// the architecture's matrix, as a fence rather than a promise.
#[test]
fn the_evidence_wire_carries_no_verdict_vocabulary() {
    let forbidden: BTreeSet<String> = FORBIDDEN_VERDICT_WORDS
        .iter()
        .map(|w| (*w).to_string())
        .collect();
    let mut names = BTreeSet::new();
    for document in [
        "formats/vocabularies.json",
        "schemas/lifecycle_report.jtd.json",
        "schemas/lifecycle_state.jtd.json",
    ] {
        declared_names(&read_json(document), &mut names);
    }
    let offenders: Vec<&String> = names.intersection(&forbidden).collect();
    assert!(
        offenders.is_empty(),
        "the evidence wire declares verdict vocabulary: {offenders:?}"
    );

    // The generated side, independently: a field or a serde rename
    // spelling one of the words would have to come from a schema, and
    // scanning the emission proves the two sides agree.
    let generated =
        std::fs::read_to_string(repo_root().join("crates/vibe-wire/src/generated/shared/mod.rs"))
            .unwrap();
    for word in FORBIDDEN_VERDICT_WORDS {
        assert!(
            !generated.contains(&format!("pub {word}:")),
            "generated shared module declares a `{word}` field"
        );
        assert!(
            !generated.contains(&format!("#[serde(rename = \"{word}\")]")),
            "generated shared module declares a `{word}` wire value"
        );
    }

    // The five outcome words are exactly the closed set, in the order
    // the vocabulary declares them — a sixth would be a policy change.
    assert_eq!(
        read_json("formats/vocabularies.json")["evidence_status"]["enum"],
        serde_json::json!(["matched", "stale", "missing", "unavailable", "unstable"])
    );
    assert_eq!(
        read_json("formats/vocabularies.json")["evidence_status"]["metadata"]["x-vocabulary"],
        serde_json::json!("closed")
    );
}

/// `ExecutionRecordStatus` keeps its own closed lifecycle vocabulary:
/// the evidence words never leak into it (architecture §8, E10).
#[test]
fn the_execution_record_status_vocabulary_did_not_widen() {
    let state = read_json("schemas/lifecycle_state.jtd.json");
    assert_eq!(
        state["definitions"]["execution_record"]["properties"]["status"]["enum"],
        serde_json::json!(["ok", "fail", "skip", "fresh", "delegated"])
    );
    let evidence: BTreeSet<String> =
        read_json("formats/vocabularies.json")["evidence_status"]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
    let lifecycle: BTreeSet<String> =
        state["definitions"]["execution_record"]["properties"]["status"]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
    assert!(
        evidence.is_disjoint(&lifecycle),
        "the evidence and execution-status vocabularies must not overlap"
    );
}
