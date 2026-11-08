use super::*;
use crate::progress::transfer::{
    Destination, Outcome, Purpose, Sample, Transfer, TransferEntry, TransferEventKind,
    TransferJournal, TransferRecord, TransferWatch,
};
use crate::progress::{Progress, ProgressEvent, ProgressEventKind, ProgressObserver};
use std::io::Read;

#[derive(Default)]
struct Recorder(Mutex<Vec<ProgressEvent>>);
impl ProgressObserver for Recorder {
    fn observe(&self, event: ProgressEvent) {
        self.0.lock().unwrap().push(event);
    }
}

struct ShortThenFail {
    remaining: usize,
}
impl Write for ShortThenFail {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.remaining == 0 {
            return Err(io::Error::new(io::ErrorKind::BrokenPipe, "fixture"));
        }
        let n = bytes.len().min(self.remaining).min(2);
        self.remaining -= n;
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::other("fixture flush failure"))
    }
}

fn record() -> TransferRecord {
    TransferRecord::http(
        Purpose::Archive,
        "https://example.test/archive",
        Destination::Cache,
    )
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-068#disk-writes")]
fn successful_partial_writes_survive_failure_interruption_and_drop() {
    for outcome in ["failed", "interrupted", "drop"] {
        let home = tempfile::tempdir().unwrap();
        let journal = TransferJournal::new(home.path());
        let mut pair = Transfer::begin(record(), None, Some(&journal))
            .reader(&b"abcdefgh"[..])
            .with_disk_writer(ShortThenFail { remaining: 3 });
        let (reader, writer) = pair.parts();
        let mut bytes = [0; 8];
        reader.read_exact(&mut bytes).unwrap();
        assert_eq!(
            writer.write_all(&bytes).unwrap_err().kind(),
            io::ErrorKind::BrokenPipe
        );
        match outcome {
            "failed" => pair.fail("fixture"),
            "interrupted" => pair.interrupted(),
            _ => drop(pair),
        }
        let entries = journal.history();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].disk_written_bytes, Some(3), "{outcome}");
        assert_ne!(
            entries[0].disk_written_bytes,
            Some(8),
            "a successful read is not a write"
        );
        assert!(journal.active().is_empty());
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-068#disk-writes")]
fn finish_flushes_exact_counter_and_parent_is_not_an_aggregate() {
    let home = tempfile::tempdir().unwrap();
    let journal = TransferJournal::new(home.path());
    let observer = Arc::new(Recorder::default());
    let progress = Progress::new(observer.clone());
    let parent = progress.task("install");
    let file = tempfile::tempfile().unwrap();
    let mut pair = Transfer::begin(record(), Some(&parent), Some(&journal))
        .reader(&b"seven!!"[..])
        .with_disk_writer(file);
    let (reader, writer) = pair.parts();
    std::io::copy(reader, writer).unwrap();
    pair.sync_all().unwrap();
    pair.finish();
    parent.set_progress(1, Some(1), "packages");
    parent.finish();
    assert_eq!(journal.history()[0].disk_written_bytes, Some(7));
    let events = observer.0.lock().unwrap();
    assert!(events.iter().any(|event| matches!(&event.kind,
        ProgressEventKind::Finished { transfer: Some(end) } if end.disk_written_bytes == Some(7))));
    assert!(events.iter().any(|event| matches!(
        &event.kind,
        ProgressEventKind::Progress {
            transfer: None,
            disk_written_bytes: None,
            ..
        }
    )));
    let values: Vec<_> = events
        .iter()
        .filter_map(|event| match event.kind {
            ProgressEventKind::Progress {
                disk_written_bytes: Some(n),
                ..
            } => Some(n),
            _ => None,
        })
        .collect();
    assert_eq!(values.first(), Some(&0));
    assert_eq!(values.last(), Some(&7));
    assert!(values.windows(2).all(|pair| pair[0] <= pair[1]));
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-068#disk-writes")]
fn failed_or_zero_writes_and_failed_flush_do_not_invent_bytes() {
    let writes = DiskWrites::new();
    let mut writer = TransferWriter {
        inner: ShortThenFail { remaining: 0 },
        writes: writes.clone(),
    };
    assert!(writer.write_all(b"x").is_err());
    assert_eq!(writes.bytes(), Some(0));
    writer.inner.remaining = 3;
    writer.write_all(b"abc").unwrap();
    assert!(writer.flush().is_err());
    assert_eq!(writes.bytes(), Some(3));
    struct Zero;
    impl Write for Zero {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Ok(0)
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let zero = DiskWrites::new();
    let mut writer = TransferWriter {
        inner: Zero,
        writes: zero.clone(),
    };
    assert_eq!(
        writer.write_all(b"not written").unwrap_err().kind(),
        io::ErrorKind::WriteZero
    );
    assert_eq!(zero.bytes(), Some(0));
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-068#disk-writes")]
fn unknown_legacy_and_uninstrumented_transfers_stay_absent() {
    let sample: Sample = serde_json::from_str(r#"{"at":"now","bytes":7,"rate_bps":2}"#).unwrap();
    assert_eq!(sample.disk_written_bytes, None);
    assert!(
        serde_json::to_value(sample)
            .unwrap()
            .get("disk_written_bytes")
            .is_none()
    );
    let home = tempfile::tempdir().unwrap();
    let journal = TransferJournal::new(home.path());
    let mut transfer = Transfer::begin(
        TransferRecord::git(
            Purpose::PackageClone,
            "https://example.test/p.git",
            Destination::Store,
        ),
        None,
        Some(&journal),
    );
    transfer.reported(100, Some(100));
    transfer.written(900);
    transfer.finish();
    let entry = &journal.history()[0];
    assert_eq!(entry.disk_written_bytes, None);
    assert_eq!(entry.written, Some(900));
    let json = serde_json::to_value(entry).unwrap();
    assert!(json.get("disk_written_bytes").is_none());
    assert_eq!(
        serde_json::from_value::<TransferEntry>(json).unwrap(),
        *entry
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-068#disk-writes")]
fn watch_preserves_same_millisecond_disk_only_samples_and_terminal_counter() {
    let home = tempfile::tempdir().unwrap();
    let journal = TransferJournal::new(home.path());
    let mut entry = TransferEntry::active(record());
    entry.bytes = 7;
    entry.disk_written_bytes = Some(0);
    entry.samples = vec![Sample {
        at: entry.record.started.clone(),
        bytes: 7,
        rate_bps: 0,
        disk_written_bytes: Some(0),
    }];
    journal.opened(&entry);
    let (mut watch, opening) = TransferWatch::open(&journal, &|_| true);
    assert_eq!(opening[0].disk_written_bytes, Some(0));
    entry.disk_written_bytes = Some(3);
    entry.samples.push(Sample {
        disk_written_bytes: Some(3),
        ..entry.samples[0].clone()
    });
    journal.sampled(&entry);
    let events = watch.poll(&|_| true);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event, TransferEventKind::Sample);
    assert_eq!(events[0].disk_written_bytes, Some(3));
    entry.close(Outcome::Failed, 1, &(Vec::new(), 1));
    journal.ended(&entry);
    assert_eq!(watch.poll(&|_| true)[0].disk_written_bytes, Some(3));
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-068#disk-writes")]
fn unrepresentable_counter_becomes_unknown_without_failing_successful_write() {
    let writes = DiskWrites::new();
    *writes.0.lock().unwrap() = Some(u64::MAX);
    let mut writer = TransferWriter {
        inner: Vec::new(),
        writes: writes.clone(),
    };
    assert_eq!(writer.write(b"x").unwrap(), 1);
    assert_eq!(writes.bytes(), None);
    assert_eq!(writer.write(b"y").unwrap(), 1);
    assert_eq!(writes.bytes(), None);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-068#disk-writes")]
fn a_new_transfer_id_has_a_new_counter_even_for_the_same_url() {
    let home = tempfile::tempdir().unwrap();
    let journal = TransferJournal::new(home.path());
    let mut first = Transfer::begin(record(), None, Some(&journal))
        .reader(&b"abc"[..])
        .with_disk_writer(tempfile::tempfile().unwrap());
    let (reader, writer) = first.parts();
    std::io::copy(reader, writer).unwrap();
    first.finish();
    Transfer::begin(record(), None, Some(&journal))
        .reader(&b""[..])
        .with_disk_writer(tempfile::tempfile().unwrap())
        .finish();
    let entries = journal.history();
    assert_eq!(entries.len(), 2);
    assert_ne!(entries[0].record.id, entries[1].record.id);
    assert_eq!(entries[0].disk_written_bytes, Some(3));
    assert_eq!(entries[1].disk_written_bytes, Some(0));
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-068#disk-writes")]
fn implicit_drop_closes_writer_before_terminal_report() {
    use std::sync::atomic::{AtomicBool, Ordering};
    struct ClosingWriter(Arc<AtomicBool>);
    impl Write for ClosingWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    impl Drop for ClosingWriter {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }
    struct EndObserver {
        closed: Arc<AtomicBool>,
        ended: AtomicBool,
    }
    impl ProgressObserver for EndObserver {
        fn observe(&self, event: ProgressEvent) {
            if matches!(
                event.kind,
                ProgressEventKind::Failed {
                    transfer: Some(_),
                    ..
                }
            ) {
                assert!(
                    self.closed.load(Ordering::SeqCst),
                    "writer outlived terminal report"
                );
                self.ended.store(true, Ordering::SeqCst);
            }
        }
    }
    let closed = Arc::new(AtomicBool::new(false));
    let observer = Arc::new(EndObserver {
        closed: closed.clone(),
        ended: AtomicBool::new(false),
    });
    let progress = Progress::new(observer.clone());
    let task = progress.task("parent");
    drop(
        Transfer::begin(record(), Some(&task), None)
            .reader(&b""[..])
            .with_disk_writer(ClosingWriter(closed)),
    );
    assert!(observer.ended.load(Ordering::SeqCst));
    task.finish();
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#M-WIDE-INTEGERS-AS-STRINGS"
)]
fn journal_and_samples_use_the_same_lossless_decimal_boundary() {
    let mut entry = TransferEntry::active(record());
    entry.disk_written_bytes = Some(u64::MAX);
    entry.samples.push(Sample {
        at: entry.record.started.clone(),
        bytes: 0,
        rate_bps: 0,
        disk_written_bytes: Some((1_u64 << 53) + 1),
    });
    let value = serde_json::to_value(&entry).unwrap();
    assert_eq!(value["disk_written_bytes"], u64::MAX.to_string());
    assert_eq!(
        value["samples"][0]["disk_written_bytes"],
        "9007199254740993"
    );
    assert_eq!(
        serde_json::from_value::<TransferEntry>(value.clone()).unwrap(),
        entry
    );
    for invalid in [
        serde_json::json!(3),
        serde_json::json!("03"),
        serde_json::json!("+3"),
        serde_json::json!("-1"),
        serde_json::json!("18446744073709551616"),
    ] {
        let mut bad = value.clone();
        bad["disk_written_bytes"] = invalid.clone();
        assert!(serde_json::from_value::<TransferEntry>(bad).is_err());
        let mut bad = value["samples"][0].clone();
        bad["disk_written_bytes"] = invalid;
        assert!(serde_json::from_value::<Sample>(bad).is_err());
    }
}
