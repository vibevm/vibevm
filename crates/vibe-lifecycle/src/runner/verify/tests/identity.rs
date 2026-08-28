//! The `evidence_id` REDs (R7.5 P2/A5).
//!
//! Split from the comparison cell for the 600-line budget, and it is a real
//! seam: what is pinned here is the IDENTITY — that the clock is not part of
//! it, that every identity and comparison member is, and that the writer's
//! label schedule is reproducible from the schema alone by another language.
//!
//! The longhand case deliberately reimplements the frame instead of calling
//! the production writer: a golden that went through the same code it is
//! meant to pin would only prove the code equals itself.

use sha2::{Digest, Sha256};
use vibe_wire::generated::shared::{
    ArtifactWitness, DigestWitness, EvidenceRun, EvidenceStatus, InputMeasurement,
    VerificationEvidence,
};

use crate::agent::tests::support::{RecordingBackend, TWO_OUTPUTS_RESULT};

use super::{
    KEY, LATER, OBSERVED, RUN_ID, contribution, executed, observed_at, reconcile, row, scratch,
};

/// The clock is not identity: two reconciliations of the same claim at
/// different wall-clock seconds carry the same id, and any comparison member
/// moves it.
#[test]
fn the_evidence_id_excludes_the_clock_and_binds_every_comparison_member() {
    let fixture = scratch();
    let backend = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    let mut run = executed(fixture.root.as_path(), &backend);
    let prefix = [contribution(&row(&["data/**"]))];

    let first = reconcile(&mut run, &backend, &prefix);
    let second = run
        .reconcile_verification(&prefix, &backend, observed_at(LATER))
        .unwrap();
    assert_ne!(first.observed_at, second.observed_at);
    assert_eq!(
        first.evidence_id, second.evidence_id,
        "the clock is an input, not part of what was measured",
    );

    for mutate in [
        |member: &mut VerificationEvidence| member.status = EvidenceStatus::Stale,
        |member: &mut VerificationEvidence| member.run.selected = "member".into(),
        |member: &mut VerificationEvidence| member.inputs[0].patterns.push("extra/**".into()),
        |member: &mut VerificationEvidence| {
            member.inputs[0].reason_code = Some("input-declaration-changed".into());
        },
        |member: &mut VerificationEvidence| member.artifacts[0].kind = "directory".into(),
        |member: &mut VerificationEvidence| member.artifacts.truncate(1),
    ] {
        let mut moved = first.clone();
        mutate(&mut moved);
        assert_ne!(
            crate::runner::verify::id::evidence_id(&moved),
            first.evidence_id,
            "every identity and comparison member is digest material",
        );
    }
}

/// The writer's schedule, written out longhand against a hand-built member:
/// the label sequence, the presence bytes and the vocabulary order are the
/// contract another language reproduces, so they are pinned here without going
/// through the production writer.
#[test]
fn the_evidence_id_schedule_is_reproducible_longhand() {
    let member = VerificationEvidence {
        artifacts: vec![ArtifactWitness {
            id: "guide".into(),
            kind: "file".into(),
            measured: Some(DigestWitness {
                algorithm: "sha256:file-v1".into(),
                bytes: None,
                digest: format!("sha256:{}", "ab".repeat(32)),
                files: None,
            }),
            measured_run_id: Some(RUN_ID.into()),
            observed: None,
            path: "docs/guide.md".into(),
            reason_code: Some("artifact-absent".into()),
            status: EvidenceStatus::Missing,
        }],
        evidence: 1,
        evidence_id: String::new(),
        inputs: vec![InputMeasurement {
            declaration_fingerprint: format!("sha256:{}", "cd".repeat(32)),
            execution: KEY.into(),
            measured: Some(DigestWitness {
                algorithm: "sha256:vibe-input-manifest-v1".into(),
                bytes: Some("3".into()),
                digest: format!("sha256:{}", "ef".repeat(32)),
                files: Some(1),
            }),
            measured_run_id: Some(RUN_ID.into()),
            observed: None,
            patterns: vec!["data/**".into()],
            phase: "create".into(),
            reason_code: Some("input-open".into()),
            status: EvidenceStatus::Unstable,
        }],
        observed_at: observed_at(OBSERVED),
        run: EvidenceRun {
            chain: vec!["create".into(), "verify".into()],
            requested: "verify".into(),
            run_id: RUN_ID.into(),
            selected: ".".into(),
            started: "2026-08-28T11:59:40Z".into(),
        },
        status: EvidenceStatus::Unstable,
    };

    let mut hash = Sha256::new();
    hash.update(b"vibe-verification-evidence-id\0epoch=1\0");
    let mut f = |label: &str, value: &[u8]| {
        hash.update((label.len() as u64).to_be_bytes());
        hash.update(label.as_bytes());
        hash.update((value.len() as u64).to_be_bytes());
        hash.update(value);
    };
    f("evidence", b"1");
    f("status", b"unstable");
    f("run.run_id", RUN_ID.as_bytes());
    f("run.selected", b".");
    f("run.requested", b"verify");
    f("run.chain.count", b"2");
    f("run.chain.item", b"create");
    f("run.chain.item", b"verify");
    f("run.started", b"2026-08-28T11:59:40Z");
    f("inputs.count", b"1");
    f("inputs.execution", KEY.as_bytes());
    f("inputs.phase", b"create");
    f(
        "inputs.declaration_fingerprint",
        format!("sha256:{}", "cd".repeat(32)).as_bytes(),
    );
    f("inputs.patterns.count", b"1");
    f("inputs.patterns.item", b"data/**");
    f("inputs.status", b"unstable");
    f("inputs.measured_run_id.present", b"1");
    f("inputs.measured_run_id", RUN_ID.as_bytes());
    f("inputs.measured.present", b"1");
    f(
        "inputs.measured.algorithm",
        b"sha256:vibe-input-manifest-v1",
    );
    f(
        "inputs.measured.digest",
        format!("sha256:{}", "ef".repeat(32)).as_bytes(),
    );
    f("inputs.measured.files.present", b"1");
    f("inputs.measured.files", b"1");
    f("inputs.measured.bytes.present", b"1");
    f("inputs.measured.bytes", b"3");
    f("inputs.observed.present", b"0");
    f("inputs.reason_code.present", b"1");
    f("inputs.reason_code", b"input-open");
    f("artifacts.count", b"1");
    f("artifacts.id", b"guide");
    f("artifacts.kind", b"file");
    f("artifacts.path", b"docs/guide.md");
    f("artifacts.status", b"missing");
    f("artifacts.measured_run_id.present", b"1");
    f("artifacts.measured_run_id", RUN_ID.as_bytes());
    f("artifacts.measured.present", b"1");
    f("artifacts.measured.algorithm", b"sha256:file-v1");
    f(
        "artifacts.measured.digest",
        format!("sha256:{}", "ab".repeat(32)).as_bytes(),
    );
    f("artifacts.measured.files.present", b"0");
    f("artifacts.measured.bytes.present", b"0");
    f("artifacts.observed.present", b"0");
    f("artifacts.reason_code.present", b"1");
    f("artifacts.reason_code", b"artifact-absent");
    let expected = format!("sha256:{:x}", hash.finalize());

    assert_eq!(crate::runner::verify::id::evidence_id(&member), expected);
}
