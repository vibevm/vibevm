//! Apply a plan the way the install command does: through the lifecycle
//! handler engine, so the package's `slot:` contributions really run.
//!
//! Shared by several test binaries; each uses the subset it needs.
#![allow(dead_code)]

use std::path::Path;
use std::sync::Arc;

use vibe_core::manifest::SpecFormat;
use vibe_core::user_config::SlotIntegrity;
use vibe_install::{ApplyReport, InstallSource, PlannedInstall, SlotLifecycleSeams};
use vibe_lifecycle::process::StreamMode;
use vibe_lifecycle::{LifecycleLease, RunMetadata};
use vibe_wire::generated::lifecycle::e1::context::RunAgentMode;

/// One explicit script contribution, as the package `owner` (`group/name`)
/// declares it for its own slot: without the selector a slot contribution
/// answers for every package of the install.
pub fn script_row(owner: &str, id: &str, point: &str, base: &str) -> String {
    format!(
        "\n[[extension]]\nid='{id}'\npoint='slot:{point}'\nhandler={{kind='script',base='{base}'}}\napplies_to={{packages=['{owner}']}}\n"
    )
}

/// Apply `planned` for the project at `root`, running its slot contributions
/// with consent already given and their output discarded.
pub fn apply<S: InstallSource + ?Sized>(
    source: &S,
    root: &Path,
    planned: PlannedInstall,
    slot_integrity: SlotIntegrity,
) -> Result<ApplyReport, vibe_install::Error> {
    apply_with_spec_format(source, root, planned, slot_integrity, SpecFormat::Mixed)
}

/// [`apply`] with the failure underneath the lifecycle wrapper, for a test
/// that matches on the install error itself.
pub fn apply_unwrapped<S: InstallSource + ?Sized>(
    source: &S,
    root: &Path,
    planned: PlannedInstall,
    slot_integrity: SlotIntegrity,
) -> Result<ApplyReport, vibe_install::Error> {
    apply(source, root, planned, slot_integrity).map_err(inner_failure)
}

/// [`apply`] writing spec files in `spec_format`.
pub fn apply_with_spec_format<S: InstallSource + ?Sized>(
    source: &S,
    root: &Path,
    planned: PlannedInstall,
    slot_integrity: SlotIntegrity,
    spec_format: SpecFormat,
) -> Result<ApplyReport, vibe_install::Error> {
    let metadata = RunMetadata {
        requested: "install".into(),
        chain: vec!["validate".into(), "install".into()],
        offline: true,
        assume_yes: true,
        agent_mode: RunAgentMode::Cli,
        force: false,
        trace_compile: false,
        run_id: vibe_lifecycle::process::allocate_run_id(root).unwrap(),
        started: "2026-11-10T00:00:00Z".into(),
        selected: ".".into(),
    };
    vibe_install::apply_with_spec_format_and_lifecycle_observed(
        source,
        planned,
        slot_integrity,
        spec_format,
        metadata,
        StreamMode::Null,
        SlotLifecycleSeams::refusing(),
        Arc::new(LifecycleLease::acquire(root).unwrap()),
    )
    // The run handle keeps the project lease; a test applies more than once.
    .map(|mut report| {
        report.lifecycle_run = None;
        report
    })
}

/// The failure a slot contribution caused, with the lifecycle wrapper and any
/// rollback wrapper removed, plus whether the rollback itself conflicted.
pub fn slot_failure(error: &vibe_install::Error) -> (String, bool) {
    let vibe_install::Error::SlotFailed {
        source, reports, ..
    } = error
    else {
        panic!("expected a slot failure, got {error}");
    };
    let failed = reports
        .iter()
        .find(|report| report.status == "fail")
        .unwrap_or_else(|| panic!("no failed slot row in {error}"));
    (
        failed.message.clone().unwrap_or_default(),
        matches!(**source, vibe_install::Error::RollbackFailed { .. }),
    )
}

/// Drop the lifecycle run record a slot run leaves under `<root>/.vibe`.
///
/// The engine keeps that record on purpose, failure included, so a test that
/// compares the project tree before and after a rolled-back apply sets it
/// aside first and then compares everything else byte for byte.
pub fn discard_run_record(root: &Path) {
    let record = root.join(".vibe");
    if record.exists() {
        std::fs::remove_dir_all(record).unwrap();
    }
}

/// The failure underneath the lifecycle wrapper, so a test can still tell a
/// clean rollback from a conflicting one.
pub fn inner_failure(error: vibe_install::Error) -> vibe_install::Error {
    match error {
        vibe_install::Error::SlotFailed { source, .. } => *source,
        other => other,
    }
}
