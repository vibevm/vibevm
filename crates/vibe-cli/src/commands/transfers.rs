//! `vibe transfers` (PROP-068 `#verbs`): the active transfers with their
//! last sample and the recent history in one document, the history alone,
//! a live feed of samples and changes, and emptying the history. Every verb
//! reads the journal under the settings root; none starts or stops a
//! transfer, and none changes an installation.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-068#verbs");

mod view;

use std::io::{Read, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::{Context as _, Result, anyhow, bail};
use serde::Serialize;
use serde::de::DeserializeOwned;
use specmark::spec;
use vibe_app::{ProcessState, ProcessTable};
use vibe_core::progress::transfer::{
    Purpose, TransferEntry, TransferEvent, TransferJournal, watch as watch_journal,
};
use vibe_wire::generated::{transfer_event, transfers_listing};

use super::app::processes::HostProcesses;
use crate::cli::{TransfersArgs, TransfersCommand, TransfersFilter};
use crate::output;

/// The protocol of the listing and of every watch line.
const LISTING_PROTOCOL: &str = "vibe-transfers/1";
const EVENT_PROTOCOL: &str = "vibe-transfer-event/1";
/// How often `watch` reads the journal: the cadence of the samples.
const TICK: Duration = Duration::from_millis(250);
/// History entries a listing keeps when `--since` does not say.
const RECENT: usize = 50;

#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-068#verbs")]
pub fn run(ctx: &output::Context, args: TransfersArgs) -> Result<()> {
    let root = vibe_core::settings::settings_dir().ok_or_else(|| {
        anyhow!("the Vibe settings root is unavailable; set VIBE_SETTINGS or make the user home resolvable")
    })?;
    let journal = TransferJournal::new(&root);
    let home = root.display().to_string();
    match args.command {
        None => list(ctx, &journal, &home, &args.filter, false),
        Some(TransfersCommand::History { filter }) => list(ctx, &journal, &home, &filter, true),
        Some(TransfersCommand::Watch) => watch(ctx, &journal),
        Some(TransfersCommand::ClearHistory) => {
            let cleared = journal.clear_history().with_context(|| {
                format!(
                    "emptying `{}`",
                    journal.root().join("history.jsonl").display()
                )
            })?;
            if ctx.is_json() {
                return list(ctx, &journal, &home, &TransfersFilter::default(), true);
            }
            let unit = if cleared == 1 { "entry" } else { "entries" };
            ctx.trailer(&format!("cleared {cleared} history {unit}"));
            Ok(())
        }
    }
}

/// Whether a process is still there: the liveness check of
/// `PROP-059#launcher-runtime`, shared with `vibe app running`.
fn alive(pid: u32) -> bool {
    !matches!(HostProcesses.probe(pid), ProcessState::Gone)
}

/// A journal entry in a generated wire shape: the two sides spell the
/// same members, so the entry crosses as JSON.
fn wire<T: DeserializeOwned>(entry: &impl Serialize) -> Result<T> {
    let value = serde_json::to_value(entry).context("encoding a transfer")?;
    serde_json::from_value(value).context("a transfer does not match its wire shape")
}

/// What a listing keeps, read from the flags.
struct Kept {
    since: Option<chrono::DateTime<chrono::Utc>>,
    package: Option<String>,
    purpose: Option<Purpose>,
}

impl Kept {
    fn from(filter: &TransfersFilter) -> Result<Self> {
        let purpose = match &filter.purpose {
            None => None,
            Some(text) => Some(Purpose::parse(text).ok_or_else(|| {
                anyhow!("unknown purpose `{text}`; use package-source, archive, native-input, package-clone, release-bundle, index-file or other")
            })?),
        };
        Ok(Self {
            since: filter.since.as_deref().map(since).transpose()?,
            package: filter.package.clone(),
            purpose,
        })
    }

    fn keeps(&self, entry: &TransferEntry) -> bool {
        let record = &entry.record;
        let package = self.package.as_deref().is_none_or(|wanted| {
            record.package.as_deref().is_some_and(|key| {
                key == wanted
                    || key
                        .strip_prefix(wanted)
                        .is_some_and(|rest| rest.starts_with('@'))
            })
        });
        let purpose = self.purpose.is_none_or(|wanted| record.purpose == wanted);
        package && purpose
    }

    fn recent(&self, entry: &TransferEntry) -> bool {
        self.since.is_none_or(|since| {
            chrono::DateTime::parse_from_rfc3339(&entry.record.started)
                .is_ok_and(|started| started >= since)
        })
    }
}

/// `--since`: a duration ago (`90s`, `30m`, `2h`, `7d`) or an instant.
fn since(text: &str) -> Result<chrono::DateTime<chrono::Utc>> {
    if let Ok(instant) = chrono::DateTime::parse_from_rfc3339(text) {
        return Ok(instant.with_timezone(&chrono::Utc));
    }
    let (number, unit) = text.split_at(text.len().saturating_sub(1));
    let count: i64 = number.parse().map_err(|_| {
        anyhow!("`{text}` is neither a duration such as `2h` nor an RFC 3339 instant")
    })?;
    let span = match unit {
        "s" => chrono::Duration::seconds(count),
        "m" => chrono::Duration::minutes(count),
        "h" => chrono::Duration::hours(count),
        "d" => chrono::Duration::days(count),
        _ => bail!("`{text}` names an unknown unit; use s, m, h or d"),
    };
    Ok(chrono::Utc::now() - span)
}

/// `vibe transfers` and `vibe transfers history`. An active file whose
/// process is gone is moved to the history first (`PROP-068#journal`).
fn list(
    ctx: &output::Context,
    journal: &TransferJournal,
    home: &str,
    filter: &TransfersFilter,
    history_only: bool,
) -> Result<()> {
    let kept = Kept::from(filter)?;
    let live = journal.sweep(&alive);
    let active: Vec<TransferEntry> = if history_only {
        Vec::new()
    } else {
        live.into_iter()
            .filter(|entry| kept.keeps(entry))
            .map(|mut entry| {
                entry.last = entry.samples.pop();
                entry.samples.clear();
                entry
            })
            .collect()
    };
    let mut history: Vec<TransferEntry> = journal
        .history()
        .into_iter()
        .rev()
        .filter(|entry| kept.keeps(entry) && kept.recent(entry))
        .collect();
    if kept.since.is_none() {
        history.truncate(RECENT);
    }
    if ctx.is_json() {
        let listing = transfers_listing::TransfersListing {
            protocol: LISTING_PROTOCOL.into(),
            home: home.to_owned(),
            active: active.iter().map(wire).collect::<Result<_>>()?,
            history: history.iter().map(wire).collect::<Result<_>>()?,
        };
        return ctx.emit_json(&listing);
    }
    view::listing(ctx, &active, &history, history_only);
    Ok(())
}

/// `vibe transfers watch`: the opening `started` lines, then one line per
/// sample and per change, until standard input closes.
fn watch(ctx: &output::Context, journal: &TransferJournal) -> Result<()> {
    let closed = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&closed);
    // A reader thread blocked on stdin is never joined: it ends with the
    // process, and the loop only reads the flag it sets.
    std::thread::spawn(move || {
        let mut sink = [0u8; 256];
        let mut stdin = std::io::stdin();
        while matches!(stdin.read(&mut sink), Ok(read) if read > 0) {}
        flag.store(true, Ordering::SeqCst);
    });
    let json = ctx.is_json();
    let mut stdout = std::io::stdout();
    watch_journal(
        journal,
        &alive,
        TICK,
        &mut || closed.load(Ordering::SeqCst),
        &mut |event| {
            let line = if json {
                event_line(event).and_then(|line| Ok(serde_json::to_string(&line)?))
            } else {
                Ok(view::event(event))
            };
            match line {
                Ok(line) => writeln!(stdout, "{line}")
                    .and_then(|()| stdout.flush())
                    .is_ok(),
                Err(_) => true,
            }
        },
    );
    Ok(())
}

/// One `vibe-transfer-event/1` line.
fn event_line(event: &TransferEvent) -> Result<transfer_event::TransferEvent> {
    let record = match &event.record {
        None => None,
        Some(record) => Some(wire(&TransferEntry {
            bytes: event.bytes.unwrap_or(0),
            peak_bps: event.peak_bps.unwrap_or(0),
            ..TransferEntry::active(record.clone())
        })?),
    };
    Ok(transfer_event::TransferEvent {
        protocol: EVENT_PROTOCOL.into(),
        at: event.at.clone(),
        event: serde_json::from_value(serde_json::Value::from(event.event.as_str()))?,
        id: event.id.clone(),
        batch: event.batch.clone(),
        bytes: event.bytes,
        total: event.total,
        rate_bps: event.rate_bps,
        peak_bps: event.peak_bps,
        record,
        message: event.message.clone(),
    })
}
