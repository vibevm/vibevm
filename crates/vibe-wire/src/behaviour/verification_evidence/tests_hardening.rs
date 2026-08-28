//! The identity, text, count and scope hardening arms — the half of
//! the suite that proves the member's ADVERTISED laws are the ones the
//! code enforces, rather than a subset of them.
//!
//! Split from `tests.rs` by the `#[path]` idiom when the suite outgrew
//! the per-file budget, along the seam between «what a status may
//! mean» (there) and «what a scalar may be» (here).

use super::tests::{
    DIGEST_A, DIGEST_B, artifact_row, base, file_witness, input_row, law_of, manifest,
};
use super::{CountDefect, EvidenceError, PathUnsafety, TextUnsafety, WitnessHalf, validate};
use crate::behaviour::compiler_trace_index::DIAGNOSTIC_CAP_BYTES;
use crate::generated::shared::EvidenceStatus;

/// Every run-header scalar is held to the shape the fragment claims —
/// the run header is what a reader prints first, so a blank phase or a
/// CR in `started` is a terminal-rewriting document, not cosmetics.
#[test]
fn the_run_header_is_held_to_its_identity_shape() {
    for id in [
        "NOT-HEX",
        "00112233445566778899aabbccddeef",
        "00112233445566778899aabbccddeeff0",
        "00112233445566778899AABBCCDDEEFF",
    ] {
        let mut evidence = base();
        evidence.run.run_id = id.to_string();
        assert_eq!(law_of(&evidence), "run-identity", "run_id {id}");
    }

    for (selected, reason) in [
        ("", PathUnsafety::Blank),
        ("members\\tool", PathUnsafety::Backslash),
        ("members/tool\n", PathUnsafety::ControlByte),
        ("/members/tool", PathUnsafety::Absolute),
        ("C:/work/demo", PathUnsafety::DriveLetter),
        ("../sibling", PathUnsafety::ParentSegment),
        ("./members", PathUnsafety::DotSegment),
        ("members//tool", PathUnsafety::EmptySegment),
    ] {
        let mut evidence = base();
        evidence.run.selected = selected.to_string();
        let error = validate(&evidence).unwrap_err();
        assert_eq!(error.law(), "run-identity", "selected {selected:?}");
        assert!(
            matches!(error, EvidenceError::UnsafeSelected { reason: found, .. } if found == reason),
            "selected {selected:?} must refuse as {reason:?}, got {error:?}"
        );
    }

    // `requested` and `started` answer to the text law, in all three
    // of its arms.
    for (field, defect) in [
        ("requested", TextUnsafety::Blank),
        ("requested", TextUnsafety::ControlByte),
        ("started", TextUnsafety::Blank),
        ("started", TextUnsafety::OverCap),
        ("started", TextUnsafety::ControlByte),
    ] {
        let value = match defect {
            TextUnsafety::Blank => "   ".to_string(),
            TextUnsafety::OverCap => "x".repeat(DIAGNOSTIC_CAP_BYTES + 1),
            TextUnsafety::ControlByte => "2026-08-28T12:00:00Z\r\n".to_string(),
        };
        let mut evidence = base();
        match field {
            "requested" => {
                evidence.run.requested = value.clone();
                // Keep the chain membership law from firing first.
                evidence.run.chain.push(value);
            }
            _ => evidence.run.started = value,
        }
        let error = validate(&evidence).unwrap_err();
        assert_eq!(error.law(), "run-identity", "{field} {defect:?}");
        assert!(
            matches!(
                &error,
                EvidenceError::UnsafeRunScalar { reason, .. } if *reason == defect
            ) || matches!(
                &error,
                EvidenceError::UnsafeChainPhase { reason, .. } if *reason == defect
            ),
            "{field} {defect:?} must name the defect, got {error:?}"
        );
    }

    let mut empty_chain = base();
    empty_chain.run.chain.clear();
    assert!(matches!(
        validate(&empty_chain).unwrap_err(),
        EvidenceError::EmptyChain
    ));

    // EVERY chain phase is checked, not merely counted — the second
    // one here, so a first-element-only scan is red.
    for (phase, defect) in [
        (String::new(), TextUnsafety::Blank),
        ("bui\0ld".to_string(), TextUnsafety::ControlByte),
        ("x".repeat(DIAGNOSTIC_CAP_BYTES + 1), TextUnsafety::OverCap),
    ] {
        let mut evidence = base();
        evidence.run.chain[1] = phase;
        let error = validate(&evidence).unwrap_err();
        assert_eq!(error.law(), "run-identity");
        assert!(
            matches!(
                &error,
                EvidenceError::UnsafeChainPhase { index: 1, reason, .. } if *reason == defect
            ),
            "{defect:?} must be named at chain[1], got {error:?}"
        );
    }

    let mut foreign_request = base();
    foreign_request.run.requested = "package".to_string();
    assert!(matches!(
        validate(&foreign_request).unwrap_err(),
        EvidenceError::RequestedOutsideChain { .. }
    ));
}

