//! Final release-install reporting, kept separate from acquisition policy.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#provenance");

use anyhow::Result;

use crate::output;

use super::archive;

#[derive(Debug)]
pub(super) struct ActivationReport {
    pub(super) path_on_current_process: bool,
    pub(super) durable_path_changed: bool,
    pub(super) advisory_home_warning: Option<String>,
}

pub(super) fn emit_outcome(
    ctx: &output::Context,
    command: &str,
    outcome: &archive::InstallOutcome,
    activation: &ActivationReport,
) -> Result<()> {
    if ctx.is_json() {
        return ctx.emit_json(&serde_json::json!({
            "ok": true,
            "command": command,
            "selector": outcome.record.selector().to_string(),
            "instance": outcome.record.instance,
            "home": outcome.home.display().to_string(),
            "source": outcome.record.source_path,
            "payload_sha256": outcome.record.payload_sha256,
            "reused": outcome.reused,
            "vibe_index_restart_required": true,
            "path_on_current_process": activation.path_on_current_process,
            "durable_path_changed": activation.durable_path_changed,
            "reopen_shell": !activation.path_on_current_process,
            "advisory_home_warning": activation.advisory_home_warning,
        }));
    }
    ctx.summary(
        "note: a running `vibe-index serve` keeps its old process; restart it to use this instance.",
    );
    if activation.path_on_current_process {
        ctx.summary("PATH is ready in this process.");
    } else if activation.durable_path_changed {
        ctx.summary("durable PATH updated; reopen the shell to resolve the stable shims.");
    } else {
        ctx.summary("durable PATH was already configured; reopen this shell to pick it up.");
    }
    if let Some(warning) = &activation.advisory_home_warning {
        ctx.summary(&format!(
            "warning: active pointer switched, but advisory VIBEVM_HOME was not updated: {warning}"
        ));
    }
    ctx.summary(&format!(
        "{} {} — active",
        if outcome.reused {
            "reused"
        } else {
            "installed"
        },
        outcome.record.selector()
    ));
    Ok(())
}
