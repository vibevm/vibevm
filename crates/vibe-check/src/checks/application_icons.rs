//! The icon files of an application package (`PROP-059#menu`): every file
//! its icon sets name exists in the package tree, a PNG is square and
//! matches the size it is listed under, and a launcher with `menu = true`
//! has an SVG or a PNG of at least 256 pixels.
//!
//! The rules live in `vibe-core` beside the manifest model they read
//! ([`vibe_core::manifest::icon_findings`]); this cell turns each finding
//! into a line of `vibe check` output.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#menu");

use std::path::{Path, PathBuf};

use specmark::cell;
use vibe_core::manifest::{Manifest, icon_findings};

use crate::{Check, CheckId, CheckOptions, CheckReport};

/// The [`CheckId::ApplicationIcons`] cell.
#[cell(seam = "Check", variant = "application-icons")]
pub struct ApplicationIconsCheck;

impl Check for ApplicationIconsCheck {
    fn id(&self) -> CheckId {
        CheckId::ApplicationIcons
    }

    fn run(&self, project_root: &Path, _opts: &CheckOptions, report: &mut CheckReport) {
        // An unreadable manifest is the manifest-validity cell's finding.
        let Ok(manifest) = Manifest::read(project_root.join(Manifest::FILENAME)) else {
            return;
        };
        if manifest.application.is_none() {
            return;
        }
        for finding in icon_findings(
            project_root,
            manifest.application.as_ref(),
            &manifest.installer_options,
        ) {
            // The icon's own field, so an editor can put the finding under it.
            report.push(crate::Finding {
                check: CheckId::ApplicationIcons,
                severity: crate::Severity::Error,
                path: finding.path.map(PathBuf::from),
                line: None,
                column: None,
                field: Some(finding.field.to_string()),
                package: None,
                message: finding.message,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn a_missing_icon_file_is_an_error() {
        let project = tempfile::tempdir().unwrap();
        fs::write(
            project.path().join("vibe.toml"),
            "[package]\nname = \"demo\"\ngroup = \"org.example\"\nkind = \"app\"\nversion = \"1.0.0\"\n\
             [application]\nid = \"demo\"\nicon = { svg = \"share/demo.svg\" }\n",
        )
        .unwrap();
        let mut report = CheckReport::default();
        ApplicationIconsCheck.run(project.path(), &CheckOptions::default(), &mut report);
        assert_eq!(report.findings.len(), 1, "{:?}", report.findings);
        assert!(report.findings[0].message.contains("share/demo.svg"));
        fs::create_dir_all(project.path().join("share")).unwrap();
        fs::write(project.path().join("share/demo.svg"), "<svg/>").unwrap();
        let mut report = CheckReport::default();
        ApplicationIconsCheck.run(project.path(), &CheckOptions::default(), &mut report);
        assert!(report.findings.is_empty(), "{:?}", report.findings);
    }
}
