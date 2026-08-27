//! Authored golden documents for the epoch-1 compile trace INDEX
//! (`schemas/compiler_trace_index/e1/index.jtd.json` — the metadata
//! half of PROP-054 `##OBS-TRACE`). The IR itself never crosses here:
//! snapshots are the registered `compiler_ir/e1` contract, this record
//! names them by filename.
//!
//! Three kinds of check sit beside each other, and they are not the
//! same kind. READER checks prove the generated strict reader refuses
//! unknown closed spellings and wrong types while carrying a newer
//! writer's extra object members (the registry's `foreign_parsers =
//! "many"` policy — no stricter ad-hoc reader on top, PROP-044 §4.2).
//! WRITER checks prove the corpus itself emits ONLY the epoch-1 member
//! set, so the forward-compatibility positive is a policy proof, not a
//! leak. The RELATIONAL laws live in the hand-written validator
//! (`behaviour::compiler_trace_index`) and their semantic reds in
//! `compiler_trace_index_validator.rs`; here the label set is pinned
//! against the schema's `x-relational-laws`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use vibe_wire::behaviour::compiler_trace_index::{
    DIAGNOSTIC_CAP_BYTES, IMPLEMENTED_LAWS, SHORT_DIGEST_HEX, SNAPSHOT_NAME_CAP,
};
use vibe_wire::generated::compiler_trace_index::e1::index::{
    CompilerTraceIndex, IrLevel, PassStatus, RunStatus, ScopeStatus,
};
use vibe_wire::generated::format_id::{ForeignParsers, FormatId};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn corpus() -> PathBuf {
    repo_root().join("formats/corpora/compiler_trace_index/e1")
}

fn read_json(relative: &str) -> serde_json::Value {
    let path = repo_root().join(relative);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} readable: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{relative} parses: {e}"))
}

fn read_corpus(name: &str) -> serde_json::Value {
    let path = corpus().join(name);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} readable: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{name} parses: {e}"))
}

