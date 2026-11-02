//! Machine-readable extras of the JSON error envelope: the registry
//! walk-failure attempts (`error_kind`, `attempts`) and the application
//! subsystem's stable `code`, with `step` and `work` when an error carries
//! them (PROP-065 `#error-codes`).

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#output-format");

use serde_json::Value;

/// The first application error of the chain, if any.
pub(crate) fn app_error(err: &anyhow::Error) -> Option<&vibe_app::AppError> {
    err.chain()
        .find_map(|cause| cause.downcast_ref::<vibe_app::AppError>())
}

/// `code`, and `step` / `work` when the application error names them.
fn stamp_app_error(payload: &mut Value, err: &anyhow::Error) {
    let (Some(error), Value::Object(map)) = (app_error(err), payload) else {
        return;
    };
    map.insert("code".into(), Value::String(error.code().into()));
    if let Some(step) = error.step() {
        map.insert("step".into(), Value::String(step.into()));
    }
    if let Some(work) = error.work() {
        map.insert("work".into(), Value::String(work.into()));
    }
    // A parked agent step (PROP-064 `#parking`): the task files and the
    // command the hosting agent re-invokes after answering.
    if let vibe_app::AppError::Agent(vibe_app::AgentError::Parked(parked)) = error {
        let vibe_app::ParkedStep {
            run_id,
            run,
            task_files,
            resume,
            ..
        } = parked.as_ref();
        map.insert("run_id".into(), Value::String(run_id.clone()));
        map.insert("run".into(), Value::String(run.clone()));
        map.insert(
            "tasks".into(),
            Value::Array(task_files.iter().cloned().map(Value::String).collect()),
        );
        map.insert("resume".into(), Value::String(resume.clone()));
    }
}

/// A forced reinstall refused because a source no longer serves the bytes
/// the lock pins: the code, the package and both hashes, so a script can
/// tell this refusal from a failed fetch without reading the prose.
fn stamp_lock_hash_mismatch(payload: &mut Value, err: &anyhow::Error) {
    let found = err
        .chain()
        .find_map(|cause| cause.downcast_ref::<crate::commands::reinstall::LockHashMismatch>());
    let (Some(mismatch), Value::Object(map)) = (found, payload) else {
        return;
    };
    map.insert(
        "code".into(),
        Value::String(crate::commands::reinstall::LockHashMismatch::CODE.into()),
    );
    map.insert(
        "package".into(),
        serde_json::json!({ "group": mismatch.group, "name": mismatch.name }),
    );
    map.insert(
        "locked_hash".into(),
        Value::String(mismatch.expected.clone()),
    );
    map.insert("source_hash".into(), Value::String(mismatch.actual.clone()));
}

/// When the anyhow chain carries a known structured error variant,
/// surface its fields as machine-readable extras alongside the
/// stringified `error` field. Today the only well-known variant is
/// `DepProviderError::AggregateNotFound` (registry walk-failure
/// with per-registry `attempts`); future structured variants
/// extend this match. JSON consumers (CI, monitoring pipelines)
/// can branch on `error_kind` and read `attempts` without parsing
/// the prose.
pub(super) fn stamp_structured_error(payload: &mut Value, err: &anyhow::Error) {
    stamp_app_error(payload, err);
    stamp_lock_hash_mismatch(payload, err);
    // Walk the anyhow chain looking for a known structured error
    // variant. Today the only match is
    // `DepProviderError::AggregateNotFound` (registry walk
    // failure with per-registry attempts), but it can be reached
    // through two surfaces:
    //
    //   - directly as a `DepProviderError` (some call sites
    //     return it without a `SolveError` wrapper);
    //   - wrapped in `SolveError::Provider(...)` when the
    //     depsolver propagates it. `#[error(transparent)]` on
    //     that variant forwards Display but, in practice,
    //     anyhow's `chain()` does NOT walk through the wrapper
    //     to the inner — so we destructure manually here rather
    //     than rely on chain depth.
    for cause in err.chain() {
        let candidate: Option<&vibe_resolver::DepProviderError> =
            if let Some(d) = cause.downcast_ref::<vibe_resolver::DepProviderError>() {
                Some(d)
            } else if let Some(vibe_resolver::SolveError::Provider(d)) =
                cause.downcast_ref::<vibe_resolver::SolveError>()
            {
                Some(d)
            } else {
                None
            };
        let Some(provider_err) = candidate else {
            continue;
        };
        if let vibe_resolver::DepProviderError::AggregateNotFound {
            group,
            name,
            attempts,
            ..
        } = provider_err
            && let Value::Object(map) = payload
        {
            map.entry("error_kind".to_string())
                .or_insert_with(|| Value::String("package_not_found_everywhere".into()));
            map.entry("package".to_string())
                .or_insert_with(|| serde_json::json!({ "group": group.as_str(), "name": name }));
            if let Ok(serialised) = serde_json::to_value(attempts) {
                map.entry("attempts".to_string()).or_insert(serialised);
            }
            return;
        }
    }
}
