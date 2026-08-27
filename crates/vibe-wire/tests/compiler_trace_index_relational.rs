//! Semantic reds for the compile trace index's WHOLE-DOCUMENT laws —
//! snapshot portability, the root's terminal word, the timing table's
//! reconciliation and order, and the diagnostic cap. Each test mutates
//! an authored golden document into exactly one violation and asserts
//! the validator names that family.
//!
//! Its sibling `compiler_trace_index_validator.rs` carries the identity
//! and structure laws; the shared fixtures are
//! `compiler_trace_index_support/mod.rs`.

mod compiler_trace_index_support;

use compiler_trace_index_support::{check, corpus, failed, ok, remove};
use serde_json::json;
use vibe_wire::behaviour::compiler_trace_index::{SnapshotUnsafety, TraceIndexError};

/// `snapshot-portability` — the filename is CONSTRUCTED from the event
/// and its scope, so a document-level red is any spelling this event
/// could not have written. The character-level table lives in the unit
/// tests; here the point is that the event's own metadata is what the
/// name has to agree with.
#[test]
fn non_canonical_snapshot_filenames_are_red() {
    for (filename, why) in [
        ("0000/a.json", "separator"),
        (r"0000\a.json", "backslash"),
        ("..", "dotdot"),
        ("0000-c:a.json", "raw colon"),
        ("0000-a.json.", "trailing dot"),
        ("0000-a.json ", "trailing space"),
        ("con.json", "device name"),
        ("0000-a<b.json", "windows angle"),
        ("0000-a\u{7}b.json", "control byte"),
        ("0000-паss.json", "non-ascii"),
        ("0000 parse.json", "space inside"),
        ("0000-parse%3a.json", "lowercase escape"),
        (
            "0000-parse-node_._static-md-000.json",
            "raw hyphen in a component",
        ),
        ("0000-p%61rse-node_._static%2Dmd-000.json", "over-encoded A"),
        (
            "000-parse-node_._static%2Dmd-000.json",
            "under-padded sequence",
        ),
        (
            "0000-parse-node_._static%2Dmd-0.json",
            "under-padded ordinal",
        ),
        ("0000-close-node_._static%2Dmd-000.json", "wrong pass"),
        ("0000-parse-unit_._static%2Dmd-000.json", "wrong scope kind"),
        ("0000-parse-node_._static%2Dxml-000.json", "wrong artifact"),
        ("0001-parse-node_._static%2Dmd-000.json", "wrong sequence"),
        ("0000-~0123456789abcdef-000.json", "invented digest"),
    ] {
        let mut doc = ok();
        doc["events"][0]["snapshot"] = json!(filename);
        let error = match check(doc) {
            Err(error) => error,
            Ok(()) => panic!("{why} ({filename}) must be red"),
        };
        assert_eq!(error.law(), "snapshot-portability", "{why}: {error}");
    }

    // Past the 96-byte ceiling no canonical form can exist, and the
    // refusal says exactly that rather than comparing spellings.
    let mut doc = ok();
    doc["events"][0]["snapshot"] = json!("0".repeat(97));
    assert!(matches!(
        check(doc).expect_err("an over-ceiling name"),
        TraceIndexError::UnsafeSnapshot {
            reason: SnapshotUnsafety::TooLong { bytes: 97 },
            ..
        }
    ));

    // Two events claiming one file: the canonical name embeds the
    // sequence, so a shared spelling cannot fit both — the family is
    // named through the construction, and the duplicate guard is the
    // belt behind it.
    let mut doc = ok();
    let shared = doc["events"][0]["snapshot"].clone();
    doc["events"][1]["snapshot"] = shared;
    assert_eq!(
        check(doc)
            .expect_err("two events claiming one snapshot")
            .law(),
        "snapshot-portability"
    );
}

/// The short form is a legal spelling of a name that would have fitted,
/// because the pressure that forces it — the absolute run directory
/// against Windows MAX_PATH — is invisible from inside the index.
#[test]
fn the_short_snapshot_form_is_green_even_when_the_full_one_would_fit() {
    let mut doc = ok();
    doc["events"][0]["snapshot"] = json!("0000-~28b4b51b8d841175-000.json");
    check(doc).expect("a writer may shorten a name that would have fitted");
}

