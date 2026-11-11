//! The few facts about a certificate that pinning needs, read straight
//! from its DER: whether it may be a trust anchor, when it is valid, and
//! which names it is for (`PROP-068#proxy`).
//!
//! A pinned certificate is accepted because its bytes are the bytes the
//! operator put in the trust set — not because a path was built to it — so
//! the two checks webpki would otherwise have made have to be made here:
//! the certificate must be current, and it must be for the host being
//! contacted. Nothing else is read, and a certificate this module cannot
//! parse is simply not pinnable.

use std::net::IpAddr;

/// One DER value: its tag, its contents and whatever follows it.
fn item(data: &[u8]) -> Option<(u8, &[u8], &[u8])> {
    let (&tag, rest) = data.split_first()?;
    let (&first, rest) = rest.split_first()?;
    let (len, rest) = if first & 0x80 == 0 {
        (usize::from(first), rest)
    } else {
        let count = usize::from(first & 0x7f);
        if count == 0 || count > 4 || rest.len() < count {
            return None;
        }
        let len = rest[..count]
            .iter()
            .fold(0usize, |acc, byte| (acc << 8) | usize::from(*byte));
        (len, &rest[count..])
    };
    if rest.len() < len {
        return None;
    }
    Some((tag, &rest[..len], &rest[len..]))
}

/// The values of a SEQUENCE, in order.
fn items(mut data: &[u8]) -> Vec<(u8, &[u8])> {
    let mut out = Vec::new();
    while let Some((tag, value, rest)) = item(data) {
        out.push((tag, value));
        data = rest;
    }
    out
}

/// What one certificate says about itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Facts {
    /// `basicConstraints` says `cA = TRUE`: the certificate may sign
    /// others, so it can be a trust anchor rather than only a pin.
    pub(crate) is_ca: bool,
    /// `notBefore` / `notAfter` as seconds since the Unix epoch.
    pub(crate) not_before: i64,
    pub(crate) not_after: i64,
    /// `subjectAltName` dNSName entries, lower-cased.
    pub(crate) dns_names: Vec<String>,
    /// `subjectAltName` iPAddress entries.
    pub(crate) ip_addresses: Vec<IpAddr>,
    /// Whether the certificate carries a `subjectAltName` at all. Without
    /// one the subject's common name is the only name it has, which is how
    /// a hand-made self-signed certificate usually looks.
    pub(crate) has_san: bool,
    /// The subject's common name, lower-cased.
    pub(crate) common_name: Option<String>,
}

