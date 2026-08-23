//! PROP-050 ##VERIFY-LINTS — the visibility hygiene cell. Every
//! diagnostic of the closure analysis over the INSTALLED world surfaces as
//! a warning: rejected allow-friends grants (the closure simply does not
//! grow there, per ##ALLOW-FRIENDS-CHECKPOINT), dead `friends` / `unfriend`
//! / `[override]` entries naming packages no chain ever met (the JPMS
//! qualified-export precedent — unknown targets warn, never fail), and
//! lockfile members whose slot manifest is missing so their declarations
//! are absent from the analysis. Advisory only: visibility drift is a
//! budget concern, not a broken build.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-050#verification");

use std::path::Path;

use specmark::cell;
use vibe_core::manifest::{Lockfile, OverrideTarget};
use vibe_core::visibility::{Diagnostic, analyze, load_installed_world};

use crate::{Check, CheckId, CheckOptions, CheckReport};

/// The [`CheckId::VisibilityHygiene`] cell.
///
/// A project without `vibe.lock` has no installed world and the cell is
/// silent. A world that cannot be loaded (missing root manifest section,
/// unreadable lock) yields ONE warning — the analysis is skipped, not
/// failed; `ManifestValidity` owns the hard parse errors.
///
/// ```
/// use std::path::Path;
/// use vibe_check::{Check, CheckId, CheckOptions, CheckReport, VisibilityHygieneCheck};
///
/// let mut report = CheckReport::default();
/// VisibilityHygieneCheck.run(Path::new("no-lock-anywhere"), &CheckOptions::default(), &mut report);
/// assert!(report.findings.is_empty());
/// ```
#[cell(seam = "Check", variant = "visibility-hygiene")]
pub struct VisibilityHygieneCheck;

impl Check for VisibilityHygieneCheck {
    fn id(&self) -> CheckId {
        CheckId::VisibilityHygiene
    }

    fn run(&self, project_root: &Path, _opts: &CheckOptions, report: &mut CheckReport) {
        if !project_root.join(Lockfile::FILENAME).is_file() {
            return; // No lock — no installed world to lint.
        }
        let world = match load_installed_world(project_root) {
            Ok(world) => world,
            Err(reason) => {
                report.warn(
                    CheckId::VisibilityHygiene,
                    None,
                    None,
                    format!("visibility analysis skipped: {reason}"),
                );
                return;
            }
        };
        let analysis = analyze(&world.graph, &world.root_id);
        for diagnostic in &analysis.diagnostics {
            report.warn(
                CheckId::VisibilityHygiene,
                None,
                None,
                diagnostic_text(diagnostic),
            );
        }
        for node in &world.unread {
            report.warn(
                CheckId::VisibilityHygiene,
                None,
                None,
                format!(
                    "`{node}` has no readable slot manifest under `vibedeps/` — its visibility \
                     declarations are missing from the analysis (run `vibe reinstall`)"
                ),
            );
        }
    }
}

/// The one-line human text for one analysis diagnostic.
fn diagnostic_text(diagnostic: &Diagnostic) -> String {
    match diagnostic {
        Diagnostic::RejectedGrant { from, to } => format!(
            "friendship grant from `{from}` to `{to}` is rejected by `{to}`'s allow-friends — \
             the friend closure does not grow through it (PROP-050 ##ALLOW-FRIENDS-CHECKPOINT)"
        ),
        Diagnostic::DeadOverrideEntry {
            declared_by,
            target,
        } => format!(
            "dead override entry on `{declared_by}`: target {} never met in any chain",
            override_target_text(target)
        ),
        Diagnostic::DeadFriendsEntry {
            declared_by,
            target,
        } => format!(
            "dead friends entry on `{declared_by}`: target `{target}` never met in any chain"
        ),
        Diagnostic::DeadUnfriendEntry {
            declared_by,
            target,
        } => format!(
            "dead unfriend entry on `{declared_by}`: target `{target}` never met in any chain"
        ),
    }
}

/// An `[override]` key as it was written: a node coordinate or an
/// `A -> B` edge coordinate.
fn override_target_text(target: &OverrideTarget) -> String {
    match target {
        OverrideTarget::Node(node) => format!("`{node}`"),
        OverrideTarget::Edge { from, to } => format!("`{from} -> {to}`"),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use tempfile::tempdir;

    use super::VisibilityHygieneCheck;
    use crate::test_support::{opts, write_minimal_project};
    use crate::{Check, CheckId, CheckReport, Severity};

    fn lock_header() -> String {
        format!(
            "[meta]\ngenerated_by = \"vibe-test\"\ngenerated_at = \"2026-08-23T00:00:00Z\"\nschema_version = {}\n",
            vibe_core::manifest::CURRENT_SCHEMA_VERSION
        )
    }

    fn cell_findings(project: &Path) -> Vec<crate::Finding> {
        let mut report = CheckReport::default();
        VisibilityHygieneCheck.run(project, &opts(), &mut report);
        report.findings
    }

    /// A dead `friends` entry (a target no chain ever met) surfaces as a
    /// warning — never an error, per the JPMS unknown-target precedent.
    #[test]
    fn diagnostics_surface_as_warnings() {
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        fs::write(
            project.path().join("vibe.toml"),
            "[project]\nname = \"demo\"\nversion = \"0.0.1\"\n\n[visibility]\nfriends = \
             [\"org.x/ghost\"]\n",
        )
        .unwrap();
        fs::write(project.path().join("vibe.lock"), lock_header()).unwrap();

        let findings = cell_findings(project.path());
        assert!(!findings.is_empty(), "the dead entry must surface");
        assert!(
            findings
                .iter()
                .all(|finding| finding.severity == Severity::Warning),
            "every visibility finding is a warning; got: {findings:?}"
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.check == CheckId::VisibilityHygiene
                    && finding.message.contains("dead friends entry on `demo`")),
            "the dead-friends warning must name the declarant; got: {findings:?}"
        );
    }

    /// A member whose slot manifest is missing on disk is a warning, and
    /// the rest of the world still analyses.
    #[test]
    fn unread_member_warns() {
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        fs::write(
            project.path().join("vibe.lock"),
            format!(
                "{}\n[[package]]\nkind = \"flow\"\ngroup = \"org.x\"\nname = \
                 \"ghost\"\nversion = \"1.0.0\"\nsource_url = \"file:///fake\"\ncontent_hash = \
                 \"sha256:00\"\nfiles_written = []\n",
                lock_header()
            ),
        )
        .unwrap();

        let findings = cell_findings(project.path());
        assert!(
            findings
                .iter()
                .any(|finding| finding.severity == Severity::Warning
                    && finding.message.contains("org.x/ghost")
                    && finding.message.contains("no readable slot manifest")),
            "got: {findings:?}"
        );
    }

    /// No `vibe.lock` — no installed world — the cell is clean.
    #[test]
    fn no_lock_is_clean() {
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        assert!(cell_findings(project.path()).is_empty());
    }

    /// A clean minimal world (lock with no members) is silent.
    #[test]
    fn clean_locked_world_is_silent() {
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        fs::write(project.path().join("vibe.lock"), lock_header()).unwrap();
        assert!(cell_findings(project.path()).is_empty());
    }
}
