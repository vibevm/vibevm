//! The wall clock of a transfer: RFC 3339 instants with milliseconds, the
//! precision `PROP-068#sampling` asks of every `at` and `started`.
//!
//! The workspace already formats seconds without a date crate
//! (`crate::timestamp`); this adds the millisecond tail on top of it rather
//! than pulling a date crate into `vibe-core` for one field.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::timestamp::{format_unix_utc, parse_unix_utc};

/// Milliseconds since the Unix epoch; a clock before 1970 reads as 0 so a
/// sample is never refused over a pathological clock.
pub(crate) fn unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or(0)
}

/// `YYYY-MM-DDTHH:MM:SS.mmmZ`.
pub(crate) fn format_millis(millis: u64) -> String {
    let seconds = format_unix_utc(millis / 1000);
    // `format_unix_utc` ends with `Z`; the milliseconds go before it.
    format!("{}.{:03}Z", &seconds[..seconds.len() - 1], millis % 1000)
}

/// The inverse of [`format_millis`]; an instant without a millisecond tail
/// is accepted too. Anything else is `None`.
pub(crate) fn parse_millis(text: &str) -> Option<u64> {
    let (whole, millis) = match text.split_once('.') {
        Some((whole, tail)) => {
            let digits = tail.strip_suffix('Z')?;
            if digits.is_empty() || digits.len() > 3 || !digits.bytes().all(|b| b.is_ascii_digit())
            {
                return None;
            }
            let scale = 10u64.pow(3 - digits.len() as u32);
            (format!("{whole}Z"), digits.parse::<u64>().ok()? * scale)
        }
        None => (text.to_owned(), 0),
    };
    Some(parse_unix_utc(&whole)? * 1000 + millis)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn milliseconds_round_trip() {
        let at = 1_761_955_200_123;
        let text = format_millis(at);
        assert_eq!(text, "2025-11-01T00:00:00.123Z");
        assert_eq!(parse_millis(&text), Some(at));
        assert_eq!(parse_millis("2025-11-01T00:00:00Z"), Some(at - 123));
        assert_eq!(parse_millis("2025-11-01T00:00:00.5Z"), Some(at - 123 + 500));
        assert_eq!(parse_millis("yesterday"), None);
    }
}
