//! Engine-owned recovery of a durable applying transaction.
//!
//! Recovery runs before every ordinary binding, uses only the stored
//! transaction and its staged bytes, and never consults the current
//! declaration: a changed or removed declaration cannot wedge it, and after
//! recovery completes the ordinary desired reconciliation runs on top.

use std::path::Path;

use anyhow::{Context, Result};

use super::nofollow::Project;
use super::stage::Stage;
use super::state::read_receipt;
use super::transaction::{Plan, execute_plan, finalize_receipt};
use crate::pkgskill::PackageSkillReport;

/// Finish any pending applying transaction. `Ok(empty)` when there is none.
pub(crate) fn recover_pending(project_root: &Path) -> Result<Vec<PackageSkillReport>> {
    let project = Project::open(project_root)?;
    let _guard = project.lock(super::nofollow::LOCK_FILE)?;
    let Some(receipt) = read_receipt(&project)? else {
        return Ok(Vec::new());
    };
    let Some(applying) = receipt.applying.clone() else {
        return Ok(Vec::new());
    };
    let plan = Plan {
        key: applying.key.clone(),
        nonce: applying.nonce.clone(),
        before: receipt.binding.clone(),
        after: applying.binding.clone(),
    };
    // A durable intent written by an older build may still carry an
    // unsupported portable rename; refuse it before a stage is even loaded,
    // so recovery never publishes one either. Unwrapped for the same reason
    // as the planning call: the guard's message is the actionable one and
    // surfaces render only the top-level message.
    super::rename::ensure_no_portable_rename(&plan.before, &plan.after)?;
    // The durable stage is authoritative, and it owns only the plan-required
    // digests — the after-file digests of the plan's changed rows. A
    // removal-only plan that retains other bindings requires no stage; any
    // required digest that is missing, corrupt, or hardlinked refuses.
    let required = plan.required_stage_digests();
    let stage = if required.is_empty() {
        None
    } else {
        Some(Stage::existing(&project, &plan.nonce, &required)?)
    };
    let reports =
        execute_plan(&project, project_root, stage.as_ref(), &plan).with_context(|| {
            format!(
                "recovering pending package-skill transaction `{}`",
                plan.key
            )
        })?;
    finalize_receipt(&project, project_root, &plan)?;
    if let Some(stage) = stage {
        stage.cleanup(&project)?;
    }
    Ok(reports
        .into_iter()
        .filter(|report| report.status != "unchanged")
        .map(|mut report| {
            report.note = Some(format!("recovered pending transaction `{}`", plan.key));
            report
        })
        .collect())
}
