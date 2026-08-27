//! The shared command-report trace member's relational laws — the
//! hand-written validation cell beside the ONE generated
//! [`CompileTraceReport`] every command report root carries
//! (install, lifecycle, update, reinstall; PROP-054 `##OBS-TRACE`,
//! R3.4 §5.4 of the implementation architecture).
//!
//! JTD owns the FORM (the closed status vocabulary, the optional
//! `run_path`, the explicit-even-when-empty lists); the laws a form
//! cannot say are named in the `compile_trace_report` fragment's
//! `metadata.x-relational-laws` (`formats/vocabularies.json`) and
//! enforced HERE, in one pure pass over the generated type with typed
//! errors. The two label sets are pinned equal by
//! `tests/compile_trace_report_wire.rs`, so an undocumented law and an
//! unimplemented label are both red (the same seam the trace index
//! carries).
//!
//! Nothing here is a second algorithm for anything the index already
//! owns: the canonical-duration rule and the 32-hex run-id rule are
//! REUSED from [`crate::behaviour::compiler_trace_index`], and the
//! timing totals stay the index's — this record carries rows, it never
//! recomputes them. The report builder validates the shared value once
//! before each root serialisation; the validator is never copied into
//! the four roots.
//!
//! Every value it reads is untrusted — a report is a file on disk (or
//! a provider's stdout), so no refusal clones the offending scalar:
//! errors carry a bounded [`ScalarPreview`] and the true byte length.

use std::cmp::Ordering;
use std::collections::BTreeSet;

use crate::behaviour::compiler_trace_index::{
    DIAGNOSTIC_CAP_BYTES, is_canonical, is_lowercase_hex,
};
use crate::generated::shared::{CompileTraceReport, TraceReportStatus};

mod errors;

pub use errors::{RunPathUnsafety, TraceReportError};

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

/// Every implemented law label, in fragment order. Set-equal to the
/// `compile_trace_report` fragment's `x-relational-laws` prefixes by the
/// wire test: a law the validator enforces but the fragment does not
/// name, and a label the fragment names but no code answers, are both
/// red.
pub const IMPLEMENTED_LAWS: &[&str] = &[
    "run-id",
    "canonical-counts",
    "run-path",
    "status-matrix",
    "count-coherence",
    "warning-cap",
    "timing-rows",
];

/// The run-directory suffix every legal `run_path` ends with — the
/// trace directory spelling `.vibe/trace/<run_id>` the writer owns.
const RUN_DIR_SUFFIX: &str = ".vibe/trace/";

/// Validate one report trace member against every relational law.
/// Pure: the value in, the first broken law out.
pub fn validate(report: &CompileTraceReport) -> Result<(), TraceReportError> {
    if !is_lowercase_hex(&report.run_id, 32) {
        return Err(TraceReportError::RunIdNotLowercaseHex {
            run_id: preview(&report.run_id),
        });
    }
    for (field, value) in [
        ("events", &report.events),
        ("snapshots", &report.snapshots),
        ("snapshot_bytes", &report.snapshot_bytes),
    ] {
        canonical_decimal(field, value)?;
    }
    path_gate(report)?;
    status_gate(report)?;
    warnings_gate(report)?;
    timings_gate(report)
}

/// One bounded preview — the same refusal discipline the trace index
/// cell uses, applied through its shared type.
fn preview(value: &str) -> crate::behaviour::compiler_trace_index::ScalarPreview {
    crate::behaviour::compiler_trace_index::ScalarPreview::of(value)
}

/// `canonical-counts` on one count member: nonempty, ASCII digits only,
/// and no leading zero unless the whole value is `0`. JTD has no
/// uint64, so the count rides a string both ways; a non-canonical
/// spelling would smuggle a narrowing or a locale-dependent render in
/// through the one member meant to be lossless.
fn canonical_decimal(field: &'static str, value: &str) -> Result<(), TraceReportError> {
    let digits = !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit());
    let canonical = digits && (value.len() == 1 || !value.starts_with('0'));
    if canonical {
        Ok(())
    } else {
        Err(TraceReportError::NonCanonicalCount {
            field,
            value: preview(value),
        })
    }
}

/// `run-path` on the optional absolute path: forward-slashed, absolute,
/// control-free, and ending in the run directory the writer owns. This
/// law owns only HOW a present path is spelled; WHEN one exists is the
/// status matrix's, enforced in [`status_gate`] from both directions.
fn path_gate(report: &CompileTraceReport) -> Result<(), TraceReportError> {
    let Some(path) = report.run_path.as_deref() else {
        return Ok(());
    };
    let bytes = path.as_bytes();
    if bytes.contains(&b'\\') {
        return Err(TraceReportError::UnsafeRunPath {
            path: preview(path),
            reason: RunPathUnsafety::Backslash,
        });
    }
    if path
        .bytes()
        .any(|byte| matches!(byte, b'\r' | b'\n' | b'\0'))
    {
        return Err(TraceReportError::UnsafeRunPath {
            path: preview(path),
            reason: RunPathUnsafety::ControlByte,
        });
    }
    let absolute = path.starts_with('/')
        || (bytes.len() >= 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && bytes[2] == b'/');
    if !absolute {
        return Err(TraceReportError::UnsafeRunPath {
            path: preview(path),
            reason: RunPathUnsafety::NotAbsolute,
        });
    }
    let suffix = format!("{RUN_DIR_SUFFIX}{}", report.run_id);
    if !path.ends_with(&suffix) {
        return Err(TraceReportError::RunPathSuffix {
            path: preview(path),
            run_id: preview(&report.run_id),
        });
    }
    Ok(())
}

