//! Fixed Cargo effect behind one Ready owner intent.

use super::*;

use crate::mechanism::build::{BuildError, BuildOutcome};
use crate::mechanism::cargo::ProtectedCargoEnvironment;

const OUTPUT_CAP: usize = 32 * 1024 * 1024;
const RECORD_CAP: usize = 64 * 1024;
const RESULT_CAP: usize = 16 * 1024;
const RECORD: &str = ".vibe/state/artifacts/fixture-tool.bin.json";
const RESULT_FORMAT: &str = "vibevm.build.cargo-result/1";
const FIXED_OUTPUT_RELATIVE: &str = if cfg!(windows) {
    "target/release/r8probe.exe"
} else {
    "target/release/r8probe"
};
const TOOLCHAIN_WARNING: &str = "host PATH/rustup toolchain and ancestor Cargo config effects remain outside the project-local Cargo home witness";

pub(crate) struct BoundCargo<'a> {
    pub lease: &'a LifecycleLease,
    pub manifest: &'a StableFileSnapshot,
    pub main: &'a StableFileSnapshot,
    pub cargo_home: &'a Path,
    pub on_spawn: Option<&'a dyn Fn(&str, &[String])>,
    pub after_apply: Option<&'a dyn Fn()>,
}

impl BoundCargo<'_> {
    pub fn environment(&self) -> ProtectedCargoEnvironment<'_> {
        ProtectedCargoEnvironment {
            cargo_home: self.cargo_home,
            on_spawn: self.on_spawn,
        }
    }
}

