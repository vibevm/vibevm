//! UTC timestamp formatting without a date-crate dependency.
//!
//! Used by `vibe init`, `vibe install`, `vibe uninstall` for lockfile
//! `generated_at` entries, and by `vibe-registry` for the per-registry
//! `meta.toml` `last_pulled_at` field. The format is
//! `YYYY-MM-DDTHH:MM:SSZ` — RFC 3339 without sub-seconds or offsets,
//! which is all the spec-level consumers need.

use std::time::{SystemTime, UNIX_EPOCH};

/// Current wall-clock time in RFC 3339 UTC: `YYYY-MM-DDTHH:MM:SSZ`.
///
/// Falls back to the Unix epoch (`1970-01-01T00:00:00Z`) if the system
/// clock is before 1970. The fallback is deliberate: lockfile IO must
/// never fail because of a pathological clock.
pub fn now_utc() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format_unix_utc(secs)
}

/// Render a UNIX epoch in seconds as `YYYY-MM-DDTHH:MM:SSZ` using the
/// Gregorian proleptic calendar.
pub fn format_unix_utc(secs: u64) -> String {
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let hour = rem / 3600;
    let minute = (rem / 60) % 60;
    let second = rem % 60;

    let (year, month, day) = gregorian_from_days(days as i64);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// Howard Hinnant's `civil_from_days`, adapted to `i64`.
fn gregorian_from_days(days_since_epoch: i64) -> (i64, u32, u32) {
    let z = days_since_epoch + 719_468;
    let era = z.div_euclid(146_097);
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

/// Parse `YYYY-MM-DDTHH:MM:SSZ` back into a UNIX epoch in seconds.
///
/// Returns `None` on malformed input. Accepts only the exact shape the
/// crate itself produces — this is not a general RFC 3339 parser.
pub fn parse_unix_utc(s: &str) -> Option<u64> {
    let bytes = s.as_bytes();
    if bytes.len() != 20 || bytes[4] != b'-' || bytes[7] != b'-' || bytes[10] != b'T'
        || bytes[13] != b':' || bytes[16] != b':' || bytes[19] != b'Z'
    {
        return None;
    }
    let year: i64 = s.get(..4)?.parse().ok()?;
    let month: u32 = s.get(5..7)?.parse().ok()?;
    let day: u32 = s.get(8..10)?.parse().ok()?;
    let hour: u64 = s.get(11..13)?.parse().ok()?;
    let minute: u64 = s.get(14..16)?.parse().ok()?;
    let second: u64 = s.get(17..19)?.parse().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let days = days_from_gregorian(year, month, day)?;
    Some((days as u64) * 86_400 + hour * 3600 + minute * 60 + second)
}

/// Inverse of `gregorian_from_days`.
fn days_from_gregorian(y: i64, m: u32, d: u32) -> Option<i64> {
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = (y - era * 400) as u64;
    let mp = if m > 2 { m - 3 } else { m + 9 } as u64;
    let doy = (153 * mp + 2) / 5 + (d as u64) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146_097 + doe as i64 - 719_468)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_formats() {
        assert_eq!(format_unix_utc(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn known_date_roundtrips() {
        // 2026-04-22T14:30:00Z — well inside the supported range.
        // 20_565 days since 1970-01-01 + 52_200 seconds = 1_776_868_200.
        let s = format_unix_utc(1_776_868_200);
        assert_eq!(s, "2026-04-22T14:30:00Z");
        assert_eq!(parse_unix_utc(&s), Some(1_776_868_200));
    }

    #[test]
    fn leap_year_edge() {
        // 2024-02-29 is a leap day. Unix seconds = 1_709_164_800.
        let s = format_unix_utc(1_709_164_800);
        assert_eq!(s, "2024-02-29T00:00:00Z");
        assert_eq!(parse_unix_utc(&s), Some(1_709_164_800));
    }

    #[test]
    fn parse_rejects_malformed() {
        assert_eq!(parse_unix_utc("2026-04-22"), None);
        assert_eq!(parse_unix_utc("2026-04-22T14:30:00"), None);
        assert_eq!(parse_unix_utc(""), None);
        assert_eq!(parse_unix_utc("2026-13-01T00:00:00Z"), None);
    }
}
