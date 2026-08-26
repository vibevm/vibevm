//! One reversible spelling per project name — no two names print one key.

use specmark::verifies;

use super::{HostOwner, HostSegmentError, decode_host_segment, encode_host_segment};

/// Every legal `[project].name` — arbitrary UTF-8, delimiters and all —
/// round-trips through the canonical spelling, and ordinary names are
/// byte-identical to what they have always been.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#HOST-ACTIVATION")]
fn every_project_name_round_trips_through_one_spelling() {
    for (project, segment) in [
        // Unchanged for the ordinary case — this is the compatibility law.
        ("demo", "demo"),
        ("my-project.v2_final~", "my-project.v2_final~"),
        ("", ""),
        ("my app", "my%20app"),
        ("a/b#c@d:e\\f%g", "a%2Fb%23c%40d%3Ae%5Cf%25g"),
        ("line\nbreak\ttab", "line%0Abreak%09tab"),
        ("nul\u{0}byte", "nul%00byte"),
        ("проект", "%D0%BF%D1%80%D0%BE%D0%B5%D0%BA%D1%82"),
        (
            "プロジェクト",
            "%E3%83%97%E3%83%AD%E3%82%B8%E3%82%A7%E3%82%AF%E3%83%88",
        ),
        // A literal `%20` cannot alias the encoding of a space.
        ("%20", "%2520"),
        ("100%", "100%25"),
    ] {
        assert_eq!(encode_host_segment(project), segment, "{project:?}");
        assert_eq!(decode_host_segment(segment).as_deref(), Ok(project));

        let owner = HostOwner::new(project);
        assert_eq!(owner.segment(), segment);
        assert_eq!(owner.to_string(), format!("__host__/{segment}"));
        assert_eq!(HostOwner::parse(&owner.to_string()), Ok(owner.clone()));
        assert_eq!(owner.project(), project);
    }

    // The two names that used to collide now cannot.
    assert_ne!(
        HostOwner::new("odd/# project").to_string(),
        HostOwner::new("odd").to_string(),
    );
    assert_eq!(
        HostOwner::new("odd/# project").to_string(),
        "__host__/odd%2F%23%20project"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#HOST-ACTIVATION")]
fn malformed_and_noncanonical_segments_refuse() {
    for (segment, fault) in [
        ("my app", HostSegmentError::UnescapedByte),
        ("a/b", HostSegmentError::UnescapedByte),
        ("a#b", HostSegmentError::UnescapedByte),
        ("%", HostSegmentError::TruncatedEscape),
        ("%2", HostSegmentError::TruncatedEscape),
        ("a%", HostSegmentError::TruncatedEscape),
        ("%2f", HostSegmentError::LowercaseEscape),
        ("%ZZ", HostSegmentError::MalformedEscape),
        ("%2G", HostSegmentError::MalformedEscape),
        // `-` is unreserved, so `%2D` is a second spelling of `-`.
        ("%2D", HostSegmentError::NonCanonical),
        ("%41", HostSegmentError::NonCanonical),
        ("%7E", HostSegmentError::NonCanonical),
        // A lone continuation byte decodes to invalid UTF-8.
        ("%80", HostSegmentError::InvalidUtf8),
        ("%FF", HostSegmentError::InvalidUtf8),
        ("%D0", HostSegmentError::InvalidUtf8),
    ] {
        assert_eq!(decode_host_segment(segment), Err(fault), "{segment:?}");
        assert!(!fault.reason().is_empty());
    }

    for spelling in ["demo", "org.example/demo", "__host__demo", "__host_/demo"] {
        assert_eq!(
            HostOwner::parse(spelling),
            Err(HostSegmentError::NotAHostOwner),
            "{spelling}"
        );
    }
}

/// The decoder accepts exactly what the encoder emits, for every byte.
#[test]
fn the_codec_is_a_bijection_over_every_byte() {
    for byte in 0u8..=0x7F {
        let project = String::from_utf8(vec![byte]).unwrap();
        let segment = encode_host_segment(&project);
        assert_eq!(decode_host_segment(&segment).as_deref(), Ok(&project[..]));
    }
    for scalar in ['é', 'ß', '中', '💡', '\u{80}', '\u{10FFFF}'] {
        let project = scalar.to_string();
        let segment = encode_host_segment(&project);
        assert_eq!(decode_host_segment(&segment).as_deref(), Ok(&project[..]));
    }
}
