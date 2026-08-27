//! The ONE human/quiet presentation of a trace member.
//!
//! Every mode reads the same finished [`CompileTraceReport`] — the value that
//! already rode the wire — and never the live writer. That is the whole design
//! rule here: a second computation of "how many events were there" is a second
//! answer, and the one an operator sees on screen must be the one a machine
//! reader gets from the JSON. So this cell is pure, total and takes no
//! recorder.
//!
//! Two shapes, and the difference is a contract rather than a taste:
//!
//! * **human** — one heading and one aligned table, printed once;
//! * **quiet** — a compact suffix APPENDED to the command's single existing
//!   summary line. Quiet's promise is exactly one line, so a second line (or a
//!   table) would break it for every script that parses it.
//!
//! `unavailable` prints an honest reason and invents no path and no timings.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE");

use vibe_wire::generated::shared::{CompileTraceReport, Duration, TimingRow, TraceReportStatus};

use crate::output;

/// The status word, spelled once for both renderings.
const fn status(report: &CompileTraceReport) -> &'static str {
    match report.status {
        TraceReportStatus::Unavailable => "unavailable",
        TraceReportStatus::Running => "running",
        TraceReportStatus::Ok => "ok",
        TraceReportStatus::Failed => "failed",
    }
}

/// Windows' verbatim prefix, forward-slashed by `machine_json_path`.
///
/// It is stripped for DISPLAY only. The JSON member keeps the exact
/// `machine_json_path` spelling, because that is the string a tool has to
/// `open()` — and on a long path the prefix is the part that makes it work.
/// A human reading `//?/C:/…/.vibe/trace/…` learns nothing from those four
/// characters and misreads the path as a UNC share.
fn display_path(path: &str) -> &str {
    path.strip_prefix("//?/").unwrap_or(path)
}

/// The compact suffix quiet mode appends to its ONE summary line.
///
/// Counts, not timings: a duration on a line a script greps is noise, and the
/// table already exists for anyone who wants microseconds. Warnings are
/// COUNTED rather than quoted — each is bounded individually, but three of
/// them concatenated is not a summary line any more.
pub(crate) fn quiet_suffix(report: &CompileTraceReport) -> String {
    let mut suffix = format!(", compile trace {}", status(report));
    if !matches!(report.status, TraceReportStatus::Unavailable) {
        suffix.push_str(&format!(
            " ({} event(s), {} snapshot(s)",
            report.events, report.snapshots
        ));
        if report.budget_exhausted {
            suffix.push_str(", budget exhausted");
        }
        if !report.finalised {
            suffix.push_str(", not finalised");
        }
        suffix.push(')');
    }
    if !report.warnings.is_empty() {
        suffix.push_str(&format!(", {} trace warning(s)", report.warnings.len()));
    }
    suffix
}

/// One heading, one aligned timing table, every warning once.
pub(crate) fn render_human(ctx: &output::Context, report: &CompileTraceReport) {
    ctx.heading(&format!("\ncompile trace: {}", status(report)));
    ctx.step(&format!("run {}", report.run_id));
    // No path is INVENTED for a trace that never opened; the reasons below say
    // why there is nothing to point at.
    if let Some(path) = report.run_path.as_deref() {
        ctx.step(&format!("path {}", display_path(path)));
    }
    if !matches!(report.status, TraceReportStatus::Unavailable) {
        ctx.step(&format!(
            "{} event(s), {} snapshot(s), {} snapshot byte(s){}{}",
            report.events,
            report.snapshots,
            report.snapshot_bytes,
            if report.budget_exhausted {
                " (budget exhausted)"
            } else {
                ""
            },
            if report.finalised {
                ""
            } else {
                " (index still running)"
            },
        ));
    }
    for line in timing_table(&report.timings) {
        ctx.step(&line);
    }
    for warning in &report.warnings {
        ctx.step(&format!("warning: {warning}"));
    }
}

