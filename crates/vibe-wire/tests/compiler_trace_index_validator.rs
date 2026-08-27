//! Semantic reds for the compile trace index's IDENTITY and STRUCTURE
//! laws — schema epoch, the scalar gates, timestamps, the scope matrix,
//! sequence density, invocation keys, the shape ladder and the event
//! matrix. Each test mutates an authored golden document into exactly
//! one violation and asserts the validator names that family.
//!
//! Its sibling `compiler_trace_index_relational.rs` carries the laws
//! that read the document as a WHOLE — snapshot portability, the root's
//! terminal word, the timing table and the diagnostic cap. The
//! corpus/reader half lives in `compiler_trace_index_wire_corpus.rs`;
//! the laws themselves are `behaviour::compiler_trace_index`.

mod compiler_trace_index_support;

use compiler_trace_index_support::{check, corpus, failed, ok, remove, running};
use serde_json::json;
use vibe_wire::behaviour::compiler_trace_index::{SCALAR_PREVIEW_BYTES, TraceIndexError};

/// `schema-epoch` — anything but 1 is refused before a field is read.
#[test]
fn wrong_schema_epoch_is_red() {
    let mut doc = ok();
    doc["schema"] = json!(2);
    let error = check(doc).expect_err("schema 2 must be red");
    assert!(matches!(error, TraceIndexError::SchemaEpoch { schema: 2 }));
    assert_eq!(error.law(), "schema-epoch");
}

/// `scalar-gates` — run id, root digest, and the free-text identity
/// scalars.
#[test]
fn unsafe_scalars_are_red() {
    let mut doc = ok();
    doc["run_id"] = json!("0123456789ABCDEF0123456789ABCDEF");
    let error = check(doc.clone()).expect_err("uppercase run_id must be red");
    assert!(matches!(
        error,
        TraceIndexError::RunIdNotLowercaseHex { .. }
    ));

    doc["run_id"] = json!("0123456789abcdef");
    assert!(matches!(
        check(doc.clone()).expect_err("short run_id"),
        TraceIndexError::RunIdNotLowercaseHex { .. }
    ));

    let mut doc = ok();
    doc["project"]["root_digest"] = json!("md5:0123");
    assert!(matches!(
        check(doc.clone()).expect_err("md5 digest"),
        TraceIndexError::RootDigestMalformed { .. }
    ));
    doc["project"]["root_digest"] = json!("sha256:zz");
    assert!(matches!(
        check(doc).expect_err("non-hex digest"),
        TraceIndexError::RootDigestMalformed { .. }
    ));

    let mut doc = ok();
    doc["scopes"][0]["label"] = json!("");
    assert_eq!(check(doc).expect_err("blank label").law(), "scalar-gates");

    let mut doc = ok();
    doc["scopes"][1]["id"] = json!("unit:org.demo.tool\n");
    assert_eq!(
        check(doc).expect_err("newline scope id").law(),
        "scalar-gates"
    );

    let mut doc = ok();
    doc["events"][0]["pass"] = json!("\u{0}");
    assert_eq!(check(doc).expect_err("NUL pass name").law(), "scalar-gates");

    let mut doc = ok();
    doc["aggregates"][0]["pass"] = json!("parse\u{0}");
    assert_eq!(
        check(doc).expect_err("NUL aggregate pass").law(),
        "scalar-gates"
    );

    let mut doc = ok();
    doc["scopes"][0]["target"] = json!("");
    assert_eq!(
        check(doc).expect_err("blank target spelling").law(),
        "scalar-gates"
    );

    // Whitespace-only is blank: an id of three spaces is not an identity.
    let mut doc = ok();
    doc["scopes"][0]["label"] = json!("   \t ");
    assert!(matches!(
        check(doc).expect_err("whitespace-only label"),
        TraceIndexError::UnsafeScalar {
            field: "scope.label",
            ..
        }
    ));

    // A bare CR splits a log line exactly as an LF does.
    let mut doc = ok();
    doc["scopes"][1]["artifact"] = json!("static-xml\r");
    assert!(matches!(
        check(doc).expect_err("carriage return in an artifact id"),
        TraceIndexError::UnsafeScalar {
            field: "scope.artifact",
            ..
        }
    ));

    let mut doc = ok();
    doc["events"][0]["pass"] = json!("par\rse");
    assert_eq!(
        check(doc)
            .expect_err("carriage return in a pass name")
            .law(),
        "scalar-gates"
    );
}

/// `scalar-gates` — epoch-1 `project.display` is exactly `"."`, so an
/// absolute developer root cannot ride into a shared trace.
#[test]
fn a_project_display_other_than_root_is_red() {
    for display in ["C:\\Users\\dev\\demo", "/home/dev/demo", "./demo", "demo"] {
        let mut doc = ok();
        doc["project"]["display"] = json!(display);
        let error = check(doc).expect_err("a non-root project display must be red");
        assert!(
            matches!(error, TraceIndexError::ProjectDisplayNotRoot { .. }),
            "{display}: {error}"
        );
        assert_eq!(error.law(), "scalar-gates");
    }
}

