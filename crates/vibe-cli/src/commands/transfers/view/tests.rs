//! The human forms of vibe transfers at a fixed width. Glyphs asserted here: ● ✓ ○.

use super::*;
use vibe_core::progress::transfer::{
    Destination, Precision, Purpose, TransferKind, TransferRecord,
};

const START: &str = "2026-11-23T08:34:58.638Z";
const END: &str = "2026-11-23T08:35:01.638Z";

fn record() -> TransferRecord {
    TransferRecord {
        id: "7:1".into(),
        batch: "7".into(),
        kind: TransferKind::Http,
        url: "http://127.0.0.1:18765/big.bin".into(),
        purpose: Purpose::Other,
        package: None,
        step: None,
        destination: Destination::Cache,
        total: Some(20_000_000),
        started: START.into(),
        pid: 7,
        command: String::new(),
        precision: Precision::Counted,
        limited: false,
        limit_bps: None,
        remapped_from: None,
        proxy: None,
        resume: None,
        continues: None,
    }
}

fn event(kind: TransferEventKind) -> TransferEvent {
    TransferEvent {
        at: START.into(),
        event: kind,
        id: "7:1".into(),
        batch: "7".into(),
        bytes: None,
        total: None,
        rate_bps: None,
        peak_bps: None,
        disk_written_bytes: None,
        record: None,
        message: None,
    }
}

#[test]
fn transfers_watch_shows_url_and_written_bytes() {
    let mut watch = Watch::default();
    let started = TransferEvent {
        bytes: Some(0),
        total: Some(20_000_000),
        rate_bps: Some(0),
        record: Some(record()),
        ..event(TransferEventKind::Started)
    };
    let sample = TransferEvent {
        at: "2026-11-23T08:35:00.638Z".into(),
        bytes: Some(10_000_000),
        total: Some(20_000_000),
        rate_bps: Some(5_000_000),
        ..event(TransferEventKind::Sample)
    };
    let finished = TransferEvent {
        at: END.into(),
        bytes: Some(20_000_000),
        disk_written_bytes: Some(12_000_000),
        ..event(TransferEventKind::Finished)
    };
    let orphan = TransferEvent {
        id: "9:9".into(),
        batch: "9".into(),
        ..event(TransferEventKind::Interrupted)
    };
    assert_eq!(
        watch.line(&started, 100),
        "● http://127.0.0.1:18765/big.bin  other · 0 B of 20.0 MB · started 08:34:58"
    );
    assert_eq!(
        watch.line(&sample, 100),
        "● http://127.0.0.1:18765/big.bin  other · 10.0 MB of 20.0 MB · 5.0 MB/s"
    );
    assert_eq!(
        watch.line(&finished, 100),
        "✓ http://127.0.0.1:18765/big.bin  other · 20.0 MB · 3s · 12.0 MB written"
    );
    assert_eq!(watch.line(&orphan, 100), "○ 9:9 · interrupted");
    assert!(watch.heads.is_empty());
}

#[test]
fn transfers_history_has_rule_heading() {
    let active = TransferEntry::active(record());
    let finished = TransferEntry {
        bytes: 20_000_000,
        elapsed_ms: Some(0),
        average_bps: Some(0),
        outcome: Some(Outcome::Finished),
        ..TransferEntry::active(record())
    };
    let history = concat!(
        "── history · 1 transfer ────────────────────────────────────\n\n",
        "  http://127.0.0.1:18765/big.bin  other · 20.0 MB · 0s · 0 B/s"
    );
    assert_eq!(
        listing_text(&[active], std::slice::from_ref(&finished), false, 100, 60),
        format!(
            "── active · 1 transfer ─────────────────────────────────────\n\n  http://127.0.0.1:18765/big.bin  other · 0 B of 20.0 MB\n\n{history}"
        )
    );
    assert_eq!(
        listing_text(&[], std::slice::from_ref(&finished), false, 100, 60),
        format!("  no active transfers\n\n{history}")
    );
    assert_eq!(listing_text(&[], &[finished], true, 100, 60), history);
}

#[test]
fn transfers_empty_listing_is_one_hint() {
    assert_eq!(
        listing_text(&[], &[], true, 100, 60),
        "  no transfers in the history"
    );
    assert_eq!(
        listing_text(&[], &[], false, 100, 60),
        "  no active transfers and none in the history"
    );
    assert_eq!(quiet_line(0, 1, true), "1 transfer in the history");
    assert_eq!(
        quiet_line(2, 0, false),
        "2 active transfers · 0 in the history"
    );
    assert_eq!(
        quiet_line(1, 2, false),
        "1 active transfer · 2 in the history"
    );
}

#[test]
fn transfers_finished_elapsed_uses_record_start_not_attach_time() {
    let mut watch = Watch::default();
    watch.line(
        &TransferEvent {
            at: "2026-11-23T08:35:00.638Z".into(),
            record: Some(record()),
            ..event(TransferEventKind::Started)
        },
        100,
    );
    assert_eq!(
        watch.line(
            &TransferEvent {
                at: END.into(),
                bytes: Some(20_000_000),
                disk_written_bytes: Some(12_000_000),
                ..event(TransferEventKind::Finished)
            },
            100
        ),
        "✓ http://127.0.0.1:18765/big.bin  other · 20.0 MB · 3s · 12.0 MB written"
    );
}

#[test]
fn transfers_finished_omits_missing_invalid_or_negative_elapsed() {
    for (started, ended) in [("invalid", END), (START, "invalid"), (END, START)] {
        let mut watch = Watch::default();
        watch.line(
            &TransferEvent {
                record: Some(TransferRecord {
                    started: started.into(),
                    ..record()
                }),
                ..event(TransferEventKind::Started)
            },
            100,
        );
        assert_eq!(
            watch.line(
                &TransferEvent {
                    at: ended.into(),
                    bytes: Some(20_000_000),
                    disk_written_bytes: Some(12_000_000),
                    ..event(TransferEventKind::Finished)
                },
                100
            ),
            "✓ http://127.0.0.1:18765/big.bin  other · 20.0 MB · 12.0 MB written"
        );
    }
    assert_eq!(
        Watch::default().line(
            &TransferEvent {
                at: END.into(),
                bytes: Some(20_000_000),
                disk_written_bytes: Some(12_000_000),
                ..event(TransferEventKind::Finished)
            },
            100
        ),
        "✓ 7:1 · 20.0 MB · 12.0 MB written"
    );
}

#[test]
fn transfers_watch_package_total_fallback_and_terminal_cache_eviction() {
    for kind in [TransferEventKind::Failed, TransferEventKind::Interrupted] {
        let mut watch = Watch::default();
        watch.line(
            &TransferEvent {
                record: Some(TransferRecord {
                    package: Some("org.example/tool@1.0.0".into()),
                    ..record()
                }),
                ..event(TransferEventKind::Started)
            },
            100,
        );
        assert_eq!(
            watch.line(
                &TransferEvent {
                    bytes: Some(10_000_000),
                    ..event(TransferEventKind::Sample)
                },
                100
            ),
            "● org.example/tool@1.0.0  other · 10.0 MB of 20.0 MB"
        );
        let expected = if kind == TransferEventKind::Failed {
            "✗ org.example/tool@1.0.0  other · 10.0 MB · broken"
        } else {
            "○ org.example/tool@1.0.0  other · 10.0 MB · interrupted"
        };
        assert_eq!(
            watch.line(
                &TransferEvent {
                    bytes: Some(10_000_000),
                    message: Some("broken".into()),
                    ..event(kind)
                },
                100
            ),
            expected
        );
        assert!(watch.heads.is_empty());
    }
}
