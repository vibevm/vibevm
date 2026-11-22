use super::{REFERENCE_HARNESSES, known_reference_harness, reference_binding_supported};
use std::collections::BTreeSet;
use vibe_core::agent_ids::AGENTS;

#[test]
fn reference_harnesses_match_agent_vocabulary() {
    let reference_harnesses: BTreeSet<_> = REFERENCE_HARNESSES.into_iter().collect();
    let vocabulary_harnesses: BTreeSet<_> = AGENTS.iter().filter_map(|row| row.harness).collect();
    assert_eq!(reference_harnesses, vocabulary_harnesses);

    for row in &AGENTS {
        if let Some(harness) = row.harness {
            assert!(known_reference_harness(harness));
            assert_eq!(reference_binding_supported(harness), row.reference_binding);
        }
    }

    assert!(!reference_binding_supported("qwen-code"));
}
