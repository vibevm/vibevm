//! Final command outcome rendering preserves the existing typed error chain.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#OPERATION-EFFECTS-LAW");
use crate::{commands, exit_code::as_exit_code};
use specmark::spec;
use std::process::ExitCode;
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-073#OPERATION-EFFECTS-LAW")]
pub(crate) fn finish(
    ctx: &crate::output::Context,
    cancellation: &vibe_core::cancellation::CancellationToken,
    result: anyhow::Result<()>,
    preview_only: bool,
) -> ExitCode {
    if result.is_err() && cancellation.acknowledged() {
        let mut external = match result
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
        // A paid LLM request is a remote effect no local restore undoes; it is
        // reported beside publication, never folded into it.
        external["llm"] = cancellation.llm_effect().into();
        // Killing the native helper is not rollback of what a plugin did
        // (spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-STOP-LAW).
        external["native"] = vibe_lifecycle::native::native_plugin_effects(cancellation).into();
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
        if !ctx.is_json() && !ctx.is_machine_progress() {
            let what = stop_trailer_what(
                ctx.stopped_step().as_deref(),
                cancellation.publication_state(),
            );
            ctx.trailer_on(crate::output::language::Stream::Err, false, &what);
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
            // A question left with Escape or Ctrl-C stopped the command; it
            // did not fail (PROP-067 `#question`).
            if crate::exit_code::is_cancelled(&err) && !ctx.is_json() {
                ctx.cancelled();
                return as_exit_code(&err);
            }
            ctx.error_with_suffix(&err, suffix.as_deref());
            as_exit_code(&err)
        }
    }
}

/// What a stopped run's trailer says: where the stop caught it and what
/// became of its changes, read from the token's publication state.
fn stop_trailer_what(step: Option<&str>, publication: &str) -> String {
    let head = match step {
        Some(step) => format!("stopped at {step}"),
        None => "stopped".to_owned(),
    };
    let changes = match publication {
        "not-started" => "nothing changed",
        "restored" => "rolled back, nothing changed",
        "committed" => "already committed, kept",
        _ => "recovery required, not rolled back",
    };
    format!("{head} · {changes}")
}

#[cfg(test)]
mod tests {
    use super::stop_trailer_what;

    #[test]
    fn stop_trailer_names_step_and_rollback() {
        assert_eq!(
            stop_trailer_what(Some("wait"), "restored"),
            "stopped at wait · rolled back, nothing changed"
        );
        assert_eq!(
            stop_trailer_what(None, "not-started"),
            "stopped · nothing changed"
        );
        assert_eq!(
            stop_trailer_what(Some("wait"), "committed"),
            "stopped at wait · already committed, kept"
        );
        assert_eq!(
            stop_trailer_what(Some("wait"), "recovery-required"),
            "stopped at wait · recovery required, not rolled back"
        );
    }
}