/// `status-matrix` (the unavailable/active member laws, INCLUDING when a
/// run path exists) plus `count-coherence` (`snapshots` ≤ `events`).
fn status_gate(report: &CompileTraceReport) -> Result<(), TraceReportError> {
    match report.status {
        TraceReportStatus::Unavailable => unavailable_gate(report)?,
        TraceReportStatus::Running => {
            require_run_path(report)?;
            if report.finalised {
                return Err(TraceReportError::RunningFinalised);
            }
        }
        TraceReportStatus::Ok | TraceReportStatus::Failed => {
            require_run_path(report)?;
            if !report.finalised {
                return Err(TraceReportError::TerminalNotFinalised {
                    status: report.status.clone(),
                });
            }
        }
    }
    if !at_most(&report.events, &report.snapshots) {
        return Err(TraceReportError::SnapshotsExceedEvents {
            events: preview(&report.events),
            snapshots: preview(&report.snapshots),
        });
    }
    Ok(())
}

/// The ACTIVE half of the presence law. The vocabulary says `run_path`
/// is absent EXACTLY for `unavailable`; enforcing only "unavailable
/// carries none" would let a `running`, `ok` or `failed` report claim a
/// trace while naming no directory a reader could open — the one member
/// that makes the record actionable, silently optional.
fn require_run_path(report: &CompileTraceReport) -> Result<(), TraceReportError> {
    if report.run_path.is_some() {
        Ok(())
    } else {
        Err(TraceReportError::ActiveWithoutRunPath {
            status: report.status.clone(),
        })
    }
}

/// The `unavailable` half: a recorder that never opened owns no
/// directory, no terminal state, no snapshot budget, no counts and no
/// rows — and it must say WHY. A vector of blanks is not a diagnostic,
/// so the reason law reads the TEXT, not the length. The rule is scoped
/// to this arm on purpose: an active run's warnings keep the one
/// existing sanitisation vocabulary (the byte cap), and this correction
/// does not invent a second one for them.
fn unavailable_gate(report: &CompileTraceReport) -> Result<(), TraceReportError> {
    if report.run_path.is_some() {
        return Err(TraceReportError::UnavailableWithRunPath);
    }
    if report.finalised {
        return Err(TraceReportError::UnavailableFinalised);
    }
    if report.budget_exhausted {
        return Err(TraceReportError::UnavailableBudgetExhausted);
    }
    for (field, value) in [
        ("events", &report.events),
        ("snapshots", &report.snapshots),
        ("snapshot_bytes", &report.snapshot_bytes),
    ] {
        if value != "0" {
            return Err(TraceReportError::UnavailableNonZero {
                field,
                carried: preview(value),
            });
        }
    }
    if !report.timings.is_empty() {
        return Err(TraceReportError::UnavailableWithTimings);
    }
    if report.warnings.is_empty() {
        return Err(TraceReportError::UnavailableSilent);
    }
    if !report
        .warnings
        .iter()
        .any(|warning| !warning.trim().is_empty())
    {
        return Err(TraceReportError::UnavailableBlankReason {
            warnings: report.warnings.len(),
        });
    }
    Ok(())
}

/// Whether `snapshots ≤ events` holds for two canonical decimal
/// strings: length first, then lexicographic over equal lengths — which
/// over ASCII digits IS numeric order. No machine integer is involved;
/// a count past `u64::MAX` compares correctly rather than refusing or
/// wrapping.
fn at_most(events: &str, snapshots: &str) -> bool {
    match snapshots.len().cmp(&events.len()) {
        Ordering::Less => true,
        Ordering::Greater => false,
        Ordering::Equal => snapshots <= events,
    }
}

/// `warning-cap`: every warning text obeys the shared diagnostic cap.
fn warnings_gate(report: &CompileTraceReport) -> Result<(), TraceReportError> {
    for (index, warning) in report.warnings.iter().enumerate() {
        let bytes = warning.len();
        if bytes > DIAGNOSTIC_CAP_BYTES {
            return Err(TraceReportError::WarningOverCap { index, bytes });
        }
    }
    Ok(())
}

/// `timing-rows`: unique, non-blank pass names and canonical durations
/// through the index's own rule. The rows themselves are the index's
/// aggregates carried verbatim; this record never recomputes them.
fn timings_gate(report: &CompileTraceReport) -> Result<(), TraceReportError> {
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for (row, timing) in report.timings.iter().enumerate() {
        let pass = timing.pass.as_str();
        if pass.trim().is_empty() || pass.bytes().any(|b| matches!(b, b'\r' | b'\n' | b'\0')) {
            return Err(TraceReportError::TimingPassUnsafe {
                row,
                pass: preview(pass),
            });
        }
        if !seen.insert(pass) {
            return Err(TraceReportError::TimingPassDuplicate {
                row,
                pass: preview(pass),
            });
        }
        for (column, duration) in [
            ("pass_total", &timing.pass_total),
            ("verify_total", &timing.verify_total),
            ("encode_total", &timing.encode_total),
        ] {
            if !is_canonical(duration) {
                return Err(TraceReportError::NonCanonicalDuration {
                    row,
                    pass: preview(pass),
                    column,
                });
            }
        }
    }
    Ok(())
}
