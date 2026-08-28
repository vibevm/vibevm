//! RED arms for the comparison and status laws of the
//! verification-evidence member, plus the positives that keep them
//! honest. The identity/text/scope hardening arms live beside this
//! file in `tests_hardening.rs`, split along that seam when the suite
//! outgrew the per-file budget.
//!
//! Every arm is a minimal mutation of one legal base value, so a
//! refusal names the law, not a fixture's accident.

use super::{EvidenceError, IMPLEMENTED_LAWS, ShapeDefect, validate};
use crate::generated::shared::{
    ArtifactWitness, DigestWitness, EvidenceRun, EvidenceStatus, InputMeasurement,
    VerificationEvidence,
};

pub(super) const RUN_ID: &str = "00112233445566778899aabbccddeeff";
pub(super) const OTHER_RUN: &str = "ffeeddccbbaa99887766554433221100";
pub(super) const DIGEST_A: &str =
    "sha256:1111111111111111111111111111111111111111111111111111111111111111";
pub(super) const DIGEST_B: &str =
    "sha256:2222222222222222222222222222222222222222222222222222222222222222";
pub(super) const EVIDENCE_ID: &str =
    "sha256:3333333333333333333333333333333333333333333333333333333333333333";
pub(super) const DECLARATION: &str =
    "sha256:4444444444444444444444444444444444444444444444444444444444444444";

/// An input-manifest witness: counted, because a manifest must say how
/// much of its declared scope it covered.
pub(super) fn manifest(digest: &str, files: u32, bytes: &str) -> DigestWitness {
    DigestWitness {
        algorithm: "sha256:vibe-input-manifest-v1".to_string(),
        digest: digest.to_string(),
        files: Some(files),
        bytes: Some(bytes.to_string()),
    }
}

/// An artifact witness: uncounted, because the content IS the witness.
pub(super) fn file_witness(digest: &str) -> DigestWitness {
    DigestWitness {
        algorithm: "sha256:file-v1".to_string(),
        digest: digest.to_string(),
        files: None,
        bytes: None,
    }
}

pub(super) fn run() -> EvidenceRun {
    EvidenceRun {
        run_id: RUN_ID.to_string(),
        selected: "members/tool".to_string(),
        requested: "verify".to_string(),
        chain: vec![
            "validate".to_string(),
            "build".to_string(),
            "test".to_string(),
            "verify".to_string(),
        ],
        started: "2026-08-28T12:00:00Z".to_string(),
    }
}

pub(super) fn input_row() -> InputMeasurement {
    InputMeasurement {
        execution: "org.demo/provider#compile".to_string(),
        phase: "build".to_string(),
        declaration_fingerprint: DECLARATION.to_string(),
        patterns: vec!["src/**".to_string()],
        status: EvidenceStatus::Matched,
        measured_run_id: Some(RUN_ID.to_string()),
        measured: Some(manifest(DIGEST_A, 7, "1234")),
        observed: Some(manifest(DIGEST_A, 7, "1234")),
        reason_code: None,
    }
}

pub(super) fn artifact_row() -> ArtifactWitness {
    ArtifactWitness {
        id: "demo".to_string(),
        kind: "file".to_string(),
        path: "target/demo.txt".to_string(),
        status: EvidenceStatus::Matched,
        measured_run_id: Some(RUN_ID.to_string()),
        measured: Some(file_witness(DIGEST_B)),
        observed: Some(file_witness(DIGEST_B)),
        reason_code: None,
    }
}

/// One legal `matched` member: one input row, one artifact row, both
/// measured under this run and both comparing equal.
pub(super) fn base() -> VerificationEvidence {
    VerificationEvidence {
        evidence: 1,
        evidence_id: EVIDENCE_ID.to_string(),
        status: EvidenceStatus::Matched,
        observed_at: "2026-08-28T12:00:05Z".parse().unwrap(),
        run: run(),
        inputs: vec![input_row()],
        artifacts: vec![artifact_row()],
    }
}

/// One legal empty member: no rows at all, and the root says so.
pub(super) fn empty() -> VerificationEvidence {
    VerificationEvidence {
        inputs: Vec::new(),
        artifacts: Vec::new(),
        status: EvidenceStatus::Unavailable,
        ..base()
    }
}

