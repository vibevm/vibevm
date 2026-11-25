//! Read-only selected data from exactly one admitted historical lock snapshot.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#EFFECT-CONSUMERS");
use crate::{cli::ShowFeaturesArgs, output};
use anyhow::{Result, bail};
use serde::Serialize;
use std::collections::BTreeMap;
use vibe_core::manifest::{Lockfile, ParameterValue};
#[derive(Serialize)]
struct Row {
    package: String,
    version: String,
    values: BTreeMap<String, ParameterValue>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    parameters: BTreeMap<String, ParameterValue>,
}
#[derive(Serialize)]
struct Report {
    protocol: &'static str,
    ok: bool,
    packages: Vec<Row>,
}
pub(super) fn run(ctx: &output::Context, args: ShowFeaturesArgs) -> Result<()> {
    let root = super::resolve_project_root(&args.path)?;
    let lock = Lockfile::read(root.join(Lockfile::FILENAME))?;
    let selected = vibe_workspace::Workspace::discover_selected(&root)?;
    if let vibe_workspace::freshness::Freshness::Stale(_) =
        vibe_workspace::freshness::configuration_freshness(&selected.workspace, &lock)
    {
        bail!(
            "retained package configuration is missing or disagrees with current consumer input; install explicitly before inspection"
        );
    }
    vibe_package_source::submitted_history::verify_submitted_inspection(&root, &lock)?;
    let mut packages = Vec::new();
    for (key, record) in &lock.configurations {
        let row = lock
            .packages
            .iter()
            .find(|p| format!("{}/{}", p.group, p.name) == *key)
            .ok_or_else(|| anyhow::anyhow!("retained configuration package is missing"))?;
        packages.push(Row {
            package: key.clone(),
            version: row.version.to_string(),
            values: record.selected_data(row.kind)?,
            parameters: record.selected_parameters(row.kind)?,
        });
    }
    let report = Report {
        protocol: "vibe-package-configuration-data/1",
        ok: true,
        packages,
    };
    if ctx.is_json() {
        ctx.emit_json(&report)?;
    } else if ctx.is_quiet() {
        let n = report.packages.len();
        ctx.summary(&format!(
            "{} {} with retained configuration data",
            n,
            if n == 1 { "package" } else { "packages" },
        ));
    } else {
        use super::configuration_view::{DataRow, data_view};
        use crate::output::language::{rule_width, viewport};
        let rows = report
            .packages
            .iter()
            .map(|row| DataRow {
                package: row.package.clone(),
                version: row.version.clone(),
                values: row
                    .values
                    .iter()
                    .map(|(key, value)| (key.clone(), value.to_string()))
                    .collect(),
                parameters: row
                    .parameters
                    .iter()
                    .map(|(key, value)| (key.clone(), value.to_string()))
                    .collect(),
            })
            .collect::<Vec<_>>();
        ctx.block(&data_view(&rows, viewport(), rule_width()));
    }
    Ok(())
}
