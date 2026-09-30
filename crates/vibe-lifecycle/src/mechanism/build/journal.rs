//! Owner journal for one fixed offline Cargo binary build.
//!
//! The intent and C.1 Ready bit precede every Cargo/rustc subprocess. A
//! freshly admitted key may run the controlled builtin once; existing keys
//! only inspect the retained result or partial state and never respawn it.

use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use vibe_core::manifest::{ArtifactKind, ExtensionHandler};
use vibe_extension_registry::resolve_mechanism;
use vibe_safefs::{Project, StableFileSnapshot};

use super::BuildExecution;
use crate::mechanism::cargo::CargoProvider;
use crate::mechanism::{BUILTIN_CARGO_NAME, BUILTIN_CARGO_PIN, BuildProvider, BuildTargetRequest};
use crate::process::propose_run_id;
use crate::{
    ExistingSubmissionInspection, JournalAdmission, LifecycleLease, ReservationDecision,
    SubmissionNamespace, SubmissionReservations, SubmissionScope,
};

const TARGET: &str = "fixture-tool";
const OUTPUT: &str = "fixture-tool.bin";
const MANIFEST: &str = "Cargo.toml";
const MAIN: &str = "src/main.rs";
const MANIFEST_BYTES: &[u8] = b"[package]\nname = \"vibe-r8-fixture\"\nversion = \"1.1.0\"\nedition = \"2021\"\n\n[[bin]]\nname = \"r8probe\"\npath = \"src/main.rs\"\n\n[workspace]\n";
const MAIN_BYTES: &[u8] = b"fn main() {\n    println!(\"r8\");\n}\n";
const ABSENT: [&str; 7] = [
    "Cargo.lock",
    "target",
    ".cargo",
    "build.rs",
    "rust-toolchain",
    "rust-toolchain.toml",
    ".vibe/state/artifacts/fixture-tool.bin.json",
];
const HOME: [&str; 2] = [".vibe", "build-operations"];
const FORMAT: &str = "vibevm.build.cargo-intent/1";
const INTENT_CAP: usize = 16 * 1024;
const C1_CAP: usize = 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub(crate) enum BuildJournalError {
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

type Result<T> = std::result::Result<T, BuildJournalError>;

fn refused(error: impl std::fmt::Display) -> BuildJournalError {
    BuildJournalError::Refused(error.to_string())
}

macro_rules! bail {
    ($($arg:tt)*) => { return Err(BuildJournalError::Refused(format!($($arg)*))) };
}
macro_rules! ensure {
    ($condition:expr, $($arg:tt)*) => { if !($condition) { bail!($($arg)*); } };
}

mod execution;
#[cfg(test)]
pub(crate) use execution::execute_fixed_cargo_with_hooks;
pub(crate) use execution::{BoundCargo, execute_fixed_cargo, verify_bound_sources};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BuildJournalInspection {
    /// The owner intent reached Ready; this observation alone does not grant
    /// another effect after an interruption.
    IntentReady {
        run_id: String,
    },
    RecoveryRequired {
        run_id: String,
    },
    Uncertain {
        run_id: String,
        partial: BuildPartialEvidence,
    },
    Completed {
        run_id: String,
        output_relative: String,
        output_sha256: String,
        artifact_record: String,
        warning: String,
    },
    Retired {
        run_id: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BuildPartialEvidence {
    pub target_root: ObservedPresence,
    pub cargo_home: ObservedPresence,
    pub cargo_lock: ObservedFile,
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

fn uncertain(lease: &LifecycleLease, run_id: String, cargo_home: &str) -> BuildJournalInspection {
    BuildJournalInspection::Uncertain {
        run_id,
        partial: inspect_partial(lease, cargo_home),
    }
}

fn inspect_partial(lease: &LifecycleLease, cargo_home: &str) -> BuildPartialEvidence {
    fn directory(lease: &LifecycleLease, parts: &[&str]) -> ObservedPresence {
        match lease.project().dir_if_present(parts) {
            Ok(Some(_)) => ObservedPresence::Present,
            Ok(None) => ObservedPresence::Absent,
            Err(error) => ObservedPresence::Unavailable(error.to_string()),
        }
    }
    fn file(lease: &LifecycleLease, path: &str) -> ObservedFile {
        match lease.project().read_file_bounded(path, 64 * 1024) {
            Ok(Some(bytes)) => ObservedFile::Present {
                sha256: sha(&bytes),
                bytes: bytes.len() as u64,
            },
            Ok(None) => ObservedFile::Absent,
            Err(error) => ObservedFile::Unavailable(error.to_string()),
        }
    }
    let cache_parts = cargo_home.split('/').collect::<Vec<_>>();
    BuildPartialEvidence {
        target_root: directory(lease, &["target"]),
        cargo_home: directory(lease, &cache_parts),
        cargo_lock: file(lease, "Cargo.lock"),
        artifact_record: file(lease, ".vibe/state/artifacts/fixture-tool.bin.json"),
    }
}

struct Subject {
    request: Vec<u8>,
    manifest: StableFileSnapshot,
    main: StableFileSnapshot,
}

struct Preflight {
    request: Vec<u8>,
    plan: Vec<u8>,
    manifest: StableFileSnapshot,
    main: StableFileSnapshot,
    absent: Vec<(String, String)>,
    cargo_home: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Intent {
    format: String,
    run_id: String,
    request_sha256: String,
    plan_sha256: String,
    manifest_sha256: String,
    main_sha256: String,
    absent: Vec<(String, String)>,
    cargo_home: String,
    stage: String,
}

/// Reserve and read back a durable intent before any Cargo invocation.
/// `IntentReady` is NOT an effect permit in this slice.
pub(crate) fn reserve_fixed_cargo_intent(
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    execution: &BuildExecution<'_>,
) -> Result<BuildJournalInspection> {
    let subject = subject(lease, execution)?;
    match SubmissionReservations::inspect_existing_request(
        lease,
        namespace,
        scope,
        key,
        &subject.request,
    )? {
        ExistingSubmissionInspection::RetiredUnreserved => {
            return Ok(BuildJournalInspection::Retired { run_id: None });
        }
        ExistingSubmissionInspection::Reserved {
            domain_run_id,
            journal_admission,
            reservation_ref,
            retired,
        } => {
            if retired {
                return Ok(BuildJournalInspection::Retired {
                    run_id: Some(domain_run_id),
                });
            }
            let intent = match read_intent(lease, &domain_run_id) {
                Ok(intent) => intent,
                Err(_) => {
                    return Ok(uncertain(
                        lease,
                        domain_run_id,
                        &cargo_home_relative(namespace, key),
                    ));
                }
            };
            return Ok(match (journal_admission, intent) {
                (JournalAdmission::Ready, Some(intent))
                    if intent.request_sha256 == sha(&subject.request)
                        && intent.plan_sha256
                            == SubmissionReservations::inspect_reserved_plan_sha256(
                                lease,
                                namespace,
                                scope,
                                key,
                                &domain_run_id,
                            )? =>
                {
                    execution::inspect_existing_result(
                        lease,
                        domain_run_id,
                        &intent,
                        reservation_ref.as_deref(),
                    )
                }
                _ => BuildJournalInspection::RecoveryRequired {
                    run_id: domain_run_id,
                },
            });
        }
        ExistingSubmissionInspection::Absent => {}
    }
    let selected = fresh_preflight(lease, execution, subject, namespace, key)?;
    let run_id = propose_run_id(lease.root())?;
    ensure!(
        matches!(SubmissionReservations::reserve(
        lease, namespace, scope, key, &selected.request, &selected.plan, &run_id,
    )?, ReservationDecision::Created { domain_run_id } if domain_run_id == run_id),
        "fixed Cargo intent did not create a fresh reservation"
    );
    let intent = Intent {
        format: FORMAT.into(),
        run_id: run_id.clone(),
        request_sha256: sha(&selected.request),
        plan_sha256: sha(&selected.plan),
        manifest_sha256: selected.manifest.sha256.clone(),
        main_sha256: selected.main.sha256.clone(),
        absent: selected.absent.clone(),
        stage: "may-effect".into(),
        cargo_home: selected.cargo_home.clone(),
    };
    let bytes = serde_json::to_vec(&intent)?;
    ensure!(bytes.len() <= INTENT_CAP, "fixed Cargo intent exceeds cap");
    let home = lease.project().ensure_subproject(&HOME).map_err(refused)?;
    let directory = home.root_dir().map_err(refused)?;
    home.publish_new_in(&directory, &intent_name(&run_id), &bytes)
        .map_err(|error| {
            refused(format!(
                "fixed Cargo intent publication: {:#}",
                error.into_report()
            ))
        })?;
    let visible = home
        .read_file_bounded_in(&directory, &intent_name(&run_id), INTENT_CAP)
        .map_err(refused)?
        .ok_or_else(|| refused("fixed Cargo intent vanished before Ready"))?;
    ensure!(visible == bytes, "fixed Cargo intent changed before Ready");
    recheck(lease, &selected)?;
    SubmissionReservations::mark_journal_ready(lease, namespace, scope, key, &run_id)?;
    ensure!(
        SubmissionReservations::journal_admission(lease, namespace, scope, key, &run_id)?
            == JournalAdmission::Ready,
        "fixed Cargo journal was not Ready on readback"
    );
    Ok(BuildJournalInspection::IntentReady { run_id })
}

fn subject(lease: &LifecycleLease, execution: &BuildExecution<'_>) -> Result<Subject> {
    lease.ensure_root(execution.project_root, "at fixed Cargo intent")?;
    ensure!(
        lease.root().to_str().is_some(),
        "fixed Cargo root must be UTF-8 because Cargo's --target-dir argv is textual"
    );
    ensure!(
        execution.build_root == BuildExecution::default_build_root() && execution.offline,
        "fixed Cargo intent requires default build root and offline posture"
    );
    let [target] = execution.targets else {
        bail!("fixed Cargo intent requires one target")
    };
    ensure!(
        target.id == TARGET
            && target.mechanism.to_string() == "build:cargo"
            && target.provider.is_none()
            && target.workdir == "."
            && target.inputs.is_none()
            && target.outputs.len() == 1,
        "fixed Cargo intent refuses variable target"
    );
    let output = &target.outputs[0];
    let select = output
        .select
        .as_ref()
        .ok_or_else(|| refused("fixed Cargo output select absent"))?;
    ensure!(
        output.id == OUTPUT
            && output.kind == ArtifactKind::Executable
            && select.as_table().len() == 2
            && select
                .as_table()
                .get("package")
                .and_then(toml::Value::as_str)
                == Some("vibe-r8-fixture")
            && select.as_table().get("bin").and_then(toml::Value::as_str) == Some("r8probe"),
        "fixed Cargo intent refuses variable output selector"
    );
    let config = target
        .config
        .as_ref()
        .ok_or_else(|| refused("fixed Cargo config absent"))?;
    ensure!(
        config.as_table().len() == 2
            && config
                .as_table()
                .get("profile")
                .and_then(toml::Value::as_str)
                == Some("release")
            && config
                .as_table()
                .get("offline")
                .and_then(toml::Value::as_bool)
                == Some(true),
        "fixed Cargo intent refuses variable config"
    );
    let selection = resolve_mechanism(
        execution.registry,
        &target.mechanism,
        target.provider.as_ref(),
        execution.routes,
    )
    .map_err(refused)?;
    ensure!(
        selection.row().pin().to_string() == BUILTIN_CARGO_PIN
            && matches!(selection.row().handler(), ExtensionHandler::Builtin { name } if name == BUILTIN_CARGO_NAME),
        "fixed Cargo intent refuses routed provider"
    );
    let request = BuildTargetRequest {
        target,
        project_root: execution.project_root,
        build_root: execution.build_root,
        offline: true,
    };
    let planned = CargoProvider.plan(&request).map_err(refused)?;
    ensure!(
        !planned.network && planned.outputs.len() == 1,
        "fixed Cargo plan may reach network or changed outputs"
    );
    let manifest = source(lease, MANIFEST, MANIFEST_BYTES.len())?;
    let main = source(lease, MAIN, MAIN_BYTES.len())?;
    ensure!(
        manifest.bytes == MANIFEST_BYTES && main.bytes == MAIN_BYTES,
        "fixed Cargo source differs from reviewed dependency-free fixture"
    );
    let request = serde_json::to_vec(&serde_json::json!({
        "operation":"build.cargo-fixed", "schema":1,
        "root":root_value(lease.root())?, "target":TARGET, "output":OUTPUT,
        "manifest_sha256":manifest.sha256, "main_sha256":main.sha256,
        "offline":true,
    }))?;
    ensure!(
        request.len() <= C1_CAP,
        "fixed Cargo request exceeds C.1 cap"
    );
    Ok(Subject {
        request,
        manifest,
        main,
    })
}

fn fresh_preflight(
    lease: &LifecycleLease,
    execution: &BuildExecution<'_>,
    subject: Subject,
    namespace: &SubmissionNamespace,
    key: &str,
) -> Result<Preflight> {
    let Subject {
        request,
        manifest,
        main,
    } = subject;
    let cargo_home = cargo_home_relative(namespace, key);
    let mut absent = Vec::with_capacity(ABSENT.len() + 1);
    for path in ABSENT
        .into_iter()
        .chain(std::iter::once(cargo_home.as_str()))
    {
        let pinned = Project::pin_absent_path(&lease.root().join(path)).map_err(|error| {
            refused(format!(
                "fixed Cargo resource `{path}` is occupied or unsafe: {error:#}"
            ))
        })?;
        ensure!(
            pinned.descends_from(lease.project()).map_err(refused)?,
            "fixed Cargo resource `{path}` is outside leased root"
        );
        absent.push((path.to_owned(), pinned.identity_token()));
    }
    let [target] = execution.targets else {
        unreachable!("fixed subject proved one target")
    };
    let request_view = BuildTargetRequest {
        target,
        project_root: execution.project_root,
        build_root: execution.build_root,
        offline: true,
    };
    let planned = CargoProvider.plan(&request_view).map_err(refused)?;
    let plan = serde_json::to_vec(&serde_json::json!({
        "format":"vibevm.build.pre-effect-plan/1",
        "root":root_value(lease.root())?, "target":TARGET,
        "provider":BUILTIN_CARGO_PIN, "offline":true,
        "manifest_sha256":manifest.sha256, "main_sha256":main.sha256,
        "metadata_argv":planned.metadata_argv, "build_argv":planned.build_argv,
        "absent":absent, "cargo_home":cargo_home,
        "toolchain_scope":"ambient-rustup-unproven",
        "created_at":execution.created_at,
    }))?;
    ensure!(plan.len() <= C1_CAP, "fixed Cargo plan exceeds C.1 cap");
    Ok(Preflight {
        request,
        plan,
        manifest,
        main,
        absent,
        cargo_home,
    })
}

fn cargo_home_relative(namespace: &SubmissionNamespace, key: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(b"vibevm.protected-cargo-home/1\0");
    hash.update(namespace.id().as_bytes());
    hash.update(b"\0");
    hash.update(key.as_bytes());
    let digest = format!("{:x}", hash.finalize());
    format!(".vibe/build-cargo-home/{}", &digest[..32])
}

fn source(lease: &LifecycleLease, path: &str, cap: usize) -> Result<StableFileSnapshot> {
    lease
        .project()
        .read_file_snapshot_bounded(path, cap)
        .map_err(refused)?
        .ok_or_else(|| refused(format!("fixed Cargo source `{path}` is absent")))
}

fn recheck(lease: &LifecycleLease, selected: &Preflight) -> Result<()> {
    ensure!(
        source(lease, MANIFEST, MANIFEST_BYTES.len())? == selected.manifest
            && source(lease, MAIN, MAIN_BYTES.len())? == selected.main,
        "fixed Cargo source changed before Ready"
    );
    for (path, token) in &selected.absent {
        let pinned = Project::pin_absent_path(&lease.root().join(path)).map_err(refused)?;
        ensure!(
            pinned.descends_from(lease.project()).map_err(refused)?
                && pinned.identity_token() == *token,
            "fixed Cargo resource `{path}` changed before Ready"
        );
    }
    Ok(())
}

fn read_intent(lease: &LifecycleLease, run_id: &str) -> Result<Option<Intent>> {
    ensure!(
        crate::process::is_valid_run_id(run_id),
        "fixed Cargo reservation has invalid run ID"
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
        "fixed Cargo intent identity differs"
    );
    Ok(Some(intent))
}

fn intent_name(run_id: &str) -> String {
    format!("{run_id}.cargo-intent.json")
}
fn sha(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn root_value(root: &Path) -> Result<serde_json::Value> {
    ensure!(root.is_absolute(), "fixed Cargo root must be absolute");
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let bytes = root.as_os_str().as_bytes();
        ensure!(
            bytes.len() <= 128 * 1024,
            "fixed Cargo root path exceeds cap"
        );
        return Ok(serde_json::json!({"encoding":"unix-bytes-hex","value":hex(bytes)}));
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let units: Vec<u16> = root.as_os_str().encode_wide().collect();
        ensure!(
            units.len() <= 64 * 1024,
            "fixed Cargo root path exceeds cap"
        );
        let value = units
            .iter()
            .map(|unit| format!("{unit:04x}"))
            .collect::<String>();
        return Ok(serde_json::json!({"encoding":"windows-utf16-units-hex","value":value}));
    }
    #[cfg(not(any(unix, windows)))]
    bail!("fixed Cargo root has no lossless path encoding")
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
