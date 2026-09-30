//! One fixed static-file effect after the owner intent is Ready.

use super::*;

pub(crate) struct BoundSource<'a> {
    pub lease: &'a LifecycleLease,
    pub source: &'a StableFileSnapshot,
    pub after_apply: Option<&'a dyn Fn()>,
}

pub(crate) fn verify_bound_source(
    bound: &BoundSource<'_>,
    resolved: &[ResolvedInput],
    target: &vibe_core::manifest::ArtifactPackageTarget,
) -> std::result::Result<(), PackageError> {
    let [input] = resolved else {
        return Err(PackageError::InputSourceMissing {
            target: target.id.clone(),
            input: SOURCE.into(),
            reason: "protected static-file input count changed".into(),
        });
    };
    let current = bound
        .lease
        .project()
        .read_file_snapshot_bounded(SOURCE, SOURCE_CAP)
        .map_err(|error| PackageError::InputSourceMissing {
            target: target.id.clone(),
            input: SOURCE.into(),
            reason: error.to_string(),
        })?
        .ok_or_else(|| PackageError::InputSourceMissing {
            target: target.id.clone(),
            input: SOURCE.into(),
            reason: "source vanished".into(),
        })?;
    if input.relative != SOURCE
        || input.digest != bound.source.sha256
        || input.bytes != bound.source.size
        || current != *bound.source
    {
        return Err(PackageError::InputStale {
            target: target.id.clone(),
            input: SOURCE.into(),
            path: SOURCE.into(),
            recorded: bound.source.sha256.clone(),
            found: current.sha256,
        });
    }
    Ok(())
}

/// Execute one fixed builtin only on the freshly journal-ready branch. An
/// existing key returns inspection and cannot reach `execute_one` again.
pub(crate) fn execute_fixed_static_file(
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    execution: &PackageExecution<'_>,
) -> Result<StaticFileJournalInspection> {
    execute_fixed_static_file_inner(lease, namespace, scope, key, execution, None, None, None)
}

#[cfg(test)]
pub(crate) fn execute_fixed_static_file_with_pre_execute_hook(
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    execution: &PackageExecution<'_>,
    before_execute: &dyn Fn(),
) -> Result<StaticFileJournalInspection> {
    execute_fixed_static_file_inner(
        lease,
        namespace,
        scope,
        key,
        execution,
        Some(before_execute),
        None,
        None,
    )
}

#[cfg(test)]
pub(crate) fn execute_fixed_static_file_with_hook(
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    execution: &PackageExecution<'_>,
    after_apply: &dyn Fn(),
) -> Result<StaticFileJournalInspection> {
    execute_fixed_static_file_inner(
        lease,
        namespace,
        scope,
        key,
        execution,
        None,
        Some(after_apply),
        None,
    )
}

#[cfg(test)]
pub(crate) fn execute_fixed_static_file_with_record_hook(
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    execution: &PackageExecution<'_>,
    after_record: &dyn Fn(),
) -> Result<StaticFileJournalInspection> {
    execute_fixed_static_file_inner(
        lease,
        namespace,
        scope,
        key,
        execution,
        None,
        None,
        Some(after_record),
    )
}

fn execute_fixed_static_file_inner(
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    execution: &PackageExecution<'_>,
    before_execute: Option<&dyn Fn()>,
    after_apply: Option<&dyn Fn()>,
    after_record: Option<&dyn Fn()>,
) -> Result<StaticFileJournalInspection> {
    let run_id = match reserve_fixed_static_file_intent(lease, namespace, scope, key, execution)? {
        StaticFileJournalInspection::IntentReady { run_id } => run_id,
        other => return Ok(other),
    };
    // The post-Ready source and absence witness is fresh. If this fails, the
    // durable intent remains and a retry can only inspect uncertainty.
    let selected = match subject(lease, execution)
        .and_then(|subject| fresh_preflight(lease, execution, subject))
    {
        Ok(selected) => selected,
        Err(_) => return Ok(uncertain(lease, run_id)),
    };
    let intent = match read_intent(lease, &run_id) {
        Ok(Some(intent)) => intent,
        Ok(None) => return Ok(StaticFileJournalInspection::RecoveryRequired { run_id }),
        Err(_) => return Ok(uncertain(lease, run_id)),
    };
    if intent.plan_sha256 != sha(&selected.plan)
        || SubmissionReservations::inspect_reserved_plan_sha256(
            lease, namespace, scope, key, &run_id,
        )? != intent.plan_sha256
        || SubmissionReservations::journal_admission(lease, namespace, scope, key, &run_id)?
            != JournalAdmission::Ready
        || recheck_before_image(lease, &selected).is_err()
    {
        return Ok(uncertain(lease, run_id));
    }
    if let Some(before_execute) = before_execute {
        before_execute();
    }
    let bound = BoundSource {
        lease,
        source: &selected.source,
        after_apply,
    };
    let [target] = execution.targets else {
        unreachable!("fixed preflight proved one target")
    };
    let outcome = match execute_one(execution, target, None, Some(&bound)) {
        Ok(outcome) => outcome,
        Err(_) => return Ok(uncertain(lease, run_id)),
    };
    if let Some(after_record) = after_record {
        after_record();
    }
    let (output, record_bytes) = match verify_actual(lease, &intent, &outcome) {
        Ok(verified) => verified,
        Err(_) => return Ok(uncertain(lease, run_id)),
    };
    let reservation_ref =
        SubmissionReservations::reservation_ref(lease, namespace, scope, key, &run_id)?;
    let completion = CompletionRecord {
        format: RESULT_FORMAT.into(),
        run_id: run_id.clone(),
        reservation_ref,
        plan_sha256: intent.plan_sha256,
        output_relative: OUTPUT_FILE.into(),
        output_sha256: output.sha256,
        output_bytes: output.size,
        artifact_record: RECORD.into(),
        artifact_record_sha256: sha(&record_bytes),
    };
    let bytes = serde_json::to_vec(&completion)?;
    ensure!(bytes.len() <= RESULT_CAP, "static-file result exceeds cap");
    let home = lease.project().subproject(&HOME).map_err(refused)?;
    let directory = home.root_dir().map_err(refused)?;
    if home
        .publish_new_in(&directory, &result_name(&run_id), &bytes)
        .is_err()
    {
        return Ok(uncertain(lease, run_id));
    }
    if !matches!(home.read_file_bounded_in(&directory, &result_name(&run_id), RESULT_CAP),
        Ok(Some(ref observed)) if *observed == bytes)
    {
        return Ok(uncertain(lease, run_id));
    }
    Ok(StaticFileJournalInspection::Completed {
        run_id,
        output_sha256: completion.output_sha256,
        artifact_record: completion.artifact_record,
    })
}

