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

/// Renders a Unix-milliseconds timestamp as a UTC calendar day
/// (`YYYY-MM-DD`) — the metrics `by_day` bucket key. Pure arithmetic
/// (the days-to-civil algorithm from Howard Hinnant's calendar notes),
/// keeping the deliberate no-calendar-crate stance of the D11 stack.
pub fn utc_date_string(ms: u64) -> String {
    let days = (ms / 86_400_000) as i64;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097); // day of era [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // year of era
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // day of year (Mar-based)
    let mp = (5 * doy + 2) / 153; // month index, Mar = 0
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
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

    #[test]
    fn utc_dates_hit_known_anchors() {
        assert_eq!(utc_date_string(0), "1970-01-01");
        // 2000-02-29 12:00:00 UTC — a leap day deep in a 400-year era.
        assert_eq!(utc_date_string(951_825_600_000), "2000-02-29");
        // 2026-07-10 00:00:00 UTC and one ms before it.
        assert_eq!(utc_date_string(1_783_641_600_000), "2026-07-10");
        assert_eq!(utc_date_string(1_783_641_599_999), "2026-07-09");
        // Year boundary.
        assert_eq!(utc_date_string(1_735_689_600_000 - 1), "2024-12-31");
        assert_eq!(utc_date_string(1_735_689_600_000), "2025-01-01");
    }
}
