//! A changed selected descriptor after the last output read must refuse.

use std::fs;

use super::SourceQueryUnavailable;
use super::m_01_b_2_cases::{fixture, seal};

#[test]
fn final_output_window_rechecks_selected_descriptor() {
    let fixture = fixture(true);
    let world = seal(&fixture);
    let record_path = fixture.dep_root.join(".vibe-slot.toml");
    let record = fs::read_to_string(&record_path).unwrap();
    let result = world.revalidate_with_hooks(
        || {},
        || {
            fs::write(
                &record_path,
                record.replace(&"a".repeat(64), &"b".repeat(64)),
            )
            .unwrap()
        },
    );
    assert!(
        matches!(
            result,
            Err(SourceQueryUnavailable::StaleWorld(
                "selected descriptor record"
            ))
        ),
        "{}",
        if result.is_ok() {
            "accepted stale world"
        } else {
            "wrong refusal"
        }
    );
}
