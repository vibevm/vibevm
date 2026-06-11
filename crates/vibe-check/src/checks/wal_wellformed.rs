//! Check 6 — WAL has the canonical sections (Current Phase,
//! Constraints, Done, Next, Issues).

specmark::scope!("spec://vibevm/VIBEVM-SPEC#linter");

use std::fs;
use std::path::{Path, PathBuf};

use specmark::cell;

use crate::{Check, CheckId, CheckOptions, CheckReport};

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
        let wal_rel = PathBuf::from("spec/WAL.md");
        let wal = project_root.join(&wal_rel);
        if !wal.exists() {
            // WAL discipline is a project convention, not part of the
            // package manager's contract. A project that hasn't opted in
            // simply has no `spec/WAL.md` — that's not a finding. The
            // well-formedness check only fires once the file exists and
            // the operator has implicitly committed to maintaining it.
            return;
        }
        let body = match fs::read_to_string(&wal) {
            Ok(s) => s,
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
    use std::fs;

    use tempfile::tempdir;

    use crate::test_support::{opts, write_minimal_project};
    use crate::{CheckId, check_project};

    #[test]
    fn wal_missing_sections_warn() {
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        fs::write(
            project.path().join("spec/WAL.md"),
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
        // `vibe init`-ed project does NOT carry `spec/WAL.md`, and
        // `vibe check` against such a project must NOT produce a
        // WalWellformed finding. Past versions of this check
        // emitted `WAL is missing — every project carries one`,
        // which conflated this repo's convention with the tool's
        // contract.
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        // Remove the WAL that `write_minimal_project` writes — we
        // want the no-WAL state.
        let wal = project.path().join("spec/WAL.md");
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
    fn wal_constraint_heading_with_parenthetical_suffix_matches() {
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        // The real project's WAL uses `## Constraints (do not violate without discussion)`.
        fs::write(
            project.path().join("spec/WAL.md"),
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
