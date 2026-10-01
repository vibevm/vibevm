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
    ActivationConflictCheck, BootDirectoryCheck, BridgeProvenanceCheck, Check, CheckId,
    CheckOptions, CheckReport, DocMediaCheck, DocPackageContractCheck, DocTranslationCheck,
    FactsSyncCheck, FeaturesGraphCheck, I18nCoverageCheck, LocalSourceFreshnessCheck,
    LockfileFilesCheck, ManifestEpochCheck, ManifestValidityCheck, RedirectBlockCheck,
    ReviewAgingCheck, Severity, SnippetPresuppositionCheck, SubskillStructureCheck,
    VisibilityHygieneCheck, WalFreshnessCheck, WalWellformedCheck, all_checks, check_project,
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
/// `vibevm/vibespecs/boot/` with two markdown files, and a WAL carrying every
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
    fs::create_dir_all(root.join("vibevm/vibespecs/boot")).unwrap();
    fs::write(root.join("vibevm/vibespecs/boot/00-core.md"), "# core\n").unwrap();
    fs::write(root.join("vibevm/vibespecs/boot/90-user.md"), "# user\n").unwrap();
    let wal = root.join("vibevm/vibespecs/WAL.md");
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

#[test]
fn declared_host_boot_checks_only_selected_vibe_not_raw_same_stem_data() {
    let project = tempdir().unwrap();
    write_minimal_project(project.path());
    let boot = vibe_core::layout::current_boot_dir();
    let wire = |name: &str| boot.join(name).to_string_lossy().replace('\\', "/");
    fs::write(project.path().join("vibe.toml"), format!(
        "[project]\ngroup='org.example'\nname='host'\nversion='1.1.0'\n\
         [[project.sources]]\nfile='{}'\nmedia='application/xml'\ninterpretation='xml-data'\nrole='resource'\nloading='scan'\n\
         [[project.sources]]\nfile='{}'\nmedia='text/markdown'\ninterpretation='commonmark'\nrole='documentation'\nloading='scan'\n\
         [[project.sources]]\nfile='{}'\nmedia='application/xml'\ninterpretation='vibe-spec'\nrole='spec'\nloading='scan'\n\
         [[project.sources]]\nfile='{}'\nmedia='application/xml'\ninterpretation='vibe-doc'\nrole='documentation'\nloading='scan'\n",
        wire("raw.xml"), wire("raw.md"), wire("selected.xml"), wire("doc.xml")
    )).unwrap();
    fs::write(
        project.path().join(boot.join("raw.xml")),
        b"<data><ordinary/></data>",
    )
    .unwrap();
    fs::write(
        project.path().join(boot.join("raw.md")),
        b"# ordinary\n\n#use is literal\n",
    )
    .unwrap();
    let selected = project.path().join(boot.join("selected.xml"));
    fs::write(
        &selected,
        b"<spec xmlns='https://vibevm.org/spec/1'><p><RULE fact='true'>selected</RULE></p></spec>",
    )
    .unwrap();
    fs::write(
        project.path().join(boot.join("doc.xml")),
        b"<spec xmlns='https://vibevm.org/spec/1'><note kind='tip'>Doc-only element.</note></spec>",
    )
    .unwrap();
    let run = || {
        let mut report = CheckReport::default();
        BootDirectoryCheck.run(project.path(), &opts(), &mut report);
        report.findings
    };
    let clean = run();
    assert!(
        clean.is_empty(),
        "ordinary same-stem sources stay raw: {clean:?}"
    );

    fs::write(&selected, b"<data><foreign/></data>").unwrap();
    let invalid = run();
    assert!(
        invalid
            .iter()
            .any(|finding| finding.check == CheckId::BootDirectory
                && finding.severity == Severity::Error
                && finding
                    .message
                    .contains("selected Vibe boot source is invalid")),
        "{invalid:?}"
    );
    fs::write(
        &selected,
        b"<spec xmlns='https://vibevm.org/spec/1'><note kind='tip'>Wrong vocabulary.</note></spec>",
    )
    .unwrap();
    let wrong_vocabulary = run();
    assert!(
        wrong_vocabulary
            .iter()
            .any(|finding| finding.message.contains("documentation vocabulary")),
        "{wrong_vocabulary:?}"
    );
}

