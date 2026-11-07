//! Test-only loss of the otherwise fully bound replay read carrier.
use super::BootReplaySet;

/// Test-only corruption of the retained read carrier. Valid prepared bindings
/// always contain a source hash; this simulates losing that whole evidence.
#[cfg(test)]
pub(crate) fn drop_source_binding_for_test(replay: &mut BootReplaySet) {
    if let BootReplaySet::Sealed(replay) = replay {
        replay.sources = None;
    }
}
