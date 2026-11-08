//! The epoch-1 optional observation preserves legacy payload bytes and unknown.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-068#disk-writes");
use vibe_wire::generated::{progress_event, transfer_event, transfer_record, transfers_listing};

const OLD_RECORD: &str = r#"{"id":"7:1","batch":"7","kind":"http","url":"https://example.test/a","purpose":"archive","destination":"cache","started":"2026-11-08T00:00:00.000Z","pid":7,"command":"install -g","precision":"counted","limited":false,"bytes":3,"peak_bps":4}"#;
const OLD_PROGRESS: &str = r#"{"protocol":"vibe-progress-event/1","seq":1,"at":"2026-11-08T00:00:00.000Z","task":1,"kind":"measurement","completed":3,"unit":"bytes","transfer":"7:1","rate_bps":4}"#;

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-068#disk-writes")]
fn legacy_absence_remains_absence_and_preserves_serialization_order() {
    let record: transfer_record::TransferRecord = serde_json::from_str(OLD_RECORD).unwrap();
    assert_eq!(record.disk_written_bytes, None);
    assert_eq!(serde_json::to_string(&record).unwrap(), OLD_RECORD);
    let progress: progress_event::ProgressEvent = serde_json::from_str(OLD_PROGRESS).unwrap();
    assert_eq!(progress.disk_written_bytes, None);
    assert_eq!(serde_json::to_string(&progress).unwrap(), OLD_PROGRESS);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-068#disk-writes")]
fn zero_and_partial_counts_survive_every_existing_projection() {
    for count in [0_u64, 2, u64::MAX] {
        let mut record: serde_json::Value = serde_json::from_str(OLD_RECORD).unwrap();
        record["disk_written_bytes"] = count.to_string().into();
        record["samples"] = serde_json::json!([{"at":"now","bytes":3,"rate_bps":4,"disk_written_bytes":count.to_string()}]);
        record["last"] = record["samples"][0].clone();
        let wire: transfer_record::TransferRecord = serde_json::from_value(record.clone()).unwrap();
        assert_eq!(
            wire.disk_written_bytes.map(|value| value.get()),
            Some(count)
        );
        assert_eq!(serde_json::to_value(wire).unwrap(), record);
        let listing = serde_json::json!({"protocol":"vibe-transfers/1","home":"/tmp/test","active":[record.clone()],"history":[]});
        let wire: transfers_listing::TransfersListing =
            serde_json::from_value(listing.clone()).unwrap();
        assert_eq!(serde_json::to_value(wire).unwrap(), listing);
        let event = serde_json::json!({"protocol":"vibe-transfer-event/1","at":"now","event":"started","id":"7:1","batch":"7","bytes":3,"disk_written_bytes":count.to_string(),"record":record});
        let wire: transfer_event::TransferEvent = serde_json::from_value(event.clone()).unwrap();
        assert_eq!(serde_json::to_value(wire).unwrap(), event);
        let mut progress: serde_json::Value = serde_json::from_str(OLD_PROGRESS).unwrap();
        progress["disk_written_bytes"] = count.to_string().into();
        let wire: progress_event::ProgressEvent = serde_json::from_value(progress.clone()).unwrap();
        assert_eq!(serde_json::to_value(wire).unwrap(), progress);
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-068#disk-writes")]
fn observational_readers_stay_permissive_but_the_counter_is_an_integer() {
    let mut record: serde_json::Value = serde_json::from_str(OLD_RECORD).unwrap();
    record["future_observation"] = true.into();
    assert!(serde_json::from_value::<transfer_record::TransferRecord>(record.clone()).is_ok());
    for value in [
        serde_json::json!(-1),
        serde_json::json!(1.5),
        serde_json::json!(3),
        serde_json::json!("03"),
        serde_json::json!("+3"),
        serde_json::json!("18446744073709551616"),
    ] {
        record["disk_written_bytes"] = value;
        assert!(serde_json::from_value::<transfer_record::TransferRecord>(record.clone()).is_err());
    }
}
