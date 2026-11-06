//! Check 6 — WAL has the canonical sections (Current Phase,
//! Constraints, Done, Next, Issues). The WAL is a spec source and lives
//! in either PROP-045 serialisation — `vibevm/vibespecs/WAL.xml` or
//! `vibevm/vibespecs/WAL.md` —
//! one document, one form; an XML WAL is read through its canonical
//! Markdown projection, so the heading scan is form-blind.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#linter");

use std::path::{Path, PathBuf};

use specmark::cell;

use crate::{Check, CheckId, CheckOptions, CheckReport};

/// How a project's WAL resolves under the one-document-one-form law.
pub(crate) enum WalResolution {
    /// No WAL in either form — the project has not opted into the
    /// convention; never a finding.
    Absent,
    /// Exactly one serialisation, at this project-relative path.
    One(PathBuf),
    /// Both serialisations present — a split brain to report loudly.
    Pair { md: PathBuf, xml: PathBuf },
}

/// Resolve the WAL (`current_wal_md()` / `current_wal_xml()`) for
/// `project_root`.
pub(crate) fn resolve_wal(project_root: &Path) -> WalResolution {
    let md = vibe_core::layout::current_wal_md();
    let xml = vibe_core::layout::current_wal_xml();
    match (
        project_root.join(&md).is_file(),
        project_root.join(&xml).is_file(),
    ) {
        (true, true) => WalResolution::Pair { md, xml },
        (true, false) => WalResolution::One(md),
        (false, true) => WalResolution::One(xml),
        (false, false) => WalResolution::Absent,
    }
}

const WAL_REQUIRED_SECTIONS: &[&str] = &[
    "current phase",
    "constraints",
    "done",
    "next",
    "known issues",
];

/// The [`CheckId::WalWellformed`] cell.
#[cell(seam = "Check", variant = "wal-wellformed")]
pub struct WalWellformedCheck;

impl Check for WalWellformedCheck {
    fn id(&self) -> CheckId {
        CheckId::WalWellformed
    }