fn verify_actual(
    lease: &LifecycleLease,
    intent: &Intent,
    outcome: &PackageOutcome,
) -> Result<(StableFileSnapshot, Vec<u8>)> {
    let [produced] = outcome.produced.as_slice() else {
        bail!("fixed static-file executor did not produce exactly one artifact")
    };
    let output = lease
        .project()
        .read_file_snapshot_bounded(OUTPUT_FILE, SOURCE_CAP)
        .map_err(refused)?
        .ok_or_else(|| refused("fixed static-file output is absent"))?;
    let record_bytes = lease
        .project()
        .read_file_bounded(RECORD, RECORD_CAP)
        .map_err(refused)?
        .ok_or_else(|| refused("fixed static-file A2 record is absent"))?;
    let record: vibe_wire::generated::artifact_record::ArtifactRecord =
        serde_json::from_slice(&record_bytes)?;
    vibe_wire::behaviour::artifact_record::validate(&record).map_err(refused)?;
    ensure!(
        produced.id == OUTPUT
            && produced.path_relative == OUTPUT_FILE
            && produced.record == RECORD
            && produced.bytes == output.size
            && produced.digest == output.sha256,
        "fixed static-file executor result differs from actual output"
    );
    ensure!(
        record.id == OUTPUT
            && record.path_relative.path == OUTPUT_FILE
            && record.path_relative.root
                == vibe_wire::generated::artifact_record::RelativeRoot::Project
            && record.digest.value == output.sha256
            && record.digest.algorithm
                == vibe_wire::generated::artifact_record::DigestAlgorithm::Sha256
            && record.producer.target == TARGET
            && record.producer.mechanism == "package:static-file"
            && record.producer.provider.key == BUILTIN_STATIC_FILE_PIN
            && record.verification.status
                == vibe_wire::generated::artifact_record::VerificationStatus::Verified
            && output.sha256 == intent.source_sha256
            && output.size == intent.source_bytes,
        "fixed static-file A2 record or bytes differ from reserved intent"
    );
    Ok((output, record_bytes))
}

pub(super) fn verify_completed(
    lease: &LifecycleLease,
    intent: &Intent,
    completion: &CompletionRecord,
) -> Result<()> {
    ensure!(
        completion.format == RESULT_FORMAT
            && completion.run_id == intent.run_id
            && completion.output_relative == OUTPUT_FILE
            && completion.artifact_record == RECORD,
        "static-file completion identity differs from intent"
    );
    let output = lease
        .project()
        .read_file_snapshot_bounded(OUTPUT_FILE, SOURCE_CAP)
        .map_err(refused)?
        .ok_or_else(|| refused("static-file completed output is absent"))?;
    let record_bytes = lease
        .project()
        .read_file_bounded(RECORD, RECORD_CAP)
        .map_err(refused)?
        .ok_or_else(|| refused("static-file completed A2 record is absent"))?;
    let record: vibe_wire::generated::artifact_record::ArtifactRecord =
        serde_json::from_slice(&record_bytes)?;
    vibe_wire::behaviour::artifact_record::validate(&record).map_err(refused)?;
    ensure!(
        output.sha256 == completion.output_sha256
            && output.size == completion.output_bytes
            && output.sha256 == intent.source_sha256
            && output.size == intent.source_bytes
            && record.digest.value == output.sha256
            && record.id == OUTPUT
            && record.path_relative.path == OUTPUT_FILE
            && record.producer.target == TARGET
            && record.producer.provider.key == BUILTIN_STATIC_FILE_PIN
            && completion.artifact_record_sha256 == sha(&record_bytes),
        "static-file completed record or output changed"
    );
    Ok(())
}

pub(super) fn read_completion(
    lease: &LifecycleLease,
    run_id: &str,
) -> Result<Option<CompletionRecord>> {
    ensure!(
        crate::process::is_valid_run_id(run_id),
        "static-file result has invalid run ID"
    );
    let Some(home) = lease.project().dir_if_present(&HOME).map_err(refused)? else {
        return Ok(None);
    };
    let Some(bytes) = lease
        .project()
        .read_file_bounded_in(&home, &result_name(run_id), RESULT_CAP)
        .map_err(refused)?
    else {
        return Ok(None);
    };
    Ok(Some(serde_json::from_slice(&bytes)?))
}

pub(super) fn result_name(run_id: &str) -> String {
    format!("{run_id}.static-file-result.json")
}
