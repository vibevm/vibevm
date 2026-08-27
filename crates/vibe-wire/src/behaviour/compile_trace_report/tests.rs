//! RED arms for every relational law of the command-report trace
//! member, plus the positives that keep them honest. Each law has at
//! least one arm; the arms are minimal mutations of one legal base
//! value, so a refusal names the law, not a fixture's accident.

use crate::behaviour::compile_trace_report::{TraceReportError, validate};
use crate::generated::shared::{CompileTraceReport, Duration, TimingRow, TraceReportStatus};

const RUN_ID: &str = "00112233445566778899aabbccddeeff";
const RUN_PATH: &str = "C:/work/demo/.vibe/trace/00112233445566778899aabbccddeeff";

fn duration(micros: u32, saturated: bool) -> Duration {
    Duration { micros, saturated }
}

fn timing(pass: &str) -> TimingRow {
    TimingRow {
        pass: pass.to_string(),
        invocations: 1,
        pass_total: duration(10, false),
        verify_total: duration(0, false),
        encode_total: duration(0, false),
    }
}

/// One legal `ok` member: finalised, pathed, counted, one timing row.
fn base() -> CompileTraceReport {
    CompileTraceReport {
        status: TraceReportStatus::Ok,
        run_id: RUN_ID.to_string(),
        run_path: Some(RUN_PATH.to_string()),
        finalised: true,
        budget_exhausted: false,
        events: "3".to_string(),
        snapshots: "3".to_string(),
        snapshot_bytes: "4096".to_string(),
        timings: vec![timing("parse"), timing("emit:static-xml")],
        warnings: Vec::new(),
    }
}

/// One legal `unavailable` member: no path, zero counts, one reason.
fn unavailable() -> CompileTraceReport {
    CompileTraceReport {
        status: TraceReportStatus::Unavailable,
        run_id: RUN_ID.to_string(),
        run_path: None,
        finalised: false,
        budget_exhausted: false,
        events: "0".to_string(),
        snapshots: "0".to_string(),
        snapshot_bytes: "0".to_string(),
        timings: Vec::new(),
        warnings: vec!["trace open refused: .vibe is not writable".to_string()],
    }
}

fn law_of(error: TraceReportError) -> &'static str {
    error.law()
}

#[test]
fn the_legal_statuses_all_validate() {
    validate(&base()).unwrap();
    validate(&unavailable()).unwrap();

    let mut running = base();
    running.status = TraceReportStatus::Running;
    running.finalised = false;
    validate(&running).unwrap();

    let mut failed = base();
    failed.status = TraceReportStatus::Failed;
    failed.warnings = vec!["final index write failed".to_string()];
    validate(&failed).unwrap();
}

#[test]
fn canonical_boundaries_stay_legal() {
    // Zero, a u64-sized count, and a count past u64::MAX are all legal
    // canonical decimals — the string carries what no machine integer
    // could. The durations exercise both legal ceiling spellings.
    let mut report = base();
    report.events = "18446744073709551617".to_string();
    report.snapshots = "18446744073709551616".to_string();
    report.snapshot_bytes = "0".to_string();
    report.timings = vec![TimingRow {
        pass: "parse".to_string(),
        invocations: u32::MAX,
        pass_total: duration(u32::MAX, false),
        verify_total: duration(u32::MAX, true),
        encode_total: duration(0, false),
    }];
    validate(&report).unwrap();
}

