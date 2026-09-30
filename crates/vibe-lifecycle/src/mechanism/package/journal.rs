//! Owner journal for one fixed local static-file package operation.
//!
//! This is deliberately narrower than the legacy package executor. Ready
//! records that a copy MAY occur; only a separately published result after
//! actual output and A2 verification records completion. Existing keys are
//! inspection only and never invoke the provider again.

use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use vibe_core::manifest::{ArtifactInput, ArtifactKind, ExtensionHandler};
use vibe_extension_registry::{MechanismSelection, resolve_mechanism};
use vibe_safefs::{PinnedAbsentPath, Project, StableFileSnapshot};

use super::protocol::ResolvedInput;
use super::{PackageError, PackageExecution, PackageOutcome, execute_one};
use crate::mechanism::{BUILTIN_STATIC_FILE_NAME, BUILTIN_STATIC_FILE_PIN};
use crate::process::propose_run_id;
use crate::{
    ExistingSubmissionInspection, JournalAdmission, LifecycleLease, ReservationDecision,
    SubmissionNamespace, SubmissionReservations, SubmissionScope,
};

const SOURCE: &str = "source.bin";
const TARGET: &str = "launcher";
const OUTPUT: &str = "launcher.bin";
const OUTPUT_DIR: &str = "target/vibe-package/launcher";
const RECORD: &str = ".vibe/state/artifacts/launcher.bin.json";
const HOME: [&str; 2] = [".vibe", "package-operations"];
const FORMAT: &str = "vibevm.package.static-file-intent/1";
const SOURCE_CAP: usize = 2 * 1024 * 1024;
const INTENT_CAP: usize = 16 * 1024;
const C1_CAP: usize = 1024 * 1024;
const RECORD_CAP: usize = 64 * 1024;
const RESULT_CAP: usize = 16 * 1024;
const OUTPUT_FILE: &str = "target/vibe-package/launcher/launcher.bin";
const RESULT_FORMAT: &str = "vibevm.package.static-file-result/1";

