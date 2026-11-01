//! Final release-install reporting, kept separate from acquisition policy.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#provenance");

use anyhow::Result;

use crate::output;

use super::super::{VvmEnv, model::InstallRecord, store::VersionStore};
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
        return ctx.emit_json(&outcome_json(command, outcome, activation, true));
    }
    ctx.hint("a running vibe-index serve keeps its old process; restart it to use this instance");
    if activation.path_on_current_process {
        ctx.hint("PATH is ready in this process");
    } else if activation.durable_path_changed {
        ctx.hint("durable PATH updated; reopen the shell to resolve the stable shims");
    } else {
        ctx.hint("durable PATH was already configured; reopen this shell to pick it up");
    }
    if let Some(warning) = &activation.advisory_home_warning {
        ctx.diagnostic(&format!(
            "active pointer switched, but advisory VIBEVM_HOME was not updated: {warning}"
        ));
    }
    let verb = match (outcome.reused, command) {
        (true, _) => "reused",
        (false, "self:reinstall") => "reinstalled",
        (false, _) => "installed",
    };
    ctx.trailer_at(
        &format!("{verb} {} · active", outcome.record.selector()),
        &outcome.home.display().to_string(),
    );
    Ok(())
}

/// Report an already verified payload without repeating activation. Callers
/// establish integrity; this helper checks only the live active identity.
pub(in crate::commands::vvm) fn report_if_current(
    ctx: &output::Context,
    env: &VvmEnv,
    store: &VersionStore,
    record: &InstallRecord,
    command: &str,
) -> Result<bool> {
    if !store.active()?.is_some_and(|active| {
        active.version_id() == record.version_id() && active.instance == record.instance
    }) {
        return Ok(false);
    }
    if ctx.is_json() {
        let outcome = archive::InstallOutcome {
            record: record.clone(),
            home: store.instance_dir(&record.version_id(), record.instance),
            reused: true,
        };
        ctx.emit_json(&outcome_json(
            command,
            &outcome,
            &ActivationReport {
                path_on_current_process: super::super::path_has_dir(
                    env.path_var.as_deref(),
                    &store.shim_dir(),
                ),
                durable_path_changed: false,
                advisory_home_warning: None,
            },
            false,
        ))?;
    }
    Ok(true)
}

/// Explain a source update no-op using the checkout actually selected by the caller.
pub(in crate::commands::vvm) fn report_source_if_current(
    ctx: &output::Context,
    env: &VvmEnv,
    store: &VersionStore,
    record: &InstallRecord,
    source_root: &std::path::Path,
) -> Result<bool> {
    if !report_if_current(ctx, env, store, record, "self:update")? {
        return Ok(false);
    }
    if !ctx.is_json() {
        ctx.summary(&format!(
            "Nothing to update: you are building from source, and the source files have not changed.\nSource directory:\n{}",
            super::super::source::external_path(source_root),
        ));
    }
    Ok(true)
}

pub(super) fn outcome_json(
    command: &str,
    outcome: &archive::InstallOutcome,
    activation: &ActivationReport,
    restart_required: bool,
) -> serde_json::Value {
    serde_json::json!({
        "ok": true,
        "command": command,
        "selector": outcome.record.selector().to_string(),
        "instance": outcome.record.instance,
        "home": outcome.home.display().to_string(),
        "source": outcome.record.source_path,
        "payload_sha256": outcome.record.payload_sha256,
        "reused": outcome.reused,
        "vibe_index_restart_required": restart_required,
        "path_on_current_process": activation.path_on_current_process,
        "durable_path_changed": activation.durable_path_changed,
        "reopen_shell": !activation.path_on_current_process,
        "advisory_home_warning": activation.advisory_home_warning,
    })
}