pub(super) fn law_of(evidence: &VerificationEvidence) -> &'static str {
    validate(evidence).unwrap_err().law()
}

/// Rewrite one input row into a legal instance of `status`.
pub(super) fn row_with(mut row: InputMeasurement, status: EvidenceStatus) -> InputMeasurement {
    match status {
        EvidenceStatus::Matched => {}
        EvidenceStatus::Stale => row.observed = Some(manifest(DIGEST_B, 8, "1400")),
        EvidenceStatus::Missing => {
            row.observed = None;
            row.reason_code = Some("declared input path is gone".to_string());
        }
        EvidenceStatus::Unavailable => {
            row.measured = None;
            row.observed = None;
            row.measured_run_id = None;
            row.reason_code = Some("execution declares no inputs".to_string());
        }
        EvidenceStatus::Unstable => {
            // The measurement stands; the RE-observation is what was
            // refused. A row with neither is `unavailable`.
            row.observed = None;
            row.reason_code = Some("source changed while it was read".to_string());
        }
    }
    row.status = status;
    row
}

/// Rewrite one artifact row into a legal instance of `status`.
pub(super) fn artifact_with(mut row: ArtifactWitness, status: EvidenceStatus) -> ArtifactWitness {
    match status {
        EvidenceStatus::Matched => {}
        EvidenceStatus::Stale => row.observed = Some(file_witness(DIGEST_A)),
        EvidenceStatus::Missing => {
            row.observed = None;
            row.reason_code = Some("declared artifact is gone".to_string());
        }
        EvidenceStatus::Unavailable => {
            row.measured = None;
            row.observed = None;
            row.measured_run_id = None;
            row.reason_code = Some("legacy output carries no witness".to_string());
        }
        EvidenceStatus::Unstable => {
            row.observed = None;
            row.reason_code = Some("artifact changed while it was read".to_string());
        }
    }
    row.status = status;
    row
}

#[test]
fn the_legal_shapes_all_validate() {
    validate(&base()).unwrap();
    validate(&empty()).unwrap();

    // A `.` selected root, an artifact-only document, and an
    // input-only document are all legal.
    let mut root = base();
    root.run.selected = ".".to_string();
    root.inputs.clear();
    validate(&root).unwrap();

    let mut inputs_only = base();
    inputs_only.artifacts.clear();
    validate(&inputs_only).unwrap();

    // An authored EMPTY pattern list is a complete empty declared
    // scope — a measured, matched row, not an unavailable one.
    let mut empty_scope = base();
    empty_scope.inputs[0].patterns.clear();
    validate(&empty_scope).unwrap();

    // A row measured by an EARLIER run is legal — that is the whole
    // point of recording which run measured what.
    let mut earlier = base();
    earlier.inputs[0].measured_run_id = Some(OTHER_RUN.to_string());
    validate(&earlier).unwrap();
}

#[test]
fn every_documented_law_has_an_implemented_label() {
    // The fragment-side half of this parity lives in the wire test;
    // here the list itself is proven free of duplicates and blanks, so
    // a copy-paste in the constant cannot fake a covered law.
    let mut sorted = IMPLEMENTED_LAWS.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), IMPLEMENTED_LAWS.len(), "duplicate law label");
    assert!(IMPLEMENTED_LAWS.iter().all(|law| !law.trim().is_empty()));
}

#[test]
fn a_foreign_epoch_and_a_malformed_evidence_id_are_refused() {
    let mut evidence = base();
    evidence.evidence = 2;
    assert_eq!(law_of(&evidence), "evidence-identity");

    for id in [
        // No scheme at all.
        "3333333333333333333333333333333333333333333333333333333333333333",
        // The scheme, and 63 hex — the LENGTH half.
        "sha256:333333333333333333333333333333333333333333333333333333333333333",
        // The scheme, 64 characters, UPPERCASE — the case half.
        "sha256:3333333333333333333333333333333333333333333333333333333333AAAAAA",
        // Another algorithm entirely.
        "blake3:3333333333333333333333333333333333333333333333333333333333333333",
        "",
    ] {
        let mut evidence = base();
        evidence.evidence_id = id.to_string();
        assert_eq!(
            law_of(&evidence),
            "evidence-identity",
            "{id} must land in the evidence-identity law"
        );
    }
}

