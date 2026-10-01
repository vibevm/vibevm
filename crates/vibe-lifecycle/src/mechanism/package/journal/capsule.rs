//! Immutable static-file owner inputs for cold, read-only attach.
//! Retains identities and exact canonical JSON, never source/output payloads.

use super::*;

const CAPSULE_FORMAT: &str = "vibevm.package.static-file-input/1";
const CAPSULE_CAP: usize = 4 * C1_CAP + 32 * 1024;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct InputCapsule {
    format: String,
    run_id: String,
    /// Same-host detection evidence, never a portable grant (inode/index ABA).
    root_token: String,
    request: String,
    plan: String,
    source_sha256: String,
    source_bytes: u64,
}

fn capsule_name(run_id: &str) -> String {
    format!("{run_id}.static-file-input.json")
}

pub(super) fn publish(
    lease: &LifecycleLease,
    run_id: &str,
    selected: &Preflight,
    intent: &Intent,
) -> Result<()> {
    let capsule = InputCapsule {
        format: CAPSULE_FORMAT.into(),
        run_id: run_id.into(),
        root_token: lease.project().root_correlation_token().map_err(refused)?,
        request: String::from_utf8(selected.request.clone()).map_err(refused)?,
        plan: String::from_utf8(selected.plan.clone()).map_err(refused)?,
        source_sha256: selected.source.sha256.clone(),
        source_bytes: selected.source.size,
    };
    validate(
        lease,
        &capsule,
        intent,
        &sha(&selected.request),
        &sha(&selected.plan),
    )?;
    let bytes = serde_json::to_vec(&capsule)?;
    ensure!(
        bytes.len() <= CAPSULE_CAP,
        "static-file input capsule exceeds cap"
    );
    let home = lease.project().subproject(&HOME).map_err(refused)?;
    let directory = home.root_dir().map_err(refused)?;
    home.publish_new_in(&directory, &capsule_name(run_id), &bytes)
        .map_err(|error| {
            refused(format!(
                "static-file input capsule publication: {:#}",
                error.into_report()
            ))
        })?;
    let visible = home
        .read_file_bounded_in(&directory, &capsule_name(run_id), CAPSULE_CAP)
        .map_err(refused)?
        .ok_or_else(|| refused("static-file input capsule vanished before Ready"))?;
    ensure!(
        visible == bytes,
        "static-file input capsule changed before Ready"
    );
    Ok(())
}

fn read(lease: &LifecycleLease, run_id: &str) -> Result<Option<InputCapsule>> {
    ensure!(
        crate::process::is_valid_run_id(run_id),
        "static-file capsule run ID invalid"
    );
    let Some(home) = lease.project().dir_if_present(&HOME).map_err(refused)? else {
        return Ok(None);
    };
    let Some(bytes) = lease
        .project()
        .read_file_bounded_in(&home, &capsule_name(run_id), CAPSULE_CAP)
        .map_err(refused)?
    else {
        return Ok(None);
    };
    Ok(Some(serde_json::from_slice(&bytes)?))
}

pub(super) fn verify_existing(
    lease: &LifecycleLease,
    run_id: &str,
    intent: &Intent,
    request_sha256: &str,
    plan_sha256: &str,
) -> Result<()> {
    let capsule =
        read(lease, run_id)?.ok_or_else(|| refused("static-file input capsule is missing"))?;
    validate(lease, &capsule, intent, request_sha256, plan_sha256)
}

fn canonical_value(bytes: &[u8]) -> Result<serde_json::Value> {
    let value: serde_json::Value = serde_json::from_slice(bytes)?;
    ensure!(
        serde_json::to_vec(&value)? == bytes,
        "static-file capsule has noncanonical JSON"
    );
    Ok(value)
}

