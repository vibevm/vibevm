//! Existing bounded duration projection, unchanged.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-RUN-RECORD");
use std::time::Instant;

pub(super) fn elapsed_ms(started: Instant) -> u32 {
    u32::try_from(started.elapsed().as_millis()).unwrap_or(u32::MAX)
}
