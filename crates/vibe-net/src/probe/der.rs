//! A minimal DER reader for a certificate's subject and issuer names.

use super::CertSummary;

fn der_item(data: &[u8]) -> Option<(u8, &[u8], &[u8])> {
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
            .fold(0usize, |acc, b| (acc << 8) | usize::from(*b));
        (len, &rest[count..])
    };
    if rest.len() < len {
        return None;
    }
    Some((tag, &rest[..len], &rest[len..]))
}

fn name_text(name: &[u8]) -> String {
    let mut parts = Vec::new();
    let mut sets = name;
    while let Some((_, set, rest)) = der_item(sets) {
        sets = rest;
        let mut attrs = set;
        while let Some((_, attr, more)) = der_item(attrs) {
            attrs = more;
            let Some((_, oid, value)) = der_item(attr) else {
                continue;
            };
            let Some((_, value, _)) = der_item(value) else {
                continue;
            };
            let key = match oid {
                [0x55, 0x04, 0x03] => "CN",
                [0x55, 0x04, 0x0a] => "O",
                [0x55, 0x04, 0x0b] => "OU",
                [0x55, 0x04, 0x06] => "C",
                _ => continue,
            };
            parts.push(format!("{key}={}", String::from_utf8_lossy(value)));
        }
    }
    if parts.is_empty() {
        "<unnamed>".to_owned()
    } else {
        parts.join(", ")
    }
}

/// The subject and issuer of a DER certificate.
///
/// ```
/// let summary = vibe_net::probe::summarize(b"not a certificate");
/// assert_eq!(summary.subject, "<unreadable>");
/// ```
pub fn summarize(der: &[u8]) -> CertSummary {
    let unreadable = || CertSummary {
        subject: "<unreadable>".into(),
        issuer: "<unreadable>".into(),
    };
    let Some((_, certificate, _)) = der_item(der) else {
        return unreadable();
    };
    let Some((_, tbs, _)) = der_item(certificate) else {
        return unreadable();
    };
    let mut fields = Vec::new();
    let mut rest = tbs;
    while let Some((tag, value, more)) = der_item(rest) {
        fields.push((tag, value));
        rest = more;
    }
    // [0] version is optional; then serial, signature, issuer, validity, subject.
    let skip = usize::from(fields.first().is_some_and(|(tag, _)| *tag == 0xa0));
    match (fields.get(skip + 2), fields.get(skip + 4)) {
        (Some((_, issuer)), Some((_, subject))) => CertSummary {
            subject: name_text(subject),
            issuer: name_text(issuer),
        },
        _ => unreadable(),
    }
}
