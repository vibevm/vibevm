//! Native declaration and accounting join; no independent identity engine.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-BOOTSTRAP-EVIDENCE");
use super::*;
use vibe_core::manifest::change::ChangeDocumentRole;
use vibe_requirements::change::read_saved_change;

pub(super) fn native_record(
    root: &Path,
    spec_root: Option<&Path>,
    id: &str,
) -> Result<Vec<u8>, CheckFailure> {
    // The native saved owner admits bounded node/manifest/role observations and
    // validates evidence from those exact bytes. Accounting adds no read policy.
    let saved = read_saved_change(root, ".", spec_root, id)
        .map_err(|e| CheckFailure::new("native-saved", id, e.to_string()))?;
    let role = saved
        .manifest
        .documents()
        .iter()
        .find(|d| d.role == ChangeDocumentRole::ReleaseImpact)
        .ok_or_else(|| CheckFailure::new("native-role", id, "Release-impact role missing"))?;
    if !saved.manifest.is_evidence_only() {
        saved
            .identity_analysis()
            .map_err(|e| CheckFailure::new("native-identity", id, e.to_string()))?;
    }
    let path = saved.manifest.directory().join(&role.path);
    let bytes = saved
        .input_observations
        .into_iter()
        .find(|(p, _)| *p == path)
        .ok_or_else(|| CheckFailure::new("native-role", id, "Release-impact observation missing"))?
        .1
        .bytes;
    Ok(bytes)
}

pub(super) fn bound_record(
    root: &Path,
    spec_root: Option<&Path>,
    id: &str,
    digest: &str,
) -> Result<Vec<u8>, CheckFailure> {
    let bytes = native_record(root, spec_root, id)?;
    if capture::sha256(&bytes) != digest {
        return Err(CheckFailure::new(
            "native-digest",
            id,
            "Native evidence differs from selected accounting bytes",
        ));
    }
    Ok(bytes)
}

pub(super) fn check(
    request: &CheckRequest,
    transition: &Transition,
    report: &mut CheckReport,
) -> Result<(), CheckFailure> {
    let path = Path::new(&request.transition);
    // Retain actual capture/delta/required-claim predicates and their gaps.
    contribution::check(path, transition, report)?;
    let Some(scope) = &transition.contribution_scope else {
        return Ok(());
    };
    let root = capture::git(
        path.parent().unwrap_or(Path::new(".")),
        &["rev-parse", "--show-toplevel"],
    )?;
    let root = std::str::from_utf8(&root)
        .map_err(|_| CheckFailure::new("native-root", "bindings", "Invalid root UTF8"))?
        .trim();
    let mut records = Vec::new();
    for binding in &scope.record_digests {
        let bytes = bound_record(Path::new(root), None, &binding.change_id, &binding.sha256)?;
        let record = serde_json::from_slice(&bytes).map_err(|_| {
            CheckFailure::new(
                "native-shape",
                &binding.change_id,
                "Invalid registered impact",
            )
        })?;
        records.push(record);
        report.checked_claims.push(format!(
            "native-release-impact:{}:{}",
            binding.change_id, binding.sha256
        ));
    }
    if matches!(request.stage, Some(CheckRequestStage::Linked)) {
        super::domain_bindings::check(path, Path::new(root), transition, &records, report)?;
        report.findings.push(Finding {
            code: "bindings-documentation-release-proof-later".into(),
            subject: "bindings".into(),
            reason: "Linked reference consistency does not verify documentation, guidance, rehearsal or final release closure.".into(),
            state: FindingState::Pending,
        });
    } else {
        report.findings.push(Finding {
            code: "bindings-linked-proof-later".into(),
            subject: "bindings".into(),
            reason: "Native roles confer no capability/migration linkage or release closure proof."
                .into(),
            state: FindingState::Pending,
        });
    }
    Ok(())
}