/// `root-coherence` — the root's word matches what the run did.
#[test]
fn root_contradictions_are_red() {
    let mut doc = ok();
    doc["failure"] = json!("a failure riding an ok run");
    let error = check(doc).expect_err("failure outside a failed run");
    assert!(matches!(
        error,
        TraceIndexError::FailureOutsideFailedRun { .. }
    ));
    assert_eq!(error.law(), "root-coherence");

    let mut doc = ok();
    remove(&mut doc, "", "finished");
    assert!(matches!(
        check(doc).expect_err("ok without finished"),
        TraceIndexError::TerminalWithoutFinished { .. }
    ));

    let mut doc = failed();
    remove(&mut doc, "", "failure");
    assert!(matches!(
        check(doc).expect_err("failed without failure text"),
        TraceIndexError::FailedRunWithoutFailure
    ));

    let mut doc = ok();
    doc["scopes"][0]["status"] = json!("pending");
    remove(&mut doc, "/scopes/0", "fingerprint");
    assert!(matches!(
        check(doc).expect_err("ok with a pending scope"),
        TraceIndexError::OkWithPendingScope { .. }
    ));

    let mut doc = ok();
    doc["scopes"][0]["status"] = json!("failed");
    remove(&mut doc, "/scopes/0", "fingerprint");
    doc["scopes"][0]["failure"] = json!("hidden by root ok");
    assert!(matches!(
        check(doc).expect_err("scope failure hidden by root ok"),
        TraceIndexError::OkWithFailedScope { .. }
    ));

    // A COMPILE-failed event hidden by root ok, with the event matrix
    // and the aggregates kept coherent so the named family is
    // root-coherence.
    for status in ["pass-failed", "verification-failed"] {
        let mut doc = ok();
        doc["events"][0]["status"] = json!(status);
        remove(&mut doc, "/events/0", "snapshot");
        remove(&mut doc, "/events/0", "encode_micros");
        if status == "pass-failed" {
            remove(&mut doc, "/events/0", "verify_micros");
        }
        doc["events"][0]["diagnostic"] = json!("hidden by root ok");
        doc["aggregates"][0]["encode_total"] = json!({ "micros": 700, "saturated": false });
        let error = check(doc).expect_err("a compile failure under root ok");
        assert!(
            matches!(error, TraceIndexError::OkWithFailedEvent { .. }),
            "{status}: {error}"
        );
    }
}

/// The trace is an OBSERVER of the compile, and an observer must not be
/// able to change what it observes. A `snapshot-failed` event — the
/// encoder could not write the file — is not a compile failure, so root
/// `ok` admits it; the same for a budget stand-down. If it did not,
/// turning on `--trace-compile` could turn a green run red, which is
/// precisely the property a diagnostic switch must not have.
#[test]
fn a_trace_side_failure_does_not_fail_an_ok_run() {
    check(corpus("ok_with_snapshot_failed.json"))
        .expect("a snapshot encode failure leaves the run ok");

    // The budget stand-down, mutated onto the complete run.
    let mut doc = ok();
    doc["events"][1]["status"] = json!("snapshot-skipped-budget");
    remove(&mut doc, "/events/1", "snapshot");
    remove(&mut doc, "/events/1", "encode_micros");
    doc["aggregates"][0]["encode_total"] = json!({ "micros": 800, "saturated": false });
    check(doc).expect("a budget stand-down leaves the run ok");
}

/// A root `failed` is the compile/lifecycle outcome, and it can be
/// reached AFTER every pass succeeded — the boot-artifact transaction
/// rolling back a StaticWrite is the accepted case. So the law asks for
/// `failed` + `finished` + a bounded `failure`, and NOT for a failed
/// scope or a failed event as corroboration.
#[test]
fn a_failed_run_needs_no_failed_scope_or_event() {
    check(corpus("failed_after_successful_compile.json"))
        .expect("a run may fail after every pass succeeded");

    // The same shape reached by mutation: heal both failures in the
    // partial document and keep only the root's word.
    let mut doc = failed();
    doc["scopes"][1]["status"] = json!("compiled");
    doc["scopes"][1]["fingerprint"] =
        json!("sha256:6666666666666666666666666666666666666666666666666666666666666666");
    remove(&mut doc, "/scopes/1", "failure");
    doc["events"][1]["status"] = json!("ok");
    remove(&mut doc, "/events/1", "diagnostic");
    doc["events"][1]["verify_micros"] = json!({ "micros": 40, "saturated": false });
    doc["events"][1]["encode_micros"] = json!({ "micros": 90, "saturated": false });
    doc["events"][1]["snapshot"] = json!("0001-close-unit_org.demo.tool_static%2Dxml-000.json");
    doc["aggregates"][1]["verify_total"] = json!({ "micros": 40, "saturated": false });
    doc["aggregates"][1]["encode_total"] = json!({ "micros": 90, "saturated": false });
    check(doc).expect("a failed root with nothing else failed is legal");
}

