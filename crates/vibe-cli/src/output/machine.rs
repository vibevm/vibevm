//! The machine progress renderer (PROP-060 `#MACHINE-STREAM`): every
//! observation becomes one `vibe-progress-event/1` JSON line on stderr,
//! unthrottled and in emission order, with the task identity and parent so
//! a consumer rebuilds the tree. It never writes stdout.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-060#MACHINE-STREAM");

use std::collections::BTreeMap;
use std::io::{self, Write};
use std::sync::Mutex;

use vibe_core::progress::transfer::TransferRecord;
use vibe_core::progress::{
    ProgressDiagnosticLevel, ProgressEvent, ProgressEventKind, ProgressObserver, TaskId,
    TransferEnd,
};
use vibe_wire::generated::progress_event as wire;

use super::sanitize_progress_text;

pub(super) struct MachineRenderer {
    state: Mutex<State>,
}

struct State {
    seq: u64,
    labels: BTreeMap<TaskId, String>,
    writer: Box<dyn Write + Send>,
}

impl MachineRenderer {
    pub(super) fn new(writer: Box<dyn Write + Send>) -> Self {
        Self {
            state: Mutex::new(State {
                seq: 0,
                labels: BTreeMap::new(),
                writer,
            }),
        }
    }

    pub(super) fn stderr() -> Self {
        Self::new(Box::new(io::stderr()))
    }
}

impl ProgressObserver for MachineRenderer {
    fn observe(&self, event: ProgressEvent) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.seq += 1;
        let mut line = wire::ProgressEvent {
            protocol: "vibe-progress-event/1".into(),
            seq: state.seq,
            at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            task: event.task_id.get(),
            parent: event.parent_id.map(TaskId::get),
            kind: wire::Kind::Detail,
            label: None,
            requires: None,
            completed: None,
            total: None,
            unit: None,
            level: None,
            message: None,
            transfer: None,
            transfer_record: None,
            rate_bps: None,
            peak_bps: None,
            disk_written_bytes: None,
        };
        let terminal = matches!(
            event.kind,
            ProgressEventKind::Finished { .. }
                | ProgressEventKind::Skipped { .. }
                | ProgressEventKind::Failed { .. }
                | ProgressEventKind::Stopped
        );
        match event.kind {
            ProgressEventKind::Started {
                label,
                requires,
                transfer,
            } => {
                line.kind = wire::Kind::Start;
                if let Some(record) = transfer {
                    line.transfer = Some(record.id.clone());
                    line.transfer_record = wire_record(*record);
                }
                line.requires = requires.map(|requires| wire::Requires {
                    key: requires.key,
                    scope: requires.scope,
                    step: requires.step,
                });
                state
                    .labels
                    .insert(event.task_id, sanitize_progress_text(&label));
            }
            ProgressEventKind::Progress {
                completed,
                total,
                unit,
                transfer,
                rate_bps,
                disk_written_bytes,
            } => {
                line.kind = wire::Kind::Measurement;
                line.completed = Some(completed);
                line.total = total;
                line.unit = Some(unit);
                line.transfer = transfer;
                line.rate_bps = rate_bps;
                line.disk_written_bytes =
                    disk_written_bytes.map(vibe_core::wire_decimal::DecimalU64::new);
            }
            ProgressEventKind::Detail { message } => {
                line.message = Some(sanitize_progress_text(&message));
            }
            ProgressEventKind::Diagnostic { level, message } => {
                line.kind = wire::Kind::Diagnostic;
                line.level = Some(match level {
                    ProgressDiagnosticLevel::Warning => wire::Level::Warning,
                    ProgressDiagnosticLevel::Error => wire::Level::Error,
                });
                line.message = Some(sanitize_progress_text(&message));
            }
            ProgressEventKind::Finished { transfer } => {
                line.kind = wire::Kind::Finish;
                end_figures(&mut line, transfer);
            }
            ProgressEventKind::Skipped { reason } => {
                line.kind = wire::Kind::Skip;
                line.message = Some(sanitize_progress_text(&reason));
            }
            ProgressEventKind::Failed { message, transfer } => {
                line.kind = wire::Kind::Fail;
                line.message = Some(sanitize_progress_text(&message));
                end_figures(&mut line, transfer);
            }
            ProgressEventKind::Stopped => {
                line.kind = wire::Kind::Fail;
                line.message = Some("stopped without an outcome".into());
            }
        }
        line.label = if terminal {
            state.labels.remove(&event.task_id)
        } else {
            state.labels.get(&event.task_id).cloned()
        };
        if let Ok(text) = serde_json::to_string(&line) {
            let _ = writeln!(state.writer, "{text}").and_then(|()| state.writer.flush());
        }
    }
}

