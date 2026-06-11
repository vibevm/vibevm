//! The check-cell oracle (conform `cell-has-oracle`, card scaffold-d):
//! every `#[cell]`-manifested check type is driven from its own
//! crate's integration tests, through the [`Check`] seam, against
//! real on-disk fixture projects with predictable findings.
//!
//! Two fixtures, two known answers:
//! - an *empty* directory yields exactly one finding — the
//!   `ManifestValidity` error (no `vibe.toml`);
//! - a *minimal clean* project (manifest + boot dir + sectioned WAL
//!   with pinned mtime) yields zero findings.
//!
//! [`Check`]: vibe_check::Check

use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

use tempfile::tempdir;
use vibe_check::{
    ActivationConflictCheck, BootDirectoryCheck, Check, CheckId, CheckOptions, CheckReport,
    FeaturesGraphCheck, I18nCoverageCheck, LockfileFilesCheck, ManifestValidityCheck,
    RedirectBlockCheck, ReviewAgingCheck, Severity, SubskillStructureCheck, WalFreshnessCheck,
    WalWellformedCheck, all_checks, check_project,
};

/// 2026-05-04T12:00:00Z — a frozen clock, so freshness / aging math
/// never depends on when the suite runs.
fn fixed_now() -> u64 {
    vibe_core::timestamp::parse_unix_utc("2026-05-04T12:00:00Z").unwrap()
}

fn opts() -> CheckOptions {
    CheckOptions {
        now_unix_utc: Some(fixed_now()),
        ..Default::default()
    }
}

/// The same minimal clean tree the unit suite uses: `vibe.toml`,
/// `spec/boot/` with two markdown files, and a WAL carrying every
/// canonical section, its mtime pinned 1h before [`fixed_now`].
fn write_minimal_project(root: &Path) {
    fs::write(
        root.join("vibe.toml"),
        r#"[project]
name = "demo"
version = "0.0.1"

[[registry]]
name = "vibespecs"
url = "https://example/vibespecs"
"#,
    )
    .unwrap();
    fs::create_dir_all(root.join("spec/boot")).unwrap();
    fs::write(root.join("spec/boot/00-core.md"), "# core\n").unwrap();
    fs::write(root.join("spec/boot/90-user.md"), "# user\n").unwrap();
    let wal = root.join("spec/WAL.md");
    fs::write(
        &wal,
        "# WAL\n\n## Current phase\n\n## Constraints\n\n## Done\n\n## Next\n\n## Known issues\n",
    )
    .unwrap();
    let one_hour_before_fixed_now =
        SystemTime::UNIX_EPOCH + Duration::from_secs(fixed_now() - 3600);
    fs::OpenOptions::new()
        .write(true)
        .open(&wal)
        .unwrap()
        .set_modified(one_hour_before_fixed_now)
        .unwrap();
}

/// Every cell answers with its own [`CheckId`] — the manifest-to-id
/// mapping each `#[cell(variant = …)]` claims.
#[test]
fn each_cell_reports_its_own_check_id() {
    let cells: Vec<(Box<dyn Check>, CheckId)> = vec![
        (Box::new(ManifestValidityCheck), CheckId::ManifestValidity),
        (Box::new(WalFreshnessCheck), CheckId::WalFreshness),
        (Box::new(WalWellformedCheck), CheckId::WalWellformed),
        (Box::new(BootDirectoryCheck), CheckId::BootDirectory),
        (Box::new(RedirectBlockCheck), CheckId::RedirectBlock),
        (Box::new(LockfileFilesCheck), CheckId::LockfileFiles),
        (Box::new(ReviewAgingCheck), CheckId::ReviewAging),
        (Box::new(FeaturesGraphCheck), CheckId::FeaturesGraph),
        (Box::new(SubskillStructureCheck), CheckId::SubskillStructure),
        (Box::new(I18nCoverageCheck), CheckId::I18nCoverage),
        (
            Box::new(ActivationConflictCheck),
            CheckId::ActivationConflict,
        ),
    ];
    assert_eq!(cells.len(), CheckId::all().len(), "one cell per CheckId");
    for (cell, expected) in &cells {
        assert_eq!(cell.id(), *expected);
    }
}

/// The registration table is the dispatch order `check_project`
/// hardcoded before the seam existed — finding order is observable
/// output and must not drift.
#[test]
fn all_checks_registers_every_cell_in_dispatch_order() {
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
        ]
    );
}

/// An empty directory has exactly one defect every run can predict:
/// no `vibe.toml`. Both the composition root and the lone
/// `ManifestValidityCheck` cell must surface it — and nothing else.
#[test]
fn empty_project_yields_exactly_the_manifest_validity_error() {
    let project = tempdir().unwrap();

    let report = check_project(project.path(), &opts());
    assert!(report.has_errors());
    assert_eq!(
        report.count(Severity::Error),
        1,
        "got: {:?}",
        report.findings
    );
    assert_eq!(report.findings.len(), 1, "got: {:?}", report.findings);
    let finding = &report.findings[0];
    assert_eq!(finding.check, CheckId::ManifestValidity);
    assert_eq!(finding.severity, Severity::Error);
    assert!(finding.message.contains("vibe.toml"), "got: {finding:?}");

    // The same answer through the seam, one cell at a time: only the
    // manifest cell finds anything in an empty tree.
    for cell in all_checks() {
        let mut solo = CheckReport::default();
        cell.run(project.path(), &opts(), &mut solo);
        let expected = if cell.id() == CheckId::ManifestValidity {
            1
        } else {
            0
        };
        assert_eq!(
            solo.findings.len(),
            expected,
            "cell {:?} on an empty dir; got: {:?}",
            cell.id(),
            solo.findings
        );
    }
}

/// The minimal clean fixture is silent through the composition root
/// AND through every cell driven individually via the seam.
#[test]
fn minimal_project_is_clean_through_the_seam() {
    let project = tempdir().unwrap();
    write_minimal_project(project.path());

    let report = check_project(project.path(), &opts());
    assert_eq!(report.findings.len(), 0, "got: {:?}", report.findings);

    for cell in all_checks() {
        let mut solo = CheckReport::default();
        cell.run(project.path(), &opts(), &mut solo);
        assert_eq!(
            solo.findings.len(),
            0,
            "cell {:?} flagged the clean fixture; got: {:?}",
            cell.id(),
            solo.findings
        );
    }
}