/// `aggregate-reconciliation` — one row per pass, counts exact, totals
/// recomputed. Includes the packet's mandated drop-one-parse-event
/// mutation: dropping the event AND renumbering the tail (so density,
/// snapshots and keys all stay coherent) leaves ONLY the aggregates
/// lying — and that is where it goes red.
#[test]
fn aggregate_lies_are_red() {
    let mut doc = ok();
    let events = doc["events"].as_array_mut().unwrap();
    events.remove(1);
    let tail = events.last_mut().unwrap();
    tail["sequence"] = json!(1);
    tail["snapshot"] = json!("0001-emit%3Astatic%2Dxml-unit_org.demo.tool_static%2Dxml-000.json");
    let error = check(doc).expect_err("dropped parse event, unchanged aggregates");
    assert!(matches!(
        error,
        TraceIndexError::AggregateCountMismatch { .. }
    ));
    assert_eq!(error.law(), "aggregate-reconciliation");

    let mut doc = ok();
    doc["aggregates"][0]["pass_total"] = json!({ "micros": 2101, "saturated": false });
    assert!(matches!(
        check(doc).expect_err("a carried total lies"),
        TraceIndexError::AggregateDurationMismatch { .. }
    ));

    let mut doc = ok();
    doc["aggregates"].as_array_mut().unwrap().remove(0);
    assert!(matches!(
        check(doc).expect_err("a pass lost its row"),
        TraceIndexError::AggregateRowMissing { .. }
    ));

    let mut doc = ok();
    doc["aggregates"][0]["pass"] = json!("gather");
    assert!(matches!(
        check(doc).expect_err("a row for a pass that never ran"),
        TraceIndexError::AggregateRowUnknown { .. }
    ));

    let mut doc = ok();
    let row = doc["aggregates"][1].clone();
    doc["aggregates"].as_array_mut().unwrap().push(row);
    assert!(matches!(
        check(doc).expect_err("one pass, two rows"),
        TraceIndexError::AggregateRowDuplicate { .. }
    ));

    // Saturation is recomputed, never trusted: an exact ceiling
    // measurement plus an already-saturated one overflows, and the flag
    // sticks without the sum ever wrapping to a small number.
    let mut doc = ok();
    doc["events"][0]["pass_micros"] = json!({ "micros": u32::MAX, "saturated": false });
    doc["events"][1]["pass_micros"] = json!({ "micros": u32::MAX, "saturated": true });
    let error = check(doc).expect_err("saturating totals must be recomputed");
    assert!(matches!(
        error,
        TraceIndexError::AggregateDurationMismatch { ref recomputed, .. }
            if recomputed.micros == u32::MAX && recomputed.saturated
    ));

    // …and the honest saturated row is green.
    let mut doc = ok();
    doc["events"][0]["pass_micros"] = json!({ "micros": u32::MAX, "saturated": false });
    doc["events"][1]["pass_micros"] = json!({ "micros": u32::MAX, "saturated": true });
    doc["aggregates"][0]["pass_total"] = json!({ "micros": u32::MAX, "saturated": true });
    check(doc).expect("an honestly saturated total reconciles");
}

/// `aggregate-reconciliation` — the CLI table is a diffable artifact:
/// the same rows in a different order would make two runs of one
/// compile print two different tables, so order is a law, not a habit.
#[test]
fn permuted_aggregate_rows_are_red() {
    let mut doc = ok();
    let rows = doc["aggregates"].as_array_mut().unwrap();
    rows.swap(0, 1);
    let error = check(doc).expect_err("the same rows in a different order");
    assert!(matches!(
        error,
        TraceIndexError::AggregateRowOutOfOrder { position: 0, .. }
    ));
    assert_eq!(error.law(), "aggregate-reconciliation");
}

/// `diagnostic-cap` — every failure/diagnostic text is bounded; the cap
/// itself is inclusive.
#[test]
fn over_cap_diagnostics_are_red() {
    let mut doc = failed();
    doc["failure"] = json!("x".repeat(8193));
    let error = check(doc).expect_err("root failure over the cap");
    assert!(matches!(
        error,
        TraceIndexError::DiagnosticOverCap { bytes: 8193, .. }
    ));
    assert_eq!(error.law(), "diagnostic-cap");

    let mut doc = failed();
    doc["scopes"][1]["failure"] = json!("x".repeat(8193));
    assert!(matches!(
        check(doc).expect_err("scope failure over the cap"),
        TraceIndexError::DiagnosticOverCap { .. }
    ));

    let mut doc = failed();
    doc["events"][1]["diagnostic"] = json!("x".repeat(8193));
    assert!(matches!(
        check(doc).expect_err("event diagnostic over the cap"),
        TraceIndexError::DiagnosticOverCap { .. }
    ));

    // Exactly the cap is legal — the bound is inclusive.
    let mut doc = failed();
    doc["failure"] = json!("x".repeat(8192));
    doc["scopes"][1]["failure"] = json!("x".repeat(8192));
    doc["events"][1]["diagnostic"] = json!("x".repeat(8192));
    check(doc).expect("a diagnostic at the cap is green");
}
