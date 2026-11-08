//! Final command outcome rendering preserves the existing typed error chain.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#OPERATION-EFFECTS-LAW");
use crate::{commands, exit_code::as_exit_code};
use std::process::ExitCode;
pub(crate) fn finish(
    ctx: &crate::output::Context,
    cancellation: &vibe_core::cancellation::CancellationToken,
    result: anyhow::Result<()>,
    preview_only: bool,
) -> ExitCode {
    if result.is_err() && cancellation.acknowledged() {
        let external = match result
            .as_ref()
            .err()
            .and_then(|error| error.downcast_ref::<vibe_publish::PublishError>())
        {
            Some(vibe_publish::PublishError::Cancelled {
                repository_creation,
                release_push,
                ..
            }) => {
                serde_json::json!({"repository_creation": repository_creation, "release_push": release_push})
            }
            _ => {
                serde_json::json!({"repository_creation": "not-classified", "release_push": "not-classified"})
            }
        };
        let report = serde_json::json!({
            "protocol": "vibe-project-cancellation/1", "outcome": "cancelled",
            "publication": cancellation.publication_state(),
            "cache_writes": if preview_only { "none" } else { "possible" }, "external_effects": external,
            "error": result.as_ref().err().map(|error| format!("{error:#}"))
        });
        if ctx.is_json() {
            println!("{report}");
        }
        if ctx.is_machine_progress() {
            eprintln!("{report}");
        }
        return ExitCode::from(130);
    }
    match result {
        Ok(()) => {
            // A command that returned without emitting its own document (a
            // pure-narration verb) still owes any plan it deferred.
            let _ = ctx.flush_json_plans();
            ExitCode::SUCCESS
        }
        Err(err) => {
            // A failure is an outcome too: release the held-back plan
            // documents rather than swallowing them with the error.
            let _ = ctx.flush_json_plans();
            // A quiet command's only line is the one below, so a traced
            // failure has nowhere else to report its trace. The wrapper is
            // consumed HERE and immediately: what comes back is the same error
            // object — same context chain for stderr, same downcast identity
            // for the exit code — and a suffix that only `HumanQuiet` prints.
            let (err, suffix) = commands::compile_trace::detach_quiet_suffix(err);
            ctx.error_with_suffix(&err, suffix.as_deref());
            as_exit_code(&err)
        }
    }
}
