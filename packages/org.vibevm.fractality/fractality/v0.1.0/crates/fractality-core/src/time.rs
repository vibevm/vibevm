//! Timestamps.
//!
//! The whole system speaks unix milliseconds (`u64`): ULIDs already encode
//! them, JSONL journals stay arithmetic-friendly, and no calendar crate is
//! needed (the D11 stack deliberately has none). Human-facing rendering is
//! relative ("3m12s ago"), which needs only arithmetic.

use std::time::{SystemTime, UNIX_EPOCH};

specmark::scope!("spec://fractality/PROP-001#model");

/// Milliseconds since the Unix epoch, now.
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Renders a duration in coarse human units: `42s`, `3m12s`, `2h05m`, `4d06h`.
pub fn format_duration_ms(ms: u64) -> String {
    let secs = ms / 1000;
    if secs < 60 {
        return format!("{secs}s");
    }
    let mins = secs / 60;
    if mins < 60 {
        return format!("{}m{:02}s", mins, secs % 60);
    }
    let hours = mins / 60;
    if hours < 24 {
        return format!("{}h{:02}m", hours, mins % 60);
    }
    format!("{}d{:02}h", hours / 24, hours % 24)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_rendering_uses_coarse_units() {
        assert_eq!(format_duration_ms(0), "0s");
        assert_eq!(format_duration_ms(42_000), "42s");
        assert_eq!(format_duration_ms(192_000), "3m12s");
        assert_eq!(format_duration_ms(2 * 3_600_000 + 5 * 60_000), "2h05m");
        assert_eq!(format_duration_ms(4 * 86_400_000 + 6 * 3_600_000), "4d06h");
    }
}