#[test]
fn a_measured_row_names_the_run_that_measured_it() {
    // A `measured` witness with no id — both row kinds.
    let mut inputs = base();
    inputs.inputs[0].measured_run_id = None;
    let error = validate(&inputs).unwrap_err();
    assert_eq!(error.law(), "measurement-identity");
    assert!(matches!(error, EvidenceError::MeasuredRunIdAbsent { .. }));

    let mut artifacts = base();
    artifacts.artifacts[0].measured_run_id = None;
    assert!(matches!(
        validate(&artifacts).unwrap_err(),
        EvidenceError::MeasuredRunIdAbsent { .. }
    ));

    // Present but not a run id.
    let mut malformed = base();
    malformed.inputs[0].measured_run_id = Some("nope".to_string());
    assert_eq!(law_of(&malformed), "measurement-identity");

    // …and the one honest exemption: an `unavailable` row has no
    // measurement to attribute, so it names no run either.
    let mut unavailable = base();
    unavailable.status = EvidenceStatus::Unavailable;
    unavailable.inputs[0] = row_with(input_row(), EvidenceStatus::Unavailable);
    unavailable.artifacts[0] = artifact_with(artifact_row(), EvidenceStatus::Unavailable);
    validate(&unavailable).unwrap();
}

/// The id and the witness are ONE fact. An `unavailable` row that
/// keeps its `measured_run_id` after dropping the witness points a
/// reader joining by run id at a measurement the row itself denies —
/// and the shape reads entirely plausibly, which is why it needs an
/// arm rather than a comment.
#[test]
fn an_orphaned_measured_run_id_is_refused_on_both_row_kinds() {
    let mut inputs = base();
    inputs.status = EvidenceStatus::Unavailable;
    inputs.inputs[0] = row_with(input_row(), EvidenceStatus::Unavailable);
    inputs.artifacts[0] = artifact_with(artifact_row(), EvidenceStatus::Unavailable);
    let mut artifacts = inputs.clone();

    inputs.inputs[0].measured_run_id = Some(RUN_ID.to_string());
    let error = validate(&inputs).unwrap_err();
    assert_eq!(error.law(), "measurement-identity");
    assert!(
        matches!(error, EvidenceError::MeasuredRunIdOrphaned { .. }),
        "an input id with no measured witness must be named, got {error:?}"
    );

    artifacts.artifacts[0].measured_run_id = Some(OTHER_RUN.to_string());
    assert!(matches!(
        validate(&artifacts).unwrap_err(),
        EvidenceError::MeasuredRunIdOrphaned { .. }
    ));

    // An `unavailable` row MAY still show a current value with no
    // baseline — the pairing law is about the measured half only, and
    // constraining `observed` here would be a rule the wire does not
    // hold.
    let mut observed_only = base();
    observed_only.status = EvidenceStatus::Unavailable;
    observed_only.inputs[0] = row_with(input_row(), EvidenceStatus::Unavailable);
    observed_only.inputs[0].observed = Some(manifest(DIGEST_A, 7, "1234"));
    observed_only.artifacts[0] = artifact_with(artifact_row(), EvidenceStatus::Unavailable);
    validate(&observed_only).unwrap();
}

#[test]
fn a_witness_names_its_algorithm_and_carries_a_sha256() {
    let mut blank_algorithm = base();
    blank_algorithm.inputs[0]
        .measured
        .as_mut()
        .unwrap()
        .algorithm = "  ".to_string();
    assert_eq!(law_of(&blank_algorithm), "witness-shape");

    for digest in ["", "deadbeef", DIGEST_A.to_uppercase().as_str()] {
        let mut evidence = base();
        evidence.artifacts[0].observed.as_mut().unwrap().digest = digest.to_string();
        assert_eq!(law_of(&evidence), "witness-shape", "digest {digest:?}");
    }
}

