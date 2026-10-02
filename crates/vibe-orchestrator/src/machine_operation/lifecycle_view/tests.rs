//! Data bounds only; no native owner proof is claimed here.
use super::*;
#[test]
fn utf8_diagnostic_prefix_and_exact_truncation() {
    for n in [4095, 4096, 4097] {
        let (s, t) = diagnostic(&"a".repeat(n), false);
        assert_eq!(s.len(), n.min(4096));
        assert_eq!(t, n > 4096);
    }
    let (s, t) = diagnostic(&("a".repeat(4095) + "é"), false);
    assert_eq!(s.len(), 4095);
    assert!(t);
    assert!(diagnostic("already bounded", true).1);
}
#[test]
fn counting_writer_accounts_for_json_escaping_before_allocation() {
    let mut v: LifecycleSubmissionView = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../formats/corpora/machine-lifecycle-submission-view/e1/valid/starting.json"
    )))
    .unwrap();
    v.requested_phase = "\"".repeat(MAX_FRAME_BYTES / 2);
    assert_eq!(size(&v), Err(ViewError::FrameLimit));
    assert!(bounded(&"a".repeat(256), 256).is_ok());
    assert!(bounded(&"a".repeat(257), 256).is_err());
    assert!(!hex(&"A".repeat(64), 64));
}