/// An input row's declaration fingerprint is an identity, so it has a
/// shape — otherwise «measured against the same declaration» would be
/// a claim no reader could check.
#[test]
fn a_declaration_fingerprint_is_a_sha256_or_it_is_not_an_identity() {
    // A BLANK fingerprint is the bounded-identity law's (it is an
    // identity scalar like any other); this law owns the ones that are
    // present and still not a digest.
    let mut blank = base();
    blank.inputs[0].declaration_fingerprint = "   ".to_string();
    assert_eq!(law_of(&blank), "bounded-identity");

    for fingerprint in [
        "sha256:",
        "sha256:deadbeef",
        DIGEST_A.trim_start_matches("sha256:"),
        &DIGEST_A.to_uppercase(),
        "not a fingerprint at all",
    ] {
        let mut evidence = base();
        evidence.inputs[0].declaration_fingerprint = fingerprint.to_string();
        let error = validate(&evidence).unwrap_err();
        assert_eq!(
            error.law(),
            "measurement-identity",
            "fingerprint {fingerprint:?}"
        );
        assert!(matches!(
            error,
            EvidenceError::DeclarationFingerprintShape { .. }
        ));
    }
}

/// The count pair, from both directions and on both row kinds. This is
/// what stops a counted manifest and an uncounted content digest from
/// ever being compared as if they claimed the same thing.
#[test]
fn the_count_pair_is_one_claim_whose_presence_the_row_decides() {
    let cases: [(&str, CountDefect); 4] = [
        ("input-bytes-missing", CountDefect::BytesMissing),
        ("input-files-missing", CountDefect::FilesMissing),
        ("input-absent", CountDefect::Absent),
        ("artifact-unexpected", CountDefect::Unexpected),
    ];
    for (case, defect) in cases {
        let mut evidence = base();
        match case {
            "input-bytes-missing" => {
                evidence.inputs[0].measured.as_mut().unwrap().bytes = None;
            }
            "input-files-missing" => {
                evidence.inputs[0].observed.as_mut().unwrap().files = None;
            }
            "input-absent" => {
                evidence.inputs[0].measured = Some(file_witness(DIGEST_A));
            }
            _ => {
                evidence.artifacts[0].observed = Some(manifest(DIGEST_B, 1, "10"));
            }
        }
        let error = validate(&evidence).unwrap_err();
        assert_eq!(error.law(), "witness-shape", "{case}");
        assert!(
            matches!(&error, EvidenceError::WitnessCountShape { defect: found, .. } if *found == defect),
            "{case} must refuse as {defect:?}, got {error:?}"
        );
    }

    // The half is named, so a reader knows which side to look at.
    let mut observed_half = base();
    observed_half.inputs[0].observed.as_mut().unwrap().bytes = None;
    assert!(matches!(
        validate(&observed_half).unwrap_err(),
        EvidenceError::WitnessCountShape {
            half: WitnessHalf::Observed,
            ..
        }
    ));
}

