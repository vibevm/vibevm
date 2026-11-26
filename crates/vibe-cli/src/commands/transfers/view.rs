//! The human forms of `vibe transfers` (PROP-068 `#verbs`, PROP-067
//! `#list`): a counted rule per section, one measured row per transfer,
//! and no trailer — a listing reports, it did nothing.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-068#verbs");

use std::time::Duration;

use vibe_core::progress::transfer::{Outcome, TransferEntry, TransferEvent, TransferEventKind};

use crate::output;
use crate::output::language::{
    Cell, Glyph, List, Meta, Role, Row, bytes, hint, rule_at, rule_width, viewport,
};

pub(super) fn listing(
    ctx: &output::Context,
    active: &[TransferEntry],
    history: &[TransferEntry],
    history_only: bool,
) {
    if ctx.is_quiet() {
        ctx.summary(&quiet_line(active.len(), history.len(), history_only));
        return;
    }
    ctx.block(&listing_text(
        active,
        history,
        history_only,
        viewport(),
        rule_width(),
    ));
}

fn count(n: usize) -> String {
    format!("{n} {}", if n == 1 { "transfer" } else { "transfers" })
}

/// Counted section rules followed by measured transfer rows, or one empty hint.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#heading")]
pub(super) fn listing_text(
    active: &[TransferEntry],
    history: &[TransferEntry],
    history_only: bool,
    width: usize,
    rule: usize,
) -> String {
    let mut parts = Vec::new();
    if !history_only && !active.is_empty() {
        parts.push(format!(
            "{}\n\n{}",
            rule_at(&format!("active · {}", count(active.len())), None, rule),
            List::new(active.iter().map(active_row).collect()).render_at(width)
        ));
    } else if !history_only && !history.is_empty() {
        parts.push(format!("  {}", hint("no active transfers")));
    }
    if !history.is_empty() {
        parts.push(format!(
            "{}\n\n{}",
            rule_at(&format!("history · {}", count(history.len())), None, rule),
            List::new(history.iter().map(history_row).collect()).render_at(width)
        ));
    }
    if parts.is_empty() {
        return format!(
            "  {}",
            hint(if history_only {
                "no transfers in the history"
            } else {
                "no active transfers and none in the history"
            })
        );
    }
    parts.join("\n\n")
}

/// The compact transfer counts used only in quiet mode.
pub(super) fn quiet_line(active: usize, history: usize, history_only: bool) -> String {
    if history_only {
        format!("{} in the history", count(history))
    } else {
        format!(
            "{active} active {} · {history} in the history",
            if active == 1 { "transfer" } else { "transfers" }
        )
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

/// The transfer identities retained between human watch lines.
#[derive(Default)]
pub(super) struct Watch {
    heads: std::collections::HashMap<String, Head>,
}

struct Head {
    what: String,
    purpose: &'static str,
    total: Option<u64>,
    started: String,
}

impl Watch {
    /// A named glyph row, retaining identity until the transfer ends.
    #[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-068#verbs")]
    pub(super) fn line(&mut self, event: &TransferEvent, width: usize) -> String {
        if let Some(record) = &event.record {
            self.heads.insert(
                event.id.clone(),
                Head {
                    what: record.package.clone().unwrap_or_else(|| record.url.clone()),
                    purpose: record.purpose.as_str(),
                    total: record.total,
                    started: record.started.clone(),
                },
            );
        }
        let head = self.heads.get(&event.id);
        let cells = match head {
            Some(head) => vec![
                Cell::new(&head.what, Role::Accent),
                Cell::new(head.purpose, Role::Dim),
            ],
            None => vec![Cell::new(&event.id, Role::Accent)],
        };
        let total = event.total.or_else(|| head.and_then(|head| head.total));
        let figure = |n| match total {
            Some(total) => format!("{} of {}", bytes(n), bytes(total)),
            None => bytes(n),
        };
        let mut meta = Vec::new();
        let (glyph, role) = match event.event {
            TransferEventKind::Started => {
                meta.push(Meta::Count(figure(event.bytes.unwrap_or(0))));
                let clock = event.at.get(11..19).unwrap_or(&event.at);
                meta.push(Meta::Note(format!("started {clock}")));
                (Glyph::Running, Role::Text)
            }
            TransferEventKind::Sample => {
                meta.push(Meta::Count(figure(event.bytes.unwrap_or(0))));
                if let Some(rate) = event.rate_bps {
                    meta.push(Meta::Note(format!("{}/s", bytes(rate))));
                }
                (Glyph::Running, Role::Text)
            }
            TransferEventKind::Finished => {
                if let Some(n) = event.bytes {
                    meta.push(Meta::Count(bytes(n)));
                }
                if let Some(elapsed) = head.and_then(|head| {
                    let started = chrono::DateTime::parse_from_rfc3339(&head.started).ok()?;
                    let ended = chrono::DateTime::parse_from_rfc3339(&event.at).ok()?;
                    (ended - started).to_std().ok()
                }) {
                    meta.push(Meta::Time(elapsed));
                }
                if let Some(written) = event.disk_written_bytes {
                    meta.push(Meta::Note(format!("{} written", bytes(written))));
                }
                (Glyph::Done, Role::Ok)
            }
            TransferEventKind::Failed | TransferEventKind::Interrupted => {
                if let Some(n) = event.bytes {
                    meta.push(Meta::Count(bytes(n)));
                }
                if event.event == TransferEventKind::Failed {
                    meta.push(Meta::Alert(
                        event.message.clone().unwrap_or_else(|| "failed".into()),
                    ));
                    (Glyph::Failed, Role::Alert)
                } else {
                    meta.push(Meta::Warn("interrupted".into()));
                    (Glyph::Skipped, Role::Dim)
                }
            }
        };
        let line = List::new(vec![Row::new(cells).glyph(glyph, role).meta(meta)]).render_at(width);
        if matches!(
            event.event,
            TransferEventKind::Finished
                | TransferEventKind::Failed
                | TransferEventKind::Interrupted
        ) {
            self.heads.remove(&event.id);
        }
        line
    }
}

#[cfg(test)]
mod tests;