/// The aligned table, as ready-to-print lines.
///
/// Pure and separately testable on purpose: alignment is the one thing here
/// that can silently rot, and asserting it through captured stdout would test
/// the terminal rather than the layout. An empty row set prints NOTHING — a
/// header over zero rows reads like a lost table.
fn timing_table(rows: &[TimingRow]) -> Vec<String> {
    if rows.is_empty() {
        return Vec::new();
    }
    const HEAD: [&str; 5] = ["pass", "invocations", "pass µs", "verify µs", "encode µs"];
    let cells: Vec<[String; 5]> = rows
        .iter()
        .map(|row| {
            [
                row.pass.clone(),
                row.invocations.to_string(),
                micros(&row.pass_total),
                micros(&row.verify_total),
                micros(&row.encode_total),
            ]
        })
        .collect();
    let mut widths = HEAD.map(str::chars).map(Iterator::count);
    for row in &cells {
        for (width, cell) in widths.iter_mut().zip(row) {
            *width = (*width).max(cell.chars().count());
        }
    }
    let line = |row: &[String; 5]| {
        row.iter()
            .zip(widths)
            .map(|(cell, width)| format!("{cell:<width$}"))
            .collect::<Vec<_>>()
            .join("  ")
            .trim_end()
            .to_string()
    };
    let mut out = vec![line(&HEAD.map(str::to_string))];
    out.extend(cells.iter().map(line));
    out
}

/// One duration cell. A SATURATED sum is marked, not silently printed as if it
/// were exact: the writer already told us the `u32` ran out, and a number that
/// is really "at least this" has to look like one.
fn micros(duration: &Duration) -> String {
    if duration.saturated {
        format!("{}+", duration.micros)
    } else {
        duration.micros.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn duration(micros: u32, saturated: bool) -> Duration {
        Duration { micros, saturated }
    }

    fn report(status: TraceReportStatus) -> CompileTraceReport {
        CompileTraceReport {
            budget_exhausted: false,
            events: "7".into(),
            finalised: matches!(status, TraceReportStatus::Ok | TraceReportStatus::Failed),
            run_id: "0".repeat(32),
            snapshot_bytes: "512".into(),
            snapshots: "2".into(),
            status,
            timings: Vec::new(),
            warnings: Vec::new(),
            run_path: Some("//?/C:/p/.vibe/trace/run".into()),
        }
    }

    #[test]
    fn the_quiet_suffix_stays_one_compact_clause() {
        let suffix = quiet_suffix(&report(TraceReportStatus::Ok));
        assert_eq!(suffix, ", compile trace ok (7 event(s), 2 snapshot(s))");
        assert!(!suffix.contains('\n'), "quiet is exactly one line");
    }

    /// `unavailable` invents no counts to look complete.
    #[test]
    fn an_unavailable_quiet_suffix_reports_no_counts() {
        let mut unavailable = report(TraceReportStatus::Unavailable);
        unavailable.events = "0".into();
        unavailable.snapshots = "0".into();
        unavailable.run_path = None;
        unavailable.warnings = vec!["the project was busy".into()];
        assert_eq!(
            quiet_suffix(&unavailable),
            ", compile trace unavailable, 1 trace warning(s)"
        );
    }

    #[test]
    fn a_running_trace_says_it_is_not_finalised() {
        let running = report(TraceReportStatus::Running);
        assert!(quiet_suffix(&running).contains("not finalised"));
    }

    #[test]
    fn the_verbatim_prefix_is_display_only() {
        assert_eq!(display_path("//?/C:/p/x"), "C:/p/x");
        assert_eq!(display_path("/home/p/x"), "/home/p/x");
    }

    #[test]
    fn the_timing_table_aligns_every_column_and_marks_saturation() {
        let rows = vec![
            TimingRow {
                encode_total: duration(5, false),
                invocations: 4,
                pass: "parse".into(),
                pass_total: duration(120, false),
                verify_total: duration(30, true),
            },
            TimingRow {
                encode_total: duration(1, false),
                invocations: 1,
                pass: "qualify-very-long".into(),
                pass_total: duration(2, false),
                verify_total: duration(3, false),
            },
        ];
        let table = timing_table(&rows);
        assert_eq!(table.len(), 3, "one header plus one line per row");
        let width = table[0].chars().count();
        // Every data line starts its second column at the header's offset.
        for line in &table[1..] {
            assert!(
                line.chars().count() <= width,
                "no row may overflow the header it is aligned to: {line}"
            );
            assert!(line.starts_with(&line.trim_start().chars().next().unwrap().to_string()));
        }
        assert!(table[1].contains("30+"), "saturation is marked: {table:?}");
        assert!(table[0].starts_with("pass"));
    }

    #[test]
    fn no_rows_print_no_header() {
        assert!(timing_table(&[]).is_empty());
    }
}