/// `bytes` is a CANONICAL unsigned decimal string, and the reason it
/// is a string at all is that a declared input set may exceed what a
/// `uint32` — or a `uint64` — can hold. A value above `u32::MAX`
/// validates without narrowing; a non-canonical spelling refuses.
#[test]
fn a_byte_count_is_lossless_and_never_narrowed() {
    for bytes in ["0", "4294967295", "4294967296", "18446744073709551616"] {
        let mut evidence = base();
        evidence.inputs[0].measured = Some(manifest(DIGEST_A, 7, bytes));
        evidence.inputs[0].observed = Some(manifest(DIGEST_A, 7, bytes));
        validate(&evidence)
            .unwrap_or_else(|e| panic!("{bytes} must ride the wire losslessly: {e}"));
    }
    for bytes in [
        "",
        "  ",
        "01",
        "1_000",
        "-1",
        "1.0",
        "0x10",
        "12 ",
        "4294967296 ",
    ] {
        let mut evidence = base();
        evidence.inputs[0].measured = Some(manifest(DIGEST_A, 7, bytes));
        evidence.inputs[0].observed = Some(manifest(DIGEST_A, 7, bytes));
        let error = validate(&evidence).unwrap_err();
        assert_eq!(error.law(), "witness-shape", "bytes {bytes:?}");
        assert!(
            matches!(error, EvidenceError::NonCanonicalByteCount { .. }),
            "bytes {bytes:?} must refuse as non-canonical"
        );
    }
    // …and a byte count above u32::MAX that MOVED is still a stale
    // comparison, not an overflow: nothing here parses the string.
    let mut stale = base();
    stale.status = EvidenceStatus::Stale;
    stale.inputs[0].status = EvidenceStatus::Stale;
    stale.inputs[0].measured = Some(manifest(DIGEST_A, 7, "4294967296"));
    stale.inputs[0].observed = Some(manifest(DIGEST_A, 7, "4294967297"));
    validate(&stale).unwrap();
}

/// A declared pattern names a scope inside this project, or it names a
/// scope this wire may not certify. Every grammar arm has its own RED.
#[test]
fn a_declared_pattern_never_leaves_the_project() {
    for (pattern, reason) in [
        ("", PathUnsafety::Blank),
        ("src\\**", PathUnsafety::Backslash),
        ("src/**\r", PathUnsafety::ControlByte),
        ("/etc/passwd", PathUnsafety::Absolute),
        ("C:/Windows/**", PathUnsafety::DriveLetter),
        ("../../secrets/**", PathUnsafety::ParentSegment),
        ("./src/**", PathUnsafety::DotSegment),
        ("src//**", PathUnsafety::EmptySegment),
    ] {
        let mut evidence = base();
        evidence.inputs[0].patterns = vec!["src/**".to_string(), pattern.to_string()];
        let error = validate(&evidence).unwrap_err();
        assert_eq!(error.law(), "pattern-safety", "pattern {pattern:?}");
        assert!(
            matches!(error, EvidenceError::UnsafePattern { index: 1, reason: found, .. } if found == reason),
            "pattern {pattern:?} must refuse as {reason:?}, got {error:?}"
        );
    }
}

/// An artifact row carries the PORTABLE path. The absolute machine
/// path durable state keeps would both fail to resolve on another
/// machine and leak the operator's home into a forwarded document.
#[test]
fn an_artifact_path_is_project_relative_and_never_the_machine_path() {
    for (path, reason) in [
        ("", PathUnsafety::Blank),
        ("target\\demo.txt", PathUnsafety::Backslash),
        ("target/demo.txt\n", PathUnsafety::ControlByte),
        ("/var/lib/demo.txt", PathUnsafety::Absolute),
        // The exact spelling `state_artifact.path` carries today.
        ("C:/work/demo/out.txt", PathUnsafety::DriveLetter),
        ("../outside/demo.txt", PathUnsafety::ParentSegment),
        ("./target/demo.txt", PathUnsafety::DotSegment),
        ("target//demo.txt", PathUnsafety::EmptySegment),
    ] {
        let mut evidence = base();
        evidence.artifacts[0].path = path.to_string();
        let error = validate(&evidence).unwrap_err();
        assert_eq!(error.law(), "path-safety", "path {path:?}");
        assert!(
            matches!(error, EvidenceError::UnsafeArtifactPath { reason: found, .. } if found == reason),
            "path {path:?} must refuse as {reason:?}, got {error:?}"
        );
    }
    // A nested relative path is exactly what the row is for.
    let mut nested = base();
    nested.artifacts[0].path = "target/debug/build/demo.txt".to_string();
    validate(&nested).unwrap();
}