#[test]
fn a_non_hex_run_id_is_refused() {
    let mut report = base();
    for id in [
        "NOT-HEX",
        // Exactly 31 and exactly 33 — the LENGTH half of the law.
        "00112233445566778899aabbccddeef",
        "00112233445566778899aabbccddeeff0",
        // Exactly 32 and hex, but UPPERCASE — the case half, proven
        // independently of length: a length-only gate would accept
        // this, and two spellings of one id are two ids to a reader.
        "00112233445566778899AABBCCDDEEFF",
        // Exactly 32, all alphabetic, none of it hex — the charset
        // half, again at the legal length.
        "gggggggggggggggggggggggggggggggg",
    ] {
        report.run_id = id.to_string();
        assert_eq!(
            law_of(validate(&report).unwrap_err()),
            "run-id",
            "{id} must land in the run-id law"
        );
    }
    // …and the run path that WOULD match the uppercase spelling does
    // not rescue it: the id is refused before any path is read.
    report.run_id = "00112233445566778899AABBCCDDEEFF".to_string();
    report.run_path = Some("C:/work/demo/.vibe/trace/00112233445566778899AABBCCDDEEFF".to_string());
    assert!(matches!(
        validate(&report).unwrap_err(),
        TraceReportError::RunIdNotLowercaseHex { .. }
    ));
}

#[test]
fn noncanonical_counts_are_refused() {
    for (field, value) in [
        ("events", "01"),
        ("events", ""),
        ("events", "1a"),
        ("events", "+1"),
        ("events", "1.0"),
        ("snapshots", " 1"),
        ("snapshot_bytes", "٠١"),
    ] {
        let mut report = base();
        match field {
            "events" => report.events = value.to_string(),
            "snapshots" => report.snapshots = value.to_string(),
            _ => report.snapshot_bytes = value.to_string(),
        }
        let error = validate(&report).unwrap_err();
        assert_eq!(
            law_of(error.clone()),
            "canonical-counts",
            "{field} = {value:?} must land in the canonical-counts law"
        );
        // Bound under a DIFFERENT name: a pattern binding called `field`
        // shadows the loop's, and `field == field` is then a tautology
        // that passes for every member the refusal could have named.
        assert!(
            matches!(
                error,
                TraceReportError::NonCanonicalCount { field: carried, .. } if carried == field
            ),
            "{field} = {value:?} must name {field} as the offending member"
        );
    }
}

#[test]
fn an_unsafe_run_path_is_refused_by_reason() {
    for path in [
        "C:\\work\\demo\\.vibe\\trace\\00112233445566778899aabbccddeeff",
        "work/demo/.vibe/trace/00112233445566778899aabbccddeeff",
        "C:/work/demo/.vibe/trace/00112233445566778899aabbccddeeff\n",
    ] {
        let mut report = base();
        report.run_path = Some(path.to_string());
        let error = validate(&report).unwrap_err();
        assert_eq!(law_of(error.clone()), "run-path", "{path:?}");
        assert!(
            matches!(error, TraceReportError::UnsafeRunPath { .. }),
            "{path:?} must name its spelling reason"
        );
    }
}

#[test]
fn a_run_path_for_another_run_is_refused() {
    let mut report = base();
    report.run_path = Some("C:/work/demo/.vibe/trace/ffeeddccbbaa99887766554433221100".to_string());
    let error = validate(&report).unwrap_err();
    assert_eq!(law_of(error.clone()), "run-path");
    assert!(matches!(error, TraceReportError::RunPathSuffix { .. }));
}

#[test]
fn an_unavailable_trace_carries_no_path_no_final_and_zero_everything() {
    let path = RUN_PATH.to_string();
    let mut with_path = unavailable();
    with_path.run_path = Some(path);
    assert_eq!(law_of(validate(&with_path).unwrap_err()), "status-matrix");

    let mut finalised = unavailable();
    finalised.finalised = true;
    assert_eq!(law_of(validate(&finalised).unwrap_err()), "status-matrix");

    for (field, value) in [("events", "1"), ("snapshots", "1"), ("snapshot_bytes", "1")] {
        let mut report = unavailable();
        match field {
            "events" => report.events = value.to_string(),
            "snapshots" => report.snapshots = value.to_string(),
            _ => report.snapshot_bytes = value.to_string(),
        }
        let error = validate(&report).unwrap_err();
        assert_eq!(
            law_of(error.clone()),
            "status-matrix",
            "{field} = {value} while unavailable"
        );
        assert!(matches!(
            error,
            TraceReportError::UnavailableNonZero { field: f, .. } if f == field
        ));
    }

    let mut with_timings = unavailable();
    with_timings.timings = vec![timing("parse")];
    assert_eq!(
        law_of(validate(&with_timings).unwrap_err()),
        "status-matrix"
    );

    let mut silent = unavailable();
    silent.warnings = Vec::new();
    assert_eq!(law_of(validate(&silent).unwrap_err()), "status-matrix");
    assert!(matches!(
        validate(&silent).unwrap_err(),
        TraceReportError::UnavailableSilent
    ));

    let mut budgeted = unavailable();
    budgeted.budget_exhausted = true;
    let error = validate(&budgeted).unwrap_err();
    assert_eq!(law_of(error.clone()), "status-matrix");
    assert!(
        matches!(error, TraceReportError::UnavailableBudgetExhausted),
        "a recorder that never opened never owned a snapshot budget"
    );
}