pub(crate) fn verify_bound_sources(bound: &BoundCargo<'_>) -> std::result::Result<(), BuildError> {
    for (path, expected) in [(MANIFEST, bound.manifest), (MAIN, bound.main)] {
        let current = bound
            .lease
            .project()
            .read_file_snapshot_bounded(path, expected.bytes.len())
            .map_err(|error| BuildError::ProtectedSource {
                target: TARGET.into(),
                reason: format!("{path}: {error:#}"),
            })?
            .ok_or_else(|| BuildError::ProtectedSource {
                target: TARGET.into(),
                reason: format!("{path} vanished"),
            })?;
        if current != *expected {
            return Err(BuildError::ProtectedSource {
                target: TARGET.into(),
                reason: format!("{path} differs from reserved source"),
            });
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Completion {
    format: String,
    run_id: String,
    reservation_ref: String,
    plan_sha256: String,
    output_relative: String,
    output_sha256: String,
    output_bytes: u64,
    artifact_record: String,
    artifact_record_sha256: String,
    cargo_home: String,
    toolchain_warning: String,
}

pub(crate) fn execute_fixed_cargo(
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    execution: &BuildExecution<'_>,
) -> Result<BuildJournalInspection> {
    execute_fixed_cargo_inner(lease, namespace, scope, key, execution, None, None)
}

#[cfg(test)]
pub(crate) fn execute_fixed_cargo_with_hooks(
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    execution: &BuildExecution<'_>,
    on_spawn: Option<&dyn Fn(&str, &[String])>,
    after_apply: Option<&dyn Fn()>,
) -> Result<BuildJournalInspection> {
    execute_fixed_cargo_inner(
        lease,
        namespace,
        scope,
        key,
        execution,
        on_spawn,
        after_apply,
    )
}

fn execute_fixed_cargo_inner(
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    execution: &BuildExecution<'_>,
    on_spawn: Option<&dyn Fn(&str, &[String])>,
    after_apply: Option<&dyn Fn()>,
) -> Result<BuildJournalInspection> {
    let run_id = match reserve_fixed_cargo_intent(lease, namespace, scope, key, execution)? {
        BuildJournalInspection::IntentReady { run_id } => run_id,
        other => return Ok(other),
    };
    let selected = match subject(lease, execution)
        .and_then(|subject| fresh_preflight(lease, execution, subject, namespace, key))
    {
        Ok(selected) => selected,
        Err(_) => {
            return Ok(uncertain(
                lease,
                run_id,
                &cargo_home_relative(namespace, key),
            ));
        }
    };
    let intent = match read_intent(lease, &run_id) {
        Ok(Some(intent)) => intent,
        _ => return Ok(uncertain(lease, run_id, &selected.cargo_home)),
    };
    if intent.plan_sha256 != sha(&selected.plan)
        || intent.cargo_home != selected.cargo_home
        || SubmissionReservations::inspect_reserved_plan_sha256(
            lease, namespace, scope, key, &run_id,
        )? != intent.plan_sha256
        || SubmissionReservations::journal_admission(lease, namespace, scope, key, &run_id)?
            != JournalAdmission::Ready
        || recheck(lease, &selected).is_err()
    {
        return Ok(uncertain(lease, run_id, &selected.cargo_home));
    }
    let cache_name = selected
        .cargo_home
        .rsplit('/')
        .next()
        .expect("fixed cache name");
    let cache = match lease
        .project()
        .ensure_subproject(&[".vibe", "build-cargo-home", cache_name])
    {
        Ok(cache) => cache,
        Err(_) => return Ok(uncertain(lease, run_id, &selected.cargo_home)),
    };
    let bound = BoundCargo {
        lease,
        manifest: &selected.manifest,
        main: &selected.main,
        cargo_home: cache.root_path(),
        on_spawn,
        after_apply,
    };
    let [target] = execution.targets else {
        unreachable!("fixed subject proved one target")
    };
    let outcome = match super::super::execute_one(execution, target, None, Some(&bound)) {
        Ok(outcome) => outcome,
        Err(_) => return Ok(uncertain(lease, run_id, &selected.cargo_home)),
    };
    let (relative, output, record_bytes) = match verify_actual(lease, &outcome) {
        Ok(verified) => verified,
        Err(_) => return Ok(uncertain(lease, run_id, &selected.cargo_home)),
    };
    let reservation_ref =
        SubmissionReservations::reservation_ref(lease, namespace, scope, key, &run_id)?;
    let completion = Completion {
        format: RESULT_FORMAT.into(),
        run_id: run_id.clone(),
        reservation_ref,
        plan_sha256: intent.plan_sha256,
        output_relative: relative,
        output_sha256: output.sha256,
        output_bytes: output.size,
        artifact_record: RECORD.into(),
        artifact_record_sha256: sha(&record_bytes),
        cargo_home: selected.cargo_home.clone(),
        toolchain_warning: TOOLCHAIN_WARNING.into(),
    };
    let bytes = serde_json::to_vec(&completion)?;
    ensure!(bytes.len() <= RESULT_CAP, "fixed Cargo result exceeds cap");
    let home = lease.project().subproject(&HOME).map_err(refused)?;
    let directory = home.root_dir().map_err(refused)?;
    if home
        .publish_new_in(&directory, &result_name(&run_id), &bytes)
        .is_err()
        || !matches!(home.read_file_bounded_in(&directory, &result_name(&run_id), RESULT_CAP),
            Ok(Some(ref observed)) if *observed == bytes)
    {
        return Ok(uncertain(lease, run_id, &selected.cargo_home));
    }
    Ok(completed(run_id, completion))
}

pub(super) fn inspect_existing_result(
    lease: &LifecycleLease,
    run_id: String,
    intent: &Intent,
    reservation_ref: Option<&str>,
) -> BuildJournalInspection {
    let completion = read_result(lease, &run_id);
    match (reservation_ref, completion) {
        (Some(expected), Ok(Some(result)))
            if result.reservation_ref == expected
                && result.plan_sha256 == intent.plan_sha256
                && result.cargo_home == intent.cargo_home
                && verify_completion(lease, &result).is_ok() =>
        {
            completed(run_id, result)
        }
        _ => uncertain(lease, run_id, &intent.cargo_home),
    }
}

fn completed(run_id: String, result: Completion) -> BuildJournalInspection {
    BuildJournalInspection::Completed {
        run_id,
        output_relative: result.output_relative,
        output_sha256: result.output_sha256,
        artifact_record: result.artifact_record,
        warning: result.toolchain_warning,
    }
}

fn verify_actual(
    lease: &LifecycleLease,
    outcome: &BuildOutcome,
) -> Result<(String, StableFileSnapshot, Vec<u8>)> {
    let [produced] = outcome.produced.as_slice() else {
        bail!("fixed Cargo executor did not produce exactly one artifact")
    };
    ensure!(
        outcome.target == TARGET
            && outcome.mechanism == "build:cargo"
            && outcome.provider == BUILTIN_CARGO_PIN
            && produced.id == OUTPUT
            && produced.record == RECORD
            && produced.path_relative == FIXED_OUTPUT_RELATIVE,
        "fixed Cargo selected result differs from reserved target"
    );
    let output = lease
        .project()
        .read_linked_regular_snapshot_bounded(&produced.path_relative, OUTPUT_CAP)
        .map_err(refused)?
        .ok_or_else(|| refused("fixed Cargo output is absent"))?;
    let record_bytes = lease
        .project()
        .read_file_bounded(RECORD, RECORD_CAP)
        .map_err(refused)?
        .ok_or_else(|| refused("fixed Cargo A2 record is absent"))?;
    let record: vibe_wire::generated::artifact_record::ArtifactRecord =
        serde_json::from_slice(&record_bytes)?;
    vibe_wire::behaviour::artifact_record::validate(&record).map_err(refused)?;
    ensure!(
        produced.bytes == output.size
            && produced.digest == output.sha256
            && record.id == OUTPUT
            && record.digest.value == output.sha256
            && record.kind == vibe_wire::generated::artifact_record::ArtifactKind::Executable
            && record.shape == vibe_wire::generated::artifact_record::ArtifactShape::File
            && record.digest.algorithm
                == vibe_wire::generated::artifact_record::DigestAlgorithm::Sha256
            && record.path_absolute == produced.path_absolute
            && record.path_relative.path == produced.path_relative
            && record.path_relative.root
                == vibe_wire::generated::artifact_record::RelativeRoot::Project
            && record.producer.target == TARGET
            && record.producer.mechanism == "build:cargo"
            && record.producer.provider.key == BUILTIN_CARGO_PIN
            && record
                .freshness
                .toolchain
                .as_ref()
                .is_some_and(|digest| digest.len() == 64)
            && record.verification.status
                == vibe_wire::generated::artifact_record::VerificationStatus::Verified,
        "fixed Cargo A2 record differs from produced output"
    );
    Ok((produced.path_relative.clone(), output, record_bytes))
}

fn verify_completion(lease: &LifecycleLease, result: &Completion) -> Result<()> {
    ensure!(
        result.format == RESULT_FORMAT
            && result.artifact_record == RECORD
            && result.toolchain_warning == TOOLCHAIN_WARNING
            && result.output_relative == FIXED_OUTPUT_RELATIVE,
        "fixed Cargo completion identity differs"
    );
    let output = lease
        .project()
        .read_linked_regular_snapshot_bounded(&result.output_relative, OUTPUT_CAP)
        .map_err(refused)?
        .ok_or_else(|| refused("completed Cargo output is absent"))?;
    let record_bytes = lease
        .project()
        .read_file_bounded(RECORD, RECORD_CAP)
        .map_err(refused)?
        .ok_or_else(|| refused("completed Cargo A2 record is absent"))?;
    let record: vibe_wire::generated::artifact_record::ArtifactRecord =
        serde_json::from_slice(&record_bytes)?;
    vibe_wire::behaviour::artifact_record::validate(&record).map_err(refused)?;
    ensure!(
        output.sha256 == result.output_sha256
            && output.size == result.output_bytes
            && record.id == OUTPUT
            && record.digest.value == output.sha256
            && record.kind == vibe_wire::generated::artifact_record::ArtifactKind::Executable
            && record.shape == vibe_wire::generated::artifact_record::ArtifactShape::File
            && record.digest.algorithm
                == vibe_wire::generated::artifact_record::DigestAlgorithm::Sha256
            && record.path_relative.path == result.output_relative
            && record.producer.target == TARGET
            && record.producer.mechanism == "build:cargo"
            && record.producer.provider.key == BUILTIN_CARGO_PIN
            && record
                .freshness
                .toolchain
                .as_ref()
                .is_some_and(|digest| digest.len() == 64)
            && result.artifact_record_sha256 == sha(&record_bytes),
        "completed Cargo result or output changed"
    );
    Ok(())
}

fn read_result(lease: &LifecycleLease, run_id: &str) -> Result<Option<Completion>> {
    ensure!(
        crate::process::is_valid_run_id(run_id),
        "fixed Cargo result has invalid run ID"
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

fn result_name(run_id: &str) -> String {
    format!("{run_id}.cargo-result.json")
}
