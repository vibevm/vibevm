//! Unit reds for the pieces a whole document cannot reach cheaply: the
//! canonical snapshot name in both forms, the saturating arithmetic, the
//! bounded scalar rendering, and the integer ceiling whose real trigger
//! would need four billion events.

use super::aggregates::{increment_count, is_canonical, saturating_add_into};
use super::snapshot::SnapshotIdentity;
use super::*;
use crate::generated::compiler_trace_index::e1::index::{PassShape, ScopeKind};

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

/// The public writer builder and the validator are one codec: every name
/// the builder hands out is a name the validator then reconstructs and
/// accepts for the very same event.
#[test]
fn the_public_builder_round_trips_through_the_validator() {
    let kind = ScopeKind::Node;
    for (sequence, invocation, pass, label, artifact) in [
        (0, 0, "parse", ".", "static-md"),
        (
            12_345,
            6_789,
            "emit:static-xml",
            "members/tool",
            "static-xml",
        ),
        (7, 2, "close", "узел/☃", "static-md"),
    ] {
        let name = SnapshotName {
            sequence,
            invocation,
            kind: &kind,
            pass,
            label,
            artifact,
        };
        let identity = SnapshotIdentity {
            sequence,
            invocation,
            kind: kind_spelling(&kind),
            pass,
            label,
            artifact,
        };
        if let Some(full) = name.full() {
            assert!(full.len() <= SNAPSHOT_NAME_CAP);
            assert_eq!(snapshot_unsafety(&full, &identity), None, "{full}");
        }
        let short = name.short();
        assert_eq!(snapshot_unsafety(&short, &identity), None, "{short}");
        // The short form is the one with a RECOMPUTED digest, so it must
        // not be transferable to a neighbouring event.
        let neighbour = SnapshotIdentity {
            pass: "some-other-pass",
            ..identity
        };
        assert!(snapshot_unsafety(&short, &neighbour).is_some());
    }
}

/// `within` is the writer's ceiling, taken together with the epoch's own:
/// the full form while it fits, the short one under pressure, and an
/// honest `None` when even 31 bytes are unaffordable.
#[test]
fn the_writer_ceiling_chooses_full_then_short_then_refuses() {
    let kind = ScopeKind::Unit;
    let name = SnapshotName {
        sequence: 0,
        invocation: 0,
        kind: &kind,
        pass: "emit:static-xml",
        label: "org.demo.tool",
        artifact: "static-xml",
    };
    let full = name.full().expect("this middle fits the epoch cap");
    let short = name.short();
    assert_eq!(
        name.within(SNAPSHOT_NAME_CAP).as_deref(),
        Some(full.as_str())
    );
    // A ceiling above the epoch's own never widens it.
    assert_eq!(name.within(usize::MAX).as_deref(), Some(full.as_str()));
    assert_eq!(name.within(full.len()).as_deref(), Some(full.as_str()));
    assert_eq!(name.within(full.len() - 1).as_deref(), Some(short.as_str()));
    assert_eq!(name.within(short.len()).as_deref(), Some(short.as_str()));
    assert_eq!(name.within(short.len() - 1), None);
    assert_eq!(name.within(0), None);
    assert_eq!(short.len(), 31, "the minimal short form is 31 bytes");
}

/// A label that is hostile in SIZE and in ENCODING still costs a bounded
/// answer, and the answer is still a name the validator reconstructs.
#[test]
fn a_hostile_label_still_yields_one_bounded_canonical_name() {
    let kind = ScopeKind::Publish;
    let huge = "☃\u{202e}%~".repeat(200_000);
    let name = SnapshotName {
        sequence: 9,
        invocation: 3,
        kind: &kind,
        pass: &huge,
        label: &huge,
        artifact: "static-md",
    };
    assert_eq!(name.full(), None, "no full form survives this middle");
    let chosen = name.within(SNAPSHOT_NAME_CAP).expect("the short form fits");
    assert!(chosen.len() <= SNAPSHOT_NAME_CAP);
    assert!(chosen.is_ascii(), "the codec emits ASCII only: {chosen}");
    assert_eq!(chosen, name.short());
    assert_eq!(
        snapshot_unsafety(
            &chosen,
            &SnapshotIdentity {
                sequence: 9,
                invocation: 3,
                kind: kind_spelling(&kind),
                pass: &huge,
                label: &huge,
                artifact: "static-md",
            }
        ),
        None
    );
}

