//! Unit reds for the pieces a whole document cannot reach cheaply: the
//! canonical snapshot name in both forms, the saturating arithmetic, the
//! bounded scalar rendering, and the integer ceiling whose real trigger
//! would need four billion events.

use super::aggregates::{increment_count, is_canonical, saturating_add_into};
use super::snapshot::SnapshotIdentity;
use super::*;

fn dur(value: u32, saturated: bool) -> Duration {
    Duration {
        micros: value,
        saturated,
    }
}

/// The running example from the accepted refresh ruling 3: one `parse`
/// over the root node's `static-md` artifact.
fn parse_at_root(sequence: u32, invocation: u32) -> SnapshotIdentity<'static> {
    SnapshotIdentity {
        sequence,
        invocation,
        kind: "node",
        pass: "parse",
        label: ".",
        artifact: "static-md",
    }
}

fn refuse(filename: &str, identity: &SnapshotIdentity<'_>) -> SnapshotUnsafety {
    match snapshot_unsafety(filename, identity) {
        Some(reason) => reason,
        None => panic!("{filename:?} must be refused"),
    }
}

#[test]
fn the_full_canonical_name_is_the_one_the_event_spells() {
    // `-` and `:` are escaped, `.` and the alphanumerics are raw, the
    // widths are zero-padded, the kind comes from the scope.
    let identity = parse_at_root(0, 0);
    assert_eq!(
        snapshot_unsafety("0000-parse-node_._static%2Dmd-000.json", &identity),
        None
    );

    let emit = SnapshotIdentity {
        sequence: 2,
        invocation: 0,
        kind: "unit",
        pass: "emit:static-xml",
        label: "org.demo.tool",
        artifact: "static-xml",
    };
    assert_eq!(
        snapshot_unsafety(
            "0002-emit%3Astatic%2Dxml-unit_org.demo.tool_static%2Dxml-000.json",
            &emit
        ),
        None
    );
}

/// Widths are Rust MINIMUM widths: a value wider than the pad uses all
/// its digits rather than being truncated into the layout.
#[test]
fn wide_sequences_and_ordinals_use_all_their_digits() {
    let identity = parse_at_root(12_345, 6_789);
    assert_eq!(
        snapshot_unsafety("12345-parse-node_._static%2Dmd-6789.json", &identity),
        None
    );
    assert!(matches!(
        refuse("2345-parse-node_._static%2Dmd-6789.json", &identity),
        SnapshotUnsafety::NotCanonical { .. }
    ));
}

/// Every near-miss the refresh ruling names, refused because the name is
/// built and compared rather than screened.
#[test]
fn every_near_miss_of_the_canonical_name_is_refused() {
    let identity = parse_at_root(0, 0);
    for (filename, why) in [
        (
            "000-parse-node_._static%2Dmd-000.json",
            "short sequence pad",
        ),
        ("0000-parse-node_._static%2Dmd-00.json", "short ordinal pad"),
        (
            "0000-p%61rse-node_._static%2Dmd-000.json",
            "over-encoded %61",
        ),
        ("0000-parse-node_._static%2dmd-000.json", "lowercase escape"),
        ("0000-parse-node_._static-md-000.json", "raw hyphen"),
        ("0000-parse-node_._static_md-000.json", "raw underscore"),
        ("0000-parse-node_.~_static%2Dmd-000.json", "raw tilde"),
        ("0000-close-node_._static%2Dmd-000.json", "wrong pass"),
        ("0000-parse-unit_._static%2Dmd-000.json", "wrong kind"),
        ("0000-parse-node_x_static%2Dmd-000.json", "wrong label"),
        ("0000-parse-node_._static%2Dxml-000.json", "wrong artifact"),
        ("0001-parse-node_._static%2Dmd-000.json", "wrong sequence"),
        ("0000-parse-node_._static%2Dmd-001.json", "wrong ordinal"),
        ("0000-parse-node_._static%2Dmd-000.txt", "wrong suffix"),
        ("0000-~0123456789abcdef-000.json", "invented digest"),
        (
            "0000-parse-node_._static%2Dmd-000.json.bak",
            "trailing junk",
        ),
    ] {
        assert!(
            matches!(
                refuse(filename, &identity),
                SnapshotUnsafety::NotCanonical { .. }
            ),
            "{why}: {filename}"
        );
    }
}