#[derive(Debug, thiserror::Error)]
pub(crate) enum StaticFileJournalError {
    #[error("{0}")]
    Refused(String),
    #[error(transparent)]
    Submission(#[from] crate::SubmissionError),
    #[error(transparent)]
    Scratch(#[from] crate::process::ScratchError),
    #[error(transparent)]
    Lease(#[from] crate::LifecycleLeaseError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

type Result<T> = std::result::Result<T, StaticFileJournalError>;

fn refused(error: impl std::fmt::Display) -> StaticFileJournalError {
    StaticFileJournalError::Refused(error.to_string())
}

macro_rules! bail {
    ($($arg:tt)*) => { return Err(StaticFileJournalError::Refused(format!($($arg)*))) };
}
macro_rules! ensure {
    ($condition:expr, $($arg:tt)*) => {
        if !($condition) { bail!($($arg)*); }
    };
}

mod execution;
#[cfg(test)]
use execution::result_name;
pub(crate) use execution::{BoundSource, execute_fixed_static_file, verify_bound_source};
#[cfg(test)]
pub(crate) use execution::{
    execute_fixed_static_file_with_hook, execute_fixed_static_file_with_pre_execute_hook,
    execute_fixed_static_file_with_record_hook,
};
use execution::{read_completion, verify_completed};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StaticFileJournalInspection {
    /// The owner intent and C.1 Ready bit were read back; no copy is granted
    /// by this foundation API.
    IntentReady {
        run_id: String,
    },
    /// No trustworthy owner intent exists for the reserved run.
    RecoveryRequired {
        run_id: String,
    },
    /// Intent exists but no verified result can be furnished. Physical
    /// observations are candidates, never a completion proof.
    Uncertain {
        run_id: String,
        partial: StaticFilePartialEvidence,
    },
    /// The owner result was published after actual output and A2 record
    /// verification. Replays never invoke the provider again.
    Completed {
        run_id: String,
        output_sha256: String,
        artifact_record: String,
    },
    Retired {
        run_id: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StaticFilePartialEvidence {
    pub output_directory: ObservedPresence,
    pub output_file: ObservedFile,
    pub artifact_record: ObservedFile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ObservedPresence {
    Absent,
    Present,
    Unavailable(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ObservedFile {
    Absent,
    Present { sha256: String, bytes: u64 },
    Unavailable(String),
}

fn uncertain(lease: &LifecycleLease, run_id: String) -> StaticFileJournalInspection {
    StaticFileJournalInspection::Uncertain {
        run_id,
        partial: inspect_partial(lease),
    }
}

fn inspect_partial(lease: &LifecycleLease) -> StaticFilePartialEvidence {
    let output_directory = match lease
        .project()
        .dir_if_present(&["target", "vibe-package", TARGET])
    {
        Ok(Some(_)) => ObservedPresence::Present,
        Ok(None) => ObservedPresence::Absent,
        Err(error) => ObservedPresence::Unavailable(error.to_string()),
    };
    let output_file = match lease
        .project()
        .read_file_snapshot_bounded(OUTPUT_FILE, SOURCE_CAP)
    {
        Ok(Some(snapshot)) => ObservedFile::Present {
            sha256: snapshot.sha256,
            bytes: snapshot.size,
        },
        Ok(None) => ObservedFile::Absent,
        Err(error) => ObservedFile::Unavailable(error.to_string()),
    };
    let artifact_record = match lease.project().read_file_bounded(RECORD, RECORD_CAP) {
        Ok(Some(bytes)) => ObservedFile::Present {
            sha256: sha(&bytes),
            bytes: bytes.len() as u64,
        },
        Ok(None) => ObservedFile::Absent,
        Err(error) => ObservedFile::Unavailable(error.to_string()),
    };
    StaticFilePartialEvidence {
        output_directory,
        output_file,
        artifact_record,
    }
}

#[derive(Debug)]
struct Preflight {
    request: Vec<u8>,
    plan: Vec<u8>,
    source: StableFileSnapshot,
    output_absent: String,
    record_absent: String,
}

#[derive(Debug)]
struct Subject {
    request: Vec<u8>,
    source: StableFileSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Intent {
    format: String,
    run_id: String,
    request_sha256: String,
    plan_sha256: String,
    target: String,
    source: String,
    source_sha256: String,
    source_bytes: u64,
    output_dir: String,
    output_before: String,
    artifact_record: String,
    record_before: String,
    stage: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CompletionRecord {
    format: String,
    run_id: String,
    reservation_ref: String,
    plan_sha256: String,
    output_relative: String,
    output_sha256: String,
    output_bytes: u64,
    artifact_record: String,
    artifact_record_sha256: String,
}

/// Reserve and durably publish one owner intent under the SAME lease used by
/// C.1. This does no provider work and never creates an output or A2 record.
pub(crate) fn reserve_fixed_static_file_intent(
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    execution: &PackageExecution<'_>,
) -> Result<StaticFileJournalInspection> {
    let subject = subject(lease, execution)?;
    match SubmissionReservations::inspect_existing_request(
        lease,
        namespace,
        scope,
        key,
        &subject.request,
    )? {
        ExistingSubmissionInspection::RetiredUnreserved => {
            return Ok(StaticFileJournalInspection::Retired { run_id: None });
        }
        ExistingSubmissionInspection::Reserved {
            domain_run_id,
            journal_admission,
            reservation_ref,
            retired,
        } => {
            if retired {
                return Ok(StaticFileJournalInspection::Retired {
                    run_id: Some(domain_run_id),
                });
            }
            let observed = match read_intent(lease, &domain_run_id) {
                Ok(observed) => observed,
                Err(_) => return Ok(uncertain(lease, domain_run_id)),
            };
            return Ok(match (journal_admission, observed) {
                (JournalAdmission::Ready, Some(intent))
                    if intent.run_id == domain_run_id
                        && intent.request_sha256 == sha(&subject.request)
                        && intent.plan_sha256
                            == SubmissionReservations::inspect_reserved_plan_sha256(
                                lease,
                                namespace,
                                scope,
                                key,
                                &domain_run_id,
                            )? =>
                {
                    match (reservation_ref, read_completion(lease, &domain_run_id)) {
                        (Some(expected_ref), Ok(Some(completion)))
                            if completion.reservation_ref == expected_ref
                                && completion.plan_sha256 == intent.plan_sha256
                                && verify_completed(lease, &intent, &completion).is_ok() =>
                        {
                            StaticFileJournalInspection::Completed {
                                run_id: domain_run_id,
                                output_sha256: completion.output_sha256,
                                artifact_record: completion.artifact_record,
                            }
                        }
                        _ => uncertain(lease, domain_run_id),
                    }
                }
                _ => StaticFileJournalInspection::RecoveryRequired {
                    run_id: domain_run_id,
                },
            });
        }
        ExistingSubmissionInspection::Absent => {}
    }
    let selected = fresh_preflight(lease, execution, subject)?;
    let run_id = propose_run_id(lease.root())?;
    ensure!(
        matches!(
            SubmissionReservations::reserve(
                lease, namespace, scope, key, &selected.request, &selected.plan, &run_id,
            )?,
            ReservationDecision::Created { domain_run_id } if domain_run_id == run_id
        ),
        "fixed static-file intent did not create a fresh reservation"
    );
    let intent = Intent {
        format: FORMAT.into(),
        run_id: run_id.clone(),
        request_sha256: sha(&selected.request),
        plan_sha256: sha(&selected.plan),
        target: TARGET.into(),
        source: SOURCE.into(),
        source_sha256: selected.source.sha256.clone(),
        source_bytes: selected.source.size,
        output_dir: OUTPUT_DIR.into(),
        output_before: selected.output_absent.clone(),
        artifact_record: RECORD.into(),
        record_before: selected.record_absent.clone(),
        stage: "may-effect".into(),
    };
    let bytes = serde_json::to_vec(&intent)?;
    ensure!(bytes.len() <= INTENT_CAP, "static-file intent exceeds cap");
    let home = lease.project().ensure_subproject(&HOME).map_err(refused)?;
    let directory = home.root_dir().map_err(refused)?;
    home.publish_new_in(&directory, &intent_name(&run_id), &bytes)
        .map_err(|error| {
            refused(format!(
                "static-file intent publication: {:#}",
                error.into_report()
            ))
        })?;
    let readback = home
        .read_file_bounded_in(&directory, &intent_name(&run_id), INTENT_CAP)
        .map_err(refused)?
        .ok_or_else(|| refused("new static-file intent vanished before Ready"))?;
    ensure!(
        readback == bytes,
        "new static-file intent changed before Ready"
    );
    recheck_before_image(lease, &selected)?;
    SubmissionReservations::mark_journal_ready(lease, namespace, scope, key, &run_id)?;
    ensure!(
        SubmissionReservations::journal_admission(lease, namespace, scope, key, &run_id)?
            == JournalAdmission::Ready,
        "static-file intent was not Ready on readback"
    );
    Ok(StaticFileJournalInspection::IntentReady { run_id })
}

fn subject(lease: &LifecycleLease, execution: &PackageExecution<'_>) -> Result<Subject> {
    lease.ensure_root(execution.project_root, "at fixed static-file intent")?;
    ensure!(
        execution.package_root == PackageExecution::default_package_root(),
        "fixed static-file intent refuses another output root"
    );
    let [target] = execution.targets else {
        bail!("fixed static-file intent requires exactly one target")
    };
    ensure!(
        target.id == TARGET
            && target.mechanism.to_string() == "package:static-file"
            && target.provider.is_none()
            && target.when.is_none()
            && target.config.is_none(),
        "fixed static-file intent refuses variable target or provider"
    );
    ensure!(
        matches!(target.inputs.as_deref(), Some([ArtifactInput::Path { path }]) if path == Path::new(SOURCE)),
        "fixed static-file intent requires one exact local source"
    );
    ensure!(
        target.outputs.len() == 1
            && target.outputs[0].id == OUTPUT
            && target.outputs[0].kind == ArtifactKind::File
            && target.outputs[0].select.is_none(),
        "fixed static-file intent requires one exact file output"
    );
    let selection = resolve_mechanism(
        execution.registry,
        &target.mechanism,
        target.provider.as_ref(),
        execution.routes,
    )
    .map_err(refused)?;
    require_builtin(&selection)?;
    let source = lease
        .project()
        .read_file_snapshot_bounded(SOURCE, SOURCE_CAP)
        .map_err(refused)?
        .ok_or_else(|| refused("fixed static-file source is absent"))?;
    let root = root_value(lease.root())?;
    let request = serde_json::to_vec(&serde_json::json!({
        "operation": "package.static-file",
        "schema": 1,
        "root": root,
        "target": TARGET,
        "source": SOURCE,
        "source_sha256": source.sha256,
        "source_bytes": source.size,
        "output": OUTPUT,
    }))?;
    ensure!(
        request.len() <= C1_CAP,
        "static-file request exceeds C.1 cap"
    );
    Ok(Subject { request, source })
}

fn fresh_preflight(
    lease: &LifecycleLease,
    execution: &PackageExecution<'_>,
    subject: Subject,
) -> Result<Preflight> {
    let Subject { request, source } = subject;
    let output = absent(lease, OUTPUT_DIR)?;
    let record = absent(lease, RECORD)?;
    let plan = serde_json::to_vec(&serde_json::json!({
        "format": "vibevm.package.pre-effect-plan/1",
        "root": root_value(lease.root())?,
        "target": TARGET,
        "provider": BUILTIN_STATIC_FILE_PIN,
        "source": SOURCE,
        "source_sha256": source.sha256,
        "source_bytes": source.size,
        "output_dir": OUTPUT_DIR,
        "output_before": output.identity_token(),
        "artifact_record": RECORD,
        "record_before": record.identity_token(),
        "created_at": execution.created_at,
    }))?;
    ensure!(plan.len() <= C1_CAP, "static-file plan exceeds C.1 cap");
    Ok(Preflight {
        request,
        plan,
        source,
        output_absent: output.identity_token(),
        record_absent: record.identity_token(),
    })
}

fn require_builtin(selection: &MechanismSelection<'_>) -> Result<()> {
    ensure!(
        selection.row().pin().to_string() == BUILTIN_STATIC_FILE_PIN
            && matches!(selection.row().handler(), ExtensionHandler::Builtin { name } if name == BUILTIN_STATIC_FILE_NAME),
        "fixed static-file intent refuses a routed provider"
    );
    Ok(())
}

fn absent(lease: &LifecycleLease, relative: &str) -> Result<PinnedAbsentPath> {
    let pinned = Project::pin_absent_path(&lease.root().join(relative)).map_err(|error| {
        refused(format!(
            "required absent resource `{relative}` is occupied or unsafe: {error:#}"
        ))
    })?;
    ensure!(
        pinned.descends_from(lease.project()).map_err(refused)?,
        "required absent resource `{relative}` is outside the leased root"
    );
    Ok(pinned)
}

fn recheck_before_image(lease: &LifecycleLease, selected: &Preflight) -> Result<()> {
    let current = lease
        .project()
        .read_file_snapshot_bounded(SOURCE, SOURCE_CAP)
        .map_err(refused)?
        .ok_or_else(|| refused("fixed static-file source vanished before Ready"))?;
    ensure!(
        current == selected.source,
        "fixed static-file source changed before Ready"
    );
    ensure!(
        absent(lease, OUTPUT_DIR)?.identity_token() == selected.output_absent,
        "fixed static-file output slot changed before Ready"
    );
    ensure!(
        absent(lease, RECORD)?.identity_token() == selected.record_absent,
        "fixed static-file artifact-record slot changed before Ready"
    );
    Ok(())
}

fn read_intent(lease: &LifecycleLease, run_id: &str) -> Result<Option<Intent>> {
    ensure!(
        crate::process::is_valid_run_id(run_id),
        "static-file reservation has invalid run ID"
    );
    let Some(home) = lease.project().dir_if_present(&HOME).map_err(refused)? else {
        return Ok(None);
    };
    let Some(bytes) = lease
        .project()
        .read_file_bounded_in(&home, &intent_name(run_id), INTENT_CAP)
        .map_err(refused)?
    else {
        return Ok(None);
    };
    let intent: Intent = serde_json::from_slice(&bytes)?;
    ensure!(
        intent.format == FORMAT && intent.run_id == run_id && intent.stage == "may-effect",
        "static-file intent identity or format differs"
    );
    Ok(Some(intent))
}

fn intent_name(run_id: &str) -> String {
    format!("{run_id}.static-file-intent.json")
}
fn sha(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn root_value(root: &Path) -> Result<serde_json::Value> {
    ensure!(
        root.is_absolute(),
        "fixed static-file root must be absolute"
    );
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let bytes = root.as_os_str().as_bytes();
        ensure!(
            bytes.len() <= 128 * 1024,
            "fixed static-file root path exceeds cap"
        );
        return Ok(serde_json::json!({"encoding":"unix-bytes-hex","value":hex(bytes)}));
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let units: Vec<u16> = root.as_os_str().encode_wide().collect();
        ensure!(
            units.len() <= 64 * 1024,
            "fixed static-file root path exceeds cap"
        );
        let value = units
            .iter()
            .map(|unit| format!("{unit:04x}"))
            .collect::<String>();
        return Ok(serde_json::json!({"encoding":"windows-utf16-units-hex","value":value}));
    }
    #[cfg(not(any(unix, windows)))]
    bail!("fixed static-file root path has no lossless encoding")
}

#[cfg(unix)]
fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        out.push(DIGITS[usize::from(byte >> 4)] as char);
        out.push(DIGITS[usize::from(byte & 15)] as char);
    }
    out
}

#[cfg(test)]
#[path = "journal/tests.rs"]
mod tests;
