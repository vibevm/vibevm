//! Immutable Build inputs retained for inspection after the caller exits.
//!
//! The capsule contains canonical JSON identity bytes, not source or output
//! payloads. It is published and read back before C.1 Ready and never grants
//! another Cargo invocation.

use super::*;

const CAPSULE_FORMAT: &str = "vibevm.build.cargo-input/1";
const CAPSULE_CAP: usize = 4 * C1_CAP + 32 * 1024;

fn file_sha(bytes: &[u8]) -> String {
    sha(bytes).trim_start_matches("sha256:").to_owned()
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct InputCapsule {
    format: String,
    run_id: String,
    /// Same-host detection evidence, never a portable grant (inode/index ABA).
    root_token: String,
    request: String,
    plan: String,
    manifest_sha256: String,
    manifest_bytes: u64,
    main_sha256: String,
    main_bytes: u64,
}

fn capsule_name(run_id: &str) -> String {
    format!("{run_id}.cargo-input.json")
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
        manifest_sha256: selected.manifest.sha256.clone(),
        manifest_bytes: selected.manifest.size,
        main_sha256: selected.main.sha256.clone(),
        main_bytes: selected.main.size,
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
        "fixed Cargo input capsule exceeds cap"
    );
    let home = lease.project().subproject(&HOME).map_err(refused)?;
    let directory = home.root_dir().map_err(refused)?;
    home.publish_new_in(&directory, &capsule_name(run_id), &bytes)
        .map_err(|error| {
            refused(format!(
                "fixed Cargo input capsule publication: {:#}",
                error.into_report()
            ))
        })?;
    let visible = home
        .read_file_bounded_in(&directory, &capsule_name(run_id), CAPSULE_CAP)
        .map_err(refused)?
        .ok_or_else(|| refused("fixed Cargo input capsule vanished before Ready"))?;
    ensure!(
        visible == bytes,
        "fixed Cargo input capsule changed before Ready"
    );
    Ok(())
}

fn read(lease: &LifecycleLease, run_id: &str) -> Result<Option<InputCapsule>> {
    ensure!(
        crate::process::is_valid_run_id(run_id),
        "fixed Cargo capsule run ID invalid"
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
        read(lease, run_id)?.ok_or_else(|| refused("fixed Cargo input capsule is missing"))?;
    validate(lease, &capsule, intent, request_sha256, plan_sha256)
}

fn canonical_value(bytes: &[u8]) -> Result<serde_json::Value> {
    let value: serde_json::Value = serde_json::from_slice(bytes)?;
    ensure!(
        serde_json::to_vec(&value)? == bytes,
        "fixed Cargo capsule holds noncanonical JSON bytes"
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
            && capsule.manifest_sha256 == intent.manifest_sha256
            && capsule.main_sha256 == intent.main_sha256
            && capsule.manifest_sha256 == file_sha(MANIFEST_BYTES)
            && capsule.main_sha256 == file_sha(MAIN_BYTES)
            && capsule.manifest_bytes == MANIFEST_BYTES.len() as u64
            && capsule.main_bytes == MAIN_BYTES.len() as u64,
        "fixed Cargo capsule identity differs from reservation and intent"
    );
    let request = canonical_value(capsule.request.as_bytes())?;
    let plan = canonical_value(capsule.plan.as_bytes())?;
    ensure!(
        request.get("operation") == Some(&serde_json::json!("build.cargo-fixed"))
            && request.get("schema") == Some(&serde_json::json!(1))
            && request.get("root") == Some(&root_value(lease.root())?)
            && request.get("target") == Some(&serde_json::json!(TARGET))
            && request.get("output") == Some(&serde_json::json!(OUTPUT))
            && request.get("manifest_sha256") == Some(&serde_json::json!(capsule.manifest_sha256))
            && request.get("main_sha256") == Some(&serde_json::json!(capsule.main_sha256))
            && request.get("offline") == Some(&serde_json::json!(true)),
        "fixed Cargo capsule request posture differs"
    );
    ensure!(
        plan.get("format") == Some(&serde_json::json!("vibevm.build.pre-effect-plan/1"))
            && plan.get("root") == Some(&root_value(lease.root())?)
            && plan.get("target") == Some(&serde_json::json!(TARGET))
            && plan.get("provider") == Some(&serde_json::json!(BUILTIN_CARGO_PIN))
            && plan.get("offline") == Some(&serde_json::json!(true))
            && plan.get("manifest_sha256") == Some(&serde_json::json!(capsule.manifest_sha256))
            && plan.get("main_sha256") == Some(&serde_json::json!(capsule.main_sha256))
            && plan.get("absent") == Some(&serde_json::json!(intent.absent))
            && plan.get("cargo_home") == Some(&serde_json::json!(intent.cargo_home))
            && plan.get("toolchain_scope") == Some(&serde_json::json!("ambient-rustup-unproven")),
        "fixed Cargo capsule plan posture differs"
    );
    Ok(())
}

/// Read-only attach after API exit. The host retains only its original
/// namespace token, trusted scope and key; no BuildExecution is reconstructed.
pub(crate) fn inspect_fixed_cargo_cold(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
) -> Result<Option<BuildJournalInspection>> {
    let namespace = SubmissionNamespace::from_host_token(namespace_token.to_owned())?;
    let Some(existing) =
        SubmissionReservations::inspect_existing_key(lease, &namespace, scope, key)?
    else {
        return Ok(None);
    };
    let run_id = existing.domain_run_id;
    let cargo_home = cargo_home_relative(&namespace, key);
    let intent = match read_intent(lease, &run_id) {
        Ok(Some(intent)) => intent,
        _ if existing.journal_admission == JournalAdmission::Ready => {
            return Ok(Some(uncertain(lease, run_id, &cargo_home)));
        }
        _ => return Ok(Some(BuildJournalInspection::RecoveryRequired { run_id })),
    };
    let capsule = match read(lease, &run_id) {
        Ok(Some(capsule)) => capsule,
        _ if existing.journal_admission == JournalAdmission::Ready => {
            return Ok(Some(uncertain(lease, run_id, &intent.cargo_home)));
        }
        _ => return Ok(Some(BuildJournalInspection::RecoveryRequired { run_id })),
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
                uncertain(lease, run_id, &intent.cargo_home)
            } else {
                BuildJournalInspection::RecoveryRequired { run_id }
            },
        ));
    }
    if existing.journal_admission != JournalAdmission::Ready {
        return Ok(Some(BuildJournalInspection::RecoveryRequired { run_id }));
    }
    Ok(Some(execution::inspect_existing_result(
        lease,
        run_id,
        &intent,
        existing.reservation_ref.as_deref(),
    )))
}