#[cfg(unix)]
#[test]
fn declared_host_boot_refuses_symlinked_or_ambiguous_selected_source() {
    use std::os::unix::fs::symlink;

    let project = tempdir().unwrap();
    let outside = tempdir().unwrap();
    write_minimal_project(project.path());
    let boot = vibe_core::layout::current_boot_dir();
    let selected_rel = boot
        .join("selected.xml")
        .to_string_lossy()
        .replace('\\', "/");
    let row = format!(
        "[[project.sources]]\nfile='{selected_rel}'\nmedia='application/xml'\ninterpretation='vibe-spec'\nrole='spec'\nloading='scan'\n"
    );
    let manifest = format!("[project]\ngroup='org.example'\nname='host'\nversion='1.1.0'\n{row}");
    fs::write(project.path().join("vibe.toml"), &manifest).unwrap();
    let selected = project.path().join(&selected_rel);
    fs::write(outside.path().join("selected.xml"), b"<spec/>").unwrap();
    symlink(outside.path().join("selected.xml"), &selected).unwrap();
    let mut report = CheckReport::default();
    BootDirectoryCheck.run(project.path(), &opts(), &mut report);
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.message.contains("cannot be classified")),
        "{:?}",
        report.findings
    );

    fs::remove_file(&selected).unwrap();
    fs::write(&selected, b"<spec xmlns='https://vibevm.org/spec/1'/>").unwrap();
    fs::write(project.path().join("vibe.toml"), format!("{manifest}[[project.sources]]\nfile='{selected_rel}'\nmedia='application/xml'\ninterpretation='xml-data'\nrole='resource'\nloading='scan'\n")).unwrap();
    let mut report = CheckReport::default();
    BootDirectoryCheck.run(project.path(), &opts(), &mut report);
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.message.contains("ambiguous descriptors")),
        "{:?}",
        report.findings
    );
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
        (
            Box::new(LocalSourceFreshnessCheck),
            CheckId::LocalSourceFreshness,
        ),
        (Box::new(ManifestEpochCheck), CheckId::ManifestEpoch),
        (Box::new(FactsSyncCheck), CheckId::FactsSync),
        (
            Box::new(SnippetPresuppositionCheck),
            CheckId::SnippetPresupposition,
        ),
        (Box::new(VisibilityHygieneCheck), CheckId::VisibilityHygiene),
        (Box::new(BridgeProvenanceCheck), CheckId::BridgeProvenance),
        (
            Box::new(DocPackageContractCheck),
            CheckId::DocPackageContract,
        ),
        (Box::new(DocTranslationCheck), CheckId::DocTranslation),
        (Box::new(DocMediaCheck), CheckId::DocMedia),
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
            CheckId::LocalSourceFreshness,
            CheckId::ManifestEpoch,
            CheckId::FactsSync,
            CheckId::SnippetPresupposition,
            CheckId::VisibilityHygiene,
            CheckId::BridgeProvenance,
            CheckId::DocPackageContract,
            CheckId::DocTranslation,
            CheckId::DocMedia,
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

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-GRAMMAR")]
fn manifest_validity_accepts_a_builtin_log_extension() {
    let project = tempdir().unwrap();
    fs::write(
        project.path().join("vibe.toml"),
        r#"[project]
name = "demo"
version = "0.1.0"

[[extension]]
id = "announce"
point = "phase:build"
handler = { kind = "builtin", name = "log" }
config = { message = "hello from {phase}" }
"#,
    )
    .unwrap();

    let mut report = CheckReport::default();
    ManifestValidityCheck.run(project.path(), &opts(), &mut report);
    assert!(report.findings.is_empty(), "got: {:?}", report.findings);
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-GRAMMAR")]
fn manifest_validity_names_an_unknown_nested_extension_field() {
    let project = tempdir().unwrap();
    fs::write(
        project.path().join("vibe.toml"),
        r#"[project]
name = "demo"
version = "0.1.0"

[[extension]]
id = "announce"
point = "phase:build"
handler = { kind = "builtin", name = "log", mystery = true }
"#,
    )
    .unwrap();

    let mut report = CheckReport::default();
    ManifestValidityCheck.run(project.path(), &opts(), &mut report);
    assert_eq!(report.findings.len(), 1, "got: {:?}", report.findings);
    let message = &report.findings[0].message;
    assert!(message.contains("unknown field"), "{message}");
    assert!(message.contains("mystery"), "{message}");
}