/// The `comparison-shape` matrix, one arm per typed defect. This is
/// the law an evidence bug would hide in: every one of these mutations
/// is a document that reads plausibly and claims something it did not
/// measure.
#[test]
fn the_comparison_matrix_refuses_every_status_that_cannot_mean_what_its_row_says() {
    let defect_of = |evidence: &VerificationEvidence| match validate(evidence).unwrap_err() {
        EvidenceError::ComparisonShape { defect, .. } => defect,
        other => panic!("expected a comparison-shape refusal, got {other:?}"),
    };

    // matched with a changed observed digest — the single most
    // important RED arm on this wire (architecture §8, E2/E7).
    let mut unequal = base();
    unequal.inputs[0].observed = Some(manifest(DIGEST_B, 7, "1234"));
    assert_eq!(defect_of(&unequal), ShapeDefect::UnequalWitnesses);

    // …and the same mutation in the counts alone: the digest is the
    // same string, the scope it covered is not.
    let mut recounted = base();
    recounted.inputs[0].observed = Some(manifest(DIGEST_A, 8, "1400"));
    assert_eq!(defect_of(&recounted), ShapeDefect::UnequalWitnesses);

    // …and in the BYTE count alone, with the file count unmoved: a
    // same-length-set content edit is exactly what E7 probes.
    let mut rebyted = base();
    rebyted.inputs[0].observed = Some(manifest(DIGEST_A, 7, "1235"));
    assert_eq!(defect_of(&rebyted), ShapeDefect::UnequalWitnesses);

    let mut matched_without_observed = base();
    matched_without_observed.inputs[0].observed = None;
    assert_eq!(
        defect_of(&matched_without_observed),
        ShapeDefect::MissingObserved
    );

    // Dropping the witness drops its id with it: `measurement-identity`
    // owns the orphaned pair and would answer first, so the fixture
    // removes both and leaves this arm testing what it names.
    let mut matched_without_measured = base();
    matched_without_measured.artifacts[0].measured = None;
    matched_without_measured.artifacts[0].measured_run_id = None;
    assert_eq!(
        defect_of(&matched_without_measured),
        ShapeDefect::MissingMeasured
    );

    let mut matched_with_reason = base();
    matched_with_reason.inputs[0].reason_code = Some("why?".to_string());
    assert_eq!(defect_of(&matched_with_reason), ShapeDefect::ReasonPresent);

    // stale whose witnesses are equal — a mismatch that matches.
    let mut stale = base();
    stale.status = EvidenceStatus::Stale;
    stale.inputs[0].status = EvidenceStatus::Stale;
    assert_eq!(defect_of(&stale), ShapeDefect::EqualWitnesses);

    // missing that carries an observation, and one that owes a reason.
    let mut missing = base();
    missing.status = EvidenceStatus::Missing;
    missing.inputs[0].status = EvidenceStatus::Missing;
    assert_eq!(defect_of(&missing), ShapeDefect::UnexpectedObserved);
    missing.inputs[0].observed = None;
    assert_eq!(defect_of(&missing), ShapeDefect::ReasonAbsent);
    missing.inputs[0].reason_code = Some(" \t ".to_string());
    assert_eq!(defect_of(&missing), ShapeDefect::ReasonBlank);
    missing.inputs[0].reason_code = Some("declared input path is gone".to_string());
    validate(&missing).unwrap();

    // unavailable that carries a measurement — then the measurement is
    // not what was unavailable (architecture §8, E3).
    let mut unavailable = base();
    unavailable.status = EvidenceStatus::Unavailable;
    unavailable.inputs[0].status = EvidenceStatus::Unavailable;
    unavailable.inputs[0].reason_code = Some("no declared inputs".to_string());
    assert_eq!(defect_of(&unavailable), ShapeDefect::UnexpectedMeasured);

    // unstable that carries an observation the run refused to accept…
    let mut unstable = base();
    unstable.status = EvidenceStatus::Unstable;
    unstable.artifacts[0].status = EvidenceStatus::Unstable;
    unstable.artifacts[0].reason_code = Some("file changed while it was read".to_string());
    assert_eq!(defect_of(&unstable), ShapeDefect::UnexpectedObserved);
    unstable.artifacts[0].observed = None;
    validate(&unstable).unwrap();

    // …and unstable with NO measurement at all: that row is
    // `unavailable`, the one honest no-measurement case, and calling
    // it unstable would smuggle a second one in. The id goes with the
    // witness — an orphaned pair is `measurement-identity`'s refusal,
    // and this arm is about the status word.
    unstable.artifacts[0].measured = None;
    unstable.artifacts[0].measured_run_id = None;
    assert_eq!(defect_of(&unstable), ShapeDefect::MissingMeasured);
}

