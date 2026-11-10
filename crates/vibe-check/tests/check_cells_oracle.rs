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
    ActivationConflictCheck, ApplicationIconsCheck, BootDirectoryCheck, BridgeProvenanceCheck,
    Check, CheckId, CheckOptions, CheckReport, DocMediaCheck, DocPackageContractCheck,
    DocTranslationCheck, FactsSyncCheck, FeaturesGraphCheck, I18nCoverageCheck,
    LocalSourceFreshnessCheck, LockSlotHashCheck, LockfileFilesCheck, ManifestEpochCheck,
    ManifestValidityCheck, RedirectBlockCheck, ReviewAgingCheck, Severity,
    SnippetPresuppositionCheck, SubskillStructureCheck, SystemPromptCheck, TemplateRenderCheck,
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
        (Box::new(ApplicationIconsCheck), CheckId::ApplicationIcons),
        (Box::new(LockSlotHashCheck), CheckId::LockSlotHash),
        (Box::new(TemplateRenderCheck), CheckId::TemplateRender),
        (Box::new(SystemPromptCheck), CheckId::SystemPrompt),
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
            CheckId::ApplicationIcons,
            CheckId::LockSlotHash,
            CheckId::TemplateRender,
            CheckId::SystemPrompt,
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

/// The template cell's own oracle (PROP-075 §2): a sound template yields
/// nothing, and a template whose skeleton render is not a manifest yields
/// one error naming the field the render produced.
#[test]
fn template_render_cell_judges_a_template_by_rendering_it() {
    let sound = concat!(
        "[package]\nname = \"tool\"\ngroup = \"org.example\"\nkind = \"app\"\n",
        "version = \"1.0.0\"\nepoch = 1\ntemplate = true\n\n",
        "[application]\nid = \"tool\"\n\n",
        "[template.versions]\nkind = \"list\"\n\n",
        "[[template.versions.item]]\nversion = \"3.1.4\"\n\n",
        "[template.digests]\nkind = \"sha256sums\"\n",
        "url = \"https://e.invalid/{version}/SHASUMS256.txt\"\n\n",
        "[[installer_options]]\nid = \"linux-x64\"\nname = \"tool {version}\"\n",
        "platform = \"linux\"\nplatform_name = \"Linux\"\narch = \"x86_64\"\n",
        "path_dirs = [\"bin\"]\n\n",
        "[[installer_options.steps]]\nid = \"unpack\"\ntitle = \"Unpack {version}\"\n",
        "kind = \"archive\"\nformat = \"tar_gz\"\n",
        "source = \"https://e.invalid/tool-{version}-{platform}.tar.gz\"\n",
        "sha256 = \"{sha256:tool-{version}-{platform}.tar.gz}\"\n",
    );
    let cell = TemplateRenderCheck;
    assert_eq!(cell.id(), CheckId::TemplateRender);

    let clean = tempdir().unwrap();
    write_minimal_project(clean.path());
    fs::write(clean.path().join("vibe.toml"), sound).unwrap();
    let mut report = CheckReport::default();
    cell.run(clean.path(), &opts(), &mut report);
    assert!(report.findings.is_empty(), "got: {:?}", report.findings);

    let broken = tempdir().unwrap();
    write_minimal_project(broken.path());
    fs::write(
        broken.path().join("vibe.toml"),
        sound.to_string() + "payload_root = \"/absolute/{version}\"\n",
    )
    .unwrap();
    let mut report = CheckReport::default();
    cell.run(broken.path(), &opts(), &mut report);
    assert_eq!(
        report.count(Severity::Error),
        1,
        "got: {:?}",
        report.findings
    );
    let finding = &report.findings[0];
    assert_eq!(finding.check, CheckId::TemplateRender);
    assert_eq!(
        finding.field.as_deref(),
        Some("installer_options[0].steps[0].payload_root"),
        "{}",
        finding.message
    );
    assert!(finding.message.contains("3.1.4"), "{}", finding.message);
}