fn validate(
    lease: &LifecycleLease,
    capsule: &InputCapsule,
    intent: &Intent,
    request_sha256: &str,
    plan_sha256: &str,
) -> Result<()> {
    ensure!(
        capsule.format == CAPSULE_FORMAT
            && capsule.run_id == intent.run_id
            && capsule.root_token == lease.project().root_correlation_token().map_err(refused)?
            && sha(capsule.request.as_bytes()) == request_sha256
            && sha(capsule.plan.as_bytes()) == plan_sha256
            && intent.request_sha256 == request_sha256
            && intent.plan_sha256 == plan_sha256
            && capsule.source_sha256 == intent.source_sha256
            && capsule.source_bytes == intent.source_bytes
            && capsule.source_sha256.len() == 64
            && capsule
                .source_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit()),
        "static-file capsule identity differs from reservation and intent"
    );
    let request = canonical_value(capsule.request.as_bytes())?;
    let plan = canonical_value(capsule.plan.as_bytes())?;
    ensure!(
        request.get("operation") == Some(&serde_json::json!("package.static-file"))
            && request.get("schema") == Some(&serde_json::json!(1))
            && request.get("root") == Some(&root_value(lease.root())?)
            && request.get("target") == Some(&serde_json::json!(TARGET))
            && request.get("source") == Some(&serde_json::json!(SOURCE))
            && request.get("source_sha256") == Some(&serde_json::json!(capsule.source_sha256))
            && request.get("source_bytes") == Some(&serde_json::json!(capsule.source_bytes))
            && request.get("output") == Some(&serde_json::json!(OUTPUT)),
        "static-file capsule request posture differs"
    );
    ensure!(
        plan.get("format") == Some(&serde_json::json!("vibevm.package.pre-effect-plan/1"))
            && plan.get("root") == Some(&root_value(lease.root())?)
            && plan.get("target") == Some(&serde_json::json!(TARGET))
            && plan.get("provider") == Some(&serde_json::json!(BUILTIN_STATIC_FILE_PIN))
            && plan.get("source") == Some(&serde_json::json!(SOURCE))
            && plan.get("source_sha256") == Some(&serde_json::json!(capsule.source_sha256))
            && plan.get("source_bytes") == Some(&serde_json::json!(capsule.source_bytes))
            && plan.get("output_dir") == Some(&serde_json::json!(intent.output_dir))
            && plan.get("output_before") == Some(&serde_json::json!(intent.output_before))
            && plan.get("artifact_record") == Some(&serde_json::json!(intent.artifact_record))
            && plan.get("record_before") == Some(&serde_json::json!(intent.record_before)),
        "static-file capsule plan posture differs"
    );
    Ok(())
}

/// Inspect a prior fixed run after its PackageExecution and lease are gone.
/// This neither observes current source bytes nor grants another copy.
pub(crate) fn inspect_fixed_static_file_cold(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
) -> Result<Option<StaticFileJournalInspection>> {
    let namespace = SubmissionNamespace::from_host_token(namespace_token.to_owned())?;
    let Some(existing) =
        SubmissionReservations::inspect_existing_key(lease, &namespace, scope, key)?
    else {
        return Ok(None);
    };
    let run_id = existing.domain_run_id;
    let intent = match read_intent(lease, &run_id) {
        Ok(Some(intent)) => intent,
        _ if existing.journal_admission == JournalAdmission::Ready => {
            return Ok(Some(uncertain(lease, run_id)));
        }
        _ => {
            return Ok(Some(StaticFileJournalInspection::RecoveryRequired {
                run_id,
            }));
        }
    };
    let capsule = match read(lease, &run_id) {
        Ok(Some(capsule)) => capsule,
        _ if existing.journal_admission == JournalAdmission::Ready => {
            return Ok(Some(uncertain(lease, run_id)));
        }
        _ => {
            return Ok(Some(StaticFileJournalInspection::RecoveryRequired {
                run_id,
            }));
        }
    };
    if validate(
        lease,
        &capsule,
        &intent,
        &existing.request_sha256,
        &existing.plan_sha256,
    )
    .is_err()
    {
        return Ok(Some(
            if existing.journal_admission == JournalAdmission::Ready {
                uncertain(lease, run_id)
            } else {
                StaticFileJournalInspection::RecoveryRequired { run_id }
            },
        ));
    }
    if existing.journal_admission != JournalAdmission::Ready {
        return Ok(Some(StaticFileJournalInspection::RecoveryRequired {
            run_id,
        }));
    }
    let inspection = match (existing.reservation_ref, read_completion(lease, &run_id)) {
        (Some(expected_ref), Ok(Some(completion)))
            if completion.reservation_ref == expected_ref
                && completion.plan_sha256 == intent.plan_sha256
                && verify_completed(lease, &intent, &completion).is_ok() =>
        {
            StaticFileJournalInspection::Completed {
                run_id,
                output_sha256: completion.output_sha256,
                artifact_record: completion.artifact_record,
            }
        }
        _ => uncertain(lease, run_id),
    };
    Ok(Some(inspection))
}
