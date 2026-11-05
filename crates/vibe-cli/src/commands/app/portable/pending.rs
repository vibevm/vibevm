//! A resumable input request keeps the completed import prefix visible.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-WAITING-REPORT");
use super::{Tally, output};
use anyhow::Result;
use vibe_app::SettingsRoot;
use vibe_wire::generated::application_import::{ApplicationImport, Pending, Refused, Skipped};
use vibe_wire::generated::shared::{
    ApplicationImportTarget, ApplicationImportWaitingInput, ApplicationPendingImportResult,
};

#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-WAITING-REPORT"
)]
pub(super) fn emit_pending(
    ctx: &output::Context,
    settings: &SettingsRoot,
    target: &ApplicationImportTarget,
    input: &ApplicationImportWaitingInput,
    prefix_results: &[ApplicationPendingImportResult],
) -> Result<Option<i32>> {
    let mut tally = Tally::default();
    tally.add_rows(prefix_results);
    if ctx.is_json() {
        ctx.emit_json(&ApplicationImport {
            protocol: vibe_app::IMPORT_PROTOCOL.into(),
            home: settings.path().display().to_string(),
            dry_run: false,
            installed: tally.installed.iter().map(|(key, _)| key.clone()).collect(),
            skipped: tally
                .skipped
                .iter()
                .map(|(key, reason)| Skipped {
                    key: key.clone(),
                    reason: reason.clone(),
                })
                .collect(),
            refused: tally
                .refused
                .iter()
                .map(|(key, code, message)| Refused {
                    key: key.clone(),
                    code: code.clone(),
                    message: message.clone(),
                })
                .collect(),
            licenses: tally.licenses,
            pending: Some(Pending {
                target: target.clone(),
                input: input.clone(),
            }),
        })?;
    } else {
        let mut lines = Vec::new();
        for (key, _) in &tally.installed {
            lines.push(format!("installed  {key}"));
        }
        for (key, reason) in &tally.skipped {
            lines.push(format!("skipped    {key}: {reason}"));
        }
        for (key, code, message) in &tally.refused {
            lines.push(format!("refused    {key}: {code}: {message}"));
        }
        lines.push(format!(
            "Import {} is waiting for input for {}: {}.",
            target.operation,
            input.key,
            input.parameters.join(", ")
        ));
        lines.push(format!("Import home: {}", target.home));
        lines.push(format!(
            "Resume in this home with: vibe app import --resume {} --secret-input <file>",
            target.operation
        ));
        ctx.block(&lines.join("\n"));
    }
    Ok(Some(2))
}