#[test]
fn a_legal_stale_row_carries_two_witnesses_that_differ() {
    let mut stale = base();
    stale.status = EvidenceStatus::Stale;
    stale.inputs[0].status = EvidenceStatus::Stale;
    stale.inputs[0].observed = Some(manifest(DIGEST_B, 8, "1400"));
    validate(&stale).unwrap();

    // A reason is optional here — the digests are the reason — but a
    // BLANK one is not a reason at all.
    stale.inputs[0].reason_code = Some(String::new());
    assert_eq!(law_of(&stale), "comparison-shape");
    stale.inputs[0].reason_code = Some("declared input changed after the measurement".to_string());
    validate(&stale).unwrap();
}

#[test]
fn the_root_status_is_the_worst_row_and_never_a_verdict_of_its_own() {
    // An empty set that claims a pass — the mutation the architecture
    // calls E3 at the root: no rows must be visibly `unavailable`.
    let mut empty_matched = empty();
    empty_matched.status = EvidenceStatus::Matched;
    assert_eq!(law_of(&empty_matched), "overall-status");

    // A stale row under a matched root.
    let mut hidden_stale = base();
    hidden_stale.inputs[0].status = EvidenceStatus::Stale;
    hidden_stale.inputs[0].observed = Some(manifest(DIGEST_B, 8, "1400"));
    assert_eq!(law_of(&hidden_stale), "overall-status");

    // Precedence, exhaustively: the worst row wins over every milder
    // one, in both row lists.
    let ladder = [
        (EvidenceStatus::Matched, EvidenceStatus::Unavailable),
        (EvidenceStatus::Unavailable, EvidenceStatus::Stale),
        (EvidenceStatus::Stale, EvidenceStatus::Missing),
        (EvidenceStatus::Missing, EvidenceStatus::Unstable),
    ];
    for (milder, worse) in ladder {
        let mut evidence = base();
        evidence.inputs[0] = row_with(input_row(), milder.clone());
        evidence.artifacts[0] = artifact_with(artifact_row(), worse.clone());
        evidence.status = worse.clone();
        validate(&evidence).unwrap_or_else(|e| panic!("{worse:?} over {milder:?}: {e}"));
        evidence.status = milder.clone();
        assert_eq!(
            law_of(&evidence),
            "overall-status",
            "{milder:?} must not stand for a document holding a {worse:?} row"
        );
    }
}

#[test]
fn one_identity_gets_one_row() {
    let mut blank = base();
    blank.inputs[0].execution = "   ".to_string();
    assert_eq!(law_of(&blank), "row-identity");

    let mut duplicate = base();
    duplicate.inputs.push(input_row());
    let error = validate(&duplicate).unwrap_err();
    assert_eq!(error.law(), "row-identity");
    assert!(matches!(
        error,
        EvidenceError::RowKeyDuplicate { first: 0, .. }
    ));

    let mut duplicate_artifact = base();
    duplicate_artifact.artifacts.push(artifact_row());
    assert_eq!(law_of(&duplicate_artifact), "row-identity");

    // An input and an artifact MAY share a spelling — they are keys of
    // different lists, and the refusal must not fire across them.
    let mut crossed = base();
    crossed.artifacts[0].id = crossed.inputs[0].execution.clone();
    validate(&crossed).unwrap();
}

#[test]
fn refusals_render_without_leaking_the_offending_value() {
    let huge = "z".repeat(4096);
    let mut evidence = base();
    evidence.evidence_id = huge;
    let rendered = validate(&evidence).unwrap_err().to_string();
    assert!(
        rendered.len() < 512,
        "a refusal must preview, not carry, an untrusted scalar: {} bytes",
        rendered.len()
    );
    assert!(rendered.contains("4096"), "the true length is reported");
}
