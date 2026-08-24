//! Adoption-facts registry schema and host-spec synchronization gate.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-046#laws");

use std::path::Path;

use specmark::cell;
use vibe_facts::{Registry, sync};

use crate::{Check, CheckId, CheckOptions, CheckReport};

/// The [`CheckId::FactsSync`] cell.
#[cell(seam = "Check", variant = "facts-sync")]
pub struct FactsSyncCheck;

impl Check for FactsSyncCheck {
    fn id(&self) -> CheckId {
        CheckId::FactsSync
    }

    fn run(&self, project_root: &Path, _opts: &CheckOptions, report: &mut CheckReport) {
        let home = project_root.join(vibe_core::layout::current_vibefacts_root());
        if !home.exists() {
            return;
        }
        let registry = match Registry::load(project_root) {
            Ok(registry) => registry,
            Err(error) => {
                report.err(
                    CheckId::FactsSync,
                    Some(vibe_core::layout::current_vibefacts_root()),
                    None,
                    format!("invalid adoption-facts registry: {error}"),
                );
                return;
            }
        };
        let mismatches = match sync::check(project_root, &registry) {
            Ok(mismatches) => mismatches,
            Err(error) => {
                report.err(
                    CheckId::FactsSync,
                    Some(vibe_core::layout::current_vibefacts_root()),
                    None,
                    format!("could not compare adoption facts with spec markers: {error}"),
                );
                return;
            }
        };
        for mismatch in mismatches {
            let message = format!(
                "fact `{}` is out of sync: spec status `{}`, registry status `{}`; \
                 spec is authoritative — run `vibe facts sync --write`",
                mismatch.address,
                mismatch.spec_status_text(),
                mismatch.registry_status_text(),
            );
            report.err(CheckId::FactsSync, mismatch.path, mismatch.line, message);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;
    use vibe_core::layout;

    use crate::test_support::opts;
    use crate::{Check, CheckId, CheckReport};

    use super::FactsSyncCheck;

    fn fixture(registry_status: &str) -> tempfile::TempDir {
        let project = tempdir().expect("tempdir");
        fs::write(
            project.path().join("vibe.toml"),
            "[project]\ngroup = \"org.example\"\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )
        .expect("manifest");
        fs::create_dir_all(
            project
                .path()
                .join(layout::current_specs_root())
                .join("common"),
        )
        .expect("spec dir");
        fs::write(
            project
                .path()
                .join(layout::current_specs_root())
                .join("common/RULE.md"),
            "# Rule {#root}\n\n@fact:RULE The rule. @status:impl/done\n",
        )
        .expect("spec");
        fs::create_dir_all(project.path().join(layout::current_vibefacts_root()))
            .expect("facts dir");
        fs::write(
            project
                .path()
                .join(layout::current_vibefacts_root())
                .join("spec.toml"),
            format!(
                "schema = 1\n\n[[fact]]\naddress = \
                 \"spec://org.example/demo/common/RULE#RULE\"\norigin = \"spec\"\nstatus = \
                 \"{registry_status}\"\n"
            ),
        )
        .expect("registry");
        project
    }

    #[test]
    fn matching_registry_is_clean() {
        let project = fixture("impl/done");
        let mut report = CheckReport::default();
        FactsSyncCheck.run(project.path(), &opts(), &mut report);
        assert!(report.findings.is_empty(), "got: {:?}", report.findings);
    }

    #[test]
    fn mismatch_is_an_error_with_repair_direction() {
        let project = fixture("spec/work");
        let mut report = CheckReport::default();
        FactsSyncCheck.run(project.path(), &opts(), &mut report);
        assert_eq!(report.findings.len(), 1, "got: {:?}", report.findings);
        let finding = &report.findings[0];
        assert_eq!(finding.check, CheckId::FactsSync);
        assert_eq!(finding.severity, crate::Severity::Error);
        assert!(finding.message.contains("impl/done"));
        assert!(finding.message.contains("spec/work"));
        assert!(finding.message.contains("facts sync --write"));
    }
}