/// `scalar-gates` — a custom target is an open-vocabulary spelling from
/// a plugin, held to the same backend id charset the compiler applies.
#[test]
fn a_custom_artifact_target_obeys_the_backend_id_charset() {
    for target in ["Acme PDF", "acme/pdf", "-leading", "Static-Md", "emit:pdf"] {
        let mut doc = ok();
        doc["scopes"][0]["target"] = json!(target);
        let error = check(doc).expect_err("an off-charset custom target must be red");
        assert!(
            matches!(error, TraceIndexError::CustomTargetCharset { .. }),
            "{target}: {error}"
        );
        assert_eq!(error.law(), "scalar-gates");
    }

    // A well-formed custom backend is exactly what the open vocabulary
    // is FOR — it stays green, unknown to this build and all.
    let mut doc = ok();
    doc["scopes"][0]["target"] = json!("acme.pdf-writer");
    check(doc).expect("a charset-legal custom target is green");
}

/// A refusal never clones the offending value: an index left behind by a
/// corrupt writer can carry a megabyte-long scalar, and the error keeps
/// a bounded head plus the true byte length instead.
#[test]
fn a_refusal_renders_a_huge_scalar_bounded() {
    let huge = "9".repeat(4 * 1024 * 1024);
    let mut doc = ok();
    doc["run_id"] = json!(huge);
    let error = check(doc).expect_err("a 4 MiB run_id must be red");
    let TraceIndexError::RunIdNotLowercaseHex { ref run_id } = error else {
        panic!("expected the run-id family, got {error}");
    };
    assert_eq!(run_id.bytes(), huge.len());
    assert!(run_id.head().len() <= SCALAR_PREVIEW_BYTES);
    let rendered = format!("{error}");
    assert!(
        rendered.len() < 256,
        "a refusal must not print the whole value ({} bytes)",
        rendered.len()
    );
    assert!(rendered.contains(&format!("({} bytes)", huge.len())));
}

/// `timestamp-coherence` — a running index is not finished, and a
/// finish never precedes the start.
#[test]
fn timestamp_contradictions_are_red() {
    let mut doc = running();
    doc["finished"] = json!("2026-08-27T11:00:05Z");
    let error = check(doc).expect_err("running with finished");
    assert!(matches!(error, TraceIndexError::FinishedWhileRunning));
    assert_eq!(error.law(), "timestamp-coherence");

    let mut doc = ok();
    doc["finished"] = json!("2026-08-26T09:00:00Z");
    let error = check(doc).expect_err("finished before started");
    assert!(matches!(
        error,
        TraceIndexError::FinishedBeforeStarted { .. }
    ));
}

/// `scope-identity` — ids are unique and every event's scope is live.
#[test]
fn scope_identity_violations_are_red() {
    let mut doc = ok();
    doc["scopes"][1]["id"] = json!("node:.");
    let error = check(doc).expect_err("duplicate scope id");
    assert!(matches!(error, TraceIndexError::DuplicateScopeId { .. }));
    assert_eq!(error.law(), "scope-identity");

    let mut doc = ok();
    doc["events"][0]["scope"] = json!("ghost");
    assert!(matches!(
        check(doc).expect_err("ghost event scope"),
        TraceIndexError::UnknownEventScope { .. }
    ));
}

/// `scope-status-coherence` — the fingerprint/failure pair follows the
/// status, for every status.
#[test]
fn scope_status_matrix_violations_are_red() {
    let mut doc = ok();
    remove(&mut doc, "/scopes/0", "fingerprint");
    let error = check(doc).expect_err("compiled scope without fingerprint");
    assert!(matches!(
        error,
        TraceIndexError::ScopeStatusIncoherent { .. }
    ));
    assert_eq!(error.law(), "scope-status-coherence");

    let mut doc = running();
    doc["scopes"][1]["fingerprint"] = json!("sha256:1");
    assert!(matches!(
        check(doc).expect_err("pending scope with fingerprint"),
        TraceIndexError::ScopeStatusIncoherent { .. }
    ));

    let mut doc = failed();
    remove(&mut doc, "/scopes/1", "failure");
    assert!(matches!(
        check(doc).expect_err("failed scope without failure"),
        TraceIndexError::ScopeStatusIncoherent { .. }
    ));

    let mut doc = failed();
    doc["scopes"][1]["fingerprint"] = json!("sha256:1");
    assert!(matches!(
        check(doc).expect_err("failed scope with fingerprint"),
        TraceIndexError::ScopeStatusIncoherent { .. }
    ));

    let mut doc = ok();
    doc["scopes"][2]["failure"] = json!("must not ride a skip");
    assert!(matches!(
        check(doc).expect_err("skipped scope with failure"),
        TraceIndexError::ScopeStatusIncoherent { .. }
    ));
}

