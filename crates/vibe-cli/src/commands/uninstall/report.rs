//! Human and canonical machine reporting for package closure removal.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#UNINSTALL-CLOSURE");
use super::plan;
use crate::output;
use anyhow::{Context, Result, bail};
use std::path::Path;
use vibe_core::Group;
use vibe_facts::{package_file_path, remove_package_file};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum AdoptionFactsDisposition {
    Absent,
    Removed(String),
    Kept(String),
}

pub(super) fn handle_adoption_facts(
    ctx: &output::Context,
    project_root: &Path,
    package: &str,
    interactive: bool,
    assume_yes: bool,
) -> Result<AdoptionFactsDisposition> {
    let file = package_file_path(project_root, package);
    if !file.is_file() {
        return Ok(AdoptionFactsDisposition::Absent);
    }
    let display = file
        .strip_prefix(project_root)
        .unwrap_or(&file)
        .to_string_lossy()
        .replace('\\', "/");
    let remove = if interactive && !assume_yes {
        ctx.suspend_progress(|| {
            crate::output::prompts::confirm(
                &format!("Remove its adoption facts ({display})?"),
                false,
            )
            .context("reading adoption-facts confirmation")
        })?
    } else {
        false
    };
    if !remove {
        return Ok(AdoptionFactsDisposition::Kept(display));
    }
    if !remove_package_file(project_root, package)? {
        bail!("adoption facts `{display}` disappeared before removal");
    }
    Ok(AdoptionFactsDisposition::Removed(display))
}

pub(super) fn emit_report(
    ctx: &output::Context,
    group: &Group,
    name: &str,
    version: &str,
    slot: &str,
    adoption_facts: &AdoptionFactsDisposition,
    removal: &plan::Plan,
) -> Result<()> {
    let removed = &removal.removed;
    if ctx.is_json() {
        use vibe_wire::generated::uninstall_report::{
            UninstallReport, UninstallReportAdoptionFacts, UninstallReportAdoptionFactsStatus,
        };
        let (status, path, hint) = match adoption_facts {
            AdoptionFactsDisposition::Absent => {
                (UninstallReportAdoptionFactsStatus::Absent, None, None)
            }
            AdoptionFactsDisposition::Removed(path) => (
                UninstallReportAdoptionFactsStatus::Removed,
                Some(path.clone()),
                None,
            ),
            AdoptionFactsDisposition::Kept(path) => (
                UninstallReportAdoptionFactsStatus::Kept,
                Some(path.clone()),
                Some("run `vibe facts clean` to drop orphaned overlays".to_owned()),
            ),
        };
        ctx.emit_json(&UninstallReport {
            ok: true,
            command: "uninstall".to_owned(),
            package: format!("{group}/{name}"),
            version: version.to_owned(),
            removed_slot: Some(slot.to_owned()),
            package_removed: Some(!slot.is_empty()),
            removed_count: removal.removed_count,
            paths: removed.iter().map(plan::slot).collect(),
            adoption_facts: Some(UninstallReportAdoptionFacts { status, path, hint }),
        })?;
        return Ok(());
    }
    let disposition = if slot.is_empty() {
        "declaration removed; package retained"
    } else {
        "removed"
    };
    if ctx.is_quiet() {
        let facts = match adoption_facts {
            AdoptionFactsDisposition::Kept(path) => {
                format!("; kept {path}; run `vibe facts clean` to drop orphaned overlays")
            }
            AdoptionFactsDisposition::Removed(path) => format!("; removed {path}"),
            AdoptionFactsDisposition::Absent => String::new(),
        };
        ctx.summary(&format!(
            "vibe uninstall: {group}/{name}@{version} {disposition}; {} packages removed{facts}",
            removed.len()
        ));
        return Ok(());
    }
    for row in removed {
        ctx.removed(&plan::slot(row));
    }
    if slot.is_empty() {
        ctx.skipped(
            &format!("{group}/{name}"),
            "still required by surviving packages",
        );
    }
    match adoption_facts {
        AdoptionFactsDisposition::Absent => {}
        AdoptionFactsDisposition::Removed(path) => ctx.removed(path),
        AdoptionFactsDisposition::Kept(path) => ctx.skipped(
            path,
            "adoption facts kept; run `vibe facts clean` to drop orphaned overlays",
        ),
    }
    // A removed slot is what "uninstalled" already says; only a retained
    // one needs its own word.
    let retained = if slot.is_empty() {
        format!(" · {disposition}")
    } else {
        String::new()
    };
    ctx.trailer(&format!(
        "uninstalled {group}/{name}@{version}{retained} · {} packages removed · boot regenerated",
        removed.len()
    ));
    Ok(())
}
