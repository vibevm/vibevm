//! The crate root's own tests: a clean project stays clean, and the
//! registration order is pinned. Out of line to keep `lib.rs` within the
//! length budget.

use super::test_support::{opts, write_minimal_project};
use super::*;

use tempfile::tempdir;

#[test]
fn clean_minimal_project_has_no_findings() {
    let project = tempdir().unwrap();
    write_minimal_project(project.path());
    let report = check_project(project.path(), &opts());
    assert!(
        !report.has_errors(),
        "expected no errors; got: {:?}",
        report.findings
    );
    // WAL freshness may surface a warning since we set a future
    // `now` but the WAL was just written. Should be 0d old →
    // safely under the 24h threshold.
    assert_eq!(report.findings.len(), 0, "got: {:?}", report.findings);
}

#[test]
fn registration_matches_the_historical_dispatch_order() {
    // The seam refactor must not reorder findings: the report's
    // emission order is observable output, pinned to the order
    // `check_project` hardcoded before `all_checks` existed. A new
    // cell is APPENDED — never inserted — so the pin grows at its
    // tail and every existing position is untouched.
    let ids: Vec<CheckId> = all_checks().iter().map(|c| c.id()).collect();
    assert_eq!(
        ids,
        vec![
            CheckId::ManifestValidity,
            CheckId::WalFreshness,
            CheckId::WalWellformed,
            CheckId::BootDirectory,
            CheckId::RedirectBlock,
            CheckId::LockfileFiles,
            CheckId::ReviewAging,
            CheckId::FeaturesGraph,
            CheckId::SubskillStructure,
            CheckId::I18nCoverage,
            CheckId::ActivationConflict,
            CheckId::LocalSourceFreshness,
            CheckId::ManifestEpoch,
            CheckId::FactsSync,
            CheckId::SnippetPresupposition,
            CheckId::VisibilityHygiene,
            CheckId::BridgeProvenance,
            CheckId::DocPackageContract,
            CheckId::DocTranslation,
            CheckId::DocMedia,
            CheckId::ApplicationIcons,
            CheckId::LockSlotHash,
            CheckId::TemplateRender,
        ]
    );
}
