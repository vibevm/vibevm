use std::io::Read;
use std::sync::{Arc, Mutex};

use super::*;
use crate::progress::{Progress, ProgressEvent, ProgressObserver};

#[derive(Default)]
struct Recorder(Mutex<Vec<ProgressEvent>>);

impl ProgressObserver for Recorder {
    fn observe(&self, event: ProgressEvent) {
        self.0.lock().unwrap().push(event);
    }
}

fn record(url: &str) -> TransferRecord {
    TransferRecord::http(Purpose::PackageSource, url, Destination::Cache)
        .package("org.example/tool@1.0.0")
        .step("fetch")
        .total(Some(3 << 20))
        .resume("vibe install -g org.example/tool@1.0.0 --resume")
}

#[test]
fn a_transfer_is_a_task_of_its_own_and_a_journal_entry() {
    let home = tempfile::tempdir().unwrap();
    let journal = TransferJournal::new(home.path());
    let recorder = Arc::new(Recorder::default());
    let transfers = Transfers::new(Some(journal.clone()), None, "install -g");
    let progress = Progress::new(recorder.clone()).with_transfers(Some(transfers));
    let step = progress.task("fetch (sources)  Fetch");
    let body = vec![1u8; 3 << 20];
    let mut reader =
        begin(record("https://u:p@example.test/t.tar.gz?x=1"), Some(&step)).reader(&body[..]);
    let id = reader.transfer().id().to_owned();
    let mut chunk = vec![0u8; 64 << 10];
    let mut seen_active = false;
    while reader.read(&mut chunk).unwrap() > 0 {
        seen_active |= journal.active().iter().any(|entry| entry.record.id == id);
    }
    reader.finish();
    step.finish();
    assert!(seen_active, "the active file exists while it runs");
    assert!(journal.active().is_empty(), "and is gone at the end");

    let events = recorder.0.lock().unwrap();
    let ProgressEventKind::Started {
        transfer: Some(started),
        ..
    } = &events[1].kind
    else {
        panic!("{:?}", events[1]);
    };
    assert_eq!(started.url, "https://example.test/t.tar.gz");
    assert_eq!(started.command, "install -g");
    assert_eq!(events[1].parent_id, Some(step.id()));
    let measured: Vec<(u64, Option<String>)> = events
        .iter()
        .filter_map(|event| match &event.kind {
            ProgressEventKind::Progress {
                completed,
                transfer,
                rate_bps,
                ..
            } => {
                assert!(rate_bps.is_some());
                Some((*completed, transfer.clone()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(measured.first().unwrap().0, 0, "the start is a sample");
    assert!(measured.len() >= 4, "one per MiB at least: {measured:?}");
    assert!(measured.windows(2).all(|pair| pair[0].0 <= pair[1].0));
    assert!(
        measured
            .iter()
            .all(|(_, transfer)| transfer.as_deref() == Some(id.as_str()))
    );
    assert!(events.iter().any(|event| matches!(
        &event.kind,
        ProgressEventKind::Finished { transfer: Some(end) } if end.bytes == 3 << 20 && end.id == id
    )));

    let history = journal.history();
    assert_eq!(history.len(), 1);
    let entry = &history[0];
    assert_eq!(entry.outcome, Some(Outcome::Finished));
    assert_eq!(entry.bytes, 3 << 20);
    assert_eq!(
        entry.series.iter().map(|b| b.bytes).sum::<u64>(),
        entry.bytes
    );
    assert_eq!(
        entry.record.resume, None,
        "only an interrupted entry names it"
    );
    assert_eq!(journal.revision(), 2, "began and ended");
}

#[test]
fn a_dropped_transfer_fails_and_an_interrupted_one_names_the_resume_line() {
    let home = tempfile::tempdir().unwrap();
    let journal = TransferJournal::new(home.path());
    drop(Transfer::begin(
        record("https://example.test/a"),
        None,
        Some(&journal),
    ));
    Transfer::begin(record("https://example.test/b"), None, Some(&journal)).interrupted();
    let next = Transfer::begin(record("https://example.test/b"), None, Some(&journal));
    let continued = next.record().continues.clone();
    next.finish();
    let history = journal.history();
    assert_eq!(history[0].outcome, Some(Outcome::Failed));
    assert_eq!(history[1].outcome, Some(Outcome::Interrupted));
    assert_eq!(
        history[1].record.resume.as_deref(),
        Some("vibe install -g org.example/tool@1.0.0 --resume")
    );
    assert_eq!(continued.as_deref(), Some(history[1].record.id.as_str()));
}

#[test]
fn an_orphaned_active_file_becomes_interrupted_and_a_watch_sees_everything() {
    let home = tempfile::tempdir().unwrap();
    let journal = TransferJournal::new(home.path());
    let mut orphan = TransferEntry::active(TransferRecord {
        pid: u32::MAX,
        ..record("https://example.test/orphan")
    });
    orphan.bytes = 30;
    orphan.samples = vec![
        Sample {
            at: orphan.record.started.clone(),
            bytes: 10,
            rate_bps: 0,
        },
        Sample {
            at: clock::format_millis(clock::parse_millis(&orphan.record.started).unwrap() + 1500),
            bytes: 30,
            rate_bps: 20,
        },
    ];
    journal.opened(&orphan);

    let running = Transfer::begin(record("https://example.test/live"), None, Some(&journal));
    let (mut watch, opening) = TransferWatch::open(&journal, &|_| false);
    // The orphan is swept into history before the opening lines.
    assert_eq!(opening.len(), 1);
    assert_eq!(opening[0].event, TransferEventKind::Started);
    assert_eq!(opening[0].id, running.id());
    assert!(opening[0].record.is_some());
    let interrupted = &journal.history()[0];
    assert_eq!(interrupted.outcome, Some(Outcome::Interrupted));
    assert_eq!(interrupted.series.iter().map(|b| b.bytes).sum::<u64>(), 30);
    assert!(interrupted.record.resume.is_some());

    let mut reader = running.reader(&[7u8; 2 << 20][..]);
    std::io::copy(&mut reader, &mut std::io::sink()).unwrap();
    let samples = watch.poll(&|_| false);
    assert!(
        samples
            .iter()
            .all(|event| event.event == TransferEventKind::Sample)
    );
    assert!(
        samples.iter().any(|event| event.bytes == Some(1 << 20)),
        "{samples:?}"
    );
    reader.finish();
    let quick = Transfer::begin(record("https://example.test/quick"), None, Some(&journal));
    quick.finish();
    let kinds: Vec<TransferEventKind> = watch.poll(&|_| false).iter().map(|e| e.event).collect();
    assert_eq!(
        kinds,
        [
            TransferEventKind::Finished,
            TransferEventKind::Started,
            TransferEventKind::Finished
        ],
        "the end, and a transfer that began and ended between reads"
    );
}

#[test]
fn a_limit_marks_the_record_and_the_history_is_trimmed() {
    let home = tempfile::tempdir().unwrap();
    let journal = TransferJournal::new(home.path());
    let transfers = Transfers::new(
        Some(journal.clone()),
        TransferLimit::new(1 << 30),
        "cache add",
    );
    let task = Progress::default()
        .with_transfers(Some(transfers))
        .task("x");
    let transfer = begin(record("https://example.test/limited"), Some(&task));
    assert!(transfer.record().limited);
    assert_eq!(transfer.record().limit_bps, Some(1 << 30));
    transfer.finish();
    for _ in 0..505 {
        Transfer::begin(record("https://example.test/many"), None, Some(&journal)).finish();
    }
    assert_eq!(journal.history().len(), 500);
    assert_eq!(journal.clear_history().unwrap(), 500);
    assert!(journal.history().is_empty());
}

#[cfg(unix)]
#[test]
fn the_journal_is_private() {
    use std::os::unix::fs::PermissionsExt;

    let home = tempfile::tempdir().unwrap();
    let journal = TransferJournal::new(home.path());
    Transfer::begin(record("https://example.test/p"), None, Some(&journal)).finish();
    let mode = std::fs::metadata(journal.root().join("history.jsonl"))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600);
}
