//! The human forms of `vibe transfers` (PROP-068 `#verbs`, PROP-067
//! `#list`): a summary line, a rule per section, one measured row per
//! transfer, and no trailer — a listing reports, it did nothing.

use std::time::Duration;

use vibe_core::progress::transfer::{Outcome, TransferEntry, TransferEvent};

use crate::output;
use crate::output::language::{Cell, List, Meta, Role, Row, bytes};

pub(super) fn listing(
    ctx: &output::Context,
    active: &[TransferEntry],
    history: &[TransferEntry],
    history_only: bool,
) {
    let summary = if history_only {
        format!("{} transfer(s) in the history", history.len())
    } else {
        format!(
            "{} active transfer(s) · {} in the history",
            active.len(),
            history.len()
        )
    };
    ctx.summary(&summary);
    if ctx.is_quiet() {
        return;
    }
    if !active.is_empty() {
        ctx.rule("active");
        ctx.block(&List::new(active.iter().map(active_row).collect()).render());
    }
    if !history.is_empty() {
        ctx.rule("history");
        ctx.block(&List::new(history.iter().map(history_row).collect()).render());
    }
}

/// `package · purpose · 12.4 MB of 40.1 MB · 2.1 MB/s`.
fn active_row(entry: &TransferEntry) -> Row {
    let last = entry.last.as_ref();
    let done = last.map_or(entry.bytes, |sample| sample.bytes);
    let figure = match entry.record.total {
        Some(total) => format!("{} of {}", bytes(done), bytes(total)),
        None => bytes(done),
    };
    let mut meta = vec![Meta::Count(figure)];
    if let Some(sample) = last {
        meta.push(Meta::Note(format!("{}/s", bytes(sample.rate_bps))));
    }
    head(entry).meta(meta)
}

/// `package · purpose · bytes · time · average`, and how it ended when it
/// did not finish.
fn history_row(entry: &TransferEntry) -> Row {
    let mut meta = vec![
        Meta::Count(bytes(entry.bytes)),
        Meta::Time(Duration::from_millis(entry.elapsed_ms.unwrap_or(0))),
        Meta::Note(format!("{}/s", bytes(entry.average_bps.unwrap_or(0)))),
    ];
    match entry.outcome {
        Some(Outcome::Failed) => meta.push(Meta::Alert(
            entry.message.clone().unwrap_or_else(|| "failed".into()),
        )),
        Some(Outcome::Interrupted) => meta.push(Meta::Warn("interrupted".into())),
        _ => {}
    }
    head(entry).meta(meta)
}

/// The package the bytes are for (the URL when there is none) and their
/// purpose.
fn head(entry: &TransferEntry) -> Row {
    let record = &entry.record;
    let what = record.package.clone().unwrap_or_else(|| record.url.clone());
    Row::new(vec![
        Cell::new(what, Role::Accent),
        Cell::new(record.purpose.as_str(), Role::Dim),
    ])
}

/// One watch line for a terminal: `at event id bytes [rate]`.
pub(super) fn event(event: &TransferEvent) -> String {
    let mut line = format!("{} {} {}", event.at, event.event.as_str(), event.id);
    if let Some(count) = event.bytes {
        line.push_str(&format!(" {}", bytes(count)));
    }
    if let Some(rate) = event.rate_bps {
        line.push_str(&format!(" {}/s", bytes(rate)));
    }
    if let Some(message) = &event.message {
        line.push_str(&format!(" {message}"));
    }
    line
}