/// Every identity scalar is nonblank, bounded and printable; every
/// diagnostic scalar is bounded and printable. A blank `phase` or a
/// blank artifact `kind` is an unjoinable claim, not a cosmetic gap.
#[test]
fn every_identity_scalar_is_named_bounded_and_printable() {
    let over_cap = "x".repeat(DIAGNOSTIC_CAP_BYTES + 1);
    let cases: [(&str, &str, TextUnsafety); 6] = [
        ("phase", "   ", TextUnsafety::Blank),
        ("phase", "build\ninjected", TextUnsafety::ControlByte),
        ("phase", over_cap.as_str(), TextUnsafety::OverCap),
        ("kind", "", TextUnsafety::Blank),
        ("kind", "file\0", TextUnsafety::ControlByte),
        ("kind", over_cap.as_str(), TextUnsafety::OverCap),
    ];
    for (field, value, defect) in cases {
        let mut evidence = base();
        match field {
            "phase" => evidence.inputs[0].phase = value.to_string(),
            _ => evidence.artifacts[0].kind = value.to_string(),
        }
        let error = validate(&evidence).unwrap_err();
        assert_eq!(error.law(), "bounded-identity", "{field} {defect:?}");
        assert!(
            matches!(
                &error,
                EvidenceError::UnsafeScalar { field: found, reason, .. }
                    if *found == field && *reason == defect
            ),
            "{field} {defect:?} must be named, got {error:?}"
        );
    }

    // Diagnostic scalars are bounded and printable but MAY be absent —
    // and a blank reason is comparison-shape's, never this law's.
    let mut long_reason = base();
    long_reason.status = EvidenceStatus::Missing;
    long_reason.inputs[0] = super::tests::row_with(input_row(), EvidenceStatus::Missing);
    long_reason.inputs[0].reason_code = Some(over_cap.clone());
    assert_eq!(law_of(&long_reason), "bounded-identity");

    let mut control_algorithm = base();
    control_algorithm.artifacts[0]
        .measured
        .as_mut()
        .unwrap()
        .algorithm = "sha256:file-v1\r".to_string();
    assert_eq!(law_of(&control_algorithm), "bounded-identity");

    // A backslash is a PATH defect only: an execution reference or a
    // kind may legitimately carry one, and a law that refused it
    // everywhere would be inventing a rule the wire does not hold.
    let mut backslash_kind = base();
    backslash_kind.artifacts[0].kind = "weird\\kind".to_string();
    validate(&backslash_kind).unwrap();

    let mut backslash_execution = base();
    backslash_execution.inputs[0].execution = "org.demo/provider#a\\b".to_string();
    validate(&backslash_execution).unwrap();
}

/// The whole hardened surface, exercised once as a legal document: a
/// stale input under a `.` root with an oversized byte count, beside
/// an unstable artifact that kept its measurement.
#[test]
fn a_fully_exercised_document_still_validates() {
    let mut evidence = base();
    evidence.run.selected = ".".to_string();
    evidence.status = EvidenceStatus::Unstable;
    evidence.inputs[0] = input_row();
    evidence.inputs[0].status = EvidenceStatus::Stale;
    evidence.inputs[0].patterns = vec!["src/**".to_string(), "docs/guide.md".to_string()];
    evidence.inputs[0].measured = Some(manifest(DIGEST_A, 7, "4294967296"));
    evidence.inputs[0].observed = Some(manifest(DIGEST_B, 8, "4294967300"));
    evidence.artifacts[0] = artifact_row();
    evidence.artifacts[0].status = EvidenceStatus::Unstable;
    evidence.artifacts[0].observed = None;
    evidence.artifacts[0].reason_code = Some("artifact moved while it was read".to_string());
    validate(&evidence).unwrap();
}