/// `skipped-scope-is-silent` — a skipped scope records its fingerprint
/// and has no pass events.
#[test]
fn skipped_scope_with_events_is_red() {
    let mut doc = ok();
    doc["events"].as_array_mut().unwrap().push(json!({
        "sequence": 3,
        "scope": "publish:org.demo.tool",
        "invocation": 0,
        "pass": "emit:static-xml",
        "input_shape": { "level": "lane", "cardinality": "artifact" },
        "output_shape": { "level": "emitted", "cardinality": "artifact" },
        "status": "ok",
        "pass_micros": { "micros": 10, "saturated": false },
        "verify_micros": { "micros": 1, "saturated": false },
        "encode_micros": { "micros": 2, "saturated": false },
        "snapshot": "0003-emit%3Astatic%2Dxml-publish_org.demo.tool_static%2Dxml-000.json"
    }));
    let error = check(doc).expect_err("an event on a skipped scope");
    assert!(matches!(
        error,
        TraceIndexError::SkippedScopeHasEvents { .. }
    ));
    assert_eq!(error.law(), "skipped-scope-is-silent");
}

/// The silence law is about SKIPS only. A compiled scope with no events
/// is the zero-document artifact — legal, and green, because a skip is
/// a claim about a reused fingerprint that an event would contradict,
/// while "nothing to run" is not a claim at all.
#[test]
fn a_compiled_scope_with_no_events_is_green() {
    check(corpus("ok_zero_document_artifact.json"))
        .expect("a zero-document artifact compiles silently");

    // The same shape under a failed root: the scope failed before its
    // first pass, and the root failure is justified by the scope.
    let mut doc = failed();
    doc["events"] = json!([]);
    doc["aggregates"] = json!([]);
    check(doc).expect("a scope that failed before its first pass is legal");
}

/// `sequence-density` — the global sequence is dense from 0 in list
/// order.
#[test]
fn non_dense_sequences_are_red() {
    let mut doc = ok();
    doc["events"][2]["sequence"] = json!(5);
    let error = check(doc).expect_err("a hole in the sequence");
    assert!(matches!(error, TraceIndexError::SequenceNotDense { .. }));
    assert_eq!(error.law(), "sequence-density");

    let mut doc = ok();
    doc["events"].as_array_mut().unwrap().remove(1);
    assert!(matches!(
        check(doc).expect_err("a dropped event leaves a hole"),
        TraceIndexError::SequenceNotDense { .. }
    ));
}

/// `invocation-key` — the ordinals of one `(scope, pass)` are DENSE
/// from zero in encounter order, not merely unique. Uniqueness would
/// admit `0, 7`, which no compiler produced.
#[test]
fn non_dense_invocation_ordinals_are_red() {
    // A repeat: the second parse spends an ordinal already gone.
    let mut doc = ok();
    doc["events"][1]["invocation"] = json!(0);
    let error = check(doc).expect_err("the same (scope, pass, invocation) twice");
    assert!(matches!(
        error,
        TraceIndexError::InvocationNotDense {
            expected: 1,
            invocation: 0,
            ..
        }
    ));
    assert_eq!(error.law(), "invocation-key");

    // Starting above zero.
    let mut doc = ok();
    doc["events"][0]["invocation"] = json!(7);
    assert!(matches!(
        check(doc).expect_err("the first invocation is ordinal 0"),
        TraceIndexError::InvocationNotDense {
            expected: 0,
            invocation: 7,
            ..
        }
    ));

    // A gap: 0 then 2.
    let mut doc = ok();
    doc["events"][1]["invocation"] = json!(2);
    assert!(matches!(
        check(doc).expect_err("a gap in the ordinals"),
        TraceIndexError::InvocationNotDense {
            expected: 1,
            invocation: 2,
            ..
        }
    ));

    // Out of order: 1 then 0 across the two parses.
    let mut doc = ok();
    doc["events"][0]["invocation"] = json!(1);
    doc["events"][1]["invocation"] = json!(0);
    assert_eq!(
        check(doc).expect_err("descending ordinals").law(),
        "invocation-key"
    );

    // A whole-artifact pass is ordinal 0, and each (scope, pass) counts
    // on its own — the emit event keeps 0 while parse runs 0 and 1.
    check(ok()).expect("the authored ordinals are dense per (scope, pass)");
}