/// A nonempty warnings vector whose every entry is blank satisfies the
/// LENGTH of the reason law and none of its meaning — the exact shape a
/// writer produces by pushing an unformatted error.
#[test]
fn an_unavailable_trace_whose_warnings_are_all_blank_is_refused() {
    for blanks in [
        vec![String::new()],
        vec!["   ".to_string()],
        vec!["\t\n".to_string()],
        vec![String::new(), "  ".to_string(), "\u{a0}".to_string()],
    ] {
        let count = blanks.len();
        let mut report = unavailable();
        report.warnings = blanks.clone();
        let error = validate(&report).unwrap_err();
        assert_eq!(law_of(error.clone()), "status-matrix", "{blanks:?}");
        assert!(
            matches!(
                error,
                TraceReportError::UnavailableBlankReason { warnings } if warnings == count
            ),
            "{blanks:?} must land in the blank-reason law naming all {count} blank(s)"
        );
    }

    // ONE nonblank entry is a reason, whatever blanks ride beside it.
    let mut report = unavailable();
    report.warnings = vec![
        String::new(),
        "trace open refused: .vibe is read-only".to_string(),
    ];
    validate(&report).unwrap();

    // The nonblank rule is the UNAVAILABLE reason law and stops there:
    // an active run's blank warning keeps the one existing sanitisation
    // vocabulary — the byte cap — and is not a second refusal family.
    let mut active = base();
    active.warnings = vec![String::new(), "   ".to_string()];
    validate(&active).unwrap();
}

/// Presence of `run_path` is the status matrix's law in BOTH directions:
/// `unavailable` carries none (above), and every active status carries
/// one. A report that claims a live trace while naming no directory is
/// unusable to the reader the member exists for.
#[test]
fn every_active_status_must_name_its_run_directory() {
    for (status, finalised) in [
        (TraceReportStatus::Running, false),
        (TraceReportStatus::Ok, true),
        (TraceReportStatus::Failed, true),
    ] {
        let mut report = base();
        report.status = status.clone();
        report.finalised = finalised;
        report.run_path = None;
        let error = validate(&report).unwrap_err();
        assert_eq!(law_of(error.clone()), "status-matrix", "{status:?}");
        let TraceReportError::ActiveWithoutRunPath { status: carried } = error else {
            panic!("{status:?} without a path must land in ActiveWithoutRunPath");
        };
        assert_eq!(carried, status, "the refusal names the offending status");

        // Restoring the path validates — the mutation is minimal, so
        // the refusal is the missing member and nothing else.
        report.run_path = Some(RUN_PATH.to_string());
        validate(&report).unwrap();
    }
}

#[test]
fn the_finalised_matrix_is_enforced_for_every_status() {
    let mut running = base();
    running.status = TraceReportStatus::Running;
    running.finalised = true;
    assert_eq!(law_of(validate(&running).unwrap_err()), "status-matrix");
    assert!(matches!(
        validate(&running).unwrap_err(),
        TraceReportError::RunningFinalised
    ));

    for status in [TraceReportStatus::Ok, TraceReportStatus::Failed] {
        let mut terminal = base();
        terminal.status = status.clone();
        terminal.finalised = false;
        let error = validate(&terminal).unwrap_err();
        assert_eq!(law_of(error.clone()), "status-matrix");
        let TraceReportError::TerminalNotFinalised { status: carried } = error else {
            panic!("a terminal status must land in TerminalNotFinalised");
        };
        assert_eq!(carried, status);
    }
}