/// The short form: `~` plus a RECOMPUTED digest, admissible whenever the
/// writer chose it, and refused the moment a hex digit moves.
#[test]
fn the_short_form_is_a_verified_digest_not_a_shape() {
    let identity = parse_at_root(0, 0);
    let real = "0000-~28b4b51b8d841175-000.json";
    assert_eq!(snapshot_unsafety(real, &identity), None);

    // One digit changed is a different digest, and therefore not a name
    // this event can have written.
    let forged = "0000-~28b4b51b8d841176-000.json";
    assert!(matches!(
        refuse(forged, &identity),
        SnapshotUnsafety::NotCanonical { .. }
    ));

    // Another event's real digest does not transfer.
    let other = SnapshotIdentity {
        pass: "close",
        ..parse_at_root(0, 0)
    };
    assert!(matches!(
        refuse(real, &other),
        SnapshotUnsafety::NotCanonical { .. }
    ));
}

/// When the full form would pass the 96-byte ceiling the short form is
/// the ONLY admissible spelling, and the refusal says so by carrying no
/// expected full name.
#[test]
fn an_overlong_full_form_leaves_only_the_short_name() {
    let identity = SnapshotIdentity {
        sequence: 0,
        invocation: 0,
        kind: "unit",
        pass: "emit:static-xml",
        label: "org.demo.enterprise.platform.reporting.subsystem",
        artifact: "static-xml",
    };
    assert_eq!(
        snapshot_unsafety("0000-~060ed29890b2fe3d-000.json", &identity),
        None
    );
    let long_full = "0000-emit%3Astatic%2Dxml-unit_org.demo.enterprise.platform.reporting.\
                     subsystem_static%2Dxml-000.json";
    assert!(long_full.len() > SNAPSHOT_NAME_CAP);
    assert!(matches!(
        refuse(long_full, &identity),
        SnapshotUnsafety::TooLong { .. }
    ));
    // A name within the cap but not the short one names only the short.
    assert!(matches!(
        refuse("0000-parse-node_._static%2Dmd-000.json", &identity),
        SnapshotUnsafety::NotCanonical { full: None, .. }
    ));
}

/// A hostile document must not be able to buy a proportional allocation
/// with a huge scalar: the over-cap filename is refused before anything
/// is built, and the full-name builder stops at the ceiling.
#[test]
fn a_huge_name_or_label_costs_a_bounded_refusal() {
    let identity = parse_at_root(0, 0);
    let huge_name = "0".repeat(4 * 1024 * 1024);
    assert_eq!(
        refuse(&huge_name, &identity),
        SnapshotUnsafety::TooLong {
            bytes: huge_name.len()
        }
    );

    let huge_label = "x".repeat(4 * 1024 * 1024);
    let huge = SnapshotIdentity {
        label: &huge_label,
        ..parse_at_root(0, 0)
    };
    // No full form fits, so only the (bounded, digest-bearing) short one
    // is named, and every rendered string stays small.
    let reason = refuse("0000-parse-node_._static%2Dmd-000.json", &huge);
    let SnapshotUnsafety::NotCanonical { full, short } = reason else {
        panic!("expected the not-canonical family");
    };
    assert!(full.is_none());
    assert!(short.bytes() <= SNAPSHOT_NAME_CAP);
    assert_eq!(short.head().len(), short.bytes());
}

#[test]
fn scalar_gate_refuses_blank_whitespace_and_line_breaks() {
    for value in ["", " ", "\t", "   \t ", "a\rb", "a\nb", "a\0b", "\r\n"] {
        assert!(
            scalar_gate("scope.id", value).is_err(),
            "{value:?} is not a safe identity scalar"
        );
    }
    for value in ["node:.", ROOT_DISPLAY, "org.demo.tool", "emit:static-xml"] {
        assert!(scalar_gate("scope.id", value).is_ok(), "{value:?} is safe");
    }
}

#[test]
fn custom_targets_obey_the_backend_id_charset() {
    for value in ["static-md", "static-xml", "acme.pdf-writer", "a", "x9"] {
        assert!(is_backend_id(value), "{value} is a backend id");
    }
    let too_long = "9".repeat(65);
    for value in [
        "",
        "-leading",
        ".dot",
        "Upper",
        "sp ace",
        "emit:x",
        too_long.as_str(),
    ] {
        assert!(!is_backend_id(value), "{value:?} is not a backend id");
    }
}