impl Facts {
    /// Read what can be read; `None` when the bytes are not a certificate.
    pub(crate) fn parse(der: &[u8]) -> Option<Self> {
        let (_, certificate, _) = item(der)?;
        let (_, tbs, _) = item(certificate)?;
        let fields = items(tbs);
        // `[0] EXPLICIT version` is optional; everything after it shifts.
        let skip = usize::from(fields.first().is_some_and(|(tag, _)| *tag == 0xa0));
        let (_, validity) = *fields.get(skip + 3)?;
        let (_, subject) = *fields.get(skip + 4)?;
        let validity = items(validity);
        let not_before = time_seconds(*validity.first()?)?;
        let not_after = time_seconds(*validity.get(1)?)?;
        let extensions = fields
            .iter()
            .skip(skip + 6)
            .find(|(tag, _)| *tag == 0xa3)
            .and_then(|(_, value)| item(value))
            .map(|(_, sequence, _)| items(sequence))
            .unwrap_or_default();
        let mut facts = Self {
            is_ca: false,
            not_before,
            not_after,
            dns_names: Vec::new(),
            ip_addresses: Vec::new(),
            has_san: false,
            common_name: common_name(subject),
        };
        for (_, extension) in extensions {
            let parts = items(extension);
            let Some((_, oid)) = parts.first().copied() else {
                continue;
            };
            // The OCTET STRING holding the extension is last: `critical`
            // sits between the OID and it only when it is `TRUE`.
            let Some((_, body)) = parts.last().copied() else {
                continue;
            };
            match oid {
                // 2.5.29.19 basicConstraints
                [0x55, 0x1d, 0x13] => {
                    if let Some((_, constraints, _)) = item(body) {
                        facts.is_ca = items(constraints)
                            .first()
                            .is_some_and(|(tag, value)| *tag == 0x01 && *value != [0x00]);
                    }
                }
                // 2.5.29.17 subjectAltName
                [0x55, 0x1d, 0x11] => {
                    if let Some((_, names, _)) = item(body) {
                        facts.has_san = true;
                        for (tag, value) in items(names) {
                            match tag {
                                // [2] dNSName
                                0x82 => facts
                                    .dns_names
                                    .push(String::from_utf8_lossy(value).to_ascii_lowercase()),
                                // [7] iPAddress
                                0x87 => {
                                    if let Ok(four) = <[u8; 4]>::try_from(value) {
                                        facts.ip_addresses.push(IpAddr::from(four));
                                    } else if let Ok(sixteen) = <[u8; 16]>::try_from(value) {
                                        facts.ip_addresses.push(IpAddr::from(sixteen));
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        Some(facts)
    }

    /// Whether this certificate is for `host` — a `subjectAltName` entry,
    /// or the common name when the certificate carries no `subjectAltName`
    /// at all. A wildcard matches exactly one label, as everywhere else.
    pub(crate) fn is_for(&self, host: &str) -> bool {
        let host = host
            .trim_start_matches('[')
            .trim_end_matches(']')
            .trim_end_matches('.')
            .to_ascii_lowercase();
        if let Ok(address) = host.parse::<IpAddr>() {
            return self.ip_addresses.contains(&address);
        }
        if self.has_san {
            return self.dns_names.iter().any(|name| matches_name(name, &host));
        }
        self.common_name
            .as_deref()
            .is_some_and(|name| matches_name(name, &host))
    }

    /// Whether `now` (seconds since the Unix epoch) is inside the
    /// certificate's own validity window.
    pub(crate) fn is_current(&self, now: i64) -> bool {
        self.not_before <= now && now <= self.not_after
    }
}

/// Whether a certificate name covers a host name, with one leading
/// wildcard label allowed.
///
/// `*.example.test` covers `a.example.test` and neither `example.test` nor
/// `a.b.example.test`: a wildcard stands for exactly one label, which is
/// the rule every other verifier on the path applies.
fn matches_name(name: &str, host: &str) -> bool {
    let name = name.trim_end_matches('.');
    if let Some(suffix) = name.strip_prefix("*.") {
        return host
            .split_once('.')
            .is_some_and(|(label, rest)| !label.is_empty() && rest == suffix);
    }
    name == host
}

/// The common name of a `Name`, lower-cased.
fn common_name(name: &[u8]) -> Option<String> {
    for (_, set) in items(name) {
        for (_, attribute) in items(set) {
            let parts = items(attribute);
            let (_, oid) = parts.first().copied()?;
            if oid == [0x55, 0x04, 0x03]
                && let Some((_, value)) = parts.get(1).copied()
            {
                return Some(String::from_utf8_lossy(value).to_ascii_lowercase());
            }
        }
    }
    None
}

/// A `UTCTime` (`YYMMDDHHMMSSZ`) or `GeneralizedTime` (`YYYYMMDDHHMMSSZ`)
/// as seconds since the Unix epoch.
fn time_seconds((tag, value): (u8, &[u8])) -> Option<i64> {
    let text = std::str::from_utf8(value).ok()?;
    let text = text.strip_suffix('Z')?;
    let (year, rest) = match tag {
        // UTCTime's two-digit year: 50..=99 is 1950..=1999 (RFC 5280).
        0x17 => {
            let (digits, rest) = text.split_at_checked(2)?;
            let two: i64 = digits.parse().ok()?;
            (if two >= 50 { 1900 + two } else { 2000 + two }, rest)
        }
        0x18 => {
            let (digits, rest) = text.split_at_checked(4)?;
            (digits.parse().ok()?, rest)
        }
        _ => return None,
    };
    if rest.len() < 10 {
        return None;
    }
    let number = |range: std::ops::Range<usize>| rest.get(range)?.parse::<i64>().ok();
    let month = number(0..2)?;
    let day = number(2..4)?;
    let hour = number(4..6)?;
    let minute = number(6..8)?;
    let second = number(8..10)?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some(days_from_civil(year, month, day) * 86_400 + hour * 3_600 + minute * 60 + second)
}

/// Days from 1970-01-01 to `year-month-day`, by Howard Hinnant's
/// `days_from_civil` — the standard closed form, no calendar table.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_epoch_and_a_few_known_days_line_up() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(1970, 1, 2), 1);
        assert_eq!(days_from_civil(1969, 12, 31), -1);
        assert_eq!(days_from_civil(2000, 3, 1), 11_017);
        assert_eq!(days_from_civil(2026, 1, 1), 20_454);
    }

    #[test]
    fn both_time_encodings_read_as_the_same_instant() {
        let utc = time_seconds((0x17, b"260101000000Z")).unwrap();
        let generalized = time_seconds((0x18, b"20260101000000Z")).unwrap();
        assert_eq!(utc, generalized);
        assert_eq!(utc, 20_454 * 86_400);
    }

    #[test]
    fn a_two_digit_year_of_fifty_or_more_is_the_twentieth_century() {
        // RFC 5280 §4.1.2.5.1, the rule that makes `49` and `50` fifty-one
        // years apart rather than one.
        assert_eq!(time_seconds((0x17, b"990101000000Z")), Some(915_148_800));
        assert_eq!(time_seconds((0x17, b"490101000000Z")), Some(2_493_072_000));
        assert!(
            time_seconds((0x17, b"500101000000Z")).unwrap()
                < time_seconds((0x17, b"490101000000Z")).unwrap()
        );
    }

    #[test]
    fn a_time_without_the_zulu_marker_is_not_read() {
        assert_eq!(time_seconds((0x17, b"260101000000")), None);
        assert_eq!(time_seconds((0x05, b"260101000000Z")), None);
    }

    #[test]
    fn a_wildcard_stands_for_exactly_one_label() {
        assert!(matches_name("*.example.test", "a.example.test"));
        assert!(!matches_name("*.example.test", "example.test"));
        assert!(!matches_name("*.example.test", "a.b.example.test"));
        assert!(!matches_name("*.example.test", ".example.test"));
        assert!(matches_name("example.test.", "example.test"));
    }

    #[test]
    fn bytes_that_are_not_a_certificate_are_simply_not_pinnable() {
        assert_eq!(Facts::parse(b"not a certificate"), None);
        assert_eq!(Facts::parse(&[]), None);
    }
}
