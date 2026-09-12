//! The `doc` package's own contract — what documentation must ship and
//! what it may never declare
//! ([PROP-057](../../../../vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml)
//! §3).
//!
//! Three rules, one idea: **documentation is read, not run.** A `doc`
//! package is never installed, never boots, never dispatches a binary
//! and never serves an MCP tool (`##KIND-DOC-MUST-NOT-EXECUTE`); it
//! ships a front door a human can open without any of that
//! (`##KIND-DOC-MUST-DOCUMENT` — the README is where a reader who found
//! the repository rather than the site lands). The fourth finding is a
//! warning and belongs to the other end of the edge: a subject naming
//! its `primary` documentation in a FOREIGN group is legal and
//! sometimes right, but it is also how a typo looks, so it is said out
//! loud once (`##REL-DOCUMENTATION-UNVERSIONED`).
//!
//! **What this cell deliberately does NOT restate.** `[[documents]]`,
//! `title` and `abstract` are refused by the manifest grammar itself
//! (`Manifest::validate`, PROP-057 `##REL-DOCUMENTS-REQUIRED`,
//! `##CARD-TITLE`, `##CARD-DESCRIPTION-AND-ABSTRACT`), so a `doc`
//! package missing any of them does not parse and `vibe check` already
//! errors on it through `manifest_validity`, citing the same anchors and
//! the fix. Re-deriving those three here would mean a second copy of the
//! manifest grammar living in the linter — the exact rot the index
//! scanner's own history records — for a message the user already gets.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#kinds");

use std::path::Path;

use specmark::cell;
use vibe_core::PackageKind;
use vibe_core::manifest::Manifest;

use crate::{Check, CheckId, CheckOptions, CheckReport};

/// The [`CheckId::DocPackageContract`] cell.
#[cell(seam = "Check", variant = "doc-package-contract")]
pub struct DocPackageContractCheck;

/// The front door of a package tree, and the one file this cell asks a
/// `doc` package for by name.
const README: &str = "README.md";

/// Read the project's own manifest, or `None` when there is none or it
/// is refused. A refused manifest is `manifest_validity`'s finding, not
/// this cell's: reporting a doc-contract defect on a manifest nobody
/// could read would be guessing at bytes.
fn manifest_of(project_root: &Path) -> Option<Manifest> {
    Manifest::read(project_root.join(Manifest::FILENAME)).ok()
}

/// The group half of a `<group>/<name>` coordinate.
fn group_of(coordinate: &str) -> Option<&str> {
    coordinate.split_once('/').map(|(group, _)| group)
}

impl Check for DocPackageContractCheck {
    fn id(&self) -> CheckId {
        CheckId::DocPackageContract
    }

    fn run(&self, project_root: &Path, _opts: &CheckOptions, report: &mut CheckReport) {
        let Some(manifest) = manifest_of(project_root) else {
            return;
        };
        let Some(package) = manifest.package.as_ref() else {
            return;
        };
        let manifest_path = Path::new(Manifest::FILENAME).to_path_buf();

        // The subject's end of the edge, checked in a package of ANY
        // kind: `[documentation]` is legal everywhere, and a `primary`
        // pointing outside the subject's own group is the shape a typo
        // takes. It stays a warning — a group may genuinely hand its
        // documentation to another publisher.
        if let Some(documentation) = manifest.documentation.as_ref()
            && let Some(primary) = documentation.primary.as_deref()
            && let Some(primary_group) = group_of(primary)
            && primary_group != package.group.as_str()
        {
            report.warn(
                CheckId::DocPackageContract,
                Some(manifest_path.clone()),
                None,
                format!(
                    "[documentation].primary names `{primary}`, published by `{primary_group}` \
                     rather than this package's own group `{own}` — legal, and sometimes \
                     deliberate, but this is also what a mistyped coordinate looks like \
                     (spec://org.vibevm.core/vibevm/common/PROP-057#REL-DOCUMENTATION-UNVERSIONED; \
                     fix: correct the coordinate, or keep it if the other group really does \
                     publish this package's primary documentation)",
                    own = package.group.as_str(),
                ),
            );
        }

        if package.kind != PackageKind::Doc {
            return;
        }

        // Documentation never enters a boot lane and never executes.
        // Each forbidden section is its own finding: an author who
        // declared two of them should be told about both, not led
        // through one recheck per section.
        let forbidden: [(&str, bool); 3] = [
            ("[boot_snippet]", manifest.boot_snippet.is_some()),
            ("[[binary]]", !manifest.binaries.is_empty()),
            ("[[mcp_server]]", !manifest.mcp_servers.is_empty()),
        ];
        for (section, present) in forbidden {
            if !present {
                continue;
            }
            report.err(
                CheckId::DocPackageContract,
                Some(manifest_path.clone()),
                None,
                format!(
                    "a `doc` package declares `{section}` — documentation is read, not run: it \
                     never enters a boot lane, dispatches a binary or serves a tool \
                     (violates \
                     spec://org.vibevm.core/vibevm/common/PROP-057#KIND-DOC-MUST-NOT-EXECUTE; \
                     fix: drop `{section}`, or ship the executable part as a package of its own \
                     kind and point at it with [documentation])"
                ),
            );
        }

        // The front door. A reader who arrived at the repository rather
        // than the site has nothing to open without it, and the
        // publication gate would refuse the package later anyway.
        if !project_root.join(README).is_file() {
            report.err(
                CheckId::DocPackageContract,
                Some(Path::new(README).to_path_buf()),
                None,
                format!(
                    "a `doc` package has no `{README}` — the pages live under `vibevm/vibespecs/` \
                     and the site renders them, but a reader who arrives at the repository needs \
                     a front door here \
                     (violates \
                     spec://org.vibevm.core/vibevm/common/PROP-057#KIND-DOC-MUST-DOCUMENT; \
                     fix: add `{README}` naming what this documentation covers and where its \
                     entry page is)"
                ),
            );
        }
    }
}

#[cfg(test)]
#[path = "doc_package_contract/tests.rs"]
mod tests;