/// `shape-ladder` — level and cardinality pair off the IR ladder.
#[test]
fn off_ladder_shapes_are_red() {
    let mut doc = ok();
    doc["events"][0]["input_shape"]["cardinality"] = json!("artifact");
    let error = check(doc).expect_err("source is never artifact-cardinal");
    assert!(matches!(
        error,
        TraceIndexError::IllegalShape { which: "input", .. }
    ));
    assert_eq!(error.law(), "shape-ladder");

    let mut doc = ok();
    doc["events"][0]["output_shape"] = json!({ "level": "closure", "cardinality": "document" });
    assert!(matches!(
        check(doc).expect_err("closure is never document-cardinal"),
        TraceIndexError::IllegalShape {
            which: "output",
            ..
        }
    ));
}

/// `event-coherence` — the status's snapshot/diagnostic/duration
/// matrix, including the packet's two mandated mutations.
#[test]
fn event_contradictions_are_red() {
    let mut doc = ok();
    doc["events"][0]["diagnostic"] = json!("an ok event carries no diagnostic");
    let error = check(doc).expect_err("ok with diagnostic");
    assert!(matches!(error, TraceIndexError::EventIncoherent { .. }));
    assert_eq!(error.law(), "event-coherence");

    let mut doc = ok();
    remove(&mut doc, "/events/0", "snapshot");
    assert!(matches!(
        check(doc).expect_err("ok without snapshot"),
        TraceIndexError::EventIncoherent { .. }
    ));

    let mut doc = ok();
    remove(&mut doc, "/events/0", "pass_micros");
    assert!(matches!(
        check(doc).expect_err("ok without pass duration"),
        TraceIndexError::EventIncoherent { .. }
    ));

    let mut doc = failed();
    doc["events"][1]["verify_micros"] = json!({ "micros": 5, "saturated": false });
    assert!(matches!(
        check(doc).expect_err("pass-failed with verify duration"),
        TraceIndexError::EventIncoherent { .. }
    ));

    let mut doc = failed();
    remove(&mut doc, "/events/1", "diagnostic");
    assert!(matches!(
        check(doc).expect_err("failed event without diagnostic"),
        TraceIndexError::EventIncoherent { .. }
    ));

    // The packet's mandated mutation: a failed event that names a
    // snapshot. (Its prefix must stay coherent so the family named is
    // event-coherence, not snapshot-portability.)
    let mut doc = failed();
    doc["events"][1]["snapshot"] = json!("0001-close-unit_org.demo.tool_static%2Dxml-000.json");
    assert!(matches!(
        check(doc).expect_err("a failed event certifies nothing"),
        TraceIndexError::EventIncoherent {
            expected: false,
            ..
        }
    ));
}

/// `event-coherence` — a duration that claims saturation without
/// sitting at the ceiling is a lie about what was measured, and it is
/// refused at the event before any total is compared.
#[test]
fn a_non_canonical_event_duration_is_red() {
    for field in ["pass_micros", "verify_micros", "encode_micros"] {
        let mut doc = ok();
        doc["events"][0][field] = json!({ "micros": 1, "saturated": true });
        let error = check(doc).expect_err("saturated below the ceiling must be red");
        assert!(
            matches!(
                error,
                TraceIndexError::NonCanonicalDuration { micros: 1, .. }
            ),
            "{field}: {error}"
        );
        assert_eq!(error.law(), "event-coherence");
    }

    // An exact measurement AT the ceiling is legal without the marker.
    let mut doc = ok();
    doc["events"][0]["pass_micros"] = json!({ "micros": u32::MAX, "saturated": false });
    doc["aggregates"][0]["pass_total"] = json!({ "micros": u32::MAX, "saturated": true });
    check(doc).expect("an unsaturated u32::MAX is a legal measurement");
}

/// `aggregate-reconciliation` — the same canonicality on a carried
/// total, checked before the comparison so the error names the lie
/// rather than the arithmetic.
#[test]
fn a_non_canonical_aggregate_total_is_red() {
    for column in ["pass_total", "verify_total", "encode_total"] {
        let mut doc = ok();
        doc["aggregates"][0][column] = json!({ "micros": 7, "saturated": true });
        let error = check(doc).expect_err("a saturated total below the ceiling must be red");
        assert!(
            matches!(
                error,
                TraceIndexError::NonCanonicalDuration { micros: 7, .. }
            ),
            "{column}: {error}"
        );
        assert_eq!(error.law(), "aggregate-reconciliation");
    }
}

/// The budget status is a legal shape of success: green when the event
/// and the aggregates tell the same story.
#[test]
fn a_budget_skip_is_green_when_coherent() {
    let mut doc = ok();
    doc["events"][1]["status"] = json!("snapshot-skipped-budget");
    remove(&mut doc, "/events/1", "snapshot");
    remove(&mut doc, "/events/1", "encode_micros");
    doc["aggregates"][0]["encode_total"] = json!({ "micros": 800, "saturated": false });
    check(doc).expect("a coherent budget skip is green");
}
