//! Version ordering — the freshest-installed rule (B-028).
//!
//! An unpinned `spec://` address resolves to the **newest installed** version
//! (semver-newest; the owner's optional-version rule, B-028 2026-08-04). This
//! module is a small semver subset — no `semver` crate, so the resolver stays
//! dependency-light — just enough to order the materialised `vibedeps/` slots.

/// The semver-newest version string among `versions`, or `None` when the
/// iterator is empty (B-028's "freshest installed" rule).
pub(super) fn newest<'a>(versions: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    versions.max_by(|a, b| cmp_versions(a, b))
}

/// Compare two version strings for freshest-selection: [`Ordering::Greater`]
/// means `a` is newer than `b`. A small semver subset, sufficient for vibedeps
/// slots:
///
/// * the dotted core (`major.minor.patch`) compares segment by segment;
/// * an all-digit segment compares numerically (`10` > `9`), any other by ASCII
///   (so `0.9.0` < `0.10.0`, the lexicographic trap a real semver avoids);
/// * a version with more segments at an equal prefix is newer (`1.0.1` > `1.0`);
/// * a pre-release tail after `-` makes a version older than the same without
///   one (`1.0.0-alpha` < `1.0.0`), and two tails compare by the same segment
///   rules through `.` (`1.0.0-alpha.2` > `1.0.0-alpha.1`).
fn cmp_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let (a_core, a_pre) = split_pre(a);
    let (b_core, b_pre) = split_pre(b);
    match cmp_dotted(a_core, b_core) {
        std::cmp::Ordering::Equal => cmp_pre(a_pre, b_pre),
        ord => ord,
    }
}

/// Split a version into its dotted core and the optional pre-release tail.
fn split_pre(s: &str) -> (&str, Option<&str>) {
    match s.split_once('-') {
        Some((core, pre)) => (core, Some(pre)),
        None => (s, None),
    }
}

/// Compare two pre-release tails: a present tail is older than none; two tails
/// compare by the dotted-segment rules.
fn cmp_pre(a: Option<&str>, b: Option<&str>) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (a, b) {
        (None, None) => Ordering::Equal,
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (Some(x), Some(y)) => cmp_dotted(x, y),
    }
}

/// Compare two dotted strings segment by segment (more segments at an equal
/// prefix is newer).
fn cmp_dotted(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let mut ai = a.split('.');
    let mut bi = b.split('.');
    loop {
        match (ai.next(), bi.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) => match cmp_segment(x, y) {
                Ordering::Equal => continue,
                ord => return ord,
            },
        }
    }
}

/// Compare a single dotted segment: all-digit segments numerically, the rest by
/// ASCII; a numeric segment is older than an alphabetic one (a total order for
/// the mixed case, e.g. a pre-release `alpha` vs `1`).
fn cmp_segment(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (is_numeric_segment(a), is_numeric_segment(b)) {
        (true, true) => {
            let na: u64 = a.parse().unwrap_or(0);
            let nb: u64 = b.parse().unwrap_or(0);
            na.cmp(&nb)
        }
        (false, false) => a.cmp(b),
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
    }
}

/// A non-empty segment of ASCII digits only.
fn is_numeric_segment(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_comparator_pairs() {
        // The semver-subset comparator as a table of (a, b, expected a-vs-b).
        use std::cmp::Ordering;
        let cases: &[(&str, &str, Ordering)] = &[
            // numeric, not lexicographic
            ("0.9.0", "0.10.0", Ordering::Less),
            ("0.10.0", "0.9.0", Ordering::Greater),
            // major dominates
            ("2.0.0", "1.9.9", Ordering::Greater),
            // more segments at an equal prefix is newer
            ("1.2", "1.2.1", Ordering::Less),
            ("1.2.1", "1.2", Ordering::Greater),
            // equality
            ("1.0.0", "1.0.0", Ordering::Equal),
            // a pre-release tail is older than the release
            ("1.0.0-alpha", "1.0.0", Ordering::Less),
            ("1.0.0", "1.0.0-alpha", Ordering::Greater),
            // pre-release tails compare by dotted segments
            ("1.0.0-alpha.1", "1.0.0-alpha.2", Ordering::Less),
            ("1.0.0-alpha.2", "1.0.0-alpha.1", Ordering::Greater),
            // more pre-release segments is newer (semver)
            ("1.0.0-alpha", "1.0.0-alpha.1", Ordering::Less),
            // alphabetic pre-release segments compare by ASCII
            ("1.0.0-alpha", "1.0.0-beta", Ordering::Less),
            ("1.0.0-beta", "1.0.0-alpha", Ordering::Greater),
        ];
        for &(a, b, expect) in cases {
            assert_eq!(cmp_versions(a, b), expect, "cmp_versions({a:?}, {b:?})");
        }
    }

    #[test]
    fn newest_picks_the_greatest_or_none() {
        assert_eq!(newest(std::iter::empty::<&str>()), None);
        assert_eq!(newest(["1.0.0"].into_iter()), Some("1.0.0"));
        assert_eq!(
            newest(["0.9.0", "0.10.0", "0.9.5"].into_iter()),
            Some("0.10.0")
        );
        assert_eq!(
            newest(["1.0.0-alpha", "1.0.0", "1.0.0-beta"].into_iter()),
            Some("1.0.0")
        );
    }
}
