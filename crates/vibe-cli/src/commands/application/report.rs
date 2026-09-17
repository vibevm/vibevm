//! Human and machine reports for global application operations.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#commands");

use std::path::Path;

use anyhow::Result;
use vibe_wire::generated::application_report::{ApplicationReport, ApplicationReportCommand};

use super::model::{ApplicationProvenance, ApplicationSelection};
use crate::output;

pub(super) fn render(
    ctx: &output::Context,
    command: &str,
    application_id: &str,
    message: &str,
    host_root: &Path,
    provenance: Option<&ApplicationProvenance>,
) -> Result<()> {
    if ctx.is_json() {
        let command = match command {
            "install" => ApplicationReportCommand::Install,
            "update" => ApplicationReportCommand::Update,
            "uninstall" => ApplicationReportCommand::Uninstall,
            _ => unreachable!("closed application command vocabulary"),
        };
        return ctx.emit_json(&ApplicationReport {
            protocol: "vibe-application-command-report/1".into(),
            ok: true,
            command,
            application_id: application_id.into(),
            host_root: vibe_core::machine_json_path(host_root),
            message: message.into(),
            selected_mode: provenance.map(|value| match value.selected {
                ApplicationSelection::Source { .. } => {
                    vibe_wire::generated::application_report::ApplicationReportSelectedMode::Source
                }
                ApplicationSelection::Binary { .. } => {
                    vibe_wire::generated::application_report::ApplicationReportSelectedMode::Binary
                }
            }),
            selected_commit: provenance.and_then(|value| match &value.selected {
                ApplicationSelection::Source { commit, .. } => commit.clone(),
                ApplicationSelection::Binary { commit, .. } => Some(commit.clone()),
            }),
            available_source_commit: provenance.and_then(|value| {
                value
                    .available_source
                    .as_ref()
                    .map(|source| source.resolved_commit.clone())
            }),
        });
    }
    ctx.summary(&format!(
        "vibe {command} -g: application `{application_id}` — {message}"
    ));
    Ok(())
}