fn valid_names() -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(corpus().join("valid"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn parse_and_validate(doc: serde_json::Value) -> CompilerTraceIndex {
    let index: CompilerTraceIndex =
        serde_json::from_value(doc).expect("parses through the generated reader");
    vibe_wire::behaviour::compiler_trace_index::validate(&index)
        .unwrap_or_else(|error| panic!("corpus document violates a relational law: {error}"));
    index
}

/// The exact epoch-1 member sets. A document the current corpus writes
/// uses exactly these keys — the writer-side half of the many-reader
/// policy (the reader is permissive; the corpus proves we do not need
/// it to be).
const ROOT_KEYS: [&str; 9] = [
    "aggregates",
    "events",
    "failure",
    "finished",
    "project",
    "run_id",
    "schema",
    "scopes",
    "started",
    // status is the tenth member on terminal/failed documents only.
];

#[test]
fn every_plain_valid_document_round_trips_and_validates() {
    for name in valid_names() {
        if name.starts_with("forward_compatible") {
            continue; // its own test: permissive carry, not byte round-trip
        }
        let authored = read_corpus(&format!("valid/{name}"));
        let index: CompilerTraceIndex = serde_json::from_value(authored.clone())
            .unwrap_or_else(|e| panic!("{name} parses: {e}"));
        vibe_wire::behaviour::compiler_trace_index::validate(&index)
            .unwrap_or_else(|error| panic!("{name} violates a relational law: {error}"));
        let round_trip = serde_json::to_value(&index).unwrap();
        assert_eq!(
            round_trip, authored,
            "{name} loses data on generated round-trip"
        );
    }
}

#[test]
fn every_plain_valid_document_emits_only_the_epoch_one_member_set() {
    let scope_keys: BTreeSet<&str> = [
        "artifact",
        "failure",
        "fingerprint",
        "id",
        "kind",
        "label",
        "status",
        "target",
    ]
    .into_iter()
    .collect();
    let event_keys: BTreeSet<&str> = [
        "diagnostic",
        "encode_micros",
        "input_shape",
        "invocation",
        "output_shape",
        "pass",
        "pass_micros",
        "scope",
        "sequence",
        "snapshot",
        "status",
        "verify_micros",
    ]
    .into_iter()
    .collect();
    let row_keys: BTreeSet<&str> = [
        "encode_total",
        "invocations",
        "pass",
        "pass_total",
        "verify_total",
    ]
    .into_iter()
    .collect();
    let root_keys: BTreeSet<&str> = ROOT_KEYS.into_iter().chain(["status"]).collect();

    for name in valid_names() {
        if name.starts_with("forward_compatible") {
            continue;
        }
        let doc = read_corpus(&format!("valid/{name}"));
        let root = doc.as_object().unwrap();
        assert!(
            root.keys().all(|k| root_keys.contains(k.as_str())),
            "{name}: root carries a non-epoch-1 member {:?}",
            root.keys().find(|k| !root_keys.contains(k.as_str()))
        );
        for (section, allowed) in [("scopes", &scope_keys), ("events", &event_keys)] {
            for entry in doc[section].as_array().unwrap() {
                let keys: BTreeSet<&str> = entry
                    .as_object()
                    .unwrap()
                    .keys()
                    .map(String::as_str)
                    .collect();
                assert!(
                    keys.is_subset(allowed),
                    "{name}: {section} entry carries a non-epoch-1 member {:?}",
                    keys.difference(allowed).next()
                );
            }
        }
        for row in doc["aggregates"].as_array().unwrap() {
            let keys: BTreeSet<&str> = row
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect();
            assert_eq!(keys, row_keys, "{name}: timing row member set");
        }
    }
}

#[test]
fn ok_run_carries_two_parses_one_whole_artifact_pass_and_exact_aggregates() {
    let index = parse_and_validate(read_corpus("valid/ok_complete.json"));
    assert_eq!(index.schema, 1);
    assert_eq!(index.status, RunStatus::Ok);
    assert_eq!(index.scopes.len(), 3);
    let parses = index
        .events
        .iter()
        .filter(|event| event.pass == "parse")
        .count();
    assert_eq!(parses, 2, "two parse invocations");
    let emit = index.events.last().unwrap();
    assert_eq!(emit.pass, "emit:static-xml");
    assert_eq!(emit.invocation, 0);
    assert_eq!(emit.input_shape.level, IrLevel::Lane);
    // Distinct certified snapshots, one per ok event.
    let snapshots: BTreeSet<&str> = index
        .events
        .iter()
        .filter_map(|event| event.snapshot.as_deref())
        .collect();
    assert_eq!(snapshots.len(), 3, "every ok event names its own snapshot");
    // The aggregate rows reconcile to the events by construction (the
    // validator proved it); pin the authored numbers the CLI table reads.
    assert_eq!(index.aggregates.len(), 2);
    let parse_row = &index.aggregates[0];
    assert_eq!(parse_row.invocations, 2);
    assert_eq!(parse_row.pass_total.micros, 2100);
    assert!(!parse_row.pass_total.saturated);
    assert_eq!(parse_row.encode_total.micros, 1500);
    let emit_row = &index.aggregates[1];
    assert_eq!(emit_row.pass, "emit:static-xml");
    assert_eq!(emit_row.pass_total.micros, 4200);
}

#[test]
fn failed_run_keeps_the_prior_snapshot_and_refuses_the_failed_event_one() {
    let index = parse_and_validate(read_corpus("valid/failed_partial.json"));
    assert_eq!(index.status, RunStatus::Failed);
    assert!(index.failure.is_some());
    let ok_event = &index.events[0];
    let failed_event = &index.events[1];
    assert_eq!(ok_event.status, PassStatus::Ok);
    assert!(
        ok_event.snapshot.is_some(),
        "the prior success keeps its snapshot"
    );
    assert_eq!(failed_event.status, PassStatus::PassFailed);
    assert!(
        failed_event.snapshot.is_none(),
        "a failed event certifies nothing"
    );
    assert!(failed_event.diagnostic.is_some());
    assert!(failed_event.pass_micros.is_some());
    assert!(
        failed_event.verify_micros.is_none(),
        "verification never ran"
    );
    // The failed scope carries its bounded failure and no fingerprint.
    let failed_scope = &index.scopes[1];
    assert_eq!(failed_scope.status, ScopeStatus::Failed);
    assert!(failed_scope.failure.is_some());
    assert!(failed_scope.fingerprint.is_none());
}

#[test]
fn running_index_shows_the_fingerprint_only_skip_with_explicit_empty_lists() {
    let authored = read_corpus("valid/running_skipped.json");
    let index = parse_and_validate(authored.clone());
    assert_eq!(index.status, RunStatus::Running);
    assert!(index.finished.is_none(), "a running index is not finished");
    let skipped = &index.scopes[0];
    assert_eq!(skipped.status, ScopeStatus::Skipped);
    assert!(skipped.fingerprint.is_some(), "the fingerprint-only case");
    assert!(skipped.failure.is_none());
    assert_eq!(index.scopes[1].status, ScopeStatus::Pending);
    assert!(index.events.is_empty());
    assert!(index.aggregates.is_empty());
    // x-empty = emit: the lists are explicit even when empty, so a
    // partial running index is readable as `"events": []`, not `{}`.
    assert_eq!(authored["events"], serde_json::json!([]));
    assert_eq!(authored["aggregates"], serde_json::json!([]));
}

/// The zero-document artifact: a terminal `ok` run whose one scope
/// compiled without a single pass invocation. It is the positive that
/// keeps `skipped-scope-is-silent` honest — silence is required of a
/// SKIP, not of every eventless scope — and it exercises `x-empty =
/// emit` on a document that is finished rather than partial.
#[test]
fn a_zero_document_artifact_is_a_finished_run_with_no_events() {
    let authored = read_corpus("valid/ok_zero_document_artifact.json");
    let index = parse_and_validate(authored.clone());
    assert_eq!(index.status, RunStatus::Ok);
    assert!(index.finished.is_some());
    assert_eq!(index.scopes.len(), 1);
    assert_eq!(index.scopes[0].status, ScopeStatus::Compiled);
    assert!(index.scopes[0].fingerprint.is_some());
    assert!(index.events.is_empty());
    assert!(index.aggregates.is_empty());
    assert_eq!(authored["events"], serde_json::json!([]));
    assert_eq!(authored["aggregates"], serde_json::json!([]));
}

/// The short canonical snapshot name, in the corpus rather than only in
/// a mutation: a unit whose label makes the full form pass the 96-byte
/// ceiling, so `~` plus the first 16 hex of the middle's SHA-256 is the
/// ONLY spelling that event can write — and the validator recomputes it.
#[test]
fn an_overlong_name_collapses_to_a_verified_digest() {
    let index = parse_and_validate(read_corpus("valid/ok_short_snapshot_name.json"));
    let snapshot = index.events[0]
        .snapshot
        .as_deref()
        .expect("the ok event names its snapshot");
    assert_eq!(snapshot, "0000-~060ed29890b2fe3d-000.json");
    assert!(snapshot.len() <= SNAPSHOT_NAME_CAP);
    // The digest is the short form's whole content: 16 lowercase hex.
    let digest = &snapshot["0000-~".len()..][..SHORT_DIGEST_HEX];
    assert!(
        digest
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "the digest is lowercase hex"
    );
}

/// Root `ok` beside a `snapshot-failed` event, and root `failed` with
/// nothing else failed: the two shapes that keep the trace an OBSERVER
/// of the compile rather than a participant in its outcome.
#[test]
fn the_trace_observer_never_decides_the_runs_outcome() {
    let observed = parse_and_validate(read_corpus("valid/ok_with_snapshot_failed.json"));
    assert_eq!(observed.status, RunStatus::Ok);
    assert_eq!(observed.events[1].status, PassStatus::SnapshotFailed);
    assert!(observed.events[1].snapshot.is_none());
    assert!(observed.events[1].diagnostic.is_some());

    let rolled_back = parse_and_validate(read_corpus("valid/failed_after_successful_compile.json"));
    assert_eq!(rolled_back.status, RunStatus::Failed);
    assert!(rolled_back.failure.is_some());
    assert!(
        rolled_back
            .scopes
            .iter()
            .all(|scope| scope.status == ScopeStatus::Compiled),
        "no scope failed"
    );
    assert!(
        rolled_back
            .events
            .iter()
            .all(|event| event.status == PassStatus::Ok),
        "no event failed"
    );
}

#[test]
fn reader_refuses_unknown_closed_status_and_wrong_types() {
    for name in ["unknown_closed_status.json", "wrong_type_sequence.json"] {
        let doc = read_corpus(&format!("invalid/{name}"));
        assert!(
            serde_json::from_value::<CompilerTraceIndex>(doc).is_err(),
            "{name} must be refused by the generated reader"
        );
    }
}

#[test]
fn forward_compatible_unknown_members_are_carried_and_ignored() {
    let authored = read_corpus("valid/forward_compatible_unknown_member.json");
    let index = parse_and_validate(authored.clone());
    assert_eq!(index.run_id, "0123456789abcdef0123456789abcdef");
    // The permissive reading DROPS the unknown members on rewrite — an
    // older reader never crashes on a newer writer, and never re-emits
    // what it did not understand. That is the whole many-reader policy.
    let round_trip = serde_json::to_value(&index).unwrap();
    assert!(round_trip.get("trace_tool").is_none());
    assert!(round_trip["events"][0].get("worker_hint").is_none());
    // No stricter ad-hoc reader was bolted on: the same document that
    // carries extra members still validates green through the typed pass.
    assert_eq!(
        index.events[0].snapshot.as_deref(),
        Some("0000-parse-node_._static%2Dmd-000.json")
    );
}

fn registry_table() -> BTreeMap<String, toml::Value> {
    let text = std::fs::read_to_string(repo_root().join("formats/REGISTRY.toml")).unwrap();
    let parsed: toml::Value = toml::from_str(&text).unwrap();
    parsed
        .get("format")
        .and_then(|v| v.as_table())
        .expect("formats/REGISTRY.toml has a [format.*] table")
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}

#[test]
fn registry_record_is_pinned() {
    let formats = registry_table();
    let record = &formats["compiler-trace-index"];
    assert_eq!(record.get("epoch").unwrap().as_integer(), Some(1));
    assert_eq!(
        record.get("schema").unwrap().as_str(),
        Some("schemas/compiler_trace_index/e1/index.jtd.json")
    );
    assert_eq!(record.get("recoverable").unwrap().as_bool(), Some(true));
    assert_eq!(
        record.get("foreign_parsers").unwrap().as_str(),
        Some("many")
    );
    assert_eq!(
        record.get("corpus").unwrap().as_str(),
        Some("formats/corpora/compiler_trace_index/e1")
    );
    assert_eq!(record.get("sunset").unwrap().as_str(), Some("none"));

    // The pinned paths exist on disk.
    assert!(
        repo_root()
            .join("schemas/compiler_trace_index/e1/index.jtd.json")
            .is_file()
    );
    assert!(corpus().join("valid").is_dir());
    assert!(corpus().join("invalid").is_dir());

    // The generated FormatId agrees with the record — registry, enum and
    // schema path are one decision, checked from all three sides.
    let variant = FormatId::ALL
        .iter()
        .copied()
        .find(|id| id.id() == "compiler-trace-index")
        .expect("FormatId carries the compiler-trace-index variant");
    assert_eq!(variant.epoch(), 1);
    assert!(variant.recoverable());
    assert_eq!(variant.foreign_parsers(), ForeignParsers::Many);
}

#[test]
fn compiler_ir_remains_the_only_snapshot_payload_contract() {
    let formats = registry_table();
    let ir = &formats["compiler-ir"];
    assert_eq!(
        ir.get("schema").unwrap().as_str(),
        Some("schemas/compiler_ir/e1/ir.jtd.json"),
        "the snapshot payload contract itself must not have moved"
    );
    assert_eq!(ir.get("epoch").unwrap().as_integer(), Some(1));

    // And this format's snapshot member is a plain filename string — no
    // IR payload, no untyped carrier, no map form anywhere in the schema.
    let schema = read_json("schemas/compiler_trace_index/e1/index.jtd.json");
    let snapshot = &schema["definitions"]["pass_event"]["optionalProperties"]["snapshot"];
    assert_eq!(
        snapshot["type"], "string",
        "the snapshot member is a plain filename string — no IR payload, no carrier"
    );
    assert_eq!(snapshot["metadata"]["x-default"], serde_json::Value::Null);
    assert_eq!(
        snapshot["metadata"]
            .as_object()
            .expect("the snapshot member carries metadata")
            .len(),
        2,
        "x-default and description, and nothing that could smuggle a payload"
    );
    // The prose is free to be rewritten, but it must still name the two
    // canonical spellings and must NOT resurrect the retired alphabet
    // grammar that preceded them.
    let prose = snapshot["metadata"]["description"]
        .as_str()
        .expect("the snapshot member is documented");
    for required in [
        "<seq:04>-<enc(pass)>-<kind>_<enc(scope-label)>_<enc(artifact)>-<ord:03>.json",
        "<seq:04>-~<digest16>-<ord:03>.json",
        "Only `ok` carries one.",
    ] {
        assert!(
            prose.contains(required),
            "the snapshot prose must name {required}"
        );
    }
    assert!(
        !schema.to_string().contains("`- _ . ~ %`"),
        "the retired alphabet grammar must not survive anywhere in the schema"
    );
    assert!(
        no_untyped_forms(&schema),
        "no empty form and no values-map anywhere"
    );
    assert_eq!(
        schema["metadata"]["x-vocabularies"],
        serde_json::json!(["timestamp", "duration", "timing_row"]),
        "the shared vocabularies pulled are the timestamp fragment plus the \
         duration/timing-row fragments the command-report trace member also \
         pulls — one generated Duration/TimingRow, not per-module copies"
    );
}

/// Recursively refuse the two JTD escape hatches an untyped carrier
/// would take: an empty form `{}` (anything goes) and `"values"`
/// (an open map). The trace index is metadata with a closed shape.
fn no_untyped_forms(node: &serde_json::Value) -> bool {
    match node {
        serde_json::Value::Object(map) => {
            !map.is_empty() && !map.contains_key("values") && map.values().all(no_untyped_forms)
        }
        serde_json::Value::Array(items) => items.iter().all(no_untyped_forms),
        _ => true,
    }
}

#[test]
fn law_labels_and_diagnostic_cap_match_the_schema() {
    let schema = read_json("schemas/compiler_trace_index/e1/index.jtd.json");
    let documented: BTreeSet<String> = schema["metadata"]["x-relational-laws"]
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
    let implemented: BTreeSet<&str> = IMPLEMENTED_LAWS.iter().copied().collect();
    let undocumented: Vec<&str> = implemented
        .iter()
        .filter(|law| !documented.contains(**law))
        .copied()
        .collect();
    let unimplemented: Vec<&String> = documented
        .iter()
        .filter(|law| !implemented.contains(law.as_str()))
        .collect();
    assert!(
        undocumented.is_empty() && unimplemented.is_empty(),
        "law parity drift:\n  implemented but undocumented: {undocumented:?}\n  \
         documented but unimplemented: {unimplemented:?}"
    );
    assert_eq!(
        schema["metadata"]["x-diagnostic-cap-bytes"].as_u64(),
        Some(DIAGNOSTIC_CAP_BYTES as u64),
        "the diagnostic cap is one decision shared by schema and validator"
    );
}