/// The final figures of a download on its `finish` or `fail` line.
fn end_figures(line: &mut wire::ProgressEvent, transfer: Option<TransferEnd>) {
    if let Some(end) = transfer {
        line.completed = Some(end.bytes);
        line.total = end.total;
        line.unit = Some("bytes".into());
        line.transfer = Some(end.id);
        line.peak_bps = Some(end.peak_bps);
        line.disk_written_bytes = end
            .disk_written_bytes
            .map(vibe_core::wire_decimal::DecimalU64::new);
    }
}

/// A transfer's record as the stream carries it: everything but the
/// process and the verb, which the stream's reader already knows
/// (`PROP-068#stream`). The vocabularies are the same words on both
/// sides, so the record crosses as JSON.
fn wire_record(record: TransferRecord) -> Option<wire::TransferRecord> {
    let mut value = serde_json::to_value(record).ok()?;
    let object = value.as_object_mut()?;
    object.remove("pid");
    object.remove("command");
    serde_json::from_value(value).ok()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use vibe_core::progress::Progress;

    use super::*;

    #[derive(Clone, Default)]
    struct Sink(Arc<Mutex<Vec<u8>>>);
    impl Write for Sink {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn every_observation_is_one_json_line_with_identity_and_parent() {
        let sink = Sink::default();
        let progress = Progress::new(Arc::new(MachineRenderer::new(Box::new(sink.clone()))));
        let root = progress.task("install");
        let step = root.progress().task("fetch: Fetch");
        step.set_progress(10, Some(20), "bytes");
        step.detail("line\u{1b}[31m");
        step.finish();
        root.skip("nothing else");
        let requirement = progress.task_requiring(
            "requires org.example/tcc@1.0.0 (runtime)",
            vibe_core::progress::Requires {
                key: "org.example/tcc@1.0.0".into(),
                scope: "runtime".into(),
                step: None,
            },
        );
        requirement.finish();
        let text = String::from_utf8(sink.0.lock().unwrap().clone()).unwrap();
        let lines: Vec<serde_json::Value> = text
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        let kinds: Vec<&str> = lines.iter().map(|l| l["kind"].as_str().unwrap()).collect();
        assert_eq!(
            kinds,
            [
                "start",
                "start",
                "measurement",
                "detail",
                "finish",
                "skip",
                "start",
                "finish"
            ]
        );
        assert!(lines[..6].iter().all(|l| l.get("requires").is_none()));
        assert_eq!(
            lines[6]["requires"],
            serde_json::json!({"key": "org.example/tcc@1.0.0", "scope": "runtime"})
        );
        assert!(
            lines[7].get("requires").is_none(),
            "only the start carries it"
        );
        assert_eq!(lines[1]["parent"], lines[0]["task"]);
        assert_eq!(lines[2]["total"], 20);
        assert_eq!(lines[4]["label"], "fetch: Fetch");
        assert!(!lines[3]["message"].as_str().unwrap().contains('\u{1b}'));
        let seqs: Vec<u64> = lines.iter().map(|l| l["seq"].as_u64().unwrap()).collect();
        assert_eq!(seqs, [1, 2, 3, 4, 5, 6, 7, 8]);
        assert!(
            lines
                .iter()
                .all(|l| l["protocol"] == "vibe-progress-event/1")
        );
    }
}