#[test]
fn snapshots_may_not_exceed_events() {
    let mut report = base();
    report.events = "2".to_string();
    report.snapshots = "3".to_string();
    let error = validate(&report).unwrap_err();
    assert_eq!(law_of(error.clone()), "count-coherence");
    assert!(matches!(
        error,
        TraceReportError::SnapshotsExceedEvents { .. }
    ));

    // Equal and less are both legal, including the past-u64 compare.
    report.snapshots = "2".to_string();
    validate(&report).unwrap();
    report.events = "18446744073709551617".to_string();
    report.snapshots = "18446744073709551616".to_string();
    validate(&report).unwrap();
    report.snapshots = "18446744073709551618".to_string();
    assert_eq!(law_of(validate(&report).unwrap_err()), "count-coherence");
}

#[test]
fn an_overlong_warning_is_refused_at_the_shared_cap() {
    let cap = crate::behaviour::compiler_trace_index::DIAGNOSTIC_CAP_BYTES;
    let mut report = base();
    report.warnings = vec!["x".repeat(cap)];
    validate(&report).unwrap();

    report.warnings = vec!["x".repeat(cap + 1)];
    let error = validate(&report).unwrap_err();
    assert_eq!(law_of(error.clone()), "warning-cap");
    assert!(matches!(
        error,
        TraceReportError::WarningOverCap { index: 0, bytes } if bytes == cap + 1
    ));
}

#[test]
fn timing_rows_need_unique_nonblank_passes_and_canonical_durations() {
    let mut blank = base();
    blank.timings = vec![timing("  ")];
    let error = validate(&blank).unwrap_err();
    assert_eq!(law_of(error.clone()), "timing-rows");
    assert!(matches!(error, TraceReportError::TimingPassUnsafe { .. }));

    let mut control = base();
    control.timings = vec![timing("par\nse")];
    assert!(matches!(
        validate(&control).unwrap_err(),
        TraceReportError::TimingPassUnsafe { .. }
    ));

    let mut duplicate = base();
    duplicate.timings = vec![timing("parse"), timing("parse")];
    let error = validate(&duplicate).unwrap_err();
    assert_eq!(law_of(error.clone()), "timing-rows");
    assert!(matches!(
        error,
        TraceReportError::TimingPassDuplicate { row: 1, .. }
    ));

    let mut noncanonical = base();
    noncanonical.timings = vec![TimingRow {
        pass: "parse".to_string(),
        invocations: 1,
        pass_total: duration(5, true),
        verify_total: duration(0, false),
        encode_total: duration(0, false),
    }];
    for column in ["pass_total", "verify_total", "encode_total"] {
        let mut row = TimingRow {
            pass: "parse".to_string(),
            invocations: 1,
            pass_total: duration(0, false),
            verify_total: duration(0, false),
            encode_total: duration(0, false),
        };
        match column {
            "pass_total" => row.pass_total = duration(5, true),
            "verify_total" => row.verify_total = duration(5, true),
            _ => row.encode_total = duration(5, true),
        }
        noncanonical.timings = vec![row];
        let error = validate(&noncanonical).unwrap_err();
        assert_eq!(law_of(error.clone()), "timing-rows", "{column}");
        assert!(matches!(
            error,
            TraceReportError::NonCanonicalDuration { column: c, .. } if c == column
        ));
    }
}

#[test]
fn refusals_preview_scalars_without_cloning_them() {
    let mut report = base();
    let huge = "x".repeat(10_000);
    report.run_id = huge;
    let error = validate(&report).unwrap_err();
    let TraceReportError::RunIdNotLowercaseHex { run_id } = error else {
        panic!("a huge non-hex run id must land in the run-id law");
    };
    assert_eq!(run_id.bytes(), 10_000);
    assert!(run_id.is_truncated());
    assert!(run_id.head().len() <= 64);
}