    fn run(&self, project_root: &Path, _opts: &CheckOptions, report: &mut CheckReport) {
        use vibe_spec::source_policy::{PackageSourceMode, observe_package_policy};
        let policy = match observe_package_policy(project_root) {
            Ok(policy) => policy,
            Err(error) => {
                report.err(self.id(), Some("vibe.toml".into()), None, error.to_string());
                return;
            }
        };
        match policy.mode() {
            PackageSourceMode::ExplicitEmpty => return,
            PackageSourceMode::Explicit => {
                report.err(
                    self.id(),
                    Some("vibe.toml".into()),
                    None,
                    policy.unsupported("WAL wellformedness").to_string(),
                );
                return;
            }
            PackageSourceMode::MissingManifest | PackageSourceMode::Legacy => {}
        }
        let wal_rel = match resolve_wal(project_root) {
            WalResolution::Absent => {
                // WAL discipline is a project convention, not part of the
                // package manager's contract. A project that hasn't opted
                // in simply has no WAL — that's not a finding. The
                // well-formedness check only fires once the file exists
                // and the operator has implicitly committed to it.
                return;
            }
            WalResolution::Pair { md, xml } => {
                report.err(
                    CheckId::WalWellformed,
                    Some(md.clone()),
                    None,
                    format!(
                        "`{}` and `{}` are one logical document in two forms — one \
                         document, one form (PROP-045); delete one of the pair",
                        md.display(),
                        xml.display()
                    ),
                );
                return;
            }
            WalResolution::One(rel) => rel,
        };
        let wal = project_root.join(&wal_rel);
        let body = match vibe_specdoc::load_spec_text(&wal) {
            Ok((s, _kind)) => s,
            Err(e) => {
                report.err(
                    CheckId::WalWellformed,
                    Some(wal_rel),
                    None,
                    format!("could not read WAL: {e}"),
                );
                return;
            }
        };
        if let Err(error) = policy.recheck_unchanged() {
            report.err(self.id(), Some("vibe.toml".into()), None, error.to_string());
            return;
        }
        // Collect every top-level (`## …`) section heading, lowercased
        // and trimmed to the first parenthesis to make the matching
        // resilient to suffixes like `(do not violate without discussion)`.
        let headings: Vec<String> = body
            .lines()
            .filter_map(|line| line.strip_prefix("## "))
            .map(|h| {
                let trimmed = h.trim().to_ascii_lowercase();
                // Drop everything from the first `(` so "constraints (do
                // not violate)" matches the bare "constraints" required
                // section.
                match trimmed.find('(') {
                    Some(i) => trimmed[..i].trim().to_string(),
                    None => trimmed,
                }
            })
            .collect();
        for required in WAL_REQUIRED_SECTIONS {
            if !headings
                .iter()
                .any(|h| h == required || h.starts_with(&format!("{required} ")))
            {
                report.warn(
                    CheckId::WalWellformed,
                    Some(wal_rel.clone()),
                    None,
                    format!("WAL is missing the canonical `## {required}` section"),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Check;
    use std::fs;

    use tempfile::tempdir;
    use vibe_core::layout;

    use crate::test_support::{opts, write_minimal_project};
    use crate::{CheckId, check_project};

    #[test]
    fn source_policy_empty_skips_wal_pair_and_explicit_or_invalid_refuse() {
        for (suffix, expected) in [
            ("sources=[]\n", 0),
            ("sources='invalid'\n", 1),
            (
                "[[package.sources]]\nfile='raw.xml'\nmedia='application/xml'\ninterpretation='xml-data'\nrole='resource'\nloading='explicit'\n",
                1,
            ),
        ] {
            let tmp = tempdir().unwrap();
            fs::create_dir_all(tmp.path().join(layout::current_specs_root())).unwrap();
            fs::write(tmp.path().join("vibe.toml"), format!("[package]\nname='guard'\ngroup='org.example'\nkind='flow'\nversion='1.0.0'\n{suffix}")).unwrap();
            fs::write(tmp.path().join(layout::current_wal_xml()), "<foreign/>").unwrap();
            fs::write(tmp.path().join(layout::current_wal_md()), "raw").unwrap();
            let mut report = crate::CheckReport::default();
            super::WalWellformedCheck.run(tmp.path(), &opts(), &mut report);
            assert_eq!(report.findings.len(), expected, "{:?}", report.findings);
            for finding in report.findings {
                assert!(finding.message.contains("SOURCE-ADMISSION-BOUNDARY"));
            }
            assert_eq!(
                fs::read_to_string(tmp.path().join(layout::current_wal_xml())).unwrap(),
                "<foreign/>"
            );
        }
    }

    #[test]
    fn wal_missing_sections_warn() {
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        fs::write(
            project.path().join(layout::current_wal_md()),
            "# WAL\n\n## Current phase\n\n(no other sections)\n",
        )
        .unwrap();
        let report = check_project(project.path(), &opts());
        let missing: Vec<&str> = report
            .findings
            .iter()
            .filter(|f| f.check == CheckId::WalWellformed)
            .map(|f| f.message.as_str())
            .collect();
        assert!(missing.iter().any(|m| m.contains("constraints")));
        assert!(missing.iter().any(|m| m.contains("done")));
        assert!(missing.iter().any(|m| m.contains("next")));
        assert!(missing.iter().any(|m| m.contains("known issues")));
    }

    #[test]
    fn wal_missing_is_not_an_error() {
        // Regression guard: WAL discipline is a project convention,
        // not part of the package manager's contract. A fresh
        // `vibe init`-ed project does NOT carry a WAL at all, and
        // `vibe check` against such a project must NOT produce a
        // WalWellformed finding. Past versions of this check
        // emitted `WAL is missing — every project carries one`,
        // which conflated this repo's convention with the tool's
        // contract.
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        // Remove the WAL that `write_minimal_project` writes — we
        // want the no-WAL state.
        let wal = project.path().join(layout::current_wal_md());
        if wal.exists() {
            fs::remove_file(&wal).unwrap();
        }
        let report = check_project(project.path(), &opts());
        assert!(
            report
                .findings
                .iter()
                .all(|f| f.check != CheckId::WalWellformed && f.check != CheckId::WalFreshness),
            "missing WAL must produce no WAL findings; got: {:?}",
            report.findings
        );
    }

    #[test]
    fn an_xml_wal_is_checked_through_its_projection() {
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        let wal = project.path().join(layout::current_wal_md());
        if wal.exists() {
            fs::remove_file(&wal).unwrap();
        }
        let md = "# WAL {#root}\n\n## Current phase {#phase}\n\n(no other sections)\n";
        let xml = vibe_specdoc::to_xml(&vibe_specdoc::from_markdown(md).unwrap());
        fs::write(project.path().join(layout::current_wal_xml()), xml).unwrap();
        let report = check_project(project.path(), &opts());
        let missing: Vec<&str> = report
            .findings
            .iter()
            .filter(|f| f.check == CheckId::WalWellformed)
            .map(|f| f.message.as_str())
            .collect();
        assert!(
            missing.iter().any(|m| m.contains("constraints")),
            "{missing:?}"
        );
        assert!(
            missing.iter().any(|m| m.contains("known issues")),
            "{missing:?}"
        );
    }

    #[test]
    fn a_wal_pair_is_a_loud_split_brain() {
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        fs::write(project.path().join(layout::current_wal_md()), "# WAL\n").unwrap();
        let xml = vibe_specdoc::to_xml(&vibe_specdoc::from_markdown("# WAL {#root}\n").unwrap());
        fs::write(project.path().join(layout::current_wal_xml()), xml).unwrap();
        let report = check_project(project.path(), &opts());
        assert!(
            report.findings.iter().any(|f| {
                f.check == CheckId::WalWellformed && f.message.contains("one document, one form")
            }),
            "got: {:?}",
            report.findings
        );
    }

    #[test]
    fn wal_constraint_heading_with_parenthetical_suffix_matches() {
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        // The real project's WAL uses `## Constraints (do not violate without discussion)`.
        fs::write(
            project.path().join(layout::current_wal_md()),
            "# WAL\n\n## Current phase\n\n## Constraints (do not violate without discussion)\n\n## Done\n\n## Next\n\n## Known issues\n",
        )
        .unwrap();
        let report = check_project(project.path(), &opts());
        // No WalWellformed findings — every required section matched.
        assert!(
            report
                .findings
                .iter()
                .all(|f| f.check != CheckId::WalWellformed),
            "got: {:?}",
            report.findings
        );
    }
}