#[test]
fn saturation_never_wraps_and_sticks() {
    let mut total = dur(u32::MAX - 5, false);
    saturating_add_into(&mut total, &dur(10, false));
    assert_eq!(total, dur(u32::MAX, true));

    // An already-saturated total stays saturated at the ceiling when a
    // later addend contributes nothing.
    let mut sticky = dur(u32::MAX, true);
    saturating_add_into(&mut sticky, &dur(0, false));
    assert_eq!(sticky, dur(u32::MAX, true));

    // A saturated addend carries the flag into a fresh total, and the
    // result is itself canonical.
    let mut fresh = dur(0, false);
    saturating_add_into(&mut fresh, &dur(u32::MAX, true));
    assert_eq!(fresh, dur(u32::MAX, true));
    assert!(is_canonical(&fresh));

    // An exact measurement at the ceiling is legal without the flag; a
    // saturation marker anywhere below it is not.
    assert!(is_canonical(&dur(u32::MAX, false)));
    assert!(!is_canonical(&dur(1, true)));
    assert!(!is_canonical(&dur(0, true)));
}

/// The `uint32` sequence ceiling, exercised at the helper rather than by
/// allocating four billion events.
#[test]
#[cfg(target_pointer_width = "64")]
fn the_sequence_ceiling_refuses_instead_of_wrapping() {
    assert_eq!(dense_sequence(0), Ok(0));
    assert_eq!(dense_sequence(u32::MAX as usize), Ok(u32::MAX));
    let overflow = dense_sequence(u32::MAX as usize + 1).expect_err("past the uint32 ceiling");
    assert!(matches!(overflow, TraceIndexError::SequenceOverflow { .. }));
    assert_eq!(overflow.law(), "sequence-density");
}

/// The other two uint32 counters have the same hostile-input obligation as
/// the global sequence. Exercise their checked boundary directly rather than
/// attempting to allocate four billion events.
#[test]
fn invocation_and_aggregate_ceilings_refuse_instead_of_wrapping() {
    let mut ordinal = u32::MAX;
    let error = advance_invocation(&mut ordinal, "scope", "parse")
        .expect_err("an invocation past the uint32 ceiling is red");
    assert!(matches!(error, TraceIndexError::InvocationOverflow { .. }));
    assert_eq!(error.law(), "invocation-key");
    assert_eq!(ordinal, u32::MAX, "the refused counter never wraps");

    let mut count = u32::MAX;
    let error = increment_count(&mut count, "parse")
        .expect_err("an aggregate count past the uint32 ceiling is red");
    assert!(matches!(
        error,
        TraceIndexError::AggregateCountOverflow { .. }
    ));
    assert_eq!(error.law(), "aggregate-reconciliation");
    assert_eq!(count, u32::MAX, "the refused count never wraps");
}

#[test]
fn a_scalar_preview_is_bounded_and_never_splits_a_character() {
    // A three-byte character does not tile 64 bytes evenly, so the cut
    // genuinely has to walk back to a boundary.
    let huge = "☃".repeat(1_000_000);
    let preview = ScalarPreview::of(&huge);
    assert_eq!(preview.bytes(), 3_000_000);
    assert_eq!(preview.head().len(), 63);
    assert_eq!(preview.head().chars().count(), 21);
    assert!(preview.head().len() <= SCALAR_PREVIEW_BYTES);
    assert!(preview.is_truncated());
    assert!(format!("{preview}").ends_with("(3000000 bytes)"));

    let short = ScalarPreview::of("node:.");
    assert!(!short.is_truncated());
    assert_eq!(format!("{short}"), "\"node:.\"");
}

#[test]
fn implemented_law_labels_are_unique_and_schema_ordered() {
    let mut seen = BTreeSet::new();
    for law in IMPLEMENTED_LAWS {
        assert!(seen.insert(*law), "law {law} is listed twice");
    }
    assert_eq!(IMPLEMENTED_LAWS.len(), 14);
    assert_eq!(SHORT_DIGEST_HEX, 16);
    assert_eq!(SNAPSHOT_NAME_CAP, 96);
}