/// A name built exactly AT the epoch cap is admissible; one byte more of
/// label pushes the same event onto the short form.
#[test]
fn the_full_form_is_admissible_exactly_at_the_cap() {
    let kind = ScopeKind::Node;
    // `0000-parse-node_<label>_static%2Dmd-000.json` — everything but the
    // label is fixed, so the label is what tunes the total length.
    let fixed = SnapshotName {
        sequence: 0,
        invocation: 0,
        kind: &kind,
        pass: "parse",
        label: "",
        artifact: "static-md",
    }
    .full()
    .expect("the empty-label spelling fits");
    let room = SNAPSHOT_NAME_CAP - fixed.len();
    let exact = "a".repeat(room);
    let one_more = "a".repeat(room + 1);
    let at_cap = SnapshotName {
        label: &exact,
        ..fixed_name(&kind)
    };
    let full = at_cap.full().expect("exactly at the cap still fits");
    assert_eq!(full.len(), SNAPSHOT_NAME_CAP);

    let over_cap = SnapshotName {
        label: &one_more,
        ..fixed_name(&kind)
    };
    assert_eq!(over_cap.full(), None);
    assert_eq!(
        over_cap.within(SNAPSHOT_NAME_CAP).as_deref(),
        Some(over_cap.short().as_str())
    );
}

fn fixed_name(kind: &ScopeKind) -> SnapshotName<'_> {
    SnapshotName {
        sequence: 0,
        invocation: 0,
        kind,
        pass: "parse",
        label: "",
        artifact: "static-md",
    }
}

/// The public aggregate builder is the validator's own recomputation, so
/// what it produces reconciles by construction — including the saturating
/// column arithmetic and the first-appearance row order.
#[test]
fn the_public_aggregate_builder_is_what_the_validator_recomputes() {
    let events = vec![
        event("parse", Some(dur(10, false)), Some(dur(1, false)), None),
        event("close", Some(dur(5, false)), Some(dur(2, false)), None),
        event(
            "parse",
            Some(dur(u32::MAX, false)),
            Some(dur(3, false)),
            Some(dur(4, false)),
        ),
    ];
    let rows = build_aggregates(&events).expect("three events reconcile");
    assert_eq!(
        rows.iter().map(|row| row.pass.as_str()).collect::<Vec<_>>(),
        vec!["parse", "close"],
        "row order is first appearance, not alphabetical",
    );
    assert_eq!(rows[0].invocations, 2);
    assert_eq!(rows[0].pass_total, dur(u32::MAX, true));
    assert_eq!(rows[0].verify_total, dur(4, false));
    assert_eq!(rows[0].encode_total, dur(4, false));
    assert_eq!(rows[1].invocations, 1);
    aggregate_gate(&events, &rows).expect("a built table is a reconciling table");

    // Empty in, empty out — and still reconciling.
    let empty = build_aggregates(&[]).expect("no events is a legal table");
    assert!(empty.is_empty());
    aggregate_gate(&[], &empty).expect("an empty table reconciles");
}

fn event(
    pass: &str,
    pass_micros: Option<Duration>,
    verify_micros: Option<Duration>,
    encode_micros: Option<Duration>,
) -> PassEvent {
    PassEvent {
        input_shape: PassShape {
            cardinality: IrCardinality::Document,
            level: IrLevel::Source,
        },
        output_shape: PassShape {
            cardinality: IrCardinality::Document,
            level: IrLevel::Document,
        },
        invocation: 0,
        pass: pass.to_string(),
        scope: "node:.".to_string(),
        sequence: 0,
        status: PassStatus::Ok,
        diagnostic: None,
        encode_micros,
        pass_micros,
        snapshot: None,
        verify_micros,
    }
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
